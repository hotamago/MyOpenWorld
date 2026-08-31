//! Vòng lặp phát triển content pack (`plan.md §P10.7`, `§19.7.3`, `PF-03`).
//!
//! Ba lệnh, và lệnh thứ ba là lệnh nguy hiểm:
//!
//! ```text
//! mow-cli pack validate content/core   # kiểm manifest, namespace, quyền
//! mow-cli pack test content/core       # chạy scenario khai trong manifest
//! mow-cli pack watch content/core      # nạp lại nóng ở dev
//! ```
//!
//! ## Vì sao nạp nóng không được ghi đè tại chỗ
//!
//! > Nạp nóng chỉ được phép ở **dev build** và luôn đi qua đường
//! > migration/version của `§19.7.3` — **không có ghi đè định nghĩa tại chỗ**,
//! > vì như vậy world đang chạy sẽ **mất khả năng replay**.
//!
//! Lý do cụ thể: event log ghi *"dùng định nghĩa `core.apple` phiên bản 3"*.
//! Nếu nạp nóng thay nội dung của v3 tại chỗ thì replay cùng log đó sẽ ra kết
//! quả khác — và `INV-22-9` hỏng mà **không có gì báo**. Thế giới vẫn chạy,
//! save vẫn mở được, chỉ là hash không còn tái lập.
//!
//! Nên [`plan_reload`] không bao giờ sinh ra một thao tác "thay tại chỗ". Mọi
//! thay đổi nội dung thành một **version mới**, và định nghĩa cũ ở lại để
//! những event đã ghi vẫn diễn giải được.
//!
//! ## Ba loại thay đổi, ba cách xử lý
//!
//! | Thay đổi | Xử lý | Vì sao |
//! |---|---|---|
//! | thêm id mới | nạp thẳng | không event nào tham chiếu nó |
//! | sửa id đang có | **version mới**, giữ bản cũ | event cũ vẫn trỏ bản cũ |
//! | xóa id đang có | **từ chối**, hoặc đánh dấu tombstone | thế giới đang tham chiếu nó |
//!
//! Dòng cuối là dòng hay bị làm sai nhất: xóa một định nghĩa mà thế giới đang
//! dùng để lại những tham chiếu treo, và chúng lộ ra rải rác hàng giờ sau chứ
//! không lộ ra lúc nạp.

use crate::registry::{Registry, RegistryError, RegistryResult};
use mow_math::StateHash;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Build hiện tại có cho nạp nóng không.
///
/// Là một **kiểu**, không phải một cờ `bool` truyền quanh: một hàm nhận
/// `BuildKind` thì chỗ gọi phải nói ra mình đang ở build nào, còn một `bool`
/// tên `dev` thì rất dễ bị truyền nhầm `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildKind {
    /// Bản phát triển — nạp nóng được.
    Dev,
    /// Bản phát hành — **không** nạp nóng.
    Release,
}

impl BuildKind {
    /// Build hiện tại, suy từ `cfg!(debug_assertions)`.
    pub fn current() -> BuildKind {
        if cfg!(debug_assertions) {
            BuildKind::Dev
        } else {
            BuildKind::Release
        }
    }

    /// Nạp nóng được không.
    pub fn allows_hot_reload(self) -> bool {
        matches!(self, BuildKind::Dev)
    }
}

/// Ảnh chụp những gì một pack định nghĩa, tại một version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackSnapshot {
    /// Pack nào.
    pub pack: String,
    /// Version.
    pub version: String,
    /// Content hash.
    pub hash: StateHash,
    /// id → hash của định nghĩa đó.
    ///
    /// Băm từng định nghĩa chứ không chỉ băm cả pack: băm cả pack chỉ trả lời
    /// *"có gì đổi không"*, còn cái cần biết là **cái gì** đổi.
    pub definitions: BTreeMap<String, StateHash>,
}

/// Một thao tác trong kế hoạch nạp lại.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReloadStep {
    /// Thêm một định nghĩa chưa từng có.
    Add {
        /// Id.
        id: String,
    },
    /// Một định nghĩa đổi nội dung ⇒ **tạo version mới, giữ bản cũ**.
    ///
    /// Không có biến thể `Replace`. Đó là chủ đích, và nó là toàn bộ nội dung
    /// của module này.
    Supersede {
        /// Id.
        id: String,
        /// Version cũ, vẫn còn để replay.
        old_version: String,
        /// Version mới.
        new_version: String,
    },
    /// Một định nghĩa biến mất ⇒ đánh dấu tombstone, không xóa.
    ///
    /// Tombstone giữ đủ để một event cũ vẫn diễn giải được, và đủ để công cụ
    /// nói *"thứ này đã bị gỡ ở version 4"* thay vì *"không tìm thấy"*.
    Tombstone {
        /// Id.
        id: String,
        /// Gỡ ở version nào.
        removed_at: String,
    },
}

impl ReloadStep {
    /// Id bị ảnh hưởng.
    pub fn id(&self) -> &str {
        match self {
            ReloadStep::Add { id }
            | ReloadStep::Supersede { id, .. }
            | ReloadStep::Tombstone { id, .. } => id,
        }
    }

    /// Bước này có đụng tới thứ thế giới đang chạy tham chiếu không.
    pub fn touches_live_references(&self) -> bool {
        !matches!(self, ReloadStep::Add { .. })
    }
}

