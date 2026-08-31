//! # `mow-law` — luật, tội, chứng cứ, thẩm quyền
//!
//! Cả crate này đi ra từ một câu ở `idea.md §12.5.1`:
//!
//! > Tội **không phải thuộc tính của hành động**. Nó là **quan hệ giữa một hành
//! > động và một bộ chuẩn mực đang có hiệu lực tại nơi hành động xảy ra**.
//!
//! Nên không có `is_crime` ở đâu cả. Có [`norms::judge`], nhận `(hành vi, nơi,
//! ai)` và trả về những cáo buộc **có thể** có — số nhiều, vì một hành vi thường
//! vi phạm nhiều hệ luật cùng lúc, và việc chúng mâu thuẫn nhau là nội dung chứ
//! không phải lỗi.
//!
//! ```text
//!  §12.5.2  Deed ──► judge ──► Charge*  ──► proof_met ──► try_case ──► Verdict
//!                     │                        │                         │
//!            norm_set đang hiệu lực     chứng cứ CÓ HẠN            có thể SAI
//!            (version lúc hành vi)      và phá hủy được          so với sự thật
//! ```
//!
//! Ba chỗ in đậm là ba quyết định thiết kế, và mỗi cái được giải thích ở module
//! tương ứng:
//!
//! - [`norms`] — vì sao version phải là *lúc hành vi*, không phải lúc xét xử.
//! - [`crime`] — vì sao rủi ro tính theo **belief**, không theo con số thật.
//! - [`trial`] — vì sao [`trial::try_case`] **không nhận** thủ phạm thật.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]

pub mod crime;
pub mod norms;
pub mod trial;

pub use crime::{Intent, Temptation, Weight, Witness};
pub use norms::{
    governing_charge, immune, judge, Charge, Deed, Enforcement, Immunity, LegalOrder, NormSet,
    ProofMode, ProofRequirement, Rule, Sanction, SanctionKind, Scope,
};
pub use trial::{proof_met, try_case, DoubleJeopardy, Evidence, Procedure, TrialContext, Verdict};
