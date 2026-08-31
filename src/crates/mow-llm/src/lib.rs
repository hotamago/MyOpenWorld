//! # `mow-llm` — cong vao mo hinh ngon ngu
//!
//! Hai phan, va phan thu hai moi la phan quan trong:
//!
//! - [`client`] — bon che do goi (`LIVE`/`RECORD`/`REPLAY`/`STUB`).
//! - [`provider`] — provider that: `OpenRouter` va moi endpoint cung luoc do
//!   `OpenAI`. Truoc no, `LIVE` va `RECORD` chi tra ve `NoProvider`.
//! - [`embed`] — sinh vector cho chi muc ky uc. Co mot ban khong can mang.
//! - [`admission`] — **thoi diem ap ket qua**. Mot the the nghi o tick `T` thi
//!   hanh dong o tick `T + D`, bat ke mo hinh tra loi nhanh hay cham. Khong co
//!   phan nay, hai lan chay tu cung mot seed se cho hai the gioi khac nhau chi
//!   vi duong truyen khac nhau.
//!
//! Doc `admission` truoc neu ban chua biet vi sao no ton tai.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

pub mod admission;
pub mod client;
pub mod embed;
pub mod provider;

pub use admission::{AdmissionError, AdmissionLedger, Admitted, Call, CallState};
pub use client::{Gateway, LlmError, LlmResult, Mode, ModelClient, Request, Response};
pub use embed::{EmbedRole, Embedder, HashingEmbedder, HttpEmbedder};
pub use provider::{Attribution, HttpReply, OpenAiCompatClient, Transport, UreqTransport};
