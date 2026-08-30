//! # `mow-core` — hạt nhân mô phỏng
//!
//! Crate này giữ **bất biến số 1** của toàn hệ thống (`idea.md §22.1`):
//!
//! > Một state change authoritative chỉ được commit qua simulation/transaction
//! > handler.
//!
//! Bất biến kiểu đó thường được viết vào tài liệu rồi bị vi phạm ở tuần thứ ba,
//! vì ai đó cần "sửa nhanh một chỗ". Ở đây nó được thực thi bằng trình biên
//! dịch, qua ba lớp:
//!
//! 1. Hàm ghi của [`ecs::Store`] là `pub(crate)` — ngoài crate không gọi được.
//! 2. [`sim::Sim`] không có `store_mut()`, và sẽ không bao giờ có.
//! 3. [`transaction::Handler`] nhận [`transaction::Ctx`] **chỉ đọc** và trả về
//!    một danh sách [`transaction::Mutation`]. Việc áp diễn ra sau khi handler
//!    đã thành công, nên một handler chết giữa chừng không để lại thế giới ở
//!    trạng thái sửa dở.
//!
//! ## Đường đi của một thay đổi
//!
//! ```text
//!   Command ──► Sim::apply
//!                 │
//!                 ├─ idempotency: request_id đã thấy chưa?      §20.2.2
//!                 ├─ tra handler theo kind                       §10.5
//!                 ├─ chạy handler với Ctx CHỈ ĐỌC
//!                 │     └─ handler đẩy Mutation + EventDraft
//!                 ├─ kiểm TOÀN BỘ mutation trước khi áp cái đầu
//!                 ├─ áp mutation                                 §22.1
//!                 ├─ ghi event (chỉ ghi thêm)                    §8.4
//!                 └─ ghi nhận request_id
//! ```
//!
//! Thất bại ở bất kỳ bước nào đều trả thế giới về nguyên trạng, kể cả bộ cấp
//! phát định danh — nếu không, số lần command *thất bại* sẽ lọt vào state hash
//! và hai lần chạy giống hệt nhau lại cho hai thế giới khác nhau.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod clock;
pub mod command;
pub mod ecs;
pub mod event;
pub mod ids;
pub mod invariant;
pub mod sim;
pub mod transaction;
pub mod value;

pub use clock::{Clock, ClockDomain, Deadline, Tick, TickSpan};
pub use command::{Command, CommandKind, CommandResult, Failure, FailureCode};
pub use ecs::{AttrKey, Attrs, Identity, Store};
pub use event::{Event, EventDraft, EventKind, EventLog, EventSeq};
pub use ids::{BranchId, EntityId, IdAllocator, PackId, StableKey, WorldId};
pub use invariant::{Cost, Invariant, InvariantReport, InvariantRunner, Violation};
pub use sim::{Sim, SimConfig};
pub use transaction::{Committed, Ctx, Handler, HandlerRegistry, Mutation};
pub use value::Value;
