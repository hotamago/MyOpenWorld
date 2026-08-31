//! Vat pham nam o **dung mot** noi (`idea.md §22.33`).
//!
//! > Mot vat pham nam o dung mot trong ba noi — cell, container hoac inventory.
//! > Chuyen cho la transaction, khong nhan doi va khong boc hoi.
//!
//! Bat bien nay duoc thi hanh bang **kieu**, khong bang kiem tra: [`ItemLocation`]
//! la mot enum, nen mot vat pham khong the nam o hai noi cung luc theo dung
//! nghia den. Cach vi pham duy nhat con lai la quen go khoi cho cu khi them vao
//! cho moi — va [`ItemLocation::move_to`] khong cho phep dieu do, vi no tra ve
//! mot gia tri moi thay vi sua tai cho.

use mow_math::{CanonicalHash, StateHasher, WorldPos};
use serde::{Deserialize, Serialize};

/// Noi mot vat pham dang o.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "where", rename_all = "snake_case")]
pub enum ItemLocation {
    /// Nam tren mat dat tai mot o.
    Cell {
        /// Vi tri.
        at: WorldPos,
    },
    /// Trong mot vat chua.
    Container {
        /// Entity cua vat chua.
        entity: u64,
    },
    /// Trong tui do cua mot thuc the.
    Inventory {
        /// Entity chu so huu.
        owner: u64,
    },
}

/// Loi khi chuyen cho.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocationError {
    /// Chuyen vao chinh no.
    #[error("vat chua khong the chua chinh no")]
    SelfContainment,
    /// Chuyen toi noi hien tai.
    #[error("vat pham da o do roi")]
    AlreadyThere,
}

impl ItemLocation {
    /// Chuyen toi mot noi khac.
    ///
    /// Tra **gia tri moi**, khong sua tai cho. Nho vay chuyen cho luon la mot
    /// phep thay the nguyen tu: khong co khoanh khac nao vat pham o ca hai noi,
    /// va cung khong co khoanh khac nao no o khong noi nao.
    pub fn move_to(
        self,
        dest: ItemLocation,
        self_entity: u64,
    ) -> Result<ItemLocation, LocationError> {
        if let ItemLocation::Container { entity } = dest {
            if entity == self_entity {
                return Err(LocationError::SelfContainment);
            }
        }
        if self == dest {
            return Err(LocationError::AlreadyThere);
        }
        Ok(dest)
    }

    /// Entity dang giu, neu co.
    pub fn holder(self) -> Option<u64> {
        match self {
            ItemLocation::Cell { .. } => None,
            ItemLocation::Container { entity } => Some(entity),
            ItemLocation::Inventory { owner } => Some(owner),
        }
    }

    /// Ten on dinh cua noi, dung trong log va trong bat bien.
    pub fn kind(self) -> &'static str {
        match self {
            ItemLocation::Cell { .. } => "cell",
            ItemLocation::Container { .. } => "container",
            ItemLocation::Inventory { .. } => "inventory",
        }
    }
}

impl CanonicalHash for ItemLocation {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.kind());
        match self {
            ItemLocation::Cell { at } => at.canonical_hash(h),
            ItemLocation::Container { entity } => {
                h.write_u64(*entity);
            }
            ItemLocation::Inventory { owner } => {
                h.write_u64(*owner);
            }
        }
    }
}
