//! Túi đồ theo **thể tích và khối lượng** (`idea.md §18.15.1`, `§18.15.2`, `PB-19`).
//!
//! ## Hai ràng buộc, không phải một
//!
//! Một con số "sức chứa" duy nhất không mô tả được thế giới vật lý. Hai giới
//! hạn khác nhau chặn hai thứ khác nhau:
//!
//! - **Thể tích** chặn *hình dạng*. Một bó rơm nhẹ nhưng không nhét vừa túi.
//! - **Khối lượng** chặn *sức mang*. Một túi vàng nhỏ xíu nhưng nhấc không nổi.
//!
//! Gộp chúng lại thì mất cả hai câu chuyện: buôn lông thú (cồng kềnh, nhẹ) và
//! buôn kim loại (gọn, nặng) trở thành cùng một bài toán.
//!
//! ## Quá tải làm **chậm**, không **chặn**
//!
//! `§18.15.2` nói rõ. Đây là một quyết định thiết kế đáng bảo vệ: chặn cứng
//! biến một khoảnh khắc kịch tính — chạy khỏi hầm mộ đang sập với quá nhiều
//! vàng — thành một hộp thoại lỗi. Cho phép mang quá tải và chịu hậu quả thì
//! người chơi phải **quyết định**, và quyết định là thứ làm nên trò chơi.
//!
//! Ngoại lệ duy nhất là thể tích: một cái túi có thể tích hữu hạn thật sự không
//! nhét thêm được. Đó là hình học, không phải luật chơi.

use crate::item::{ItemDef, ItemStack};
use mow_math::{CanonicalHash, Mass, StateHasher, Unit, Volume};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Sức chứa của một vật chứa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capacity {
    /// Thể tích tối đa. **Cứng** — hình học không thương lượng.
    pub volume: Volume,
    /// Khối lượng mang thoải mái. Vượt qua thì chậm dần, không bị chặn.
    pub comfortable_mass: Mass,
    /// Khối lượng tối đa nhấc nổi. Vượt qua thì không đi được nữa.
    pub max_mass: Mass,
}

impl CanonicalHash for Capacity {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.volume.get());
        h.write_i64(self.comfortable_mass.get());
        h.write_i64(self.max_mass.get());
    }
}

/// Lỗi khi bỏ đồ vào.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InventoryError {
    /// Không đủ thể tích.
    #[error("không đủ chỗ: cần {need} mL, còn {free} mL")]
    NoVolume {
        /// Thể tích cần.
        need: i64,
        /// Thể tích còn.
        free: i64,
    },
    /// Không biết loại vật phẩm này.
    #[error("không có định nghĩa cho `{0}`")]
    UnknownDef(String),
}

/// Túi đồ.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    /// Các chồng, theo thứ tự `(def, quality)`.
    stacks: Vec<ItemStack>,
    /// Vật phẩm instance, theo định danh entity.
    instances: Vec<u64>,
}

/// Tình trạng tải, dùng cho UI (`§18.15.1`).
///
/// **Hai thanh riêng**, không phải một. Người chơi phải thấy được mình đang bị
/// chặn bởi cái nào — vì hai cái đó giải bằng hai cách khác nhau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Load {
    /// Thể tích đang dùng.
    pub volume_used: Volume,
    /// Thể tích tối đa.
    pub volume_max: Volume,
    /// Khối lượng đang mang.
    pub mass_carried: Mass,
    /// Khối lượng thoải mái.
    pub mass_comfortable: Mass,
    /// Khối lượng tối đa.
    pub mass_max: Mass,
}

impl Load {
    /// Tỉ lệ thể tích đã dùng, `[0,1]`, kẹp ở 1.
    pub fn volume_fraction(self) -> Unit {
        Unit::from_frac(self.volume_used.get(), self.volume_max.get().max(1)).unwrap_or(Unit::ONE)
    }

    /// Tỉ lệ khối lượng so với mức thoải mái. Có thể **vượt 1**.
    ///
    /// Trả `Fx` chứ không phải `Unit` vì quá tải là trạng thái hợp lệ, và kẹp
    /// nó về 1 sẽ giấu mất chính thông tin mà người chơi cần thấy.
    pub fn mass_ratio(self) -> mow_math::Fx {
        mow_math::Fx::from_frac(self.mass_carried.get(), self.mass_comfortable.get().max(1))
            .unwrap_or(mow_math::Fx::ZERO)
    }

    /// Có đang quá tải không.
    pub fn is_overloaded(self) -> bool {
        self.mass_carried > self.mass_comfortable
    }

    /// Có nặng tới mức không đi nổi không.
    pub fn is_immobilized(self) -> bool {
        self.mass_carried > self.mass_max
    }

