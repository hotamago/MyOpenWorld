//! Cổng, transfer nguyên tử và chế độ tiếp xúc đa thế giới.
//!
//! Ba việc, và chúng dính chặt vào nhau nên nằm chung một crate:
//!
//! - [`portal`] — vòng đời cổng và **transfer chín bước** với escrow hai pha
//!   (`§6.2`, `INV-22-8`).
//! - [`clock`] — bước 5 của transfer: rebase deadline theo miền đồng hồ của
//!   chính tiến trình (`§4.5`, `INV-22-42`).
//! - [`contact`] — bước 3: kiểm dịch, hàng cấm, cư trú, tranh chấp (`§6.4`).
//!
//! Tách `clock` và `contact` ra khỏi `portal` không phải để chúng dùng lại được
//! ở chỗ khác — chúng gần như chỉ dùng ở đây. Tách vì mỗi cái là một **bước
//! trong chín bước** dễ bị viết vội thành một dòng `if`, và một file riêng bắt
//! người viết phải nghĩ đủ.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod clock;
pub mod contact;
pub mod portal;

pub use clock::{rebase_processes, Process, RebaseAudit, RebaseError, RebaseReason};
pub use contact::{Cargo, ContactRegime, Decision, Failure};
pub use portal::{
    count_copies, recover, AccessPolicy, EscrowLedger, EscrowPhase, EscrowRecord, NeedsProfile,
    Portal, PortalState, Recovery, SurvivalWarning, TransferError, Traveller, WorldConditions,
};
