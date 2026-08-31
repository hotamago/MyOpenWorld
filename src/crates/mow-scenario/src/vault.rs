//! Seed Vault (`idea.md §7.6.5`, `PF-04`).
//!
//! Sáu việc, và mỗi việc chống một cách thua cụ thể:
//!
//! | Việc | Chống được gì |
//! |---|---|
//! | duyệt, tìm, gắn thẻ | một thư mục 300 worldseed không tên tuổi |
//! | preview có **báo cáo rủi ro** | tạo xong mới biết thế giới không sống được |
//! | fork giữ quan hệ cha–con | mất dấu bản gốc sau ba lần sửa |
//! | diff **ở mức dữ liệu** | `diff` văn bản báo đổi khi chỉ đảo thứ tự khóa |
//! | kiểm plugin **trước** khi tạo | lỗi giữa chừng genesis |
//! | xuất/nhập **có checksum** | một file tải về đã hỏng mà không ai biết |
//!
//! ## Diff ở mức dữ liệu, không phải mức văn bản
//!
//! `§7.6.5` nói rõ *"diff hai worldseed **ở mức dữ liệu, không phải mức văn
//! bản**"*. Khác biệt không phải thẩm mỹ: một YAML đổi thứ tự khóa, đổi thụt
//! lề, hay thêm một dòng chú thích thì `diff` văn bản báo đầy màn hình còn thế
//! giới sinh ra **y hệt**. Ngược lại, đổi `seed` từ `null` thành một con số là
//! một dòng thay đổi nhỏ xíu mà **cả thế giới khác đi**.
//!
//! Nên [`diff`] so từng trường có nghĩa, và mỗi khác biệt mang theo
//! [`Impact`] — thứ trả lời câu hỏi người dùng thật sự hỏi: *"đổi cái này thì
//! thế giới có khác không"*.
//!
//! ## Báo cáo rủi ro là bắt buộc trong preview
//!
//! Một worldseed hợp lệ về cú pháp vẫn có thể sinh ra một thế giới chết: không
//! ai sống nổi, không có nước, mọi thế lực thù nhau từ tick 0. Những thứ đó
//! **kiểm được trước khi tạo**, và kiểm sau khi tạo thì người dùng đã mất một
//! lần chờ sinh thế giới.

use crate::worldseed::Worldseed;
use mow_math::{CanonicalHash, StateHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Một mục trong kho.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultEntry {
    /// Worldseed.
    pub seed: Worldseed,
    /// Thẻ do người dùng gắn.
    pub tags: BTreeSet<String>,
    /// Fork từ mục nào — **giữ nguyên quan hệ cha–con** (`§7.6.5`).
    pub forked_from: Option<String>,
    /// Nguồn: `official`, `community`, `local`.
    pub origin: String,
}

impl VaultEntry {
    /// **Checksum truyền tải** của cả mục — bắt mọi hư hại trên đường đi.
    ///
    /// Không dùng lại `Worldseed::canonical_hash`. Hai hàm trả lời hai câu hỏi
    /// khác nhau, và trộn chúng làm một là một lỗi im lặng:
    ///
    /// | Hash | Câu hỏi | Có tính `description` không |
    /// |---|---|---|
    /// | [`world_identity`](VaultEntry::world_identity) | *"có phải cùng một thế giới không"* | **không** — mô tả không đổi thế giới |
    /// | `checksum` | *"file có tới nơi nguyên vẹn không"* | **có** — mọi byte đều phải khớp |
    ///
    /// Dùng identity làm checksum thì một gói bị hỏng đúng ở phần mô tả, thẻ,
    /// hoặc quan hệ cha–con sẽ **qua được** kiểm tra.
    pub fn hash(&self) -> StateHash {
        let mut h = StateHasher::with_domain("mow.vault.bundle.v1");
        // Băm dạng tuần tự hóa: mọi trường đều vào, kể cả những trường không
        // ảnh hưởng thế giới.
        let bytes = serde_json::to_vec(self).expect("VaultEntry luôn tuần tự hóa được");
        h.write_bytes(&bytes);
        h.finish()
    }

    /// Hash **danh tính thế giới**: hai mục cùng giá trị này sinh ra cùng một
    /// thế giới, dù tên, thẻ hay mô tả có khác.
    pub fn world_identity(&self) -> StateHash {
        let mut h = StateHasher::with_domain("mow.worldseed.hash.v1");
        self.seed.canonical_hash(&mut h);
        h.finish()
    }
}

/// Kho worldseed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedVault {
    entries: BTreeMap<String, VaultEntry>,
}

