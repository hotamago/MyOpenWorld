//! # `mow-culture` — thông điệp, tôn giáo, thế giới ngầm
//!
//! Ba module, một câu hỏi chung: **cái gì làm người ta tin và làm theo?**
//!
//! | Module | Câu trả lời | Cái bị thay thế |
//! |---|---|---|
//! | [`message`] | tùy **xu hướng bắt chước** | một hệ số lan truyền duy nhất |
//! | [`religion`] | **bằng chứng tốn kém** người khác đã trả | `faith_point` |
//! | [`underworld`] | cơ hội hợp pháp **không còn** | một loại entity "băng đảng" |
//!
//! Cột thứ ba là cột đáng đọc. Mỗi dòng là một thiết kế rẻ hơn, chạy được, và
//! xóa mất chính thứ đáng chơi:
//!
//! - Một hệ số lan truyền làm thời trang và tín ngưỡng lan như nhau.
//! - `faith_point` làm giảng đạo suông hiệu quả ngang với hành hương ba tháng.
//! - Một loại entity riêng cho băng đảng làm cho việc chúng mua chức quan trở
//!   thành một chỗ nối giữa hai hệ thống — và chỗ nối đó là chỗ hỏng.

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

pub mod message;
pub mod religion;
pub mod underworld;

pub use message::{
    consider, dominant_version, Adoption, Bias, Reception, Rumour, SocialEvidence, Translation,
};
pub use religion::{conviction, credibility, Doctrine, Observance, Religion, Rite};
pub use underworld::{
    black_market_price, gambling_craving, recruitment_pool, Addiction, Cohort, Racket, Substance,
    Syndicate,
};
