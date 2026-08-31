//! Lão hóa và tử vong (`idea.md §9.5.6`).
//!
//! Hai mô hình, và việc chọn giữa chúng là một thuộc tính của **loài**, không
//! phải một hằng số của thế giới:
//!
//! - **Gompertz**: nguy cơ tử vong tăng theo hàm mũ với tuổi. Người, thú, phần
//!   lớn sinh vật. Thời gian nhân đôi nguy cơ là khoảng 8 năm ở người.
//! - **Lão hóa không đáng kể**: nguy cơ gần như không đổi theo tuổi. Rùa
//!   Galápagos, thủy tức, và — trong thế giới này — tiên cùng một số sinh vật
//!   phép thuật. Chúng vẫn chết, chỉ là không chết **vì già**.
//!
//! Phân biệt này quan trọng với gameplay: một loài lão hóa không đáng kể tạo ra
//! những cá thể có trí nhớ hàng thế kỷ, và đó là một động lực xã hội hoàn toàn
//! khác. `§9.11` gọi đây là một trong năm trục cách biệt liên loài.
//!
//! ## Tác động qua effect, không ghi thẳng chỉ số
//!
//! `§22.20` cấm effect ghi thẳng base stat. Lão hóa cũng vậy: nó không trừ dần
//! `strength` của một cụ già. Nó **áp một effect** làm giảm strength, và effect
//! đó đi qua modifier pipeline như mọi effect khác. Nhờ vậy một liều thuốc trẻ
//! hóa chỉ cần gỡ effect, không phải đoán xem base stat lẽ ra là bao nhiêu.

use mow_math::{CanonicalHash, Prob, StateHasher};
use serde::{Deserialize, Serialize};

/// Mô hình lão hóa của một loài.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SenescenceModel {
    /// Nguy cơ tử vong tăng theo hàm mũ.
    Gompertz {
        /// Nguy cơ nền ở tuổi 0, phần triệu mỗi năm.
        baseline_ppm_per_year: u64,
        /// Số năm để nguy cơ nhân đôi.
        doubling_years: u32,
    },
    /// Nguy cơ gần như không đổi theo tuổi.
    Negligible {
        /// Nguy cơ cố định, phần triệu mỗi năm.
        ///
        /// **Không bằng 0.** Một loài bất tử tuyệt đối sẽ tích lũy vô hạn qua
        /// thời gian mô phỏng dài, và dân số của nó sẽ nuốt toàn bộ thế giới.
        /// Ngoài ra "không bao giờ chết" không phải một câu chuyện — "chết vì
        /// tai nạn chứ không vì già" thì có.
        annual_ppm: u64,
    },
}

impl SenescenceModel {
    /// Nguy cơ tử vong trong một năm ở tuổi `age_years`.
    pub fn annual_mortality(&self, age_years: u32) -> Prob {
        match self {
            SenescenceModel::Gompertz {
                baseline_ppm_per_year,
                doubling_years,
            } => {
                let d = (*doubling_years).max(1);
                // Nhân đôi mỗi `d` năm. Lũy thừa trên số nguyên, chặn ở trần để
                // không tràn: 2^40 lần nguy cơ nền đã là chắc chắn từ lâu.
                let so_lan = (age_years / d).min(40);
                let ppm = baseline_ppm_per_year.saturating_mul(1u64 << so_lan);
                Prob::from_ppm(ppm.min(999_999)).unwrap_or(Prob::ALWAYS)
            }
            SenescenceModel::Negligible { annual_ppm } => {
                Prob::from_ppm((*annual_ppm).min(999_999)).unwrap_or(Prob::NEVER)
            }
        }
    }

    /// Tuổi mà nguy cơ tử vong hằng năm vượt một ngưỡng.
    ///
    /// Dùng cho UI ("tuổi thọ kỳ vọng") và cho AI ("ta còn bao lâu"). Trả `None`
    /// với loài lão hóa không đáng kể ở ngưỡng cao — chúng không bao giờ tới đó.
    pub fn age_at_risk(&self, threshold: Prob) -> Option<u32> {
        (0..2_000u32).find(|&age| self.annual_mortality(age) >= threshold)
    }

    /// Có phải lão hóa không đáng kể không.
    pub fn is_negligible(&self) -> bool {
        matches!(self, SenescenceModel::Negligible { .. })
    }
}

impl CanonicalHash for SenescenceModel {
    fn canonical_hash(&self, h: &mut StateHasher) {
        match self {
            SenescenceModel::Gompertz {
                baseline_ppm_per_year,
                doubling_years,
            } => {
                h.write_str("gompertz");
                h.write_u64(*baseline_ppm_per_year);
                h.write_u64(u64::from(*doubling_years));
            }
            SenescenceModel::Negligible { annual_ppm } => {
                h.write_str("negligible");
                h.write_u64(*annual_ppm);
            }
        }
    }
}

/// Giai đoạn đời, để UI và AI khỏi phải tự phân loại theo tuổi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeStage {
    /// Chưa trưởng thành.
    Juvenile,
    /// Đã trưởng thành.
    Adult,
    /// Đã qua đỉnh cao thể chất.
    Elder,
}

/// Mốc tuổi của một loài.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifeStages {
    /// Tuổi trưởng thành.
    ///
    /// `§12.7.2`: ràng buộc ưng thuận dùng chính con số này, và validator ở tầng
    /// engine từ chối hành động thân mật khi thiếu nó. Đó là lý do nó là một
    /// trường **bắt buộc** chứ không phải một tùy chọn.
    pub maturity_years: u32,
    /// Tuổi bắt đầu suy giảm.
    pub elder_years: u32,
}

impl LifeStages {
    /// Giai đoạn ở một tuổi.
    pub fn stage_at(self, age_years: u32) -> LifeStage {
        if age_years < self.maturity_years {
            LifeStage::Juvenile
        } else if age_years < self.elder_years {
            LifeStage::Adult
        } else {
            LifeStage::Elder
        }
    }
}

impl CanonicalHash for LifeStages {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(u64::from(self.maturity_years));
        h.write_u64(u64::from(self.elder_years));
    }
}

/// Id của effect lão hóa ứng với một tuổi.
///
/// Trả về **tên effect**, không phải một con số trừ vào chỉ số. `§22.20`: effect
/// chỉ tác động qua modifier pipeline và không bao giờ ghi base stat.
pub fn senescence_effect(stages: LifeStages, age_years: u32) -> Option<&'static str> {
    match stages.stage_at(age_years) {
        LifeStage::Juvenile => Some("core.effect.immature"),
        LifeStage::Adult => None,
        LifeStage::Elder => Some("core.effect.aged"),
    }
}