/// Kế hoạch nạp lại, chưa thi hành.
///
/// Trả về một **kế hoạch** chứ không tự làm: nạp nóng đổi thế giới đang chạy,
/// nên người phát triển phải đọc được nó sẽ làm gì trước khi nó làm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReloadPlan {
    /// Pack nào.
    pub pack: String,
    /// Từ version nào.
    pub from_version: String,
    /// Sang version nào.
    pub to_version: String,
    /// Các bước.
    pub steps: Vec<ReloadStep>,
}

impl ReloadPlan {
    /// Có gì để làm không.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Những id mà thế giới đang chạy có thể đang tham chiếu.
    pub fn affects(&self) -> Vec<&str> {
        self.steps
            .iter()
            .filter(|s| s.touches_live_references())
            .map(ReloadStep::id)
            .collect()
    }

    /// **Không có bước nào ghi đè tại chỗ.**
    ///
    /// Hàm này luôn trả `true` theo cấu trúc — [`ReloadStep`] không có biến thể
    /// nào làm chuyện đó. Nó tồn tại để một test khẳng định điều đó, và để
    /// nếu ai thêm biến thể `Replace` thì phải sửa cả đây và test.
    pub fn preserves_replay(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(
                s,
                ReloadStep::Add { .. }
                    | ReloadStep::Supersede { .. }
                    | ReloadStep::Tombstone { .. }
            )
        })
    }
}

/// Vì sao không nạp nóng được.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReloadError {
    /// Đang ở bản phát hành.
    #[error(
        "nạp nóng chỉ có ở dev build (§P10.7) — bản phát hành phải khởi động lại \
         để nội dung mới đi qua đúng đường nạp"
    )]
    NotDevBuild,
    /// Version không tăng.
    #[error(
        "pack `{pack}` đổi nội dung nhưng version vẫn là `{version}` — nội dung mới \
         phải mang version mới, không thì event đã ghi sẽ diễn giải ra kết quả khác"
    )]
    VersionNotBumped {
        /// Pack.
        pack: String,
        /// Version không đổi.
        version: String,
    },
}

/// Dựng kế hoạch nạp lại từ hai ảnh chụp (`§19.7.3`).
///
/// Không thi hành gì. Nó **không** nhận `&mut Registry` — một hàm dựng kế hoạch
/// mà sửa được sổ thì sớm muộn sẽ có người gọi nó để "chỉ xem thử" rồi phát
/// hiện thế giới đã đổi.
pub fn plan_reload(
    cu: &PackSnapshot,
    moi: &PackSnapshot,
    build: BuildKind,
) -> Result<ReloadPlan, ReloadError> {
    if !build.allows_hot_reload() {
        return Err(ReloadError::NotDevBuild);
    }
    if cu.hash != moi.hash && cu.version == moi.version {
        return Err(ReloadError::VersionNotBumped {
            pack: moi.pack.clone(),
            version: moi.version.clone(),
        });
    }

    let mut steps = Vec::new();
    let cac_id: BTreeSet<&String> = cu
        .definitions
        .keys()
        .chain(moi.definitions.keys())
        .collect();

    for id in cac_id {
        match (cu.definitions.get(id), moi.definitions.get(id)) {
            (None, Some(_)) => steps.push(ReloadStep::Add { id: id.clone() }),
            (Some(a), Some(b)) if a != b => steps.push(ReloadStep::Supersede {
                id: id.clone(),
                old_version: cu.version.clone(),
                new_version: moi.version.clone(),
            }),
            (Some(_), None) => steps.push(ReloadStep::Tombstone {
                id: id.clone(),
                removed_at: moi.version.clone(),
            }),
            // Không đổi, hoặc không tồn tại ở cả hai phía.
            _ => {}
        }
    }

    Ok(ReloadPlan {
        pack: moi.pack.clone(),
        from_version: cu.version.clone(),
        to_version: moi.version.clone(),
        steps,
    })
}

/// Kết quả một lần chạy `pack test`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestReport {
    /// Pack nào.
    pub pack: String,
    /// Kịch bản pack khai và kết quả từng cái.
    pub scenarios: Vec<(String, bool)>,
}

impl TestReport {
    /// Đạt hết chưa.
    pub fn passed(&self) -> bool {
        self.scenarios.iter().all(|(_, ok)| *ok)
    }

    /// Kịch bản nào trượt.
    pub fn failures(&self) -> Vec<&str> {
        self.scenarios
            .iter()
            .filter(|(_, ok)| !ok)
            .map(|(n, _)| n.as_str())
            .collect()
    }

    /// **Pack không khai test nào cũng là một phát hiện**, không phải "đạt".
    ///
    /// `pack test` trả về xanh cho một pack không có test là cách nhanh nhất
    /// để cả một thư viện mod không có test nào mà ai cũng tin là đã kiểm.
    pub fn has_no_tests(&self) -> bool {
        self.scenarios.is_empty()
    }
}

/// Ảnh chụp một pack đang nằm trong sổ.
///
/// `definitions` cần hash của **từng** định nghĩa, thứ mà `Registry` không giữ
/// — nên chỗ gọi truyền vào. Tách như vậy để module này không phải biết cách
/// một định nghĩa được băm, việc của tầng nội dung.
pub fn snapshot(
    reg: &Registry,
    pack: &str,
    definitions: BTreeMap<String, StateHash>,
) -> RegistryResult<PackSnapshot> {
    let m = reg
        .manifest(pack)
        .ok_or_else(|| RegistryError::PackAbsent(pack.to_owned()))?;
    let h = reg.hash_of(pack).expect("manifest có thì hash có");
    Ok(PackSnapshot {
        pack: pack.to_owned(),
        version: m.version.clone(),
        hash: h,
        definitions,
    })
}
