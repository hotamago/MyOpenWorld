//! # `mow-divine` — thần linh, linh hồn và quyền năng
//!
//! Toàn bộ crate phục vụ một câu ở `§14.2`:
//!
//! > Một thần bão **không trực tiếp đặt `city.destroyed = true`**.
//!
//! Thần là entity rất mạnh **vẫn nằm trong law** (`§14.1` loại 1), nên
//! `INV-22-1` áp cho thần y hệt áp cho một người nông dân: state change
//! authoritative chỉ commit qua handler.
//!
//! - [`authority`] — ba loại thần, domain authority, capability thu hồi được.
//! - [`soul`] — chính sách linh hồn, triệu hồi, thăng thần không xóa lịch sử.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod authority;
pub mod soul;

pub use authority::{
    DivineError, Domain, DomainAct, FieldProposal, God, GodKind, Grant, Intervention,
};
pub use soul::{Ascension, AscensionPath, CarriedOver, Soul, SoulError, SoulPolicy, SoulState};