/// Vì sao một thao tác kho thất bại.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VaultError {
    /// Không có mục đó.
    #[error("kho không có worldseed `{0}`")]
    NotFound(String),
    /// Trùng id.
    #[error("kho đã có worldseed `{0}`")]
    Duplicate(String),
    /// Checksum không khớp khi nhập.
    #[error(
        "checksum không khớp cho `{id}`: gói ghi {expected}, nội dung thật là {actual}. \
         File có thể đã hỏng trên đường tải"
    )]
    ChecksumMismatch {
        /// Worldseed nào.
        id: String,
        /// Ghi trong gói.
        expected: String,
        /// Tính từ nội dung.
        actual: String,
    },
}

impl SeedVault {
    /// Kho rỗng.
    pub fn new() -> SeedVault {
        SeedVault::default()
    }

    /// Số mục.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Rỗng chưa.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Thêm một worldseed.
    pub fn add(&mut self, e: VaultEntry) -> Result<(), VaultError> {
        if self.entries.contains_key(&e.seed.id) {
            return Err(VaultError::Duplicate(e.seed.id.clone()));
        }
        self.entries.insert(e.seed.id.clone(), e);
        Ok(())
    }

    /// Lấy một mục.
    pub fn get(&self, id: &str) -> Option<&VaultEntry> {
        self.entries.get(id)
    }

    /// Tìm theo chuỗi con trong id/mô tả và theo thẻ.
    ///
    /// Trả về thứ tự **ổn định** (theo id), không theo điểm khớp: một danh sách
    /// đảo thứ tự giữa hai lần tìm cùng một chuỗi làm người dùng mất chỗ.
    pub fn search(&self, text: &str, tags: &BTreeSet<String>) -> Vec<&VaultEntry> {
        let t = text.to_lowercase();
        self.entries
            .values()
            .filter(|e| {
                let khop_chu = t.is_empty()
                    || e.seed.id.to_lowercase().contains(&t)
                    || e.seed.description.to_lowercase().contains(&t);
                let khop_the = tags.is_empty() || tags.is_subset(&e.tags);
                khop_chu && khop_the
            })
            .collect()
    }

    /// Gắn thẻ.
    pub fn tag(&mut self, id: &str, tag: &str) -> Result<(), VaultError> {
        self.entries
            .get_mut(id)
            .ok_or_else(|| VaultError::NotFound(id.to_owned()))?
            .tags
            .insert(tag.to_owned());
        Ok(())
    }

    /// **Fork**: sao chép một worldseed thành bản mới, giữ quan hệ cha–con.
    ///
    /// Version bản mới bắt đầu lại từ 1, và `forked_from` trỏ về cha. Không
    /// dùng `parent.version + 1`: bản fork là một dòng riêng, và đánh số tiếp
    /// của cha sẽ làm hai dòng đụng số nhau sau vài lần sửa.
    pub fn fork(&mut self, cha: &str, id_moi: &str) -> Result<(), VaultError> {
        let goc = self
            .entries
            .get(cha)
            .ok_or_else(|| VaultError::NotFound(cha.to_owned()))?
            .clone();
        let mut seed = goc.seed.clone();
        id_moi.clone_into(&mut seed.id);
        seed.version = 1;
        self.add(VaultEntry {
            seed,
            tags: goc.tags.clone(),
            forked_from: Some(cha.to_owned()),
            origin: "local".to_owned(),
        })
    }

