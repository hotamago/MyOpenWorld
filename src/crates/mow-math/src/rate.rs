//! Miền 3 — **tốc độ theo thời gian**: số hữu tỉ `num/den` trên mỗi tick.
//!
//! Nhu cầu tụt, đồ vật hao mòn, đồng hồ của một thế giới chạy nhanh hơn thế
//! giới khác — tất cả đều là "bao nhiêu đơn vị trên bao nhiêu tick". Lưu chúng
//! dưới dạng fixed-point sẽ tích lũy sai số: `1/3` đơn vị mỗi tick sau 3 tick
//! phải là đúng `1`, không phải `0.99998`.
//!
//! Hữu tỉ cộng với **số dư mang theo** (`carry`) cho phép tích phân đóng: hỏi
//! "sau 400 000 tick thì tụt bao nhiêu" trả lời bằng một phép chia, không phải
//! 400 000 vòng lặp. Đó chính là thứ `§9.7` cần để homeostasis không phải chạy
//! mỗi tick cho mỗi thực thể.

use crate::error::{overflow, MathError, MathResult};
use serde::{Deserialize, Serialize};

/// Tốc độ hữu tỉ: `num` đơn vị trên mỗi `den` tick.
///
/// Bất biến: `den > 0`. Phân số **không** được rút gọn tự động — hai tốc độ
/// bằng nhau về giá trị vẫn là hai giá trị khác nhau nếu viết khác nhau, và
/// điều đó cố ý: rút gọn ngầm sẽ làm state hash đổi khi ta chỉ sửa cách viết
/// dữ liệu content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rate {
    num: i64,
    den: i64,
}

impl Rate {
    /// Đứng yên.
    pub const ZERO: Rate = Rate { num: 0, den: 1 };

    /// Dựng `num` đơn vị mỗi `den` tick.
    pub fn new(num: i64, den: i64) -> MathResult<Rate> {
        if den <= 0 {
            return Err(MathError::BadDenominator { den });
        }
        Ok(Rate { num, den })
    }

    /// `n` đơn vị mỗi tick.
    pub fn per_tick(num: i64) -> Rate {
        Rate { num, den: 1 }
    }

    /// Tử số.
    #[inline]
    pub const fn num(self) -> i64 {
        self.num
    }

    /// Mẫu số, luôn dương.
    #[inline]
    pub const fn den(self) -> i64 {
        self.den
    }

    /// Đảo dấu.
    #[inline]
    pub fn negate(self) -> Rate {
        Rate {
            num: -self.num,
            den: self.den,
        }
    }

    /// Nhân với hệ số hữu tỉ. Dùng cho modifier lên tốc độ.
    pub fn scaled(self, k_num: i64, k_den: i64) -> MathResult<Rate> {
        if k_den <= 0 {
            return Err(MathError::BadDenominator { den: k_den });
        }
        let num = self
            .num
            .checked_mul(k_num)
            .ok_or_else(|| overflow("Rate::scaled", self.num, k_num))?;
        let den = self
            .den
            .checked_mul(k_den)
            .ok_or_else(|| overflow("Rate::scaled", self.den, k_den))?;
        Ok(Rate { num, den })
    }

    /// Cộng hai tốc độ. Mẫu số kết quả là tích, không rút gọn.
    pub fn add(self, other: Rate) -> MathResult<Rate> {
        let den = self
            .den
            .checked_mul(other.den)
            .ok_or_else(|| overflow("Rate::add", self.den, other.den))?;
        let a = self
            .num
            .checked_mul(other.den)
            .ok_or_else(|| overflow("Rate::add", self.num, other.den))?;
        let b = other
            .num
            .checked_mul(self.den)
            .ok_or_else(|| overflow("Rate::add", other.num, self.den))?;
        let num = a
            .checked_add(b)
            .ok_or_else(|| overflow("Rate::add", a, b))?;
        Ok(Rate { num, den })
    }

