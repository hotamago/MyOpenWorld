//! Vật phẩm: định nghĩa, chất lượng, tình trạng (`idea.md §8.5`–`§8.7`).
//!
//! ## `CraftQuality` và `Condition` là hai thứ khác hẳn nhau (`§22.34`)
//!
//! > `CraftQuality` **bất biến** sau khi chế tác; sửa chữa chỉ phục hồi
//! > `Condition`.
//!
//! Gộp chúng lại thành một "độ bền" duy nhất là sai lầm phổ biến nhất trong hệ
//! vật phẩm, và nó xóa mất một loạt thứ:
//!
//! - Một thanh kiếm của bậc thầy, đã cùn, vẫn **là** kiếm của bậc thầy. Mài lại
//!   thì nó trở lại sắc bén như xưa. Nếu chỉ có một con số, mài lại một thanh
//!   kiếm rẻ tiền sẽ cho ra kiếm của bậc thầy.
//! - Danh tiếng thợ rèn không tồn tại được. Không ai trả giá cao cho tay nghề
//!   nếu tay nghề bị mài mòn đi.
//! - Đồ cổ không có ý nghĩa. Một món đồ cũ kỹ nhưng tuyệt tác là một câu
//!   chuyện; một món đồ "độ bền 30%" thì không.
//!
//! Nên `CraftQuality` **không có** hàm nào sửa được nó sau khi dựng.

use mow_math::{CanonicalHash, Mass, StateHasher, Unit, Volume};
use serde::{Deserialize, Serialize};

/// Chất lượng chế tác. **Bất biến suốt đời món đồ.**
///
/// Không có `set_quality`, không có `quality_mut`. Muốn một món đồ tốt hơn thì
/// phải chế tác một món mới.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftQuality {
    /// Vụng về.
    Crude,
    /// Bình thường.
    Plain,
    /// Khéo.
    Fine,
    /// Tinh xảo.
    Superior,
    /// Kiệt tác.
    Masterwork,
}

impl CraftQuality {
    /// Hệ số nhân lên hiệu quả, thang phần trăm.
    pub fn multiplier_percent(self) -> i64 {
        match self {
            CraftQuality::Crude => 70,
            CraftQuality::Plain => 100,
            CraftQuality::Fine => 120,
            CraftQuality::Superior => 145,
            CraftQuality::Masterwork => 180,
        }
    }

    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            CraftQuality::Crude => "crude",
            CraftQuality::Plain => "plain",
            CraftQuality::Fine => "fine",
            CraftQuality::Superior => "superior",
            CraftQuality::Masterwork => "masterwork",
        }
    }
}

impl CanonicalHash for CraftQuality {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

/// Tình trạng của **một bộ phận** của món đồ (`PB-20`).
///
/// Theo bộ phận chứ không phải một con số duy nhất: một cây rìu có lưỡi và cán,
/// và chúng hỏng theo hai cách khác nhau cần hai cách sửa khác nhau. Một con số
/// duy nhất không nói được "cán còn tốt, lưỡi mẻ".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartCondition {
    /// Bộ phận: `blade`, `haft`, `binding`.
    pub part: String,
    /// Tình trạng, `[0,1]`.
    pub condition: Unit,
}

impl CanonicalHash for PartCondition {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.part);
        h.write_i64(self.condition.get().raw());
    }
}

/// Một lần sửa chữa đã ghi lại (`PB-20`).
///
/// Lịch sử sửa chữa là **dữ liệu của thế giới**, không phải trang trí: nó cho
/// biết ai đã chạm vào món đồ, và một món đồ được một thợ rèn nổi tiếng sửa thì
/// khác một món đồ được vá tạm ngoài chiến trường.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRecord {
    /// Ai sửa.
    pub by: u64,
    /// Tick.
    pub at_tick: u64,
    /// Bộ phận nào.
    pub part: String,
    /// Phục hồi được bao nhiêu.
    pub restored: Unit,
}

impl CanonicalHash for RepairRecord {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.by);
        h.write_u64(self.at_tick);
        h.write_str(&self.part);
        h.write_i64(self.restored.get().raw());
    }
}

