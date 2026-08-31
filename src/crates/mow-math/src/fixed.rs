//! Miền 1 — **tỉ lệ chuẩn hóa `[0,1]` và đại lượng vô hướng nhỏ**: Q16.16.
//!
//! Bước nhỏ nhất là `2^-16 ≈ 1.53e-5`. Đủ cho `focus`, `visibility`, `fatigue`,
//! `CraftQuality`. **Không đủ** cho xác suất hiếm — xem [`crate::prob`] và bảng
//! miền ở `plan.md §P10.2.1`.

#![allow(clippy::many_single_char_names)]
use crate::error::{overflow, MathError, MathResult};
use serde::{Deserialize, Serialize};

/// Số bit phần thập phân.
pub const FRAC_BITS: u32 = 16;

/// Giá trị thô của `1.0`.
pub const ONE_RAW: i64 = 1 << FRAC_BITS;

/// Số Q16.16 với backing `i64`.
///
/// Lưu ở dạng thô (`raw = value * 2^16`). Hai `Fx` bằng nhau khi và chỉ khi raw
/// bằng nhau, nên `Eq`, `Ord` và `Hash` đều dùng được trên đường commit — điều
/// mà `f64` không cho phép.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Fx(i64);

impl Fx {
    /// `0`.
    pub const ZERO: Fx = Fx(0);
    /// `1`.
    pub const ONE: Fx = Fx(ONE_RAW);
    /// Giá trị lớn nhất biểu diễn được.
    pub const MAX: Fx = Fx(i64::MAX);
    /// Giá trị nhỏ nhất biểu diễn được.
    pub const MIN: Fx = Fx(i64::MIN);
    /// Bước nhỏ nhất khác 0, tức `2^-16`.
    pub const EPSILON: Fx = Fx(1);

    /// Dựng từ biểu diễn thô. Chỉ dùng ở tầng persistence và test.
    #[inline]
    pub const fn from_raw(raw: i64) -> Fx {
        Fx(raw)
    }

    /// Biểu diễn thô. Đây là thứ được ghi xuống đĩa và đưa vào hash.
    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Dựng từ số nguyên. Lỗi nếu không biểu diễn được.
    #[inline]
    pub fn from_int(n: i64) -> MathResult<Fx> {
        n.checked_mul(ONE_RAW)
            .map(Fx)
            .ok_or_else(|| overflow("Fx::from_int", n, ONE_RAW))
    }

    /// Dựng từ phân số `num/den`, làm tròn **về phía 0**.
    ///
    /// Làm tròn về 0 là lựa chọn có chủ đích: nó đối xứng qua gốc, nên đảo dấu
    /// đầu vào cho kết quả đảo dấu chính xác. Làm tròn xuống (`floor`) không có
    /// tính chất đó và sẽ tạo lệch hệ thống khi cộng dồn đại lượng có dấu.
    pub fn from_frac(num: i64, den: i64) -> MathResult<Fx> {
        if den == 0 {
            return Err(MathError::DivideByZero {
                op: "Fx::from_frac",
            });
        }
        let scaled = i128::from(num)
            .checked_mul(i128::from(ONE_RAW))
            .ok_or_else(|| overflow("Fx::from_frac", num, den))?;
        let q = scaled / i128::from(den);
        i64::try_from(q)
            .map(Fx)
            .map_err(|_| overflow("Fx::from_frac", num, den))
    }

    /// Phần nguyên, cắt về phía âm vô cùng.
    #[inline]
    pub const fn floor_int(self) -> i64 {
        self.0 >> FRAC_BITS
    }

    /// Làm tròn về số nguyên gần nhất, nửa lẻ làm tròn lên.
    #[inline]
    pub const fn round_int(self) -> i64 {
        (self.0 + (ONE_RAW / 2)) >> FRAC_BITS
    }

