//! Trang bị theo **bộ phận cơ thể**, có lớp (`idea.md §18.15.4`, `PB-21`).
//!
//! ## Vì sao không phải một danh sách slot cố định
//!
//! Cách thông thường: `head`, `chest`, `legs`, `hands`, `feet`. Nó hoạt động
//! cho tới khi có một loài không giống người, và trong thế giới này thì luôn có.
//!
//! Một loài bốn tay cần bốn chỗ đeo găng. Một loài rắn không có chân. Một con
//! nhện có tám. Với danh sách cố định, mỗi loài mới là một lần sửa engine — và
//! content pack của cộng đồng thì không sửa được engine.
//!
//! Ở đây chỗ mặc **suy ra từ sơ đồ cơ thể**: `ItemDef::equip_slots` trỏ tới
//! bộ phận, và bộ phận nào có trong sơ đồ thì mặc được. Loài bốn tay tự nhiên
//! có bốn chỗ, không cần ai làm gì cả.
//!
//! ## Lớp và che phủ quyết định thương tích rơi vào đâu
//!
//! Nhiều món cùng che một bộ phận thì xếp theo `layer` — áo lót trong, giáp
//! ngoài. Khi một đòn đánh trúng, nó xuyên qua từ ngoài vào, và mỗi lớp có cơ
//! hội cản. `coverage` quyết định xác suất đòn đánh trúng chỗ có giáp hay chỗ
//! hở, nên một bộ giáp che 70% thật sự để lại 30% chỗ hở — chứ không phải giảm
//! 30% sát thương ở mọi chỗ.

use crate::item::ItemDef;
use mow_math::{CanonicalHash, StateHasher, Unit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Một món đã mặc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipped {
    /// Entity của món đồ.
    pub entity: u64,
    /// Loại.
    pub def: String,
    /// Bộ phận mà nó đang che.
    pub slot: String,
    /// Lớp.
    pub layer: u8,
    /// Phần che phủ.
    pub coverage: Unit,
}

impl CanonicalHash for Equipped {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.entity);
        h.write_str(&self.def);
        h.write_str(&self.slot);
        h.write_u64(u64::from(self.layer));
        h.write_i64(self.coverage.get().raw());
    }
}

/// Lỗi khi mặc.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EquipError {
    /// Món này không mặc được.
    #[error("`{0}` không phải trang bị — nó không khai báo `equip_slots`")]
    NotEquipment(String),
    /// Cơ thể không có bộ phận đó.
    #[error(
        "giải phẫu không có `{slot}`. Chỗ mặc suy ra từ sơ đồ cơ thể, nên một loài \
         không có bộ phận đó thì không mặc được — đây là kết quả đúng, không phải lỗi."
    )]
    NoSuchSlot {
        /// Bộ phận thiếu.
        slot: String,
    },
    /// Lớp đó đã có món khác.
    #[error("`{slot}` đã có món ở lớp {layer}")]
    LayerOccupied {
        /// Bộ phận.
        slot: String,
        /// Lớp.
        layer: u8,
    },
}

/// Toàn bộ trang bị của một thực thể.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    items: Vec<Equipped>,
}

impl Equipment {
    /// Rỗng.
    pub fn new() -> Equipment {
        Equipment::default()
    }

