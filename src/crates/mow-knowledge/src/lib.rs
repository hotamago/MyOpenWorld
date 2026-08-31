//! # `mow-knowledge` — đồ thị tri thức, truyền dạy, sách, thể chế
//!
//! Ba module, một đường đi:
//!
//! ```text
//!  graph      cái gì có thể biết      ── blockers() nói THIẾU GÌ, không nói "chưa đủ điều kiện"
//!    │
//!  teaching   biết thì truyền thế nào ── truyền dạy LÀM MẤT độ chính xác, luôn luôn
//!    │
//!  school     xã hội truyền ra sao    ── gác cửa quyết định ai lên được địa vị
//! ```
//!
//! ## Sợi chỉ xuyên suốt: **độ chính xác là một trường**
//!
//! Không phải mọi tri thức trong world đều đúng như nhau. `fidelity` đi từ nguồn
//! qua từng lần dạy, từng lần chép, từng thế hệ — và nó **chỉ giảm**. Muốn tăng
//! thì phải nghiên cứu lại từ bằng chứng, không phải học lại từ thầy.
//!
//! Từ một trường đó rơi ra: phê bình văn bản, bản gốc thất lạc, dị giáo sinh ra
//! từ một lỗi dịch, và hai trường phái cùng tin mình chính thống. Bỏ trường đó
//! đi thì cả bốn biến mất cùng lúc, và tri thức trong world trở thành một danh
//! sách cờ bật/tắt.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]

pub mod graph;
pub mod school;
pub mod teaching;

pub use graph::{blockers, Blocker, KnowledgeGraph, Level, Node, Requirements, Understanding};
pub use school::{Archive, Examination, Institution, Lineage, Rejection};
pub use teaching::{read, teach, Corpus, Learner, Setting, Taught, Teacher, Text};
