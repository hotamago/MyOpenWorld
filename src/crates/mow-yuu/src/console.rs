//! Console True God: query, proposal, command (`idea.md §15.5`, `§16`, `PF-08`).
//!
//! ## Ba loại lệnh, và vì sao phải phân biệt
//!
//! `§15.5`:
//!
//! | Loại | Làm gì | Đổi state không |
//! |---|---|---|
//! | [`Request::Query`] | chỉ phân tích | **không** |
//! | [`Request::Proposal`] | tạo preview/plan, chờ commit | chưa |
//! | [`Request::Command`] | thực hiện ngay | có, **vẫn qua transaction và log** |
//!
//! Dòng cuối là dòng dễ mất nhất. "True God yêu cầu thực hiện ngay" nghe như
//! một lý do chính đáng để bỏ qua transaction — và bỏ qua nó thì một thao tác
//! của người chơi không nằm trong nhật ký, không replay được, và `INV-22-9`
//! hỏng ở đúng chỗ khó tìm nhất.
//!
//! Nên [`Outcome::Committed`] **luôn** mang một `EventSeq`. Không có nhánh nào
//! đổi state mà không sinh event.
//!
//! ## Tự snapshot trước thay đổi phá hủy diện rộng
//!
//! `§15.5`: *"Với thay đổi phá hủy diện rộng, Yuu **tự** snapshot trước
//! commit"*. Chữ "tự" nghĩa là người dùng không phải nhớ — và đó là chỗ quy
//! tắc này có giá trị, vì người sắp xóa một lục địa thường đang tập trung vào
//! việc xóa lục địa.
//!
//! Ngưỡng ở [`Plan::is_destructive`], và nó dựa vào **phạm vi**, không dựa vào
//! ý định: một thao tác chạm 10 000 thực thể là phá hủy diện rộng dù người
//! dùng gọi nó là "dọn dẹp".
//!
//! ## Mơ hồ thì đề xuất, không đoán rồi làm
//!
//! `§15.5`: *"Nếu yêu cầu mơ hồ nhưng có thể suy ra từ state, Yuu tự tạo
//! phương án mặc định an toàn và **trình preview**"*. Nên
//! [`Console::handle`] hạ một `Command` mơ hồ xuống thành một `Proposal` chứ
//! không đoán rồi thi hành.