    /// Hệ số tốc độ di chuyển, thang phần trăm.
    ///
    /// `100` là bình thường, `0` là không nhúc nhích. Giảm **liên tục** từ mức
    /// thoải mái tới mức tối đa: không có bậc thang, nên người chơi cảm nhận
    /// được cái giá của từng món đồ nhặt thêm thay vì bị chặn đột ngột.
    pub fn speed_percent(self) -> i64 {
        let mang = self.mass_carried.get();
        let thoai_mai = self.mass_comfortable.get();
        let toi_da = self.mass_max.get();
        if mang <= thoai_mai {
            return 100;
        }
        if mang >= toi_da {
            return 0;
        }
        let vuot = mang - thoai_mai;
        let khoang = (toi_da - thoai_mai).max(1);
        // Giảm tuyến tính từ 100 xuống 0 trong khoảng quá tải.
        100 - (vuot * 100 / khoang)
    }
}

impl Inventory {
    /// Túi rỗng.
    pub fn new() -> Inventory {
        Inventory::default()
    }

    /// Các chồng.
    pub fn stacks(&self) -> &[ItemStack] {
        &self.stacks
    }

    /// Các vật phẩm instance.
    pub fn instances(&self) -> &[u64] {
        &self.instances
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.stacks.is_empty() && self.instances.is_empty()
    }

    /// Tính tải hiện tại.
    pub fn load(
        &self,
        defs: &BTreeMap<String, ItemDef>,
        cap: Capacity,
        instance_mass: Mass,
        instance_volume: Volume,
    ) -> Load {
        let mut v = 0i64;
        let mut m = 0i64;
        for s in &self.stacks {
            if let Some(d) = defs.get(&s.def) {
                v += d.volume.get() * i64::from(s.count);
                m += d.mass.get() * i64::from(s.count);
            }
        }
        Load {
            volume_used: Volume::new(v + instance_volume.get()),
            volume_max: cap.volume,
            mass_carried: Mass::new(m + instance_mass.get()),
            mass_comfortable: cap.comfortable_mass,
            mass_max: cap.max_mass,
        }
    }

    /// Bỏ một chồng vào.
    ///
    /// Thể tích là ràng buộc **cứng** — hình học. Khối lượng thì không: quá tải
    /// được phép, và hậu quả nằm ở [`Load::speed_percent`].
    pub fn insert(
        &mut self,
        stack: ItemStack,
        defs: &BTreeMap<String, ItemDef>,
        cap: Capacity,
    ) -> Result<(), InventoryError> {
        let def = defs
            .get(&stack.def)
            .ok_or_else(|| InventoryError::UnknownDef(stack.def.clone()))?;

        let hien_tai = self.load(defs, cap, Mass::ZERO, Volume::ZERO);
        let can = def.volume.get() * i64::from(stack.count);
        let con = cap.volume.get() - hien_tai.volume_used.get();
        if can > con {
            return Err(InventoryError::NoVolume {
                need: can,
                free: con,
            });
        }

        // Gộp vào chồng sẵn có trước.
        let mut con_lai = Some(stack);
        for s in &mut self.stacks {
            if let Some(c) = con_lai.take() {
                con_lai = s.merge(&c, def.max_stack);
            }
            if con_lai.is_none() {
                break;
            }
        }
        if let Some(c) = con_lai {
            self.stacks.push(c);
            // Sắp theo `(def, quality)`: thứ tự duyệt đi vào state hash, và nó
            // cũng là thứ tự hiện trong giao diện — người chơi thấy đồ giống
            // nhau nằm cạnh nhau thay vì rải rác theo thứ tự nhặt.
            self.stacks
                .sort_by(|a, b| a.def.cmp(&b.def).then(a.quality.cmp(&b.quality)));
        }
        Ok(())
    }

    /// Lấy ra `n` món của một loại. Trả số thực sự lấy được.
    pub fn take(&mut self, def_id: &str, n: u32) -> u32 {
        let mut con_can = n;
        for s in &mut self.stacks {
            if s.def != def_id || con_can == 0 {
                continue;
            }
            let lay = con_can.min(s.count);
            s.count -= lay;
            con_can -= lay;
        }
        self.stacks.retain(|s| s.count > 0);
        n - con_can
    }

    /// Đếm số món của một loại.
    pub fn count(&self, def_id: &str) -> u32 {
        self.stacks
            .iter()
            .filter(|s| s.def == def_id)
            .map(|s| s.count)
            .sum()
    }

    /// Thêm một vật phẩm instance.
    pub fn add_instance(&mut self, entity: u64) {
        if !self.instances.contains(&entity) {
            self.instances.push(entity);
            self.instances.sort_unstable();
        }
    }

    /// Gỡ một vật phẩm instance.
    pub fn remove_instance(&mut self, entity: u64) -> bool {
        let truoc = self.instances.len();
        self.instances.retain(|e| *e != entity);
        self.instances.len() != truoc
    }
}

impl CanonicalHash for Inventory {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.stacks.iter(), |hh, s| s.canonical_hash(hh));
        h.write_seq(self.instances.iter().copied(), |hh, e| {
            hh.write_u64(e);
        });
    }
}
