//! Kinh tế nhỏ: nguồn, công thức, thị trường địa phương (`idea.md §12.2`, `PB-13`).
//!
//! ## Tài nguyên phải có **nguồn thật**
//!
//! Đây là ràng buộc quan trọng nhất của cả module, và nó dễ bị vi phạm theo một
//! cách trông rất vô hại: cho một cửa hàng "luôn có 50 ổ bánh". Nghe tiện, và
//! nó phá kinh tế từ gốc:
//!
//! - Bánh mì xuất hiện từ hư không, nên giá bánh không bao giờ phản ánh mùa
//!   màng, chiến tranh, hay hạn hán.
//! - Không ai có lý do làm nông, vì bánh mì vốn đã vô hạn.
//! - Người chơi phát hiện ra trong mười phút và khai thác nó.
//!
//! Nên mọi thứ trong kinh tế này đến từ một [`Source`] có trữ lượng hữu hạn và
//! tốc độ tái tạo xác định, hoặc từ một [`Recipe`] biến thứ này thành thứ khác.
//! Không có ngoại lệ.
//!
//! ## Giá **hình thành**, không được đặt
//!
//! `§22.35`: vật phẩm không lưu giá trị. Giá là kết quả của cung, cầu, và
//! belief của người đánh giá. [`Market::clearing_price`] tính nó ra; không có
//! hàm nào đặt giá.

use mow_math::{CanonicalHash, Money, Rate, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Một nguồn tài nguyên trong thế giới.
///
/// Hữu hạn và tái tạo có tốc độ. Một mỏ sắt cạn thì cạn thật; một khu rừng bị
/// chặt trụi cần thời gian để mọc lại.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Định danh.
    pub id: String,
    /// Loại tài nguyên nó cho ra.
    pub yields: String,
    /// Trữ lượng còn lại.
    pub remaining: i64,
    /// Trữ lượng tối đa.
    pub capacity: i64,
    /// Tốc độ tái tạo mỗi tick. `Rate::ZERO` cho tài nguyên không tái tạo.
    ///
    /// Hữu tỉ vì tốc độ tái tạo của một khu rừng là rất nhỏ — nhỏ hơn một đơn
    /// vị mỗi tick — và làm tròn nó về 0 sẽ khiến rừng không bao giờ mọc lại.
    pub regen: Rate,
    /// Số dư của tích phân tái tạo.
    pub carry: i64,
}

impl Source {
    /// Khai thác. Trả về số thực sự lấy được.
    ///
    /// Không bao giờ lấy quá trữ lượng còn lại. Đó là toàn bộ điểm.
    pub fn extract(&mut self, want: i64) -> i64 {
        let lay = want.clamp(0, self.remaining);
        self.remaining -= lay;
        lay
    }

    /// Tái tạo qua `ticks` tick.
    pub fn regenerate(&mut self, ticks: u64) {
        if self.regen == Rate::ZERO || self.remaining >= self.capacity {
            return;
        }
        if let Ok((d, c)) = self.regen.integrate(ticks, self.carry) {
            self.carry = c;
            self.remaining = (self.remaining + d).clamp(0, self.capacity);
        }
    }

    /// Đã cạn chưa.
    pub fn is_depleted(&self) -> bool {
        self.remaining == 0
    }

    /// Cạn vĩnh viễn — không tái tạo và đã hết.
    pub fn is_exhausted(&self) -> bool {
        self.is_depleted() && self.regen == Rate::ZERO
    }
}

impl CanonicalHash for Source {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_str(&self.yields);
        h.write_i64(self.remaining);
        h.write_i64(self.capacity);
        self.regen.canonical_hash(h);
        h.write_i64(self.carry);
    }
}

/// Một công thức chế biến.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    /// Định danh.
    pub id: String,
    /// Nguyên liệu: `(loại, số lượng)`.
    pub inputs: Vec<(String, u32)>,
    /// Sản phẩm.
    pub outputs: Vec<(String, u32)>,
    /// Số tick để làm.
    pub ticks: u64,
    /// Kỹ năng cần, thang `0`–`100`.
    pub skill_required: u8,
}