    /// Chuỗi tổ tiên của một mục, gần nhất trước.
    ///
    /// Có vòng thì dừng — một kho bị sửa tay có thể có `a → b → a`, và một hàm
    /// duyệt cây mà treo ở đó là một cách hỏng tệ hơn dữ liệu sai.
    pub fn ancestry(&self, id: &str) -> Vec<&str> {
        let mut v = Vec::new();
        let mut da_qua = BTreeSet::new();
        let mut cur = id;
        while let Some(e) = self.entries.get(cur) {
            let Some(cha) = e.forked_from.as_deref() else {
                break;
            };
            if !da_qua.insert(cha.to_owned()) {
                break;
            }
            v.push(cha);
            cur = cha;
        }
        v
    }

    /// Xuất một mục thành gói có checksum.
    pub fn export(&self, id: &str) -> Result<Bundle, VaultError> {
        let e = self
            .entries
            .get(id)
            .ok_or_else(|| VaultError::NotFound(id.to_owned()))?;
        Ok(Bundle {
            entry: e.clone(),
            checksum: e.hash(),
        })
    }

    /// Nhập một gói, **kiểm checksum trước**.
    ///
    /// Kiểm trước khi thêm vào kho: nhập rồi mới kiểm nghĩa là một gói hỏng đã
    /// nằm trong kho ở thời điểm phát hiện, và người dùng phải tự xóa nó.
    pub fn import(&mut self, b: &Bundle) -> Result<(), VaultError> {
        let that = b.entry.hash();
        if that != b.checksum {
            return Err(VaultError::ChecksumMismatch {
                id: b.entry.seed.id.clone(),
                expected: b.checksum.short(),
                actual: that.short(),
            });
        }
        self.add(b.entry.clone())
    }
}

/// Gói xuất/nhập (`§7.6.5`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    /// Nội dung.
    pub entry: VaultEntry,
    /// Checksum của nội dung.
    pub checksum: StateHash,
}

/// Mức độ một khác biệt ảnh hưởng tới thế giới sinh ra.
///
/// Đây là thứ biến một danh sách khác biệt thành một câu trả lời. Không có nó,
/// người dùng nhìn 40 dòng diff mà vẫn không biết hai thế giới có khác nhau
/// không.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    /// Chỉ đổi chữ nghĩa: mô tả, tên. Thế giới sinh ra **y hệt**.
    Cosmetic,
    /// Đổi điều kiện ban đầu: thế giới khác nhưng vẫn cùng địa hình nền.
    InitialConditions,
    /// Đổi seed, profile hoặc pack: **cả thế giới khác đi**.
    WholeWorld,
}

/// Một khác biệt giữa hai worldseed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Difference {
    /// Trường nào.
    pub field: String,
    /// Bên trái.
    pub left: String,
    /// Bên phải.
    pub right: String,
    /// Ảnh hưởng.
    pub impact: Impact,
}

/// So hai worldseed **ở mức dữ liệu** (`§7.6.5`).
///
/// Đổi thứ tự khóa trong YAML, thêm chú thích, đổi thụt lề — không cái nào
/// xuất hiện ở đây, vì đầu vào đã là cấu trúc đã phân tích.
pub fn diff(a: &Worldseed, b: &Worldseed) -> Vec<Difference> {
    let mut v = Vec::new();
    let mut them = |field: &str, l: String, r: String, impact: Impact| {
        if l != r {
            v.push(Difference {
                field: field.to_owned(),
                left: l,
                right: r,
                impact,
            });
        }
    };

    them(
        "description",
        a.description.clone(),
        b.description.clone(),
        Impact::Cosmetic,
    );
    them(
        "generation_profile",
        a.generation_profile.clone(),
        b.generation_profile.clone(),
        Impact::WholeWorld,
    );
    them(
        "resolved_seed",
        a.resolved_seed().to_string(),
        b.resolved_seed().to_string(),
        Impact::WholeWorld,
    );
    them(
        "packs",
        a.packs.join(","),
        b.packs.join(","),
        Impact::WholeWorld,
    );
    them(
        "genesis",
        format!("{} bước", a.genesis.len()),
        format!("{} bước", b.genesis.len()),
        Impact::InitialConditions,
    );
    them(
        "named_entities",
        a.named_entities.len().to_string(),
        b.named_entities.len().to_string(),
        Impact::InitialConditions,
    );
    v
}

