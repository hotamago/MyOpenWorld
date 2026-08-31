//! # `mow-yuu` — control plane Yuu và console True God
//!
//! `§15.1`: Yuu **không phải một LLM toàn quyền duy nhất**. Nó là một control
//! plane gồm các module có quyền hạn rõ, và:
//!
//! > Một persona "Yuu" thống nhất giao tiếp với người chơi, nhưng bên trong
//! > các module **không chia sẻ quyền tùy tiện**.
//!
//! Nên crate này không có một kiểu `YuuContext` toàn năng. Năm nhóm module,
//! mỗi cái nhận đúng thứ nó cần:
//!
//! - [`forge`] — World Architect, Species Foundry (có viability check),
//!   Law Forge (có sandbox). `§15.1`–`§15.3`, `PF-06`.
//! - [`audit`] — Auditor dùng **chung** bộ invariant với harness; Historian chỉ
//!   dùng event có thật. `§22.17`, `PF-07`.
//! - [`console`] — query/proposal/command, preview, tự snapshot, rollback.
//!   `§15.5`, `§16`, `PF-08`.
//! - [`possession`] — hóa thân, phân tầng prompt, provenance. `§16.3`, `§16.4`,
//!   `PF-09`.
//! - [`dossier`]/[`prompt`]/[`parse`]/[`without_model`]/[`answer`]/[`yuu`] —
//!   tư vấn cho True God (`§3.1` bước 2, `§1.2.4`). Engine truyền một
//!   [`Dossier`]; [`Yuu`] chỉ được nói những gì trích được về một sự kiện có
//!   thật trong đó, và câu nào không trích được thì bị cắt — lý do luôn hiện
//!   trong `Answer::stripped`, không bao giờ giấu.
//!
//! Không module nào ở đây commit state. Chúng sinh ra **đề xuất** và **báo
//! cáo**; `INV-22-1` giữ nguyên hiệu lực cho Yuu như cho mọi thứ khác.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod answer;
pub mod audit;
pub mod console;
pub mod dossier;
pub mod forge;
pub mod parse;
pub mod possession;
pub mod prompt;
pub mod without_model;
pub mod yuu;

pub use answer::{Answer, Proposal, StripReason, Stripped};
pub use audit::{AuditReport, Auditor, Channels, Chronicle, ChronicleError, Finding, Line};
pub use console::{
    provenance, Console, ConsoleError, ConsoleLog, Intervention, Op, Outcome, Plan, PreviewReason,
    Request, Snapshot, SnapshotReason, NGUONG_PHA_HUY_DIEN_RONG,
};
pub use dossier::{Dossier, EventBrief, FolkBrief};
pub use forge::{
    Conditions, ForgeError, ForgedLaw, Inviable, LawForge, SpeciesDraft, SpeciesFoundry,
    WorldTemplate, TRAN_KCAL_MOI_NGAY,
};
pub use parse::read_answer;
pub use possession::{
    Consent, EmbodimentLock, Fragment, Layer, MemoryPolicy, Possession, PromptError, PromptStack,
    Provenance,
};
pub use prompt::{prompt_of, PROMPT_ID, PROMPT_VERSION};
pub use without_model::{suggested_questions, without_model};
pub use yuu::{Yuu, ROUTE_ROLE};

// `answer::Line` KHÔNG được re-export ở đây: `audit::Line` đã chiếm tên này ở
// gốc crate. Hai kiểu phục vụ hai bảo đảm khác nhau (biên niên sử đã xảy ra,
// so với câu tư vấn đã kiểm chứng), xem tài liệu `answer::Line`. Dùng
// `mow_yuu::answer::Line` khi cần kiểu của module tư vấn.