impl Recipe {
    /// Có đủ nguyên liệu không.
    pub fn can_make(&self, stock: &BTreeMap<String, i64>) -> bool {
        self.inputs
            .iter()
            .all(|(k, n)| stock.get(k).copied().unwrap_or(0) >= i64::from(*n))
    }

    /// Trừ nguyên liệu và cộng sản phẩm. Trả `false` nếu thiếu.
    pub fn apply(&self, stock: &mut BTreeMap<String, i64>) -> bool {
        if !self.can_make(stock) {
            return false;
        }
        for (k, n) in &self.inputs {
            *stock.entry(k.clone()).or_default() -= i64::from(*n);
        }
        for (k, n) in &self.outputs {
            *stock.entry(k.clone()).or_default() += i64::from(*n);
        }
        true
    }
}

impl CanonicalHash for Recipe {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_seq(self.inputs.iter(), |hh, (k, n)| {
            hh.write_str(k);
            hh.write_u64(u64::from(*n));
        });
        h.write_seq(self.outputs.iter(), |hh, (k, n)| {
            hh.write_str(k);
            hh.write_u64(u64::from(*n));
        });
        h.write_u64(self.ticks);
        h.write_u64(u64::from(self.skill_required));
    }
}

/// Một lệnh mua hoặc bán.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    /// Ai đặt.
    pub trader: u64,
    /// Loại hàng.
    pub good: String,
    /// Số lượng.
    pub quantity: u32,
    /// Giá mà **người này** sẵn sàng — belief của họ, không phải giá trị của vật.
    pub limit_price: Money,
}

impl CanonicalHash for Order {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.trader);
        h.write_str(&self.good);
        h.write_u64(u64::from(self.quantity));
        h.write_i64(self.limit_price.get());
    }
}

/// Một giao dịch đã khớp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trade {
    /// Người mua.
    pub buyer: u64,
    /// Người bán.
    pub seller: u64,
    /// Hàng.
    pub good: String,
    /// Số lượng.
    pub quantity: u32,
    /// Giá khớp.
    pub price: Money,
}

/// Thị trường địa phương.
///
/// **Địa phương** là một từ có nghĩa: `§12.17` yêu cầu hàng hóa không teleport,
/// nên giá ở hai làng cách nhau ba ngày đường **phải** khác nhau. Một thị trường
/// toàn cầu duy nhất sẽ xóa mất thương mại như một hoạt động.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Market {
    /// Nơi này ở đâu.
    pub place: String,
    bids: Vec<Order>,
    asks: Vec<Order>,
    /// Giá khớp gần nhất của từng loại hàng — **quan sát được**, không phải giá trị.
    last_price: BTreeMap<String, Money>,
}

impl Market {
    /// Thị trường rỗng tại một nơi.
    pub fn new(place: &str) -> Market {
        Market {
            place: place.to_owned(),
            ..Market::default()
        }
    }

    /// Đặt lệnh mua.
    pub fn bid(&mut self, o: Order) {
        self.bids.push(o);
    }

    /// Đặt lệnh bán.
    pub fn ask(&mut self, o: Order) {
        self.asks.push(o);
    }

    /// Giá khớp gần nhất, `None` nếu chưa có giao dịch nào.
    ///
    /// `None` là thông tin quan trọng: một mặt hàng chưa từng được bán thì
    /// **không có giá**, và nhân vật phải đoán. Trả về 0 thay cho `None` sẽ
    /// biến "chưa ai biết" thành "miễn phí".
    pub fn last(&self, good: &str) -> Option<Money> {
        self.last_price.get(good).copied()
    }