    /// Mặc một món.
    ///
    /// `body_parts` là danh sách bộ phận mà **cơ thể này** có. Truyền vào thay
    /// vì tra một bảng toàn cục: đó là cách một loài bốn tay có bốn chỗ đeo găng
    /// mà không ai phải sửa engine.
    pub fn equip(
        &mut self,
        entity: u64,
        def: &ItemDef,
        slot: &str,
        body_parts: &[String],
    ) -> Result<(), EquipError> {
        if def.equip_slots.is_empty() {
            return Err(EquipError::NotEquipment(def.id.clone()));
        }
        if !def.equip_slots.iter().any(|s| s == slot) {
            return Err(EquipError::NoSuchSlot {
                slot: slot.to_owned(),
            });
        }
        if !body_parts.iter().any(|p| p == slot) {
            return Err(EquipError::NoSuchSlot {
                slot: slot.to_owned(),
            });
        }

        let layer = def.layer.unwrap_or(1);
        if self
            .items
            .iter()
            .any(|e| e.slot == slot && e.layer == layer)
        {
            return Err(EquipError::LayerOccupied {
                slot: slot.to_owned(),
                layer,
            });
        }

        self.items.push(Equipped {
            entity,
            def: def.id.clone(),
            slot: slot.to_owned(),
            layer,
            coverage: def.coverage,
        });
        // Sắp theo `(slot, layer)`: thứ tự này **là** thứ tự xuyên giáp, nên nó
        // phải là dữ liệu chứ không phải thứ tự mặc.
        self.items
            .sort_by(|a, b| a.slot.cmp(&b.slot).then(a.layer.cmp(&b.layer)));
        Ok(())
    }

    /// Cởi một món.
    pub fn unequip(&mut self, entity: u64) -> bool {
        let truoc = self.items.len();
        self.items.retain(|e| e.entity != entity);
        self.items.len() != truoc
    }

    /// Mọi món đang mặc.
    pub fn items(&self) -> &[Equipped] {
        &self.items
    }

    /// Các lớp che một bộ phận, **từ ngoài vào trong**.
    ///
    /// Đòn đánh xuyên theo đúng thứ tự này. Từ ngoài vào trong chứ không phải
    /// ngược lại: áo choàng phải cản trước áo giáp, và áo giáp trước áo lót.
    pub fn layers_over(&self, slot: &str) -> Vec<&Equipped> {
        let mut v: Vec<&Equipped> = self.items.iter().filter(|e| e.slot == slot).collect();
        v.sort_by(|a, b| b.layer.cmp(&a.layer).then(a.def.cmp(&b.def)));
        v
    }

    /// Tổng che phủ của một bộ phận, `[0,1]`.
    ///
    /// Kết hợp theo xác suất bù, không cộng: hai món cùng che 60% thì phần hở
    /// là `0.4 × 0.4 = 16%`, không phải `0%`. Cộng thẳng sẽ khiến hai món giáp
    /// tầm thường cho ra một bộ giáp kín tuyệt đối.
    pub fn coverage_of(&self, slot: &str) -> Unit {
        let mut ho = Unit::ONE;
        for e in self.items.iter().filter(|e| e.slot == slot) {
            ho = ho.and(e.coverage.complement());
        }
        ho.complement()
    }

    /// Đòn đánh vào một bộ phận có trúng chỗ hở không.
    ///
    /// `roll` là một giá trị `[0,1]` đã rút từ dòng ngẫu nhiên có tên. Truyền
    /// vào thay vì tự rút: hàm này phải thuần để test được và để replay đúng.
    pub fn hits_gap(&self, slot: &str, roll: Unit) -> bool {
        roll > self.coverage_of(slot)
    }

    /// Chỗ mặc khả dụng cho một cơ thể.
    ///
    /// Giao của những gì món đồ khai báo và những gì cơ thể có.
    pub fn available_slots<'a>(def: &'a ItemDef, body_parts: &'a [String]) -> Vec<&'a String> {
        def.equip_slots
            .iter()
            .filter(|s| body_parts.contains(s))
            .collect()
    }

    /// Tổng bảo vệ của một bộ phận từ mọi lớp, thang phần trăm.
    ///
    /// Cần bảng định nghĩa để biết giá trị bảo vệ của từng món.
    pub fn protection_of(&self, slot: &str, defs: &BTreeMap<String, ItemDef>) -> i64 {
        self.layers_over(slot)
            .iter()
            .filter_map(|e| defs.get(&e.def))
            .map(|d| {
                // Bảo vệ tỉ lệ với che phủ: một tấm giáp che nửa người thì cản
                // được nửa số đòn, không phải giảm nửa sát thương mọi đòn.
                d.coverage.get().raw() * 100 / mow_math::fixed::ONE_RAW
            })
            .sum()
    }
}

impl CanonicalHash for Equipment {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.items.iter(), |hh, e| e.canonical_hash(hh));
    }
}