    /// Cộng có kiểm tra tràn.
    #[inline]
    // Trùng tên với `std::ops` là **có chủ đích**: `a.add(b)?` đọc như
    // phép toán thông thường mà vẫn bắt xử lý tràn. Trait thật trả thẳng giá
    // trị và không có chỗ cho lỗi — đúng thứ `§P10.2.1` cấm trên đường commit.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, rhs: Fx) -> MathResult<Fx> {
        self.0
            .checked_add(rhs.0)
            .map(Fx)
            .ok_or_else(|| overflow("Fx::add", self, rhs))
    }

    /// Trừ có kiểm tra tràn.
    #[inline]
    // Trùng tên với `std::ops` là **có chủ đích**: `a.sub(b)?` đọc như
    // phép toán thông thường mà vẫn bắt xử lý tràn. Trait thật trả thẳng giá
    // trị và không có chỗ cho lỗi — đúng thứ `§P10.2.1` cấm trên đường commit.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, rhs: Fx) -> MathResult<Fx> {
        self.0
            .checked_sub(rhs.0)
            .map(Fx)
            .ok_or_else(|| overflow("Fx::sub", self, rhs))
    }

    /// Nhân, trung gian `i128`, làm tròn về phía 0.
    // Trùng tên với `std::ops` là **có chủ đích**: `a.mul(b)?` đọc như
    // phép toán thông thường mà vẫn bắt xử lý tràn. Trait thật trả thẳng giá
    // trị và không có chỗ cho lỗi — đúng thứ `§P10.2.1` cấm trên đường commit.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, rhs: Fx) -> MathResult<Fx> {
        let p = i128::from(self.0)
            .checked_mul(i128::from(rhs.0))
            .ok_or_else(|| overflow("Fx::mul", self, rhs))?;
        // Dịch phải trên số âm là làm tròn xuống, nên xử lý dấu tường minh để
        // giữ tính đối xứng đã hứa ở `from_frac`.
        let q = if p < 0 {
            -((-p) >> FRAC_BITS)
        } else {
            p >> FRAC_BITS
        };
        i64::try_from(q)
            .map(Fx)
            .map_err(|_| overflow("Fx::mul", self, rhs))
    }

    /// Chia, trung gian `i128`, làm tròn về phía 0.
    // Trùng tên với `std::ops` là **có chủ đích**: `a.div(b)?` đọc như
    // phép toán thông thường mà vẫn bắt xử lý tràn. Trait thật trả thẳng giá
    // trị và không có chỗ cho lỗi — đúng thứ `§P10.2.1` cấm trên đường commit.
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, rhs: Fx) -> MathResult<Fx> {
        if rhs.0 == 0 {
            return Err(MathError::DivideByZero { op: "Fx::div" });
        }
        let n = i128::from(self.0) << FRAC_BITS;
        let q = n / i128::from(rhs.0);
        i64::try_from(q)
            .map(Fx)
            .map_err(|_| overflow("Fx::div", self, rhs))
    }

    /// Nhân với số nguyên.
    #[inline]
    pub fn scale_int(self, k: i64) -> MathResult<Fx> {
        self.0
            .checked_mul(k)
            .map(Fx)
            .ok_or_else(|| overflow("Fx::scale_int", self, k))
    }

    /// Kẹp vào `[lo, hi]`.
    #[inline]
    pub fn clamp(self, lo: Fx, hi: Fx) -> Fx {
        if self.0 < lo.0 {
            lo
        } else if self.0 > hi.0 {
            hi
        } else {
            self
        }
    }

    /// Trị tuyệt đối; `Fx::MIN` không có trị tuyệt đối biểu diễn được.
    #[inline]
    pub fn abs(self) -> MathResult<Fx> {
        self.0
            .checked_abs()
            .map(Fx)
            .ok_or_else(|| overflow("Fx::abs", self, 0))
    }

    /// Nội suy tuyến tính `self + (other - self) * t`.
    pub fn lerp(self, other: Fx, t: Unit) -> MathResult<Fx> {
        let d = other.sub(self)?;
        self.add(d.mul(t.get())?)
    }
}

impl core::fmt::Display for Fx {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // In ra 6 chữ số thập phân từ số nguyên — không dùng số thực ở bất kỳ
        // bước nào, kể cả khi chỉ để hiển thị.
        let neg = self.0 < 0;
        let mag = i128::from(self.0).unsigned_abs();
        let int_part = mag >> FRAC_BITS;
        let frac_raw = mag & ((1u128 << FRAC_BITS) - 1);
        let frac_micro = (frac_raw * 1_000_000) >> FRAC_BITS;
        if neg {
            write!(f, "-")?;
        }
        write!(f, "{int_part}.{frac_micro:06}")
    }
}

impl schemars::JsonSchema for Fx {
    fn schema_name() -> String {
        "Fx".to_owned()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // Trên đường truyền, Q16.16 là **số nguyên thô**, không phải số thực.
        // Nếu để schema mô tả nó là `number`, một client JS sẽ vô tình đưa nó về
        // `f64` và determinism vỡ ngay ở biên hệ thống.
        <i64>::json_schema(gen)
    }
}

/// Tỉ lệ chuẩn hóa, bất biến `0 ≤ v ≤ 1`.
///
/// Kiểu riêng chứ không phải quy ước đặt tên: một hàm nhận `Unit` không cần
/// kiểm tra lại miền, và không thể vô tình nhận một `Fx` mang giá trị 37.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Unit(Fx);

impl Unit {
    /// `0`.
    pub const ZERO: Unit = Unit(Fx::ZERO);
    /// `1`.
    pub const ONE: Unit = Unit(Fx::ONE);

    /// Dựng, lỗi nếu ngoài `[0,1]`.
    pub fn new(v: Fx) -> MathResult<Unit> {
        if v < Fx::ZERO || v > Fx::ONE {
            return Err(MathError::OutOfDomain {
                domain: "Unit",
                value: v.to_string(),
                min: "0.000000".into(),
                max: "1.000000".into(),
            });
        }
        Ok(Unit(v))
    }

    /// Dựng bằng cách kẹp. Dùng khi nguồn dữ liệu đã biết là gần đúng.
    pub fn saturating(v: Fx) -> Unit {
        Unit(v.clamp(Fx::ZERO, Fx::ONE))
    }

    /// Dựng từ phân số, kẹp vào miền.
    pub fn from_frac(num: i64, den: i64) -> MathResult<Unit> {
        Ok(Unit::saturating(Fx::from_frac(num, den)?))
    }

    /// Giá trị bên trong.
    #[inline]
    pub const fn get(self) -> Fx {
        self.0
    }

    /// Phần bù `1 - self`.
    #[inline]
    pub fn complement(self) -> Unit {
        Unit(Fx(ONE_RAW - self.0 .0))
    }

    /// Nhân hai tỉ lệ. Tích của hai giá trị trong `[0,1]` luôn trong `[0,1]`,
    /// và `Fx::mul` chỉ tràn khi toán hạng vượt `2^47`, nên phép này toàn phần.
    pub fn and(self, other: Unit) -> Unit {
        Unit(Fx(
            ((i128::from(self.0 .0) * i128::from(other.0 .0)) >> FRAC_BITS) as i64,
        ))
    }
}

impl core::fmt::Display for Unit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl schemars::JsonSchema for Unit {
    fn schema_name() -> String {
        "Unit".to_owned()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <i64>::json_schema(gen)
    }
}