/// Định nghĩa một loại vật phẩm, từ content pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDef {
    /// Định danh có namespace.
    pub id: String,
    /// Khối lượng một đơn vị.
    pub mass: Mass,
    /// Thể tích một đơn vị.
    pub volume: Volume,
    /// Xếp chồng được tối đa bao nhiêu. `1` nghĩa là không xếp chồng.
    pub max_stack: u32,
    /// Các bộ phận có tình trạng riêng.
    pub parts: Vec<String>,
    /// Chỗ mặc trên cơ thể, nếu là trang bị (`PB-21`).
    ///
    /// Tham chiếu **chức năng hoặc bộ phận** của sơ đồ cơ thể, không phải một
    /// danh sách slot cố định. Nhờ vậy một loài bốn tay tự nhiên có bốn chỗ đeo
    /// găng mà không cần sửa engine.
    pub equip_slots: Vec<String>,
    /// Lớp mặc: số nhỏ nằm trong. Áo lót 0, áo giáp 2, áo choàng 3.
    pub layer: Option<u8>,
    /// Che phủ bao nhiêu phần của bộ phận — quyết định thương tích rơi vào đâu.
    pub coverage: Unit,
    /// Thẻ phân loại: `weapon`, `food`, `tool`.
    pub tags: Vec<String>,
}

impl ItemDef {
    /// Có xếp chồng được không.
    pub fn stackable(&self) -> bool {
        self.max_stack > 1
    }
}

impl CanonicalHash for ItemDef {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_i64(self.mass.get());
        h.write_i64(self.volume.get());
        h.write_u64(u64::from(self.max_stack));
        h.write_seq(self.parts.iter(), |hh, p| {
            hh.write_str(p);
        });
        h.write_seq(self.equip_slots.iter(), |hh, s| {
            hh.write_str(s);
        });
        h.write_option(self.layer, |hh, l| {
            hh.write_u64(u64::from(l));
        });
        h.write_i64(self.coverage.get().raw());
        h.write_seq(self.tags.iter(), |hh, t| {
            hh.write_str(t);
        });
    }
}

/// Một vật phẩm ở mức **instance** — có lịch sử riêng.
///
/// `§22.32`: instance là một ECS entity có component. Struct này là *component
/// dữ liệu* của entity đó, không phải một bảng vật phẩm song song.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemInstance {
    /// Loại.
    pub def: String,
    /// Chất lượng chế tác. **Chỉ đọc sau khi dựng.**
    quality: CraftQuality,
    /// Ai chế tác. Bất biến, như chất lượng.
    crafted_by: Option<u64>,
    /// Tick chế tác.
    crafted_at: u64,
    /// Tình trạng theo bộ phận.
    pub conditions: Vec<PartCondition>,
    /// Lịch sử sửa chữa.
    pub repairs: Vec<RepairRecord>,
}

impl ItemInstance {
    /// Chế tác một món đồ mới.
    ///
    /// Đây là **chỗ duy nhất** `quality` được đặt. Sau đây nó bất biến.
    pub fn craft(
        def: &ItemDef,
        quality: CraftQuality,
        crafted_by: Option<u64>,
        at_tick: u64,
    ) -> ItemInstance {
        ItemInstance {
            def: def.id.clone(),
            quality,
            crafted_by,
            crafted_at: at_tick,
            conditions: def
                .parts
                .iter()
                .map(|p| PartCondition {
                    part: p.clone(),
                    condition: Unit::ONE,
                })
                .collect(),
            repairs: Vec::new(),
        }
    }

    /// Chất lượng chế tác. Không có phiên bản `mut`.
    pub fn quality(&self) -> CraftQuality {
        self.quality
    }

    /// Ai chế tác.
    pub fn crafted_by(&self) -> Option<u64> {
        self.crafted_by
    }

    /// Tick chế tác.
    pub fn crafted_at(&self) -> u64 {
        self.crafted_at
    }

    /// Tình trạng của bộ phận yếu nhất — thứ quyết định món đồ có dùng được không.
    ///
    /// Lấy nhỏ nhất chứ không phải trung bình: một cây rìu có cán gãy thì không
    /// dùng được, dù lưỡi vẫn hoàn hảo. Trung bình sẽ nói nó còn 50% và sai.
    pub fn worst_condition(&self) -> Unit {
        self.conditions
            .iter()
            .map(|c| c.condition)
            .min()
            .unwrap_or(Unit::ONE)
    }

