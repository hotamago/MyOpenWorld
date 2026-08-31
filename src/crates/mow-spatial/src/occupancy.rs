//! Occupancy: o nao dang bi thuc the nao chiem.
//!
//! Cau truc nay ton tai de tra loi mot cau hoi rat hay duoc hoi tren duong
//! commit — "co ai o o nay khong" — ma khong phai duyet toan bo thuc the.
//!
//! Chi muc ngoc: `BTreeMap` ca hai chieu. `HashMap` se nhanh hon, nhung
//! `§P10.3` cam duyet `HashMap` tren duong commit, va occupancy **duoc duyet**
//! — vd khi giai quyet dong thoi hai nguoi cung buoc vao mot o.

use mow_core::EntityId;
use mow_math::{CanonicalHash, StateHasher, WorldPos};
use std::collections::{BTreeMap, BTreeSet};

/// Ai dang o dau.
#[derive(Debug, Default, Clone)]
pub struct Occupancy {
    tai_o: BTreeMap<WorldPos, BTreeSet<EntityId>>,
    cua_ai: BTreeMap<EntityId, WorldPos>,
}

impl Occupancy {
    /// Rong.
    pub fn new() -> Occupancy {
        Occupancy::default()
    }

    /// Dat mot thuc the vao mot o.
    ///
    /// Tu dong go no khoi o cu. Khong lam viec do thi mot thuc the se xuat hien
    /// o hai cho cung luc, va moi truy van tam nhin se thay no hai lan.
    pub fn place(&mut self, id: EntityId, at: WorldPos) {
        if let Some(cu) = self.cua_ai.insert(id, at) {
            if cu != at {
                self.go_khoi(id, cu);
            }
        }
        self.tai_o.entry(at).or_default().insert(id);
    }

    /// Go mot thuc the khoi luoi.
    pub fn remove(&mut self, id: EntityId) {
        if let Some(cu) = self.cua_ai.remove(&id) {
            self.go_khoi(id, cu);
        }
    }

    fn go_khoi(&mut self, id: EntityId, at: WorldPos) {
        if let Some(s) = self.tai_o.get_mut(&at) {
            s.remove(&id);
            // Don o rong: neu khong, ban do se day nhung tap rong o moi noi
            // tung co ai di qua, va no lon len mai mai.
            if s.is_empty() {
                self.tai_o.remove(&at);
            }
        }
    }

    /// Ai dang o mot o, theo thu tu dinh danh.
    pub fn at(&self, pos: WorldPos) -> impl Iterator<Item = EntityId> + '_ {
        self.tai_o.get(&pos).into_iter().flatten().copied()
    }

    /// Vi tri cua mot thuc the.
    pub fn position_of(&self, id: EntityId) -> Option<WorldPos> {
        self.cua_ai.get(&id).copied()
    }

    /// O co ai khong.
    pub fn is_occupied(&self, pos: WorldPos) -> bool {
        self.tai_o.contains_key(&pos)
    }

    /// Moi thuc the trong hinh vuong ban kinh `r` quanh `center`.
    ///
    /// Duyet theo thu tu `WorldPos` tang dan, nen ket qua on dinh — dieu kien
    /// de mot truy van tam nhin cho cung ket qua o hai lan chay.
    pub fn in_range(&self, center: WorldPos, r: i64) -> Vec<EntityId> {
        let lo = WorldPos::new(
            center.x.saturating_sub(r),
            center.y.saturating_sub(r),
            center.z,
        );
        let hi = WorldPos::new(
            center.x.saturating_add(r),
            center.y.saturating_add(r),
            center.z,
        );
        self.tai_o
            .range(lo..=hi)
            .filter(|(p, _)| {
                p.z == center.z && (p.x - center.x).abs() <= r && (p.y - center.y).abs() <= r
            })
            .flat_map(|(_, s)| s.iter().copied())
            .collect()
    }

    /// So thuc the dang duoc theo doi.
    pub fn len(&self) -> usize {
        self.cua_ai.len()
    }

    /// Rong hay khong.
    pub fn is_empty(&self) -> bool {
        self.cua_ai.is_empty()
    }
}

impl CanonicalHash for Occupancy {
    fn canonical_hash(&self, h: &mut StateHasher) {
        // Chi hash `cua_ai`: `tai_o` la chi muc nguoc, suy ra duoc hoan toan.
        // Hash ca hai la thua, va thua o day co gia — no lam moi lan tinh hash
        // duyet gap doi so ban ghi.
        h.write_seq(self.cua_ai.iter(), |hh, (id, p)| {
            id.canonical_hash(hh);
            p.canonical_hash(hh);
        });
    }
}
