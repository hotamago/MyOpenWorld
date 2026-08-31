//! # `mow-items` — vat pham
//!
//! Ba bat bien dinh hinh crate nay:
//!
//! - `§22.32` — vat pham o muc **instance** la mot ECS entity; o muc
//!   **stack/aggregate** la component du lieu tren entity vat chua. **Khong co
//!   he vat pham thu hai nam ngoai ECS.**
//! - `§22.33` — mot vat pham nam o **dung mot** trong ba noi: o dat, vat chua,
//!   hoac tui do. Chuyen cho la transaction, khong nhan doi va khong boc hoi.
//! - `§22.34` — `CraftQuality` **bat bien** sau khi che tac; sua chua chi phuc
//!   hoi `Condition`.
//!
//! Bat bien thu ba la thu de bi vi pham nhat, va [`item::ItemInstance`] thi
//! hanh no bang chu ky: khong co ham nao sua duoc `quality` sau khi dung.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

pub mod equipment;
pub mod inventory;
pub mod item;
pub mod legend;
pub mod location;

pub use equipment::{EquipError, Equipment, Equipped};
pub use inventory::{Capacity, Inventory, InventoryError, Load};
pub use item::{CraftQuality, ItemDef, ItemInstance, ItemStack, PartCondition, RepairRecord};
pub use legend::{
    Claim, Deed, Discrepancy, Fate, Legend, Path, Provenance, SapientItem, SocialPower,
    NGUONG_TAY_NGHE, SO_VIEC_DANG_KE_THANH_HUYEN_THOAI,
};
pub use location::{ItemLocation, LocationError};
