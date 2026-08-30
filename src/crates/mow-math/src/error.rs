//! Lỗi số học. Mọi lỗi ở đây là **xác định**: cùng đầu vào luôn cho cùng lỗi,
//! không phụ thuộc nền tảng hay thứ tự luồng (`idea.md §22.11`).

use thiserror::Error;

/// Lỗi của mọi phép toán trên đường commit.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MathError {
    /// Tràn số.
    #[error("tràn số trong `{op}`: {lhs} ⊕ {rhs}")]
    Overflow {
        /// Tên phép toán, ví dụ `Fx::mul`.
        op: &'static str,
        /// Toán hạng trái, đã ghi thành chuỗi để lỗi không cần generic.
        lhs: String,
        /// Toán hạng phải.
        rhs: String,
    },

    /// Chia cho 0.
    #[error("chia cho 0 trong `{op}`")]
    DivideByZero {
        /// Tên phép toán.
        op: &'static str,
    },

    /// Giá trị nằm ngoài miền hợp lệ của kiểu.
    #[error("giá trị {value} ngoài miền [{min}, {max}] của {domain}")]
    OutOfDomain {
        /// Tên kiểu có miền bị vi phạm.
        domain: &'static str,
        /// Giá trị vi phạm.
        value: String,
        /// Cận dưới hợp lệ.
        min: String,
        /// Cận trên hợp lệ.
        max: String,
    },

    /// Mẫu số của một tỉ lệ hữu tỉ bằng 0 hoặc âm.
    #[error("mẫu số không hợp lệ: {den}")]
    BadDenominator {
        /// Mẫu số đã nhận.
        den: i64,
    },
}

/// Kết quả của phép toán xác định.
pub type MathResult<T> = Result<T, MathError>;

/// Dựng lỗi tràn mà không phải viết `.to_string()` ở mọi chỗ gọi.
pub(crate) fn overflow<A: core::fmt::Display, B: core::fmt::Display>(
    op: &'static str,
    lhs: A,
    rhs: B,
) -> MathError {
    MathError::Overflow {
        op,
        lhs: lhs.to_string(),
        rhs: rhs.to_string(),
    }
}