/// Hai worldseed có sinh ra cùng một thế giới không.
///
/// Câu hỏi người dùng thật sự hỏi, và nó **không** bằng "hai file giống nhau".
pub fn same_world(a: &Worldseed, b: &Worldseed) -> bool {
    diff(a, b).iter().all(|d| d.impact == Impact::Cosmetic)
}

/// Một rủi ro Yuu tìm thấy trong preview (`§7.6.5`).
///
/// Không `Deserialize`: `code` là bảng mã tĩnh của engine. Preview là **kết quả
/// tính toán**, dựng lại được bất cứ lúc nào từ worldseed — nó đi ra UI, không
/// đi vào save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Risk {
    /// Mã ổn định để UI tra và dịch.
    pub code: &'static str,
    /// Mô tả.
    pub detail: String,
    /// Chặn hẳn việc tạo, hay chỉ cảnh báo.
    ///
    /// Phân biệt này quan trọng: một thế giới khắc nghiệt là **lựa chọn hợp
    /// lệ**, còn một thế giới thiếu pack thì tạo sẽ hỏng giữa chừng.
    pub blocking: bool,
}

/// Preview trước khi tạo (`§7.6.5`).
///
/// Không `Deserialize`, cùng lý do với [`Risk`]: dựng lại rẻ hơn lưu, và lưu
/// một preview cũ là cách để hiện lên báo cáo rủi ro không còn đúng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Preview {
    /// Worldseed nào.
    pub worldseed_id: String,
    /// Seed đã giải.
    pub resolved_seed: u64,
    /// Pack cần và có mặt chưa.
    pub required_packs: Vec<(String, bool)>,
    /// Số bước genesis.
    pub genesis_steps: usize,
    /// Thực thể có tên.
    pub named_entities: Vec<String>,
    /// Báo cáo rủi ro.
    pub risks: Vec<Risk>,
}

impl Preview {
    /// Tạo được không.
    pub fn creatable(&self) -> bool {
        !self.risks.iter().any(|r| r.blocking)
    }

    /// Những rủi ro chặn hẳn.
    pub fn blockers(&self) -> Vec<&Risk> {
        self.risks.iter().filter(|r| r.blocking).collect()
    }
}

/// Dựng preview, **kiểm plugin trước khi tạo** (`§7.6.5`).
///
/// `available_packs` là những pack đang có trong máy. Thiếu một cái thì đó là
/// rủi ro **chặn** — `§7.6.5` nói rõ *"thiếu thì báo trước khi tạo, không lỗi
/// giữa chừng"*.
pub fn preview(seed: &Worldseed, available_packs: &BTreeSet<String>) -> Preview {
    let mut risks = Vec::new();

    let required_packs: Vec<(String, bool)> = seed
        .packs
        .iter()
        .map(|p| (p.clone(), available_packs.contains(p)))
        .collect();
    for (p, co) in &required_packs {
        if !co {
            risks.push(Risk {
                code: "pack.missing",
                detail: format!("worldseed cần pack `{p}` nhưng máy không có"),
                blocking: true,
            });
        }
    }

    // Worldseed sai hình dạng thì cũng chặn — và mỗi lỗi thành một dòng riêng
    // chứ không gộp thành "không hợp lệ".
    if let Err(loi) = seed.validate() {
        for l in loi {
            risks.push(Risk {
                code: "worldseed.invalid",
                detail: l,
                blocking: true,
            });
        }
    }

    // Không có bước genesis nào: hợp lệ, nhưng gần như chắc chắn là nhầm.
    // Cảnh báo, **không** chặn — một world trống là một lựa chọn có thật.
    if seed.genesis.is_empty() {
        risks.push(Risk {
            code: "genesis.empty",
            detail: "không có bước genesis nào: world sẽ trống trơn ở tick 0".to_owned(),
            blocking: false,
        });
    }

    Preview {
        worldseed_id: seed.id.clone(),
        resolved_seed: seed.resolved_seed(),
        required_packs,
        genesis_steps: seed.genesis.len(),
        named_entities: seed.named_entities.keys().cloned().collect(),
        risks,
    }
}
