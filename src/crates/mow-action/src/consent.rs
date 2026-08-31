//! Ràng buộc ưng thuận ở **tầng engine** (`idea.md §12.7.2`, `§12.7.5`, `§22.26`, `PB-24`).
//!
//! > Mechanic thân mật chỉ hợp lệ giữa các bên `Sapient` đã qua `maturity_years`
//! > và có capacity ưng thuận; validator từ chối **tại thời điểm tạo action**.
//! > **Không plugin, không override nào cấp được ngoại lệ.**
//!
//! ## Vì sao nó nằm ở đây và không ở đâu khác
//!
//! Cả tài liệu này nói về một thế giới nơi *mọi thứ đều có thể xảy ra*, và về
//! những hệ thống có thể mở rộng bằng content pack của cộng đồng. Ràng buộc
//! này là **ngoại lệ tuyệt đối** của cả hai nguyên tắc đó, và nó phải nằm ở
//! chỗ mà không ai với tới được:
//!
//! - Không ở content pack — pack nào cũng sửa được.
//! - Không ở luật DSL — luật do LLM sinh ra và Yuu duyệt.
//! - Không ở một cờ cấu hình — cờ nào cũng tắt được.
//! - **Ở đây, trong engine, không có tham số nào nới nó ra.**
//!
//! Ba điều kiện, và tất cả phải đúng cùng lúc. Kiểm ở **thời điểm tạo action**
//! chứ không phải lúc thực thi: một action không hợp lệ không được tồn tại
//! trong hàng đợi, không được xuất hiện trong dòng thời gian, và không được để
//! lại dấu vết nào ngoài một [`ConsentDenial`].

use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Năng lực ưng thuận của một thực thể.
///
/// Mọi trường là **bắt buộc**. Không có `Default`, và đó là chủ đích: một giá
/// trị mặc định ở đây sẽ là một lỗ hổng, vì code quên đặt sẽ lấy mặc định thay
/// vì báo lỗi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentCapacity {
    /// Có tag `Sapient` không.
    pub sapient: bool,
    /// Tuổi hiện tại, năm.
    pub age_years: u32,
    /// Tuổi trưởng thành của loài.
    pub maturity_years: u32,
    /// Có đang tỉnh táo và tự chủ không.
    ///
    /// Sai khi bất tỉnh, bị mê hoặc, đang chịu effect khống chế ý chí, hoặc bị
    /// ép buộc bằng quyền lực. Đây là điều kiện mà một hệ thống chỉ kiểm tuổi
    /// sẽ bỏ sót.
    pub has_agency: bool,
}

/// Vì sao ưng thuận không hợp lệ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenialReason {
    /// Không phải thực thể có tri giác.
    NotSapient,
    /// Chưa qua tuổi trưởng thành của loài.
    BelowMaturity,
    /// Không có năng lực tự chủ.
    NoAgency,
}

impl DenialReason {
    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            DenialReason::NotSapient => "not_sapient",
            DenialReason::BelowMaturity => "below_maturity",
            DenialReason::NoAgency => "no_agency",
        }
    }
}

/// Một lần từ chối.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentDenial {
    /// Bên nào không đủ điều kiện.
    pub party: EntityId,
    /// Vì sao.
    pub reason: DenialReason,
}

impl core::fmt::Display for ConsentDenial {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} — {}", self.party, self.reason.as_str())
    }
}

/// Kiểm một bên.
fn check_one(who: EntityId, c: ConsentCapacity) -> Option<ConsentDenial> {
    if !c.sapient {
        return Some(ConsentDenial {
            party: who,
            reason: DenialReason::NotSapient,
        });
    }
    if c.age_years < c.maturity_years {
        return Some(ConsentDenial {
            party: who,
            reason: DenialReason::BelowMaturity,
        });
    }
    if !c.has_agency {
        return Some(ConsentDenial {
            party: who,
            reason: DenialReason::NoAgency,
        });
    }
    None
}

/// Kiểm ưng thuận cho một hành động thân mật.
///
/// Trả về **mọi** lý do từ chối, không dừng ở lý do đầu tiên: log kiểm toán cần
/// biết đầy đủ, và một bên có thể không đủ điều kiện vì nhiều lý do cùng lúc.
///
/// Hàm này **không có tham số nới lỏng**. Không có `allow_override`, không có
/// `bypass`, không có `admin`. Nếu một ngày ai đó thêm một tham số như thế, họ
/// phải sửa chữ ký hàm này và mọi chỗ gọi, và bài test
/// `khong_co_duong_nao_nem_qua` sẽ đỏ.
pub fn validate(parties: &[(EntityId, ConsentCapacity)]) -> Result<(), Vec<ConsentDenial>> {
    // Một bên thì không phải là "giữa các bên". Đây không phải một chi tiết
    // hình thức: nó chặn một loại action mà validator hai-bên sẽ cho qua.
    if parties.len() < 2 {
        return Err(parties
            .iter()
            .map(|(w, _)| ConsentDenial {
                party: *w,
                reason: DenialReason::NoAgency,
            })
            .collect());
    }

    let tu_choi: Vec<ConsentDenial> = parties
        .iter()
        .filter_map(|(w, c)| check_one(*w, *c))
        .collect();

    if tu_choi.is_empty() {
        Ok(())
    } else {
        Err(tu_choi)
    }
}

/// Những loại action chịu ràng buộc này.
///
/// Danh sách nằm trong engine, không trong content. Một pack **có thể** thêm
/// action mới vào danh sách này qua [`IntimacyRegistry::require_consent`],
/// nhưng **không thể** gỡ cái nào ra.
#[derive(Debug, Default)]
pub struct IntimacyRegistry {
    kinds: std::collections::BTreeSet<String>,
}

impl IntimacyRegistry {
    /// Sổ với các loại action chuẩn của engine.
    pub fn standard() -> IntimacyRegistry {
        let mut r = IntimacyRegistry::default();
        for k in [
            "core.intimacy",
            "core.courtship.physical",
            "core.reproduction",
        ] {
            r.kinds.insert(k.to_owned());
        }
        r
    }

    /// Thêm một loại action vào diện chịu ràng buộc.
    ///
    /// **Chỉ thêm được, không gỡ được.** Không có `remove`, và đó là toàn bộ
    /// điểm: một content pack có thể mở rộng phạm vi bảo vệ, không thu hẹp nó.
    pub fn require_consent(&mut self, kind: &str) {
        self.kinds.insert(kind.to_owned());
    }

    /// Loại action này có chịu ràng buộc không.
    pub fn requires_consent(&self, kind: &str) -> bool {
        self.kinds.contains(kind)
    }

    /// Mọi loại đang chịu ràng buộc, theo thứ tự.
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.kinds.iter().map(String::as_str)
    }
}
