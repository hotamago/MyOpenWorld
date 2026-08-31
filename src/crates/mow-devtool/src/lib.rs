//! # `mow-devtool` — cong go loi cho agent
//!
//! `plan.md §P7` goi phan nay la "thu khien du an co the phat trien bang agent
//! thay vi bang nguoi ngoi choi thu", va no duoc xay o **Giai doan 0, truoc ca
//! gameplay**. Ly do thuc dung: mot he thong lon ma chi co con nguoi vao choi
//! de kiem tra thi vong lap phan hoi dai hang gio. Voi harness, no dai hang
//! giay.
//!
//! Crate nay **khong co trong ban phat hanh** (`§P10.5`). Feature `devtool` tat
//! mac dinh, va `deploy/docker/server.Dockerfile` co mot buoc chung minh dieu
//! do bang cach quet symbol trong binary — bien "chung toi tin la khong co"
//! thanh "build fail neu co".

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod budget;
pub mod determinism;
pub mod repro;
pub mod soak;

pub use budget::{
    active_at, check as check_budgets, Budget, BudgetReport, Failure, Measurement, Metric, Phase,
    BANG_NGAN_SACH,
};
pub use determinism::{bisect, checkpoints_upto, compare, Divergence, Runnable, Verdict};
pub use repro::{Manifest, ReproBundle, ReproError, ReproResult};
pub use soak::{
    health_report, Explanations, HealthReport, MemoryTrace, Sample, SoakRun, Warning, SO_NAM,
    SO_WORLD,
};