use mow_core::{BranchId, EntityId, EventSeq, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Ba mức can thiệp (`§16.2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intervention {
    /// Qua avatar, phép, sứ giả — **cư dân cảm nhận được**.
    Diegetic,
    /// Sửa dữ liệu có provenance `true_god`; cư dân chỉ biết nếu có observation.
    Administrative,
    /// Bỏ qua physical law — **nhưng không bỏ qua engine invariant**.
    HardOverride,
}

impl Intervention {
    /// Mức này có bỏ qua được engine invariant không.
    ///
    /// **Không, không mức nào.** `§16.2` phân biệt rõ: engine invariant là
    /// *host safety policy đứng ngoài simulation*, không phải một sức mạnh lớn
    /// hơn tồn tại trong thế giới. Hàm này tồn tại để câu trả lời nằm trong
    /// code chứ không nằm trong trí nhớ người viết handler.
    pub fn bypasses_engine_invariants(self) -> bool {
        false
    }

    /// Cư dân có cơ hội biết chuyện gì đã xảy ra không.
    pub fn observable_in_world(self) -> bool {
        matches!(self, Intervention::Diegetic)
    }
}

/// Một thao tác trong kế hoạch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// Đặt một thuộc tính.
    SetAttr {
        /// Thực thể nào.
        entity: EntityId,
        /// Khóa.
        key: String,
        /// Giá trị mới.
        value: i64,
    },
    /// Tạo thực thể.
    Spawn {
        /// Loài.
        species: String,
        /// Bao nhiêu.
        count: u32,
    },
    /// Xóa thực thể.
    Despawn {
        /// Những ai.
        entities: Vec<EntityId>,
    },
    /// Sửa một định nghĩa trong registry.
    RedefineContent {
        /// Id nào.
        id: String,
    },
}

impl Op {
    /// Thao tác này chạm bao nhiêu thực thể.
    pub fn scope(&self) -> u64 {
        match self {
            Op::SetAttr { .. } => 1,
            Op::Spawn { count, .. } => u64::from(*count),
            Op::Despawn { entities } => entities.len() as u64,
            // Sửa một định nghĩa chạm **mọi** thực thể dùng nó. Không biết là
            // bao nhiêu, nên coi như lớn — nghiêng về phía snapshot.
            Op::RedefineContent { .. } => u64::MAX,
        }
    }

    /// Thao tác này có xóa thứ gì không.
    pub fn destroys(&self) -> bool {
        matches!(self, Op::Despawn { .. } | Op::RedefineContent { .. })
    }
}

/// Số thực thể bị chạm để một kế hoạch được coi là phá hủy diện rộng.
///
/// 1000 — cỡ dân số một thị trấn. Dưới mức đó, một sai lầm sửa tay được; trên
/// mức đó thì không ai dựng lại nổi bằng tay.
pub const NGUONG_PHA_HUY_DIEN_RONG: u64 = 1_000;

/// Một kế hoạch chưa commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Mô tả ngắn.
    pub summary: String,
    /// Mức can thiệp.
    pub intervention: Intervention,
    /// Các thao tác.
    pub ops: Vec<Op>,
}

impl Plan {
    /// Tổng số thực thể bị chạm.
    pub fn total_scope(&self) -> u64 {
        self.ops
            .iter()
            .fold(0u64, |a, o| a.saturating_add(o.scope()))
    }

    /// **Có phải thay đổi phá hủy diện rộng không** (`§15.5`).
    ///
    /// Dựa vào phạm vi, **không** dựa vào ý định: một thao tác chạm 10 000
    /// thực thể là phá hủy diện rộng dù người dùng gọi nó là "dọn dẹp".
    pub fn is_destructive(&self) -> bool {
        self.ops.iter().any(Op::destroys) && self.total_scope() >= NGUONG_PHA_HUY_DIEN_RONG
    }
}

/// Yêu cầu gửi tới console (`§15.5`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Request {
    /// Chỉ phân tích.
    Query {
        /// Hỏi gì.
        question: String,
    },
    /// Tạo preview, chờ commit.
    Proposal {
        /// Kế hoạch.
        plan: Plan,
    },
    /// Thực hiện ngay.
    Command {
        /// Kế hoạch.
        plan: Plan,
        /// Người dùng có nói rõ mọi tham số không.
        ///
        /// Mơ hồ thì `§15.5` bảo **trình preview**, không đoán rồi làm.
        unambiguous: bool,
    },
}

/// Ảnh chụp tự động trước một thay đổi lớn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Chụp ở tick nào.
    pub at: Tick,
    /// Nhánh nào.
    pub branch: BranchId,
    /// Vì sao chụp — **tự động** hay do người yêu cầu.
    pub reason: SnapshotReason,
}

/// Vì sao có ảnh chụp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotReason {
    /// Yuu tự chụp vì kế hoạch phá hủy diện rộng (`§15.5`).
    AutomaticBeforeDestructive,
    /// Người dùng yêu cầu.
    UserRequested,
}

/// Kết quả xử lý một yêu cầu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Trả lời một câu hỏi. **Không đổi state.**
    Answer {
        /// Nội dung.
        text: String,
    },
    /// Trình preview, chờ người dùng bấm commit.
    Preview {
        /// Kế hoạch.
        plan: Plan,
        /// Bao nhiêu thực thể bị chạm.
        scope: u64,
        /// Sẽ tự chụp ảnh trước không.
        will_snapshot: bool,
        /// Vì sao chỉ là preview chứ chưa làm.
        reason: PreviewReason,
    },
    /// Đã commit.
    ///
    /// **Luôn** mang một `EventSeq` — không có nhánh nào đổi state mà không
    /// sinh event.
    Committed {
        /// Event ghi lại việc này.
        event: EventSeq,
        /// Ảnh chụp trước đó, nếu có.
        snapshot: Option<Snapshot>,
    },
}

/// Vì sao một yêu cầu chỉ ra preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewReason {
    /// Người dùng gửi `Proposal`, tức là đã chọn xem trước.
    Requested,
    /// Yêu cầu mơ hồ, Yuu hạ xuống preview thay vì đoán (`§15.5`).
    AmbiguousRequest,
}

/// Vì sao một yêu cầu bị từ chối.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConsoleError {
    /// Không chụp được ảnh trước một thay đổi phá hủy.
    #[error(
        "kế hoạch phá hủy {scope} thực thể nhưng không chụp được ảnh trước — \
         §15.5 bắt tự snapshot, và một thay đổi không hoàn tác được thì không commit"
    )]
    SnapshotFailed {
        /// Phạm vi.
        scope: u64,
    },
}

/// Nhật ký của console — để rollback và để audit view lọc theo provenance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleLog {
    /// Mọi lần commit, theo thứ tự.
    pub commits: Vec<(EventSeq, Plan, Option<Snapshot>)>,
}

impl ConsoleLog {
    /// Những lần commit có ảnh chụp — tức là những chỗ **rollback được**.
    pub fn rollback_points(&self) -> Vec<(EventSeq, &Snapshot)> {
        self.commits
            .iter()
            .filter_map(|(e, _, s)| s.as_ref().map(|s| (*e, s)))
            .collect()
    }

    /// Rollback về ngay trước một lần commit.
    ///
    /// Chỉ về được những chỗ có ảnh chụp. Trả `None` chứ không cố dựng lại từ
    /// event — dựng lại được thì tốt, nhưng "có lẽ dựng lại được" không phải
    /// thứ để hứa với người vừa xóa nhầm một lục địa.
    pub fn rollback_to(&self, event: EventSeq) -> Option<&Snapshot> {
        self.commits
            .iter()
            .find(|(e, _, _)| *e == event)
            .and_then(|(_, _, s)| s.as_ref())
    }
}

/// Console True God (`§15.5`, `§16`).
#[derive(Debug, Clone, Default)]
pub struct Console {
    log: ConsoleLog,
    next_event: u64,
    next_tick: u64,
}

impl Console {
    /// Console mới.
    pub fn new() -> Console {
        Console::default()
    }

    /// Nhật ký.
    pub fn log(&self) -> &ConsoleLog {
        &self.log
    }

    /// Xử lý một yêu cầu (`§15.5`).
    ///
    /// `answer` là hàm trả lời câu hỏi — truyền vào để console không phải biết
    /// cách đọc state, và để một `Query` **không thể** đổi state: hàm nhận
    /// `&str` và trả `String`, không có `&mut` nào ở đây.
    pub fn handle(
        &mut self,
        req: Request,
        answer: impl Fn(&str) -> String,
    ) -> Result<Outcome, ConsoleError> {
        match req {
            Request::Query { question } => Ok(Outcome::Answer {
                text: answer(&question),
            }),

            Request::Proposal { plan } => Ok(self.preview(plan, PreviewReason::Requested)),

            // Mơ hồ thì hạ xuống preview, **không đoán rồi làm**.
            Request::Command {
                plan,
                unambiguous: false,
            } => Ok(self.preview(plan, PreviewReason::AmbiguousRequest)),

            Request::Command {
                plan,
                unambiguous: true,
            } => self.commit(plan),
        }
    }

    fn preview(&self, plan: Plan, reason: PreviewReason) -> Outcome {
        Outcome::Preview {
            scope: plan.total_scope(),
            will_snapshot: plan.is_destructive(),
            reason,
            plan,
        }
    }

    /// Commit một kế hoạch — **vẫn qua transaction và log** (`§15.5`).
    pub fn commit(&mut self, plan: Plan) -> Result<Outcome, ConsoleError> {
        // Tự chụp trước, nếu phá hủy diện rộng.
        let snapshot = if plan.is_destructive() {
            self.next_tick += 1;
            Some(Snapshot {
                at: Tick(self.next_tick),
                branch: BranchId(0),
                reason: SnapshotReason::AutomaticBeforeDestructive,
            })
        } else {
            None
        };

        self.next_event += 1;
        let event = EventSeq(self.next_event);
        self.log.commits.push((event, plan, snapshot.clone()));
        Ok(Outcome::Committed { event, snapshot })
    }
}

/// Tra provenance: thao tác nào của console sinh ra event nào.
///
/// Đây là thứ audit view ở `§18.12` lọc theo — người chơi phải phân biệt được
/// *"chuyện này xảy ra vì thế giới vận hành thế"* và *"chuyện này xảy ra vì tôi
/// đã bấm nút"*.
pub fn provenance(log: &ConsoleLog) -> BTreeMap<EventSeq, Intervention> {
    log.commits
        .iter()
        .map(|(e, p, _)| (*e, p.intervention))
        .collect()
}
