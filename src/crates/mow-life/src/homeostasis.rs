//! Nhu cầu và cân bằng nội môi (`idea.md §9.7`, `§22.24`).
//!
//! Bất biến `§22.24`:
//!
//! > Nhu cầu không được tick theo từng entity; giá trị suy ra bằng **tích phân
//! > đóng** từ `last_update_tick`.
//!
//! ## Vì sao không phải một vòng lặp
//!
//! Vòng lặp per-tick per-entity là cách viết tự nhiên nhất và nó hỏng theo hai
//! hướng cùng lúc:
//!
//! **Hiệu năng.** Một thế giới có hàng trăm nghìn thực thể, mỗi thực thể có
//! sáu nhu cầu. Đó là hàng triệu phép trừ mỗi tick, cho một đại lượng mà không
//! ai đọc trong 99,99% số tick.
//!
//! **Tính đúng đắn với LOD.** Một thực thể ở mức `Far` không chạy vòng lặp nào.
//! Khi nó quay lại `Active`, cái đói của nó phải bằng đúng cái đói của một
//! thực thể chưa từng rời đi — nếu không, đi ra khỏi tầm nhìn trở thành một
//! cách bất tử. Với tích phân đóng thì điều đó **miễn phí**: giá trị là hàm
//! của `now - last_update_tick`, và LOD không xuất hiện trong công thức.
//!
//! ## Đánh thức theo ngưỡng
//!
//! Thay vì kiểm tra mỗi tick "đã đói chưa", scheduler tính **tick sẽ đói** rồi
//! ngủ tới đó ([`Need::next_threshold_tick`]). Một thực thể no bụng không tốn
//! một chu kỳ CPU nào cho tới khi nó thật sự đói.

use mow_core::{ClockDomain, Tick};
use mow_math::{CanonicalHash, MathResult, Rate, StateHasher};
use serde::{Deserialize, Serialize};

/// Thang của một nhu cầu: `0` là cạn kiệt, `SCALE` là đầy.
///
/// Số nguyên chứ không phải Q16.16: nhu cầu là một đại lượng thô mà người chơi
/// đọc dưới dạng "3/4 thanh", và một phần nghìn của thanh đó không mang thông
/// tin nào. Thang 10 000 cho đủ độ phân giải để tích phân không bị làm tròn về
/// 0 ở tốc độ chậm nhất.
pub const SCALE: i64 = 10_000;

/// Một nhu cầu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Need {
    /// Định danh có namespace: `core.hunger`, `core.sleep`.
    pub id: String,
    /// Giá trị tại `last_update`, thang `[0, SCALE]`.
    pub value: i64,
    /// Tốc độ đổi mỗi tick. Âm là cạn dần.
    pub rate: Rate,
    /// Số dư của tích phân, để chia nhỏ khoảng không làm mất mát.
    pub carry: i64,
    /// Tick của lần cập nhật cuối, **theo `domain`**.
    pub last_update: Tick,
    /// Miền đồng hồ. Bắt buộc (`§4.5`).
    ///
    /// Gần như luôn là [`ClockDomain::Proper`]: cái đói đi theo thực thể qua
    /// cổng. Nếu để [`ClockDomain::WorldLocal`], một người bước sang thế giới
    /// chảy nhanh gấp mười sẽ chết đói trong một bước chân.
    pub domain: ClockDomain,
}

impl Need {
    /// Dựng một nhu cầu đầy.
    pub fn full(id: &str, rate: Rate, now: Tick) -> Need {
        Need {
            id: id.to_owned(),
            value: SCALE,
            rate,
            carry: 0,
            last_update: now,
            domain: ClockDomain::Proper,
        }
    }

    /// Giá trị **hiện tại**, suy ra bằng tích phân đóng.
    ///
    /// Không đổi state — đây là một truy vấn. Giá trị lưu trong `value` chỉ là
    /// mốc neo; giá trị thật luôn là hàm của `now`.
    pub fn value_at(&self, now: Tick) -> MathResult<i64> {
        let Some(dt) = now.since(self.last_update) else {
            // Hỏi về quá khứ: trả giá trị tại mốc neo thay vì ngoại suy ngược.
            // Ngoại suy ngược sẽ cho những con số vô lý ở biên fork nhánh.
            return Ok(self.value);
        };
        let (delta, _) = self.rate.integrate(dt, self.carry)?;
        Ok((self.value + delta).clamp(0, SCALE))
    }

