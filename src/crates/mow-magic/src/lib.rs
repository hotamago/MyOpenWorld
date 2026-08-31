//! # `mow-magic` — luật, sandbox, vật phẩm mang hành vi
//!
//! Bốn module, và cả bốn phục vụ **một** câu ở `§15.3`:
//!
//! > Không dùng `eval` hoặc chạy code do LLM sinh trực tiếp.
//!
//! Tôn trọng câu đó tốn nhiều hơn nó nghe:
//!
//! | Module | Cái nó thay cho `eval` | Cái nó phải tự làm |
//! |---|---|---|
//! | [`dsl`] | cây biểu thức, tập phép toán đóng | kiểm kiểu, kiểm **đơn vị**, đảm bảo dừng |
//! | [`sandbox`] | wasmtime có fuel và whitelist | từ chối nạp module xin sai context |
//! | [`artifact`] | vật phẩm mang **tham chiếu**, không mang mã | tám cổng, mỗi cổng một đường phá |
//! | [`secrecy`] | view lọc trước khi dựng prompt | quét mọi prompt như lưới cuối |
//!
//! Cột thứ ba là cái giá. Cột thứ hai là thứ mua được bằng cái giá đó: một thế
//! giới nơi luật do mô hình đề xuất vẫn **kiểm được trước khi chạy**, và nơi một
//! cây trượng không mở được cửa sau nào mà spell thường không có.

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

pub mod artifact;
pub mod dsl;
pub mod sandbox;
pub mod secrecy;

pub use artifact::{
    check_synthesis, Bearer, Behaviour, Blocked, Charges, Gate, GateRequirement, Revelation,
    Synthesis, SynthesisError, Talent, GATES,
};
pub use dsl::{Expr, Quantity, Rule, RuleError, Unit, MAX_DEPTH};
pub use sandbox::{
    Capability, ContextKind, Fuel, Invocation, LawHistory, LoadError, ModuleManifest,
    ModuleRegistry, Outcome, Sandbox, ALLOWED_IMPORTS,
};
pub use secrecy::{audit_prompt, audit_session, ItemView, Leak, Secret, SecretRegistry};
