//! # `mow-director` — storylet và biên niên sử hai lớp
//!
//! Hai module, và cả hai đều tồn tại để **không có lời giải thích nào do model
//! viết sau khi mọi chuyện đã xong** (`§22.17`).
//!
//! | Module | Câu hỏi nó trả lời | Bằng dữ liệu gì |
//! |---|---|---|
//! | [`storylet`] | vì sao chuyện này xảy ra, và vì sao chuyện kia **không** | vị từ trên state thật, salience có phân rã |
//! | [`chronicle`] | người ta tin gì, và **ai đã bẻ nó** | chuỗi kể lại, mỗi mắt có động cơ |
//!
//! Cột thứ ba là chỗ hai module này đắt hơn phương án rẻ. Phương án rẻ cho
//! storylet là một danh sách sự kiện và một lần tung xúc xắc; cho biên niên sử
//! là một trường `legend: String`. Cả hai đều chạy, và cả hai đều không trả lời
//! được câu hỏi ở cột thứ hai.

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

pub mod chronicle;
pub mod storylet;

pub use chronicle::{Chronicle, Divergence, Fact, Legend, Retelling};
pub use storylet::{Audit, Boost, Director, Perturbation, Precondition, Storylet, WorldFacts};
