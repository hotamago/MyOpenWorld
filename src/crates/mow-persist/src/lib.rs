//! # `mow-persist` — lưu trữ bền
//!
//! Crate này có **một** trait và **một** hiện thực (SQLite), cộng một bộ test
//! hợp đồng viết sẵn. Đó là chủ đích, và nó là kết quả của một tranh luận cụ
//! thể trong `plan.md §P3.4`.
//!
//! Cám dỗ tự nhiên là dựng cả Postgres lẫn SQLite ngay từ Giai đoạn 0 "cho
//! xong". Nhưng viết hai hiện thực trước khi có một workload thật nào uốn nắn
//! interface thì cả hai đều sẽ sai theo cùng một kiểu, và ta phải sửa hai chỗ.
//! Ngược lại, để tới Giai đoạn C mới *nghĩ* tới trait thì lúc đó code đã bám
//! chặt vào SQLite và việc tách ra sẽ là một cuộc đại phẫu.
//!
//! Đường giữa: **đường nối có từ đầu, hiện thực chỉ một**. Bộ test hợp đồng ở
//! [`contract`] được viết ngay bây giờ, chạy trên SQLite ngay bây giờ, và tới
//! `PC-20` thì Postgres phải vượt qua **đúng bộ đó, không sửa một dòng**. Nếu
//! nó phải sửa thì trait đã rò rỉ chi tiết cài đặt, và đó chính là thứ ta muốn
//! phát hiện.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod contract;
pub mod error;
pub mod sqlite;
pub mod store;

pub use error::{PersistError, PersistResult};
pub use sqlite::SqliteStore;
pub use store::{BranchRecord, EventRecord, Snapshot, Store};
