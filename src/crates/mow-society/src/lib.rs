//! # `mow-society` — kinh te va doi song
//!
//! Hai nguyen tac, va ca hai deu chong lai cung mot cam do: **khai bao thay vi
//! mo phong**.
//!
//! - [`economy`] — tai nguyen co **nguon that**, huu han va tai tao co toc do.
//!   Gia **hinh thanh** tu cung cau, khong duoc dat (`§22.35`).
//! - [`household`] — dia diem co **hang doi that**, va do thi tiep xuc **noi
//!   len tu hanh vi** thay vi duoc khai bao.
//!
//! Cam do o ca hai cho la giong nhau: cho cua hang "luon co 50 o banh", hoac
//! noi "hai nguoi nay quen nhau". Ca hai deu tien, va ca hai deu xoa mat chinh
//! thu ma the gioi nay ton tai de tao ra.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

pub mod drift;
pub mod economy;
pub mod household;
pub mod personality;
pub mod reputation;
pub mod social;

pub use drift::{Act, ActiveCause, DriftAuditor, DriftReport, Finding, Verdict};
pub use economy::{Market, Order, Recipe, Source, Trade};
pub use household::{ContactGraph, Household, HouseholdStage, Kinship, Place, PlaceKind};
pub use personality::{
    Affect, CauseKind, CauseRef, Clinical, Personality, SelfNarrative, TraitChange, TraitField,
    Traits, Values,
};
pub use reputation::{Belief, Norm, NormOrder, NormSet, Reputation, ReputationKey};
pub use social::{
    apply_outcome, volition, Bond, Exchange, ExchangeKind, Payer, SocialState, Volition,
};
