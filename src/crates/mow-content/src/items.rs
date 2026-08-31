//! Vật phẩm (`idea.md §8.5`).
//!
//! ## Vì sao [`ItemDef`] tối giản đến mức này
//!
//! `§8.5` nói rõ: **vật phẩm là entity, không phải một bảng riêng.** Khối lượng,
//! thể tích, chất lượng, hao mòn và nguồn gốc là *component* trên entity đó, và
//! `mow-items` sở hữu chúng.
//!
//! Nên file `metadata.yaml` ở đây cố ý **không** chứa những con số ấy. Nếu nó
//! chứa, sẽ có hai nơi trả lời câu hỏi "một ổ bánh nặng bao nhiêu", và hai nơi
//! đó sẽ lệch. Cái nó khai là phần không ai khác khai được: định danh ổn định,
//! tên hiển thị, và các nhãn mà hệ khác tra cứu theo.

use crate::error::ContentError;
use crate::loader::{load_directory, parse_simple, DefRegistry, Definition};
use crate::text::LocalizedText;
use serde::Serialize;
use std::path::Path;

/// Phiên bản schema mà bộ nạp này hiểu.
pub const ITEM_SCHEMA: &str = "item_def/v1";

/// Sổ vật phẩm, tra theo id và lặp theo id tăng dần.
pub type ItemRegistry = DefRegistry<ItemDef>;

/// Định nghĩa một loại vật phẩm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemDef {
    /// Định danh ổn định, trùng tên thư mục chứa nó.
    pub id: String,
    /// Tên hiển thị theo ngôn ngữ.
    pub name: LocalizedText,
    /// Nhãn phân loại, đã sắp xếp và khử trùng lặp.
    pub tags: Vec<String>,
    /// Đường dẫn tương đối tới script hành vi, nếu có.
    pub script: Option<String>,
}

impl Definition for ItemDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl ItemDef {
    /// Vật phẩm này có tag đó không.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Đọc một `metadata.yaml` đã có sẵn trong bộ nhớ.
    ///
    /// `path` chỉ dùng để dựng thông báo lỗi; nó không cần tồn tại thật.
    pub fn from_metadata(
        path: &Path,
        directory_name: &str,
        text: &str,
    ) -> Result<ItemDef, ContentError> {
        let p = parse_simple(path, directory_name, text, ITEM_SCHEMA)?;
        Ok(ItemDef {
            id: p.id,
            name: p.name,
            tags: p.tags,
            script: p.script,
        })
    }
}

/// Nạp mọi vật phẩm từ một thư mục `items/`.
pub fn load_items(dir: impl AsRef<Path>) -> Result<ItemRegistry, ContentError> {
    let map = load_directory(dir.as_ref(), ItemDef::from_metadata)?;
    Ok(DefRegistry::from_map(map))
}

#[cfg(test)]
mod tests {
    use super::{load_items, ItemDef};
    use crate::error::ContentError;
    use std::path::{Path, PathBuf};

    fn items_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core/items")
    }

    #[test]
    fn nap_vat_pham_tu_content_core() {
        let r = load_items(items_dir()).expect("content/core/items phải nạp được");
        let ids: Vec<&str> = r.ids().collect();
        assert_eq!(ids, ["bread", "iron_ingot", "water_flask"]);
        assert!(r.get("bread").expect("có").has_tag("food"));
    }

    #[test]
    fn ten_hien_thi_co_ca_hai_ngon_ngu() {
        let r = load_items(items_dir()).expect("nạp được");
        let bread = r.get("bread").expect("có");
        assert_eq!(bread.name.get("en"), "Bread");
        assert_eq!(bread.name.get("vi"), "Bánh mì");
    }

    #[test]
    fn id_lech_ten_thu_muc_la_loi() {
        let path = PathBuf::from("content/core/items/loaf/metadata.yaml");
        let text = "id: bread\nname: { en: \"Bread\" }\ntags: [food]\n";
        let e = ItemDef::from_metadata(&path, "loaf", text).expect_err("phải lỗi");
        assert!(matches!(e, ContentError::IdMismatch { .. }), "{e}");
    }

    #[test]
    fn ten_rong_la_loi() {
        let path = PathBuf::from("content/core/items/bread/metadata.yaml");
        let text = "id: bread\nname: { en: \"\" }\n";
        let e = ItemDef::from_metadata(&path, "bread", text).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(s.contains("name.en"), "{s}");
    }
}