    /// Tích phân đóng qua `ticks` tick, mang theo số dư.
    ///
    /// Trả `(delta, carry_mới)`. `carry` vào là số dư còn lại của lần tích phân
    /// trước, tính theo cùng mẫu số. Chuỗi lời gọi
    /// `integrate(100) → integrate(200) → integrate(700)` cho tổng `delta` bằng
    /// đúng một lời gọi `integrate(1000)` — đó là bất biến khiến LOD chuyển mức
    /// không làm mất mát tích lũy (`§22.14`).
    pub fn integrate(self, ticks: u64, carry: i64) -> MathResult<(i64, i64)> {
        if ticks == 0 {
            return Ok((0, carry));
        }
        let total = (self.num as i128)
            .checked_mul(ticks as i128)
            .ok_or_else(|| overflow("Rate::integrate", self.num, ticks))?
            .checked_add(carry as i128)
            .ok_or_else(|| overflow("Rate::integrate", self.num, carry))?;
        let den = self.den as i128;
        // Chia làm tròn xuống với số dư không âm, để `carry` luôn nằm trong
        // `[0, den)` bất kể dấu của tốc độ. Phép `/` của Rust cắt về 0, cho số
        // dư âm khi tử số âm, và như thế `carry` sẽ không phải bất biến ổn định.
        let mut q = total / den;
        let mut r = total % den;
        if r < 0 {
            q -= 1;
            r += den;
        }
        let delta = i64::try_from(q).map_err(|_| overflow("Rate::integrate", q, den))?;
        let carry_out = i64::try_from(r).map_err(|_| overflow("Rate::integrate", r, den))?;
        Ok((delta, carry_out))
    }

    /// Số tick tối thiểu để tích lũy đủ `amount` đơn vị.
    ///
    /// Đây là hàm ngược của [`Rate::integrate`] và là thứ lập lịch đánh thức
    /// (`§9.7` wake-up theo ngưỡng) cần: thay vì kiểm tra mỗi tick xem cái đói
    /// đã chạm ngưỡng chưa, tính thẳng ra tick sẽ chạm rồi ngủ tới đó.
    ///
    /// Trả `None` khi tốc độ bằng 0 hoặc ngược dấu với `amount`, tức là ngưỡng
    /// sẽ không bao giờ tới.
    pub fn ticks_to_accumulate(self, amount: i64, carry: i64) -> Option<u64> {
        if amount == 0 {
            return Some(0);
        }
        if self.num == 0 || (amount > 0) != (self.num > 0) {
            return None;
        }

        let den = self.den as i128;
        let carry = carry as i128;
        let amount = amount as i128;

        // Điều kiện phải khớp **đúng** phép làm tròn của `integrate`, vốn là
        // chia làm tròn xuống. Viết `|num*t| / den >= |amount|` nghe hợp lý
        // nhưng sai lệch một tick ở phía âm, vì làm tròn xuống đẩy độ lớn của
        // delta âm **lên**. Một tick lệch ở đây nghĩa là bộ lập lịch đánh thức
        // dậy sau khi ngưỡng đã qua, và cái đói vọt qua mốc chết mà không ai
        // kịp phản ứng.
        let tu_so = if self.num > 0 {
            // Cần floor((num·t + carry)/den) ≥ amount ⟺ num·t + carry ≥ den·amount.
            den.checked_mul(amount)?.checked_sub(carry)?
        } else {
            // Cần floor((num·t + carry)/den) ≤ amount
            //   ⟺ num·t + carry ≤ den·amount + den − 1
            //   ⟺ (−num)·t ≥ carry − den·amount − den + 1.
            carry
                .checked_sub(den.checked_mul(amount)?)?
                .checked_sub(den)?
                .checked_add(1)?
        };
        if tu_so <= 0 {
            return Some(0);
        }
        let mau_so = (self.num as i128).abs();
        u64::try_from((tu_so + mau_so - 1) / mau_so).ok()
    }
}

impl core::fmt::Display for Rate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}/{} mỗi tick", self.num, self.den)
    }
}

impl schemars::JsonSchema for Rate {
    fn schema_name() -> String {
        "Rate".to_owned()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{InstanceType, ObjectValidation, SchemaObject};
        let mut obj = ObjectValidation::default();
        obj.properties
            .insert("num".to_owned(), <i64>::json_schema(gen));
        obj.properties
            .insert("den".to_owned(), <i64>::json_schema(gen));
        obj.required.insert("num".to_owned());
        obj.required.insert("den".to_owned());
        schemars::schema::Schema::Object(SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            object: Some(Box::new(obj)),
            ..Default::default()
        })
    }
}
