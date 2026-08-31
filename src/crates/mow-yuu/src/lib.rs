//! # `mow-yuu` — control plane Yuu và console True God
//!
//! `§15.1`: Yuu **không phải một LLM toàn quyền duy nhất**. Nó là một control
//! plane gồm các module có quyền hạn rõ, và:
//!
//! > Một persona "Yuu" thống nhất giao tiếp với người chơi, nhưng bên trong
//! > các module **không chia sẻ quyền tùy tiện**.
//!
//! Nên crate này không có một kiểu `YuuContext` toàn năng. Bốn module, mỗi cái
//! nhận đúng thứ nó cần:
//!
//! - [`forge`] — World Architect, Species Foundry (có viability check),
//!   Law Forge (có sandbox). `§15.1`–`§15.3`, `PF-06`.
//! - [`audit`] — Auditor dùng **chung** bộ invariant với harness; Historian chỉ
//!   dùng event có thật. `§22.17`, `PF-07`.
//! - [`console`] — query/proposal/command, preview, tự snapshot, rollback.
//!   `§15.5`, `§16`, `PF-08`.
//! - [`possession`] — hóa thân, phân tầng prompt, provenance. `§16.3`, `§16.4`,
//!   `PF-09`.
//!
//! Không module nào ở đây commit state. Chúng sinh ra **đề xuất** và **báo
//! cáo**; `INV-22-1` giữ nguyên hiệu lực cho Yuu như cho mọi thứ khác.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod audit;
pub mod console;
pub mod forge;
pub mod possession;

pub use audit::{AuditReport, Auditor, Channels, Chronicle, ChronicleError, Finding, Line};
pub use console::{
    provenance, Console, ConsoleError, ConsoleLog, Intervention, Op, Outcome, Plan, PreviewReason,
    Request, Snapshot, SnapshotReason, NGUONG_PHA_HUY_DIEN_RONG,
};
pub use forge::{
    Conditions, ForgeError, ForgedLaw, Inviable, LawForge, SpeciesDraft, SpeciesFoundry,
    WorldTemplate, TRAN_KCAL_MOI_NGAY,
};
pub use possession::{
    Consent, EmbodimentLock, Fragment, Layer, MemoryPolicy, Possession, PromptError, PromptStack,
    Provenance,
};
