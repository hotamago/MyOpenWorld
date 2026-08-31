//! Chứng cứ và xét xử (`idea.md §12.5.3`, `PD-03`).
//!
//! ## Phán quyết **có thể sai**, và đó là tính năng
//!
//! > Thủ tục nào thì cũng chỉ ra một phán quyết, và phán quyết **có thể sai so
//! > với ground truth**. Sự lệch giữa hai thứ đó chính là chất liệu cho lịch sử.
//!
//! Vì vậy [`try_case`] **không nhận** thủ phạm thật làm tham số. Không phải vì
//! quên, mà vì nếu nó nhận thì sớm muộn sẽ có ai đó dùng — một dòng "nếu bị cáo
//! đúng là thủ phạm thì tăng nhẹ khả năng kết tội", nghe rất hợp lý, và án oan
//! biến mất khỏi thế giới cùng với vu khống, ngoại phạm dựng sẵn, và mọi vở kịch
//! tòa án từng được viết.
//!
//! Sự thật chỉ dùng ở [`Verdict::was_correct`], và hàm đó dành cho audit và cho
//! `§18.11`, không dành cho tòa.
//!
//! ## Thủ tục là **dữ liệu của tổ chức**, không hard-code
//!
//! Xử theo bằng chứng, theo lời thề, theo đấu thần thánh, theo tra tấn, theo bói
//! toán, hay theo hội đồng trưởng lão — [`Procedure`] liệt kê chúng, và mỗi cái
//! có một cách khác nhau để đi từ chứng cứ tới phán quyết. Nền văn minh nào dùng
//! thủ tục nào là chuyện của content pack.

use crate::crime::Witness;
use crate::norms::{Charge, ProofMode, ProofRequirement};
use mow_core::{EntityId, Tick};
use serde::{Deserialize, Serialize};

/// Một mẩu chứng cứ.
///
/// Chứng cứ là **dữ liệu thật trong world, có thời hạn tồn tại và có thể bị phá
/// hủy**. Trường `decays_at` là chỗ phi tang trở thành một nước đi: kẻ phạm tội
/// chỉ cần sống sót qua thời hạn đó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Evidence {
    /// Vật chứng: vật phẩm, dấu vết trên ô, thương tích, dấu vết phép.
    Physical {
        /// Mô tả.
        what: String,
        /// Hết hiệu lực lúc nào. Máu khô, dấu chân bị mưa xóa.
        decays_at: Tick,
        /// Đã bị phá hủy chưa.
        destroyed: bool,
    },
    /// Lời khai.
    Testimony(Witness),
    /// Văn bản — **có thể giả mạo** nếu ai đó đủ skill.
    Document {
        /// Mô tả.
        what: String,
        /// Có phải giả không. Tòa **không** đọc trường này; chỉ audit đọc.
        forged: bool,
        /// Kỹ năng của người làm giả, `0`–`1000`. Cao thì khó bị phát hiện.
        forgery_skill: u16,
    },
    /// Phép truy vấn sự thật.
    ///
    /// Là một spell trong knowledge graph, nên nó có điều kiện, chi phí, tỉ lệ
    /// thất bại và **có counter**. Nền văn minh nào phát triển được nó thì tư
    /// pháp đổi hẳn bản chất — và giới quyền lực sẽ nghiên cứu cách chống lại.
    TruthSpell {
        /// Kết quả phép trả về.
        says_guilty: bool,
        /// Có bị hóa giải không. Nếu có, kết quả vô nghĩa.
        countered: bool,
    },
}

impl Evidence {
    /// Chứng cứ này còn dùng được ở tick `now` không.
    pub fn is_available(&self, now: Tick) -> bool {
        match self {
            Evidence::Physical {
                decays_at,
                destroyed,
                ..
            } => !destroyed && now.0 <= decays_at.0,
            Evidence::Testimony(w) => w.will_testify(),
            Evidence::Document { .. } => true,
            Evidence::TruthSpell { countered, .. } => !countered,
        }
    }

    /// Nó thỏa yêu cầu chứng cứ nào.
    fn satisfies(&self, req: &ProofRequirement) -> bool {
        matches!(
            (self, req),
            (
                Evidence::Physical { .. },
                ProofRequirement::PhysicalEvidence
            ) | (Evidence::Document { .. }, ProofRequirement::Document)
                | (Evidence::TruthSpell { .. }, ProofRequirement::TruthSpell)
        )
    }
}

/// Thủ tục xét xử — **dữ liệu của tổ chức**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Procedure {
    /// Xử theo bằng chứng.
    Evidentiary,
    /// Xử theo lời thề: ai thề nặng hơn thì thắng.
    Compurgation,
    /// Đấu thần thánh: ai khỏe hơn thì đúng.
    TrialByCombat,
    /// Tra tấn: đủ đau thì nhận, bất kể có làm hay không.
    Torture,
    /// Hội đồng trưởng lão: xử theo danh tiếng của bị cáo.
    ElderCouncil,
}

/// Kết quả một phiên xử.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    /// Bị cáo.
    pub defendant: EntityId,
    /// Có kết tội không.
    pub guilty: bool,
    /// Thủ tục đã dùng.
    pub procedure: Procedure,
    /// Bao nhiêu chứng cứ được chấp nhận.
    pub evidence_accepted: usize,
    /// Vì sao — phân rã để `§18.13` dựng câu trả lời từ dữ liệu.
    pub reasons: Vec<String>,
}

