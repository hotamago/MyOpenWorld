//! # `mow-effect` — hieu ung
//!
//! Ba bat bien, va ca ba deu ve mot chu de: **hieu ung khong duoc phep vo hinh
//! hay khong truy duoc**.
//!
//! - `§22.20` — effect chi tac dong qua [`modifier`] pipeline, khong bao gio
//!   ghi base stat. Nho vay cau hoi "vi sao suc manh cua toi la 12" luon co cau
//!   tra loi, va go mot lo nguyen khong can doan xem base stat le ra la bao nhieu.
//! - `§22.21` — moi de xuat di qua chuoi giam thieu **ward -> vat lieu -> khang**
//!   truoc khi thanh hieu ung da ap.
//! - `§22.22` — moi effect khai bao `perceptible_as`. Khong co effect vo hinh
//!   voi moi giac quan, vi mot the gioi co nhung the do sung sot khong ai dieu
//!   tra duoc thi khong choi duoc.
//!
//! [`disease`] la ung dung day du cua ca ba, cong mot phan tang hai muc de dich
//! benh chay duoc o quy mo thanh pho.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]

pub mod disease;
pub mod effect;
pub mod modifier;

pub use disease::{Compartments, Infection, Pathogen, Stage};
pub use effect::{
    mitigate, Effect, EffectError, EffectProposal, Mitigated, MitigationStep, Perceptible, Ward,
};
pub use modifier::{resolve, Modifier, Op, Resolved, Stacking, Step};
