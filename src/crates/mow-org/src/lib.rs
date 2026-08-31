//! # `mow-org` — tổ chức, nhà nước, chính danh, hành động tập thể, tài nguyên chung
//!
//! Bốn module, và chúng nối vào nhau thành một vòng phản hồi mà không ai phải
//! viết riêng:
//!
//! ```text
//!   state      năng lực nhà nước ──► coverage_by_district ──► mow-law
//!     ▲                                                          │
//!     │                                                          ▼
//!  thuế thu được ◄── kinh tế ◄── trật tự ◄── tuân thủ ◄── legitimacy
//!                                                             ▲
//!  collective  ──► nổi dậy ──────────────────────────────────┘
//!  commons     ──► tài nguyên cạn ──► đói ──► động cơ phạm tội
//! ```
//!
//! Cắt thuế thì độ phủ tụt, trộm cắp tăng, thương nhân bỏ đi, thuế còn ít hơn.
//! Không có dòng nào trong crate này viết "vòng xoáy suy tàn" — nó là hệ quả.
//!
//! Bốn quyết định thiết kế, mỗi cái ở module tương ứng:
//!
//! - [`state`] — vì sao `coverage_by_district` phải **sinh ra**, không viết tay.
//! - [`legitimacy`] — vì sao ba động cơ tuân thủ không được gộp thành một chỉ số.
//! - [`collective`] — vì sao **phân bố** ngưỡng quyết định, không phải trung bình.
//! - [`commons`] — vì sao mỗi yếu tố quản trị thiếu phải hỏng theo kiểu riêng.

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

pub mod collective;
pub mod commons;
pub mod crisis;
pub mod legitimacy;
pub mod state;

pub use collective::{cascade, Cascade, Participant, Signal};
pub use commons::{diagnose, Commons, Diagnosis, Governance, Harvest, Principle, PRINCIPLES};
pub use crisis::{
    decide, displacement_belief, flows, respond, Aftermath, Belief, Capacity, Diaspora, Household,
    Magnitude, Role, NGUONG_SUP_DO,
};
pub use legitimacy::{Compliance, Legitimacy, Motive, Source};
pub use state::{Directive, District, Outcome, StateCapacity};