impl Verdict {
    /// Phán quyết này có **đúng** so với sự thật không.
    ///
    /// **Chỉ dùng cho audit và cho `§18.11`.** Tòa không gọi hàm này, và không
    /// có đường nào để nó gọi được: sự thật không nằm trong tham số của
    /// [`try_case`].
    pub fn was_correct(&self, actual_actor: Option<EntityId>) -> bool {
        match actual_actor {
            Some(a) => self.guilty == (a == self.defendant),
            None => !self.guilty,
        }
    }
}

/// Chứng cứ đã đủ theo yêu cầu của điều luật chưa.
pub fn proof_met(charge: &Charge, evidence: &[Evidence], now: Tick) -> bool {
    let co_san: Vec<&Evidence> = evidence.iter().filter(|e| e.is_available(now)).collect();

    let du = |req: &ProofRequirement| -> bool {
        match req {
            ProofRequirement::WitnessCount(n) => {
                let dem = co_san
                    .iter()
                    .filter(|e| matches!(e, Evidence::Testimony(_)))
                    .count();
                u32::try_from(dem).unwrap_or(u32::MAX) >= *n
            }
            khac => co_san.iter().any(|e| e.satisfies(khac)),
        }
    };

    match charge.proof_mode {
        ProofMode::AnyOf => charge.proof_required.iter().any(du),
        ProofMode::AllOf => charge.proof_required.iter().all(du),
    }
}

/// Xử một vụ.
///
/// **Không nhận thủ phạm thật.** Xem docstring của module.
pub fn try_case(
    defendant: EntityId,
    charge: &Charge,
    evidence: &[Evidence],
    procedure: Procedure,
    now: Tick,
    // Dùng cho những thủ tục không dựa vào chứng cứ.
    ctx: &TrialContext,
) -> Verdict {
    let co_san: Vec<&Evidence> = evidence.iter().filter(|e| e.is_available(now)).collect();
    let mut reasons = Vec::new();

    let guilty = match procedure {
        Procedure::Evidentiary => {
            let du = proof_met(charge, evidence, now);
            reasons.push(if du {
                format!("chứng cứ đủ theo {}", charge.norm_set)
            } else {
                "chứng cứ không đủ".to_owned()
            });
            du
        }
        Procedure::Compurgation => {
            // Ai gom được nhiều người thề hơn thì thắng. Danh tiếng, không sự thật.
            let ket = ctx.defendant_oath_helpers < ctx.accuser_oath_helpers;
            reasons.push(format!(
                "thề: bị cáo {} người, bên tố {} người",
                ctx.defendant_oath_helpers, ctx.accuser_oath_helpers
            ));
            ket
        }
        Procedure::TrialByCombat => {
            let ket = ctx.defendant_strength < ctx.accuser_strength;
            reasons.push("đấu thần thánh: kẻ mạnh hơn được coi là đúng".to_owned());
            ket
        }
        Procedure::Torture => {
            // Đủ đau thì nhận, bất kể có làm hay không. Đây là lý do thủ tục này
            // cho ra án oan một cách có hệ thống, và mô hình phải nói được điều đó.
            let ket = ctx.pain_applied > ctx.defendant_endurance;
            reasons.push(format!(
                "tra tấn: đau {} vượt sức chịu {}",
                ctx.pain_applied, ctx.defendant_endurance
            ));
            ket
        }
        Procedure::ElderCouncil => {
            let ket = ctx.defendant_reputation < 0;
            reasons.push(format!(
                "hội đồng xét danh tiếng: {}",
                ctx.defendant_reputation
            ));
            ket
        }
    };

    Verdict {
        defendant,
        guilty,
        procedure,
        evidence_accepted: co_san.len(),
        reasons,
    }
}

/// Những gì các thủ tục phi chứng cứ cần biết.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TrialContext {
    /// Số người thề cho bị cáo.
    pub defendant_oath_helpers: u32,
    /// Số người thề cho bên tố.
    pub accuser_oath_helpers: u32,
    /// Sức mạnh bị cáo.
    pub defendant_strength: u16,
    /// Sức mạnh bên tố.
    pub accuser_strength: u16,
    /// Mức đau đã áp.
    pub pain_applied: u16,
    /// Sức chịu đựng của bị cáo.
    pub defendant_endurance: u16,
    /// Danh tiếng bị cáo, `-1000`..`1000`.
    pub defendant_reputation: i16,
}

/// Đã xử rồi thì không xử lại cùng một hành vi (`§12.14`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DoubleJeopardy {
    da_xu: Vec<(u64, String, String)>,
}

impl DoubleJeopardy {
    /// Rỗng.
    pub fn new() -> DoubleJeopardy {
        DoubleJeopardy::default()
    }

    /// Ghi nhận một vụ đã xử.
    pub fn record(&mut self, defendant: EntityId, norm_set: &str, act: &str) {
        self.da_xu
            .push((defendant.0, norm_set.to_owned(), act.to_owned()));
    }

    /// Vụ này đã xử chưa.
    ///
    /// Theo **bộ luật**, không theo toàn cục: bị nước A xử rồi vẫn có thể bị
    /// nước B xử, và đó chính là chuyện `§12.14` gọi là xung đột thẩm quyền.
    pub fn already_tried(&self, defendant: EntityId, norm_set: &str, act: &str) -> bool {
        self.da_xu
            .iter()
            .any(|(d, n, a)| *d == defendant.0 && n == norm_set && a == act)
    }
}
