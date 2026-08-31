//! Miền 4 và 5 — **đại lượng vật lý và tiền tệ**: số nguyên có đơn vị khai báo
//! trong *kiểu*, không phải trong tên biến.
//!
//! `plan.md §P10.2.1` nói rõ: "Mỗi kiểu khai báo đơn vị và thang trong type,
//! không phải trong tên biến." Một `i64` tên là `mass_mmu` vẫn cộng được với
//! một `i64` tên là `energy_j` và trình biên dịch không phàn nàn. Các newtype
//! dưới đây làm phép cộng đó thành lỗi biên dịch.
//!
//! Chuyển đổi giữa các đơn vị **luôn là hàm tường minh có kiểm tra tràn**;
//! không có `From`/`Into` ngầm giữa hai đơn vị khác nhau.

#![allow(clippy::many_single_char_names)]
use crate::error::{overflow, MathError, MathResult};
use serde::{Deserialize, Serialize};

/// Sinh một đơn vị số nguyên có tên.
///
/// Mỗi đơn vị có cùng bộ phép toán, nhưng **không** trộn được với đơn vị khác.
macro_rules! unit_scalar {
    ($(#[$meta:meta])* $name:ident, $suffix:literal) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
            Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(i64);

        impl $name {
            #[doc = "Giá trị 0."]
            pub const ZERO: $name = $name(0);
            #[doc = "Giá trị lớn nhất."]
            pub const MAX: $name = $name(i64::MAX);

            #[doc = "Dựng từ số nguyên đơn vị nhỏ nhất."]
            #[inline]
            pub const fn new(v: i64) -> $name { $name(v) }

            #[doc = "Giá trị thô theo đơn vị nhỏ nhất."]
            #[inline]
            pub const fn get(self) -> i64 { self.0 }

            #[doc = "Cộng có kiểm tra tràn."]
            //
            // Trùng tên với `std::ops::Add` là **có chủ đích**: `a.add(b)?` đọc
            // như phép cộng thông thường mà vẫn bắt người viết xử lý tràn. Không
            // hiện thực trait thật được vì trait trả thẳng giá trị và không có
            // chỗ cho lỗi — đúng thứ mà `§P10.2.1` cấm trên đường commit.
            #[allow(clippy::should_implement_trait)]
            #[inline]
            pub fn add(self, rhs: $name) -> MathResult<$name> {
                self.0.checked_add(rhs.0).map($name)
                    .ok_or_else(|| overflow(concat!(stringify!($name), "::add"), self.0, rhs.0))
            }

            #[doc = "Trừ có kiểm tra tràn."]
            #[allow(clippy::should_implement_trait)]
            #[inline]
            pub fn sub(self, rhs: $name) -> MathResult<$name> {
                self.0.checked_sub(rhs.0).map($name)
                    .ok_or_else(|| overflow(concat!(stringify!($name), "::sub"), self.0, rhs.0))
            }

            #[doc = "Nhân với số nguyên không đơn vị."]
            #[inline]
            pub fn times(self, k: i64) -> MathResult<$name> {
                self.0.checked_mul(k).map($name)
                    .ok_or_else(|| overflow(concat!(stringify!($name), "::times"), self.0, k))
            }

            #[doc = "Nhân với một tỉ lệ chuẩn hóa, làm tròn về phía 0."]
            pub fn scaled_by(self, r: crate::fixed::Unit) -> $name {
                let p = (i128::from(self.0)) * (i128::from(r.get().raw()));
                $name((if p < 0 { -((-p) >> crate::fixed::FRAC_BITS) }
                       else { p >> crate::fixed::FRAC_BITS }) as i64)
            }

            #[doc = "Chia theo hệ số hữu tỉ `num/den`, làm tròn về phía 0."]
            pub fn ratio(self, num: i64, den: i64) -> MathResult<$name> {
                if den == 0 {
                    return Err(MathError::DivideByZero {
                        op: concat!(stringify!($name), "::ratio"),
                    });
                }
                let v = (i128::from(self.0)) * (i128::from(num)) / (i128::from(den));
                i64::try_from(v).map($name)
                    .map_err(|_| overflow(concat!(stringify!($name), "::ratio"), num, den))
            }

            #[doc = "Kẹp vào khoảng."]
            #[inline]
            pub fn clamp(self, lo: $name, hi: $name) -> $name {
                $name(self.0.clamp(lo.0, hi.0))
            }

            #[doc = "Trừ nhưng không xuống dưới 0. Dùng cho kho, máu, năng lượng."]
            #[inline]
            pub fn saturating_sub(self, rhs: $name) -> $name {
                $name(self.0.saturating_sub(rhs.0).max(0))
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}{}", self.0, $suffix)
            }
        }

        impl schemars::JsonSchema for $name {
            fn schema_name() -> String { stringify!($name).to_owned() }
            fn json_schema(
                gen: &mut schemars::gen::SchemaGenerator,
            ) -> schemars::schema::Schema {
                <i64>::json_schema(gen)
            }
        }
    };
}

unit_scalar!(
    /// Khối lượng, đơn vị **mMU** (milli mass unit) — xem `idea.md §21`.
    Mass, "mMU"
);
unit_scalar!(
    /// Năng lượng, đơn vị **joule**.
    Energy, "J"
);
unit_scalar!(
    /// Thể tích, đơn vị **mL**.
    Volume, "mL"
);
unit_scalar!(
    /// Năng lượng thức ăn, đơn vị **kcal**.
    Food, "kcal"
);
unit_scalar!(
    /// Nhiệt độ tuyệt đối, đơn vị **mK** (milli kelvin).
    ///
    /// Tuyệt đối chứ không phải Celsius: hiệu hai nhiệt độ tuyệt đối là một
    /// đại lượng có nghĩa, còn hiệu hai nhiệt độ Celsius thì phụ thuộc gốc.
    Temp, "mK"
);
unit_scalar!(
    /// Tiền, đơn vị nhỏ nhất của hệ tiền tệ đang xét (`§12.8.2`).
    ///
    /// Không có kiểu "tiền thực": mọi giao dịch là số nguyên. Pha loãng đồng xu
    /// (`§12.8.3`) đổi *hàm lượng kim loại* của item, không đổi kiểu này.
    Money, "¤"
);
unit_scalar!(
    /// Khoảng thời gian tính bằng tick.
    Ticks, "t"
);

impl Temp {
    /// Điểm đóng băng của nước, `273.15 K`.
    pub const FREEZING: Temp = Temp(273_150);
    /// Điểm sôi của nước ở áp suất chuẩn.
    pub const BOILING: Temp = Temp(373_150);

    /// Dựng từ độ Celsius nguyên. Hàm tường minh, không phải `From`.
    pub fn from_celsius(c: i64) -> MathResult<Temp> {
        c.checked_mul(1000)
            .and_then(|v| v.checked_add(Temp::FREEZING.0))
            .map(Temp)
            .ok_or_else(|| overflow("Temp::from_celsius", c, 1000))
    }

    /// Về độ Celsius, làm tròn xuống. Chỉ để hiển thị.
    pub fn to_celsius_floor(self) -> i64 {
        (self.0 - Temp::FREEZING.0).div_euclid(1000)
    }
}

impl Ticks {
    /// Số tick không âm dưới dạng `u64`, dùng cho [`crate::rate::Rate`].
    pub fn as_u64(self) -> MathResult<u64> {
        u64::try_from(self.0).map_err(|_| MathError::OutOfDomain {
            domain: "Ticks",
            value: self.0.to_string(),
            min: "0".into(),
            max: u64::MAX.to_string(),
        })
    }
}
