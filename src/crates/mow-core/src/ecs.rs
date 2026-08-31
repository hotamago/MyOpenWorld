//! Kho thực thể, dựng trên `bevy_ecs` (`plan.md §P3.1`).
//!
//! Tầng này cố tình mỏng. `bevy_ecs` lo lưu trữ và truy vấn theo archetype;
//! phần thêm vào ở đây là ba thứ mà một ECS đa dụng không thể tự có:
//!
//! 1. **Chỉ mục theo [`EntityId`] có thứ tự.** `Entity` của bevy là chỉ số nội
//!    bộ, tái sử dụng sau khi despawn và không ổn định qua save/load. Định danh
//!    của thế giới phải ổn định suốt đời, kể cả khi thực thể đi qua cổng.
//! 2. **Duyệt theo thứ tự xác định.** Thứ tự archetype phụ thuộc lịch sử chèn;
//!    chỉ mục [`std::collections::BTreeMap`] ở đây cho một thứ tự duy nhất,
//!    độc lập với việc thực thể được tạo ra thế nào (`plan.md §P10.3`).
//! 3. **Ranh giới ghi.** Mọi hàm đổi state ở đây là `pub(crate)`. Từ ngoài
//!    crate, `Store` chỉ đọc được. Đó là cách `§22.1` được thực thi bằng trình
//!    biên dịch chứ không bằng quy ước.

use crate::ids::EntityId;
use crate::value::Value;
use bevy_ecs::prelude::*;
use mow_math::{CanonicalHash, StateHasher};
use std::collections::BTreeMap;

/// Khóa thuộc tính, có namespace: `core.position`, `mypack.blessing`.
pub type AttrKey = String;

/// Định danh bền của thực thể, gắn vào mỗi entity của bevy.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity(pub EntityId);

/// Túi thuộc tính động.
///
/// Vì sao có nó bên cạnh component có kiểu của bevy: content pack phải định
/// nghĩa được thuộc tính mới **mà không cần biên dịch lại engine** (`§19.7`).
/// Component có kiểu vẫn là đường đi cho dữ liệu nóng của engine; túi này là
/// đường đi cho dữ liệu do dữ liệu định nghĩa.
///
/// [`BTreeMap`] chứ không phải `HashMap`: thứ tự duyệt đi thẳng vào state hash.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs(pub BTreeMap<AttrKey, Value>);

/// Kho thực thể.
pub struct Store {
    world: World,
    /// Chỉ mục ổn định. `BTreeMap` cho thứ tự duyệt xác định.
    index: BTreeMap<EntityId, Entity>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Store {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Store")
            .field("entities", &self.index.len())
            .finish_non_exhaustive()
    }
}

impl Store {
    /// Kho rỗng.
    pub fn new() -> Store {
        Store {
            world: World::new(),
            index: BTreeMap::new(),
        }
    }

    // ── Đọc ─────────────────────────────────────────────────────────────────

    /// Thực thể có tồn tại không.
    pub fn contains(&self, id: EntityId) -> bool {
        self.index.contains_key(&id)
    }

