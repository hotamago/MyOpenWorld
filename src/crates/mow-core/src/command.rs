//! Command và mã thất bại (`idea.md §10.5`, `plan.md §P10.1`).
//!
//! Một hành động thất bại phải trả về **mã thất bại có trong registry**, không
//! được im lặng bỏ qua. Lý do rất thực tế: khi một NPC đứng yên suốt ba ngày
//! mô phỏng, câu hỏi duy nhất đáng hỏi là "nó *đã thử* làm gì và vì sao không
//! được". Nếu thất bại không để lại dấu vết thì câu hỏi đó không trả lời được,
//! và bạn sẽ đi đọc log LLM để đoán — sai hướng, tốn hàng giờ.

use crate::ids::{EntityId, WorldId};
use crate::value::Value;
use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Loại command, có namespace giống [`crate::event::EventKind`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandKind(pub String);

impl CommandKind {
    /// Dựng.
    pub fn of(s: &str) -> CommandKind {
        CommandKind(s.to_owned())
    }

    /// Namespace.
    pub fn namespace(&self) -> &str {
        self.0.split('.').next().unwrap_or("")
    }
}

impl CanonicalHash for CommandKind {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.0);
    }
}

/// Một yêu cầu thay đổi thế giới.
///
/// Command **không phải** là sự kiện: nó là *ý định*, và ý định có thể bị từ
/// chối. Sự kiện là thứ đã xảy ra. Trộn hai khái niệm này là lỗi kiến trúc phổ
/// biến nhất trong event sourcing, và nó biểu hiện thành nhật ký chứa những
/// việc chưa từng xảy ra.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    /// Loại.
    pub kind: CommandKind,
    /// Thế giới đích.
    pub world: WorldId,
    /// Ai yêu cầu. `None` nghĩa là engine hoặc True God.
    pub actor: Option<EntityId>,
    /// Tham số.
    pub payload: Value,
    /// Khóa idempotency (`§20.2.2`).
    ///
    /// Cùng một `request_id` gửi hai lần chỉ được có tác dụng một lần. Cần thiết
    /// vì kết quả LLM có thể tới muộn, tới hai lần, hoặc tới sau khi đã fallback.
    pub request_id: Option<u64>,
}

impl Command {
    /// Dựng.
    pub fn new(kind: &str, world: WorldId, payload: Value) -> Command {
        Command {
            kind: CommandKind::of(kind),
            world,
            actor: None,
            payload,
            request_id: None,
        }
    }

    /// Gắn chủ thể.
    #[must_use]
    pub fn by(mut self, actor: EntityId) -> Command {
        self.actor = Some(actor);
        self
    }

    /// Gắn khóa idempotency.
    #[must_use]
    pub fn with_request_id(mut self, id: u64) -> Command {
        self.request_id = Some(id);
        self
    }
}

impl CanonicalHash for Command {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.kind.canonical_hash(h);
        self.world.canonical_hash(h);
        self.actor.canonical_hash(h);
        self.payload.canonical_hash(h);
        h.write_option(self.request_id, |hh, v| {
            hh.write_u64(v);
        });
    }
}

/// Mã thất bại. Là dữ liệu, không phải chuỗi tự do.
///
/// Chuỗi tự do sẽ trở thành `"failed"`, `"could not"`, `"Failed."` và ba biến
/// thể nữa của cùng một nguyên nhân, rồi không thống kê được gì.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    /// Không có handler nào nhận loại command này.
    UnknownCommand,
    /// Payload thiếu trường hoặc sai kiểu.
    MalformedPayload,
    /// Thực thể được nhắc tới không tồn tại.
    NoSuchEntity,
    /// Điều kiện tiên quyết không thỏa (`§10.5`).
    PreconditionFailed,
    /// Chủ thể không biết hành động này (`§22.4`).
    ActionNotKnown,
    /// Không đủ quyền — chủ sở hữu, chức vụ, hoặc capability của plugin.
    Forbidden,
    /// Không đủ tài nguyên: vật liệu, tiền, sức, nhiên liệu WASM.
    Insufficient,
    /// Vi phạm ràng buộc ưng thuận ở tầng engine (`§12.7.2`, `§22.26`).
    ///
    /// Mã riêng chứ không gộp vào [`FailureCode::Forbidden`]: đây là ràng buộc
    /// **không plugin và không override nào cấp ngoại lệ được**, nên nó phải
    /// phân biệt được trong thống kê và trong log kiểm toán.
    ConsentViolation,
    /// Tràn số hoặc lỗi miền số học.
    Arithmetic,
    /// Vi phạm bất biến — đây là **bug của engine**, không phải lỗi người chơi.
    InvariantViolated,
    /// Command trùng `request_id` đã xử lý.
    DuplicateRequest,
}

impl CanonicalHash for FailureCode {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

impl FailureCode {
    /// Tên ổn định, dùng trong log, metric và trên đường truyền.
    pub fn as_str(&self) -> &'static str {
        match self {
            FailureCode::UnknownCommand => "unknown_command",
            FailureCode::MalformedPayload => "malformed_payload",
            FailureCode::NoSuchEntity => "no_such_entity",
            FailureCode::PreconditionFailed => "precondition_failed",
            FailureCode::ActionNotKnown => "action_not_known",
            FailureCode::Forbidden => "forbidden",
            FailureCode::Insufficient => "insufficient",
            FailureCode::ConsentViolation => "consent_violation",
            FailureCode::Arithmetic => "arithmetic",
            FailureCode::InvariantViolated => "invariant_violated",
            FailureCode::DuplicateRequest => "duplicate_request",
        }
    }
}

/// Một thất bại, kèm chi tiết đọc được.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    /// Mã.
    pub code: FailureCode,
    /// Giải thích cho người đọc. **Không** dùng để phân nhánh logic.
    pub detail: String,
}

impl Failure {
    /// Dựng.
    pub fn new(code: FailureCode, detail: impl Into<String>) -> Failure {
        Failure {
            code,
            detail: detail.into(),
        }
    }

    /// Thiếu trường trong payload.
    pub fn missing(field: &str) -> Failure {
        Failure::new(
            FailureCode::MalformedPayload,
            format!("thiếu trường `{field}`"),
        )
    }

    /// Sai kiểu.
    pub fn wrong_type(field: &str, mong_doi: &str, thuc_te: &str) -> Failure {
        Failure::new(
            FailureCode::MalformedPayload,
            format!("`{field}` phải là {mong_doi}, nhận được {thuc_te}"),
        )
    }
}

impl core::fmt::Display for Failure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.detail)
    }
}

impl std::error::Error for Failure {}

/// Kết quả của việc áp một command.
pub type CommandResult<T> = Result<T, Failure>;