    /// Hiệu quả thực tế, thang phần trăm.
    ///
    /// Tích của chất lượng chế tác và tình trạng. Đây là chỗ hai khái niệm gặp
    /// nhau — và gặp nhau **bằng một phép nhân**, chứ không phải bằng cách gộp
    /// thành một trường.
    pub fn effectiveness_percent(&self) -> i64 {
        let q = self.quality.multiplier_percent();
        let c = self.worst_condition().get().raw() * 100 / mow_math::fixed::ONE_RAW;
        q * c / 100
    }

    /// Hao mòn một bộ phận.
    pub fn wear(&mut self, part: &str, amount: Unit) {
        if let Some(c) = self.conditions.iter_mut().find(|c| c.part == part) {
            let moi = c
                .condition
                .get()
                .sub(amount.get())
                .unwrap_or(mow_math::Fx::ZERO);
            c.condition = Unit::saturating(moi);
        }
    }

    /// Sửa chữa. **Chỉ phục hồi `Condition`; `CraftQuality` không đổi** (`§22.34`).
    ///
    /// Trả `false` nếu không có bộ phận đó.
    pub fn repair(&mut self, part: &str, amount: Unit, by: u64, at_tick: u64) -> bool {
        let Some(c) = self.conditions.iter_mut().find(|c| c.part == part) else {
            return false;
        };
        let truoc = c.condition;
        let moi = c
            .condition
            .get()
            .add(amount.get())
            .unwrap_or(mow_math::Fx::ONE);
        c.condition = Unit::saturating(moi);
        let thuc_te = Unit::saturating(
            c.condition
                .get()
                .sub(truoc.get())
                .unwrap_or(mow_math::Fx::ZERO),
        );

        self.repairs.push(RepairRecord {
            by,
            at_tick,
            part: part.to_owned(),
            restored: thuc_te,
        });
        true
    }

    /// Món đồ đã hỏng hẳn chưa.
    pub fn is_broken(&self) -> bool {
        self.worst_condition() == Unit::ZERO
    }
}

impl CanonicalHash for ItemInstance {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.def);
        self.quality.canonical_hash(h);
        h.write_option(self.crafted_by, |hh, v| {
            hh.write_u64(v);
        });
        h.write_u64(self.crafted_at);
        h.write_seq(self.conditions.iter(), |hh, c| c.canonical_hash(hh));
        h.write_seq(self.repairs.iter(), |hh, r| r.canonical_hash(hh));
    }
}

/// Một chồng vật phẩm giống hệt nhau (`§8.5.2`, `§22.32`).
///
/// **Không phải một entity riêng.** Đây là một component dữ liệu gắn trên
/// entity vật chứa — cái túi, cái hòm, hay chính nhân vật. Ba mươi mũi tên
/// giống hệt nhau không cần ba mươi entity, và cũng không cần một bảng vật phẩm
/// thứ hai nằm ngoài ECS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    /// Loại.
    pub def: String,
    /// Số lượng.
    pub count: u32,
    /// Chất lượng chung. Đồ trong một chồng phải giống hệt nhau, kể cả chất lượng.
    pub quality: CraftQuality,
}

impl ItemStack {
    /// Gộp hai chồng nếu chúng tương thích. Trả phần dư không gộp được.
    pub fn merge(&mut self, other: &ItemStack, max_stack: u32) -> Option<ItemStack> {
        if self.def != other.def || self.quality != other.quality {
            return Some(other.clone());
        }
        let cho_trong = max_stack.saturating_sub(self.count);
        let chuyen = cho_trong.min(other.count);
        self.count += chuyen;
        let con = other.count - chuyen;
        if con == 0 {
            None
        } else {
            Some(ItemStack {
                def: other.def.clone(),
                count: con,
                quality: other.quality,
            })
        }
    }

    /// Tách một phần ra thành chồng mới.
    pub fn split(&mut self, n: u32) -> Option<ItemStack> {
        if n == 0 || n >= self.count {
            return None;
        }
        self.count -= n;
        Some(ItemStack {
            def: self.def.clone(),
            count: n,
            quality: self.quality,
        })
    }
}

impl CanonicalHash for ItemStack {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.def);
        h.write_u64(u64::from(self.count));
        self.quality.canonical_hash(h);
    }
}
