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
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod determinism;
pub mod repro;

pub use determinism::{bisect, checkpoints_upto, compare, Divergence, Runnable, Verdict};
pub use repro::{Manifest, ReproBundle, ReproError, ReproResult};
