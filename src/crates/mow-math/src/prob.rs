//! Miền 2 — **xác suất nhỏ và tỉ lệ hiếm**: `u64` thang cố định `Q0.64`.
//!
//! Lý do miền này tồn tại, viết ra để không ai gộp nó lại vào [`crate::fixed`]:
//!
//! ```text
//! 2^-16                     = 1.5259e-05     bước nhỏ nhất của Q16.16
//! mutation_rate_per_locus   = 2.1e-08        idea.md §21.2
//! 2.1e-8 * 65536            = 0.0014  ──►  làm tròn thành 0
//! ```
//!
//! Nghĩa là nếu lưu tỉ lệ đột biến vào Q16.16 thì **đột biến biến mất khỏi thế
//! giới** — không phải hiếm đi, mà bằng 0 tuyệt đối, mãi mãi. Cùng lỗi đó áp
//! cho xác suất lây bệnh và tỉ lệ backfire của phép.
//!
//! `Prob` lưu tử số trên mẫu số ngầm `2^64`, nên bước nhỏ nhất là `5.4e-20`.
//! Lấy mẫu là **so sánh nguyên** với một `u64` đều — không có phép chia, không
//! có số thực, và phân phối đúng đến từng bit.

use crate::error::{MathError, MathResult};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Xác suất trong `[0,1)` với thang `2^64`.
///
/// Không biểu diễn được `1.0` một cách chính xác — đó là chủ đích. Một sự kiện
/// "chắc chắn xảy ra" là một nhánh điều khiển, không phải một phép tung xúc xắc;
/// dùng [`Prob::ALWAYS`] khi thật sự cần và biết rằng nó là `1 - 5.4e-20`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Prob(u64);

impl Prob {
    /// Không bao giờ xảy ra.
    pub const NEVER: Prob = Prob(0);
    /// Xác suất lớn nhất biểu diễn được, `1 - 2^-64`.
    pub const ALWAYS: Prob = Prob(u64::MAX);

    /// Dựng từ biểu diễn thô (tử số trên `2^64`).
    #[inline]
    pub const fn from_raw(raw: u64) -> Prob {
        Prob(raw)
    }

    /// Biểu diễn thô.
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Dựng từ phân số `num/den`, làm tròn xuống.
    ///
    /// Làm tròn xuống ở đây an toàn hơn làm tròn gần nhất: một xác suất hiếm bị
    /// ước lượng thấp đi `5e-20` không quan sát được, còn một xác suất bị làm
    /// tròn **lên** từ 0 sẽ tạo ra sự kiện mà dữ liệu nói là không thể.
    pub fn from_frac(num: u64, den: u64) -> MathResult<Prob> {
        if den == 0 {
            return Err(MathError::DivideByZero {
                op: "Prob::from_frac",
            });
        }
        if num >= den {
            return Err(MathError::OutOfDomain {
                domain: "Prob",
                value: format!("{num}/{den}"),
                min: "0".into(),
                max: "<1".into(),
            });
        }
        // num * 2^64 / den, trọn vẹn trong u128 vì num < den <= u64::MAX.
        let scaled = (num as u128) << 64;
        Ok(Prob((scaled / (den as u128)) as u64))
    }

    /// Dựng từ ký hiệu khoa học thập phân `mantissa × 10^-exp10`.
    ///
    /// Đây là dạng mà dữ liệu content viết ra (`2.1e-8` là `from_sci(21, 9)`),
    /// nên có một hàm dựng chuyên cho nó thay vì bắt mỗi chỗ gọi tự tính lũy
    /// thừa và tự làm hỏng.
    pub fn from_sci(mantissa: u64, exp10: u32) -> MathResult<Prob> {
        let den = 10u64
            .checked_pow(exp10)
            .ok_or_else(|| crate::error::overflow("Prob::from_sci", mantissa, exp10))?;
        Prob::from_frac(mantissa, den)
    }

    /// Dựng từ phần triệu.
    #[inline]
    pub fn from_ppm(ppm: u64) -> MathResult<Prob> {
        Prob::from_frac(ppm, 1_000_000)
    }

    /// Phần bù `1 - p`.
    #[inline]
    pub fn complement(self) -> Prob {
        Prob(u64::MAX - self.0)
    }

    /// Xác suất cả hai sự kiện **độc lập** cùng xảy ra.
    #[inline]
    pub fn and(self, other: Prob) -> Prob {
        Prob((((self.0 as u128) * (other.0 as u128)) >> 64) as u64)
    }

    /// Xác suất ít nhất một trong hai sự kiện độc lập xảy ra.
    #[inline]
    pub fn or(self, other: Prob) -> Prob {
        self.complement().and(other.complement()).complement()
    }

    /// Xác suất sự kiện xảy ra **ít nhất một lần** trong `n` lần thử độc lập.
    ///
    /// Tính bằng lũy thừa nhị phân trên phần bù, `O(log n)`. Vòng lặp `n` lần
    /// là cách viết tự nhiên nhưng ở đây `n` có thể là số tick của một đời
    /// người, nên nó sẽ trở thành điểm nóng thật.
    pub fn at_least_once_in(self, n: u64) -> Prob {
        let mut base = self.complement();
        let mut acc = Prob::ALWAYS;
        let mut e = n;
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.and(base);
            }
            base = base.and(base);
            e >>= 1;
        }
        acc.complement()
    }

    /// Nhân với một hệ số hữu tỉ `num/den`, bão hòa ở `ALWAYS`.
    ///
    /// Dùng cho modifier: "loài này đột biến nhanh gấp 3 lần" là
    /// `rate.scaled(3, 1)`, không phải một phép nhân số thực ở chỗ gọi.
    pub fn scaled(self, num: u64, den: u64) -> MathResult<Prob> {
        if den == 0 {
            return Err(MathError::DivideByZero {
                op: "Prob::scaled",
            });
        }
        let v = (self.0 as u128) * (num as u128) / (den as u128);
        Ok(Prob(v.min(u64::MAX as u128) as u64))
    }

    /// Tung xúc xắc. `true` với đúng xác suất này.
    ///
    /// So sánh nguyên trên một `u64` đều: không chia, không số thực, và mỗi giá
    /// trị thô tương ứng đúng một trong `2^64` kết quả có thể của bộ sinh.
    #[inline]
    pub fn sample<R: Rng + ?Sized>(self, rng: &mut R) -> bool {
        rng.gen::<u64>() < self.0
    }

    /// Xấp xỉ thập phân, **chỉ để hiển thị và ghi log**.
    ///
    /// Trả về `(mantissa, exp10)` sao cho giá trị ≈ `mantissa × 10^-exp10`, với
    /// `mantissa` có tối đa 4 chữ số. Không bao giờ dùng kết quả này để tính.
    pub fn to_sci_approx(self) -> (u64, u32) {
        if self.0 == 0 {
            return (0, 0);
        }
        let mut exp = 0u32;
        // v giữ giá trị p × 10^exp ở thang 2^64.
        let mut v = self.0 as u128;
        while v < (1u128 << 64) / 10_000 && exp < 40 {
            v *= 10;
            exp += 1;
        }
        let mantissa = ((v * 10_000) >> 64) as u64;
        (mantissa, exp + 4)
    }
}

impl core::fmt::Display for Prob {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (m, e) = self.to_sci_approx();
        if m == 0 {
            return write!(f, "0");
        }
        write!(f, "{}.{:03}e-{}", m / 1000, m % 1000, e)
    }
}

impl schemars::JsonSchema for Prob {
    fn schema_name() -> String {
        "Prob".to_owned()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <u64>::json_schema(gen)
    }
}
