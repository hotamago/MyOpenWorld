//! # `mow-schema` — kieu content
//!
//! Noi dung cua crate nay **duoc sinh ra** tu `schemas/content/*.json` bang
//! pipeline 2 (`plan.md §P4.1`). Dung sua `generated.rs` bang tay: CI so ma
//! sinh voi ma da commit va se do.

#![deny(missing_docs)]

mod generated;

pub use generated::CONTENT_SCHEMAS;