    /// Số thực thể.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Toàn bộ định danh, **theo thứ tự tăng dần**.
    ///
    /// `DoubleEndedIterator` để `.next_back()` lấy được id lớn nhất — thứ mà
    /// genesis cần để gán tên cho thực thể vừa tạo, và làm được trong `O(1)`
    /// thay vì duyệt hết.
    pub fn ids(&self) -> impl DoubleEndedIterator<Item = EntityId> + '_ {
        self.index.keys().copied()
    }

    /// Túi thuộc tính của một thực thể.
    pub fn attrs(&self, id: EntityId) -> Option<&BTreeMap<AttrKey, Value>> {
        let e = *self.index.get(&id)?;
        self.world.get::<Attrs>(e).map(|a| &a.0)
    }

    /// Một thuộc tính.
    pub fn attr(&self, id: EntityId, key: &str) -> Option<&Value> {
        self.attrs(id)?.get(key)
    }

    /// Thuộc tính dưới dạng số nguyên.
    pub fn attr_int(&self, id: EntityId, key: &str) -> Option<i64> {
        match self.attr(id, key) {
            Some(Value::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Thuộc tính dưới dạng **tham chiếu thực thể**.
    ///
    /// Tách khỏi [`Store::attr_int`] vì hai thứ này có kiểu khác nhau trong
    /// [`Value`], và trộn chúng là một lỗi im lặng: `attr_int` trên một
    /// `Value::Uint` trả `None`, nên một điều kiện tiên quyết sẽ **luôn thất
    /// bại** thay vì báo sai kiểu. Nó biểu hiện thành "không nhặt được đồ" mà
    /// không có thông báo nào giải thích.
    pub fn attr_entity(&self, id: EntityId, key: &str) -> Option<EntityId> {
        match self.attr(id, key) {
            Some(Value::Uint(v)) => Some(EntityId(*v)),
            _ => None,
        }
    }

    /// Thuộc tính dưới dạng chuỗi.
    pub fn attr_text(&self, id: EntityId, key: &str) -> Option<&str> {
        match self.attr(id, key) {
            Some(Value::Text(v)) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Mọi thực thể **có** thuộc tính này, theo thứ tự định danh.
    ///
    /// Đây là truy vấn hay dùng nhất trên đường commit, nên nó trả về iterator
    /// đã sắp xếp sẵn thay vì bắt mỗi chỗ gọi tự nhớ sắp xếp.
    pub fn with_attr<'a>(&'a self, key: &'a str) -> impl Iterator<Item = EntityId> + 'a {
        self.index
            .iter()
            .filter(move |(_, e)| {
                self.world
                    .get::<Attrs>(**e)
                    .is_some_and(|a| a.0.contains_key(key))
            })
            .map(|(id, _)| *id)
    }

    /// Truy cập component có kiểu, cho các crate miền dùng đường nhanh.
    pub fn component<T: Component>(&self, id: EntityId) -> Option<&T> {
        let e = *self.index.get(&id)?;
        self.world.get::<T>(e)
    }

    // ── Ghi: `pub(crate)`, chỉ transaction gọi được ─────────────────────────

    pub(crate) fn spawn(&mut self, id: EntityId) -> bool {
        if self.index.contains_key(&id) {
            return false;
        }
        let e = self.world.spawn((Identity(id), Attrs::default())).id();
        self.index.insert(id, e);
        true
    }

    pub(crate) fn despawn(&mut self, id: EntityId) -> bool {
        match self.index.remove(&id) {
            Some(e) => self.world.despawn(e),
            None => false,
        }
    }

    pub(crate) fn set_attr(&mut self, id: EntityId, key: AttrKey, v: Value) -> bool {
        let Some(&e) = self.index.get(&id) else {
            return false;
        };
        if let Some(mut a) = self.world.get_mut::<Attrs>(e) {
            a.0.insert(key, v);
            true
        } else {
            false
        }
    }

    pub(crate) fn remove_attr(&mut self, id: EntityId, key: &str) -> bool {
        let Some(&e) = self.index.get(&id) else {
            return false;
        };
        self.world
            .get_mut::<Attrs>(e)
            .is_some_and(|mut a| a.0.remove(key).is_some())
    }

    /// Đường nhanh cho dữ liệu nóng của engine: component có kiểu của bevy
    /// thay vì túi thuộc tính động. Chưa có crate miền nào dùng tới, nhưng
    /// đường này phải tồn tại từ đầu — thêm nó sau, khi đã có hàng chục nghìn
    /// thực thể chạy qua túi động, sẽ là một cuộc di trú chứ không phải một
    /// lựa chọn.
    #[allow(dead_code)]
    pub(crate) fn insert_component<T: Component>(&mut self, id: EntityId, c: T) -> bool {
        let Some(&e) = self.index.get(&id) else {
            return false;
        };
        self.world.entity_mut(e).insert(c);
        true
    }
}

impl CanonicalHash for Store {
    fn canonical_hash(&self, h: &mut StateHasher) {
        // Duyệt theo `BTreeMap`, tức theo `EntityId` tăng dần. Đây là chỗ mà
        // "cấm iterate `HashMap` trên đường commit" (`§P10.3`) thật sự được
        // trả giá: nếu chỉ mục là `HashMap`, hash sẽ khác nhau giữa hai lần
        // chạy của **cùng một** thế giới.
        h.write_seq(self.index.iter(), |hh, (id, e)| {
            id.canonical_hash(hh);
            let attrs = self.world.get::<Attrs>(*e);
            hh.write_option(attrs, |hhh, a| {
                hhh.write_seq(a.0.iter(), |h4, (k, v)| {
                    h4.write_str(k);
                    v.canonical_hash(h4);
                });
            });
        });
    }
}
