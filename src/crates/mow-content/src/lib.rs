//! # `mow-content` — vật liệu, vật phẩm và sự kiện nằm trong dữ liệu
//!
//! `idea.md §8.2` mô tả vật liệu bằng **thuộc tính**, `§18.5.1` nói diện mạo của
//! một ô **suy ra từ chính những thuộc tính đó**, và `§19.7` nói content pack chỉ
//! được **thêm định nghĩa vào registry**. Ba mục đó chỉ đúng nếu định nghĩa là
//! dữ liệu. Một `enum` mười một nhánh trong Rust cộng một bảng màu chép tay
//! trong tầng vẽ vi phạm cả ba: thêm một vật liệu phải sửa hai ngôn ngữ, và
//! không có gì buộc hai chỗ đó phải khớp nhau.
//!
//! Crate này là bộ nạp. **Thêm một vật liệu là thêm một thư mục.**
//!
//! ## Bố cục
//!
//! ```text
//! content/core/
//!   blocks/<id>/metadata.yaml
//!   items/<id>/metadata.yaml
//!   events/<id>/metadata.yaml
//! ```
//!
//! Một thư mục một thực thể, vì một thực thể rồi sẽ có nhiều hơn một file:
//! sprite ghi đè, script hành vi, bảng bản địa hóa. Đặt sẵn chỗ cho chúng ngay
//! từ đầu rẻ hơn việc chuyển từ `blocks/topsoil.yaml` sang thư mục về sau, lúc
//! đã có pack của người khác trỏ vào đường dẫn cũ.
//!
//! ## Vì sao nằm trong `content/core/` chứ không phải một thư mục `assets/` mới
//!
//! Dự án **đã có** cơ chế content pack: `mow_plugin::Registry::add_from_dir`,
//! `pack.yaml`, capability (`§19.7`), content hash (`§22.30`) và
//! `mow-cli pack validate`. `plan.md §P10.7` nói thẳng rằng `content/core/` phải
//! đi qua đúng cơ chế mà cộng đồng dùng — không có đường đặc quyền cho nội dung
//! chính thức.
//!
//! Dựng một cây `assets/` song song nghĩa là hai nguồn sự thật, hai bộ kiểm, và
//! nội dung chính thức không còn kiểm thử được đường mà mod đi. Nên vật liệu nằm
//! **trong** pack `core`, và crate này chỉ đọc phần bên trong pack đó.
//!
//! ## Ba bất biến crate này giữ
//!
//! - **Thứ tự xác định.** Lặp theo id tăng dần, và thư mục được sắp trước khi
//!   đọc. Thứ tự `read_dir` của hệ điều hành không lọt vào kết quả, cũng không
//!   lọt vào thứ tự báo lỗi.
//! - **`id` khớp tên thư mục.** Chép một thư mục rồi quên sửa `id` là lỗi hay
//!   gặp nhất của bố cục này, và nó là **lỗi lúc nạp**, không phải một vật liệu
//!   mất tích phát hiện sau ba tuần.
//! - **Không có số thực.** Màu là `u32`, độ cứng là `u8`. Nội dung đi vào content
//!   hash của save; số thực làm tròn khác nhau giữa các nền tảng và sẽ làm cùng
//!   một pack cho hai hash trên hai máy.
//!
//! ## Dùng
//!
//! ```no_run
//! # fn main() -> Result<(), mow_content::ContentError> {
//! // Cả pack một lượt — đường thường dùng.
//! let content = mow_content::load_pack("content/core")?;
//! let topsoil = content.blocks.get("topsoil").expect("có trong core");
//! assert_eq!(topsoil.color, 0x006b_5a3e);
//!
//! // Hoặc từng loại một.
//! let blocks = mow_content::load_blocks("content/core/blocks")?;
//! for b in blocks.iter() {
//!     println!("{} {}", b.id, mow_content::format_hex_color(b.color));
//! }
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]

pub mod blocks;
pub mod color;
pub mod error;
pub mod events;
pub mod items;
pub mod loader;
pub mod text;

pub use blocks::{load_blocks, BlockDef, BlockRegistry, BLOCK_SCHEMA, MAX_HARDNESS};
pub use color::{format_hex_color, parse_hex_color};
pub use error::ContentError;
pub use events::{load_events, EventDef, EventRegistry, EVENT_SCHEMA};
pub use items::{load_items, ItemDef, ItemRegistry, ITEM_SCHEMA};
pub use loader::{DefRegistry, Definition, METADATA_FILE};
pub use text::LocalizedText;

