//! # `mow-econ` — sở hữu, tiền tệ, tín dụng, lao động, vận chuyển
//!
//! Bốn module, và mỗi cái xoay quanh **một** sự tách đôi mà nếu gộp lại thì cả
//! một lớp hiện tượng biến mất:
//!
//! | Module | Tách đôi | Gộp lại thì mất gì |
//! |---|---|---|
//! | [`property`] | possession ≠ claim | trộm cắp, đồ gian, tranh chấp thừa kế |
//! | [`money`] | mệnh giá ≠ giá trị nội tại | pha loãng xu, luật Gresham, lạm phát có nguyên nhân |
//! | [`credit`] | nợ ≠ tài sản | khủng hoảng dây chuyền |
//! | [`logistics`] | kho A ≠ kho B | địa lý kinh tế, cướp đường, thành phố cảng |
//!
//! Dòng cuối là dòng dễ mắc nhất: bản đầu tiên của mọi hệ thống kinh tế đều cho
//! hàng teleport giữa hai kho, và nó *chạy* — chỉ là khoảng cách thôi không tốn
//! gì nữa, nên không còn lý do gì để có thương nhân.

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

pub mod credit;
pub mod logistics;
pub mod money;
pub mod property;

pub use credit::{Ledger, Loan, Seniority};
pub use logistics::{specialize, Handover, LabourContract, Leg, ObservedTrade, Progress, Shipment};
pub use money::{Coinage, EconomyProfile, Faucet, MonetaryStage, MoneyDiagnosis, Sink};
pub use property::{Basis, Claim, Ownership, Right, RIGHTS};