    /// Đưa mốc neo tới `now`. Gọi khi cần ghi giá trị vào state.
    ///
    /// Sau khi gọi, `value_at(now)` cho cùng kết quả — nhưng bây giờ nó rẻ hơn
    /// và giá trị nhìn thấy được trong dump.
    pub fn settle(&mut self, now: Tick) -> MathResult<()> {
        let Some(dt) = now.since(self.last_update) else {
            return Ok(());
        };
        let (delta, carry) = self.rate.integrate(dt, self.carry)?;
        self.value = (self.value + delta).clamp(0, SCALE);
        self.carry = carry;
        self.last_update = now;
        Ok(())
    }

    /// Tick mà nhu cầu sẽ chạm `threshold`, hoặc `None` nếu không bao giờ.
    ///
    /// Đây là thứ khiến "không tick per-entity" khả thi: scheduler đặt một lần
    /// đánh thức tại tick này và quên thực thể đi cho tới lúc đó.
    pub fn next_threshold_tick(&self, threshold: i64, now: Tick) -> Option<Tick> {
        let hien_tai = self.value_at(now).ok()?;
        let can = threshold - hien_tai;
        if can == 0 {
            return Some(now);
        }
        let t = self.rate.ticks_to_accumulate(can, self.carry)?;
        now.plus(t)
    }

    /// Bổ sung, ví dụ ăn hoặc ngủ.
    pub fn replenish(&mut self, amount: i64, now: Tick) -> MathResult<()> {
        self.settle(now)?;
        self.value = (self.value + amount).clamp(0, SCALE);
        Ok(())
    }

    /// Tỉ lệ đầy, dùng cho UI. Chia nguyên, không có số thực.
    pub fn percent(&self, now: Tick) -> i64 {
        self.value_at(now).unwrap_or(0) * 100 / SCALE
    }
}

impl CanonicalHash for Need {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_i64(self.value);
        self.rate.canonical_hash(h);
        h.write_i64(self.carry);
        self.last_update.canonical_hash(h);
        self.domain.canonical_hash(h);
    }
}

/// Một ngưỡng đã khai báo, với hệ quả của nó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Threshold {
    /// Giá trị chạm.
    pub at: i64,
    /// Effect được áp khi chạm.
    pub effect: String,
    /// Nhãn cho UI: "đói", "kiệt sức".
    pub label: String,
}

/// Toàn bộ nhu cầu của một thực thể.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Homeostasis {
    needs: Vec<Need>,
}

impl Homeostasis {
    /// Rỗng.
    pub fn new() -> Homeostasis {
        Homeostasis::default()
    }

    /// Thêm một nhu cầu. Trùng id thì thay.
    pub fn insert(&mut self, n: Need) {
        if let Some(cu) = self.needs.iter_mut().find(|x| x.id == n.id) {
            *cu = n;
        } else {
            self.needs.push(n);
            // Giữ sắp xếp theo id: thứ tự duyệt đi vào state hash.
            self.needs.sort_by(|a, b| a.id.cmp(&b.id));
        }
    }

    /// Một nhu cầu.
    pub fn get(&self, id: &str) -> Option<&Need> {
        self.needs.iter().find(|n| n.id == id)
    }

    /// Một nhu cầu, để sửa.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Need> {
        self.needs.iter_mut().find(|n| n.id == id)
    }

    /// Mọi nhu cầu, theo thứ tự id.
    pub fn iter(&self) -> impl Iterator<Item = &Need> {
        self.needs.iter()
    }

    /// Số nhu cầu.
    pub fn len(&self) -> usize {
        self.needs.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.needs.is_empty()
    }

    /// Tick đánh thức sớm nhất trong số mọi nhu cầu.
    ///
    /// Đây là hàm mà scheduler gọi. Nó trả về **một** tick cho cả thực thể, nên
    /// một thực thể có sáu nhu cầu vẫn chỉ chiếm một mục trong hàng đợi.
    pub fn next_wakeup(&self, thresholds: &[(String, i64)], now: Tick) -> Option<Tick> {
        thresholds
            .iter()
            .filter_map(|(id, th)| self.get(id)?.next_threshold_tick(*th, now))
            .min()
    }

    /// Đưa mọi mốc neo tới `now`.
    pub fn settle_all(&mut self, now: Tick) -> MathResult<()> {
        for n in &mut self.needs {
            n.settle(now)?;
        }
        Ok(())
    }
}

impl CanonicalHash for Homeostasis {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.needs.iter(), |hh, n| n.canonical_hash(hh));
    }
}
