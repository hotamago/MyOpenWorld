//! Rebase deadline khi qua cổng — **bước 5 của `§6.2`** (`§4.5`, `§22.42`, `PE-10`).
//!
//! Phép rebase một deadline đã nằm ở [`mow_core::clock::rebase`]; module này
//! **không viết lại nó**. Thứ còn thiếu ở tầng dưới là ba việc mà chỉ có chỗ
//! gọi mới làm được:
//!
//! 1. Rebase **mọi** tiến trình của entity, không sót cái nào.
//! 2. Ghi lại **đã đổi gì và vì sao** — `§4.5` yêu cầu rebase *"ghi event"*.
//! 3. Chứng minh được là đã không sót: [`RebaseAudit::covers_all`].
//!
//! ## Vì sao "không sót" là một yêu cầu riêng
//!
//! `§4.5` gọi đây là *"bước dễ quên nhất và là bước gây ra loại bug tệ nhất"*.
//! Cái quên không phải là quên gọi `rebase` — cái đó lộ ra ngay. Cái quên là
//! **gọi cho ba trong bốn tiến trình**: tuổi, đói, ủ bệnh được rebase, còn thai
//! kỳ thì không vì nó nằm ở một component khác mà người viết code hôm đó không
//! nghĩ tới.
//!
//! Bug đó không panic, không sai kiểu, và chỉ lộ ra khi có người mang thai đi
//! qua cổng — có thể hàng trăm giờ chơi sau. Nên [`rebase_processes`] nhận **cả
//! danh sách** và trả về một biên bản đếm được, thay vì để chỗ gọi tự lặp.

use mow_core::clock::{rebase, Clock, ClockDomain, Deadline};
use mow_math::MathError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Một tiến trình có thời hạn đang chạy trên entity.
///
/// `id` có tiền tố để tra được nguồn: `disease.plague.incubation`,
/// `pregnancy.term`, `contract.loan.7741`, `research.deadline.astral`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Process {
    /// Định danh tiến trình.
    pub id: String,
    /// Hạn của nó, **mang theo miền đồng hồ** — `Deadline` không dựng được nếu
    /// thiếu miền, nên một tiến trình không khai miền là thứ không biểu diễn nổi.
    pub deadline: Deadline,
}

/// Một dòng biên bản: một tiến trình đã được xử lý thế nào.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseLine {
    /// Tiến trình nào.
    pub process: String,
    /// Miền của nó.
    pub domain: ClockDomain,
    /// Mốc trước, theo đồng hồ world nguồn.
    pub before: u64,
    /// Mốc sau, theo đồng hồ world đích.
    pub after: u64,
    /// Vì sao đổi, hoặc vì sao **không** đổi.
    ///
    /// Dòng "không đổi" quan trọng ngang dòng "đã đổi": nó là bằng chứng rằng
    /// tiến trình đó đã được xem xét chứ không phải bị bỏ sót.
    pub reason: RebaseReason,
}

/// Vì sao một tiến trình được đổi hoặc giữ nguyên.
///
/// Là một **enum có tag ổn định**, không phải một câu tiếng Việt. Biên bản này
/// đi vào save và vào event, nên nó phải đọc được sau khi bản dịch đổi và phải
/// so sánh được bằng máy — câu chữ để [`RebaseReason::describe`] lo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RebaseReason {
    /// Proper time đi theo entity nên được quy đổi.
    ProperFollowsEntity,
    /// World-local neo vào world đã ký nên giữ nguyên.
    LocalAnchoredToOrigin,
    /// Divine vốn đã tuyệt đối nên giữ nguyên.
    DivineIsAbsolute,
    /// Law-defined thuộc quyền của luật nên engine không đụng.
    LawOwnsThisClock,
}

impl RebaseReason {
    /// Câu giải thích cho người đọc.
    pub fn describe(self) -> &'static str {
        match self {
            RebaseReason::ProperFollowsEntity => {
                "proper time đi theo entity nên quy đổi theo tỉ lệ hai world"
            }
            RebaseReason::LocalAnchoredToOrigin => {
                "world_local neo vào world đã ký; con nợ bỏ trốn không làm nợ đáo hạn sớm"
            }
            RebaseReason::DivineIsAbsolute => {
                "divine là đồng hồ chung toàn multiverse, vốn đã tuyệt đối"
            }
            RebaseReason::LawOwnsThisClock => {
                "law_defined thuộc quyền của luật; engine tự đổi là vượt quyền"
            }
        }
    }
}

/// Biên bản một lần rebase — đi vào event của transfer.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RebaseAudit {
    /// Từng tiến trình.
    pub lines: Vec<RebaseLine>,
}

impl RebaseAudit {
    /// Biên bản có phủ hết danh sách tiến trình đầu vào không.
    ///
    /// Chỗ gọi dùng hàm này để khẳng định mình không sót. So theo **tập id**,
    /// không theo số lượng: hai tiến trình cùng tên bị gộp thì đếm vẫn khớp mà
    /// thực tế đã mất một cái.
    pub fn covers_all(&self, processes: &[Process]) -> bool {
        processes
            .iter()
            .all(|p| self.lines.iter().any(|l| l.process == p.id))
    }

    /// Những tiến trình đã thật sự đổi số.
    pub fn changed(&self) -> impl Iterator<Item = &RebaseLine> {
        self.lines.iter().filter(|l| l.before != l.after)
    }
}

/// Lỗi rebase.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RebaseError {
    /// Quy đổi tràn hoặc chia cho 0.
    #[error("rebase `{process}` lỗi số học: {source}")]
    Math {
        /// Tiến trình nào.
        process: String,
        /// Lỗi gốc.
        #[source]
        source: MathError,
    },
}

/// Vì sao một miền đổi hoặc không đổi. Bốn dòng, không có dòng thứ năm.
fn reason(domain: ClockDomain) -> RebaseReason {
    match domain {
        ClockDomain::Proper => RebaseReason::ProperFollowsEntity,
        ClockDomain::WorldLocal => RebaseReason::LocalAnchoredToOrigin,
        ClockDomain::Divine => RebaseReason::DivineIsAbsolute,
        ClockDomain::LawDefined => RebaseReason::LawOwnsThisClock,
    }
}

/// Rebase **mọi** tiến trình, trả về danh sách mới kèm biên bản.
///
/// Hoặc xong hết hoặc không đụng gì: transfer là nguyên tử (`§22.8`), nên một
/// nửa số deadline bị đổi rồi báo lỗi là trạng thái không được tồn tại.
pub fn rebase_processes(
    processes: &[Process],
    tu: &Clock,
    dich: &Clock,
) -> Result<(Vec<Process>, RebaseAudit), RebaseError> {
    let mut moi = Vec::with_capacity(processes.len());
    let mut audit = RebaseAudit::default();

    for p in processes {
        let sau = rebase(p.deadline, tu, dich).map_err(|e| RebaseError::Math {
            process: p.id.clone(),
            source: e,
        })?;
        audit.lines.push(RebaseLine {
            process: p.id.clone(),
            domain: p.deadline.domain,
            before: p.deadline.at.0,
            after: sau.at.0,
            reason: reason(p.deadline.domain),
        });
        moi.push(Process {
            id: p.id.clone(),
            deadline: sau,
        });
    }

    Ok((moi, audit))
}