use std::path::Path;

/// Thư mục con của pack, theo loại nội dung.
pub const BLOCKS_DIR: &str = "blocks";
/// Thư mục con chứa vật phẩm.
pub const ITEMS_DIR: &str = "items";
/// Thư mục con chứa sự kiện.
pub const EVENTS_DIR: &str = "events";

/// Toàn bộ định nghĩa mà crate này đọc được từ một content pack.
#[derive(Debug, Clone, Default)]
pub struct PackContent {
    /// Vật liệu ô lưới.
    pub blocks: BlockRegistry,
    /// Loại vật phẩm.
    pub items: ItemRegistry,
    /// Loại sự kiện.
    pub events: EventRegistry,
}

/// Nạp cả ba loại nội dung từ thư mục gốc của một pack.
///
/// Thư mục con **vắng mặt** cho ra sổ rỗng, không phải lỗi: phần lớn pack của
/// cộng đồng chỉ thêm một loại, và bắt mọi pack tạo ba thư mục trống chỉ để nạp
/// được là bắt người ta tạo rác.
///
/// Ngược lại, [`load_blocks`] gọi trực tiếp trên một đường dẫn không tồn tại
/// **là lỗi** — ở đó người gọi đã nói rõ mình muốn thư mục nào, nên một sổ rỗng
/// im lặng chỉ là một lỗi gõ phím không ai thấy.
pub fn load_pack(dir: impl AsRef<Path>) -> Result<PackContent, ContentError> {
    let dir = dir.as_ref();
    let blocks = dir.join(BLOCKS_DIR);
    let items = dir.join(ITEMS_DIR);
    let events = dir.join(EVENTS_DIR);

    Ok(PackContent {
        blocks: if blocks.is_dir() {
            load_blocks(&blocks)?
        } else {
            BlockRegistry::new()
        },
        items: if items.is_dir() {
            load_items(&items)?
        } else {
            ItemRegistry::new()
        },
        events: if events.is_dir() {
            load_events(&events)?
        } else {
            EventRegistry::new()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{load_pack, PackContent};
    use std::path::{Path, PathBuf};

    fn core_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core")
    }

    #[test]
    fn nap_ca_pack_core() {
        // Ngưỡng dưới, không phải con số chính xác: một pack lớn lên theo thời
        // gian, và một test đếm chính xác sẽ đỏ mỗi lần có người thêm nội dung.
        let c: PackContent = load_pack(core_dir()).expect("content/core phải nạp được");
        assert!(c.blocks.len() >= 11, "chỉ có {} vật liệu", c.blocks.len());
        assert!(!c.items.is_empty());
        assert!(!c.events.is_empty());
    }

    #[test]
    fn pack_khong_co_ba_thu_muc_do_thi_ra_so_rong() {
        // `palettes/` là một thư mục thật của pack `core` nhưng không chứa
        // `blocks/`, `items/` hay `events/`.
        let c = load_pack(core_dir().join("palettes")).expect("không được là lỗi");
        assert!(c.blocks.is_empty() && c.items.is_empty() && c.events.is_empty());
    }

    #[test]
    fn nap_hai_lan_ra_ket_qua_giong_het() {
        // Vế xác định, kiểm bằng cách so hai lần nạp: nếu thứ tự `read_dir` lọt
        // được vào kết quả thì hai lần chạy trên cùng một máy vẫn có thể khác.
        let a = load_pack(core_dir()).expect("nạp được");
        let b = load_pack(core_dir()).expect("nạp được");
        assert_eq!(a.blocks, b.blocks);
        assert_eq!(a.items, b.items);
        assert_eq!(a.events, b.events);
    }

    #[test]
    fn dinh_nghia_serialize_duoc_de_tang_ve_sinh_bang_mau_tu_du_lieu() {
        // Lý do `Serialize` có mặt: bảng màu của `web/` phải sinh ra được từ đây
        // thay vì được chép tay lần thứ hai.
        let c = load_pack(core_dir()).expect("nạp được");
        let json = serde_json::to_string(&c.blocks).expect("serialize được");
        assert!(json.contains("\"topsoil\""), "{json}");
        assert!(json.contains(&0x006b_5a3e.to_string()), "màu là số nguyên");
    }
}
