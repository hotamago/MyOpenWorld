//! # `mow-scenario` — DSL kich ban
//!
//! Kich ban la YAML `given/when/then`, chay duoc ca trong CI lan qua MCP.
//!
//! Bon quy tac cua phan `bind` (`plan.md §P7.3`) deu duoc thi hanh o day, va
//! moi quy tac ngan mot cach that bai cu the:
//!
//! 1. **Bo chon phai co thu tu toan phan** — `order` bat buoc ket thuc bang
//!    `id asc`. Thieu no thi hai lan chay co the chon hai thuc the khac nhau,
//!    va kich ban tro nen chap chon. Mot kich ban chap chon te hon khong co
//!    kich ban, vi no day ca doi bo qua mau do.
//! 2. **Khong khop la LOI**, khong phai bo qua. Mot kich ban xanh vi bo chon
//!    khong tim thay ai la loai ket qua sai te nhat.
//! 3. **Ket qua rang buoc duoc ghi vao bao cao** — alias nao tro toi id nao —
//!    de khi kich ban do thi doc log la biet no dang noi ve ai.
//! 4. **Khong viet id tho vao kich ban**: id sinh ra tu genesis va se doi khi
//!    worldseed doi, con bo chon thi van dung.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod genesis;
pub mod model;
pub mod predicate;
pub mod prehistory;
pub mod runner;
pub mod slice;
pub mod testing;
pub mod vault;
pub mod worldseed;

pub use genesis::{GenesisError, GenesisResult};
pub use model::{Assertion, Binding, LlmMode, Scenario, Step};
pub use predicate::{Op, Predicate, Term, Val};
pub use prehistory::{
    detail_chunk, run_prehistory, ChunkDetail, ChunkError, MacroDelta, MacroEvent, MacroKind,
    PrehistoryConfig, TICK_MOI_NAM,
};
pub use runner::{run, AssertResult, Report, RunError, WorldFactory};
pub use slice::{act, build_empty_world, build_slice_world, observe_as, preview, Preview};
pub use vault::{Bundle, Difference, Impact, Risk, SeedVault, VaultEntry, VaultError};
pub use worldseed::{GenesisStep, Lockfile, Worldseed};