    /// Khớp lệnh và trả các giao dịch.
    ///
    /// Người trả cao nhất khớp với người bán rẻ nhất; giá khớp là **trung điểm**.
    /// Trung điểm chứ không phải giá của một bên: nếu luôn lấy giá người bán thì
    /// người mua không bao giờ được lợi, và không ai có động cơ mặc cả.
    pub fn clear(&mut self) -> Vec<Trade> {
        // Sắp theo giá, phá hòa bằng `trader` — thứ tự đặt lệnh không được
        // quyết định ai khớp trước.
        self.bids.sort_by(|a, b| {
            b.limit_price
                .cmp(&a.limit_price)
                .then(a.trader.cmp(&b.trader))
        });
        self.asks.sort_by(|a, b| {
            a.limit_price
                .cmp(&b.limit_price)
                .then(a.trader.cmp(&b.trader))
        });

        let mut ra = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.bids.len() && j < self.asks.len() {
            let (mua, ban) = (self.bids[i].clone(), self.asks[j].clone());
            if mua.good != ban.good {
                // Hàng khác loại: đẩy con trỏ của bên có tên nhỏ hơn.
                if mua.good < ban.good {
                    i += 1;
                } else {
                    j += 1;
                }
                continue;
            }
            if mua.limit_price < ban.limit_price {
                // Người trả cao nhất vẫn dưới giá rẻ nhất — không có giao dịch nào.
                break;
            }

            let sl = mua.quantity.min(ban.quantity);
            let gia = Money::new(i64::midpoint(mua.limit_price.get(), ban.limit_price.get()));
            ra.push(Trade {
                buyer: mua.trader,
                seller: ban.trader,
                good: mua.good.clone(),
                quantity: sl,
                price: gia,
            });
            self.last_price.insert(mua.good.clone(), gia);

            self.bids[i].quantity -= sl;
            self.asks[j].quantity -= sl;
            if self.bids[i].quantity == 0 {
                i += 1;
            }
            if self.asks[j].quantity == 0 {
                j += 1;
            }
        }

        self.bids.retain(|o| o.quantity > 0);
        self.asks.retain(|o| o.quantity > 0);
        ra
    }

    /// Giá cân bằng ước lượng — **hình thành từ cung cầu**, không được đặt.
    ///
    /// Không có `set_price`. Nếu ai đó cần một giá cố định, họ đang muốn một
    /// bảng giá, và bảng giá là thứ `§22.35` cấm.
    pub fn clearing_price(&self, good: &str) -> Option<Money> {
        let cao_nhat = self
            .bids
            .iter()
            .filter(|o| o.good == good)
            .map(|o| o.limit_price)
            .max()?;
        let re_nhat = self
            .asks
            .iter()
            .filter(|o| o.good == good)
            .map(|o| o.limit_price)
            .min()?;
        if cao_nhat < re_nhat {
            return None;
        }
        Some(Money::new(i64::midpoint(cao_nhat.get(), re_nhat.get())))
    }

    /// Chênh lệch cung cầu của một loại hàng.
    ///
    /// Dương là thiếu hàng. Đây là tín hiệu mà AI dùng để quyết định làm nghề gì.
    pub fn imbalance(&self, good: &str) -> i64 {
        let cau: i64 = self
            .bids
            .iter()
            .filter(|o| o.good == good)
            .map(|o| i64::from(o.quantity))
            .sum();
        let cung: i64 = self
            .asks
            .iter()
            .filter(|o| o.good == good)
            .map(|o| i64::from(o.quantity))
            .sum();
        cau - cung
    }

    /// Số lệnh đang chờ.
    pub fn open_orders(&self) -> usize {
        self.bids.len() + self.asks.len()
    }
}

impl CanonicalHash for Market {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.place);
        h.write_seq(self.bids.iter(), |hh, o| o.canonical_hash(hh));
        h.write_seq(self.asks.iter(), |hh, o| o.canonical_hash(hh));
        h.write_seq(self.last_price.iter(), |hh, (k, v)| {
            hh.write_str(k);
            hh.write_i64(v.get());
        });
    }
}
