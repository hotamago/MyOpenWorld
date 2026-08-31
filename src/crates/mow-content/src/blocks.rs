//! Vật liệu ô lưới (`idea.md §8.2`, `§18.5.1`).
//!
//! ## Vì sao đây là dữ liệu chứ không phải một `enum`
//!
//! Một `enum` mười một nhánh trong Rust cộng một bảng màu trong tầng vẽ nghĩa là
//! **thêm một vật liệu phải sửa hai ngôn ngữ**. Hai chỗ đó không có gì buộc phải
//! khớp nhau, nên chúng sẽ lệch: một vật liệu có trong mô phỏng mà không có màu
//! hiện ra hồng cánh sen, và không ai biết cho tới khi nhìn thấy nó.
//!
//! Ở đây, thêm một vật liệu là thêm một thư mục. Màu, độ cứng và tính chất nằm
//! cùng một chỗ, đi qua cùng một bộ kiểm, và cùng vào content hash của pack.
//!
//! ## Vì sao `hardness` là số nguyên 0..=100
//!
//! Nó là **thang quy ước**, không phải một đại lượng vật lý có đơn vị. Một thang
//! quy ước không cần số thực, và số thực trên đường này sẽ làm hash của pack
//! khác nhau giữa hai nền tảng (`§22.30`).

use crate::error::ContentError;
use crate::loader::{
    check_id, check_schema, load_directory, normalize_tags, DefRegistry, Definition,
};
use crate::text::LocalizedText;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Phiên bản schema mà bộ nạp này hiểu.
pub const BLOCK_SCHEMA: &str = "block_def/v1";

/// Trần của [`BlockDef::hardness`].
pub const MAX_HARDNESS: u8 = 100;

/// Sổ vật liệu, tra theo id và lặp theo id tăng dần.
pub type BlockRegistry = DefRegistry<BlockDef>;

/// Định nghĩa một vật liệu ô lưới.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockDef {
    /// Định danh ổn định, trùng tên thư mục chứa nó.
    pub id: String,

    /// Tên hiển thị theo ngôn ngữ.
    pub name: LocalizedText,

    /// Màu nền của ô, dạng `0x00RRGGBB` (`§18.5.1`).
    ///
    /// Đây là **màu gốc**, chưa nghiêng theo biome và chưa tối theo độ sâu. Tầng
    /// vẽ suy ra phần còn lại; giữ cả một cây biến thể ở đây sẽ biến bảng vật
    /// liệu thành bảng cảnh quan.
    pub color: u32,

    /// Độ cứng theo thang quy ước `0..=100`, quyết định thời gian đào.
    pub hardness: u8,

    /// Có chảy không.
    ///
    /// Tách khỏi [`BlockDef::walkable`] là chủ đích: magma chảy nhưng không đi
    /// qua được, còn không khí đi qua được mà không chảy. Gộp hai trường sẽ làm
    /// một trong hai câu hỏi không hỏi được nữa.
    pub liquid: bool,

    /// Có chắn tầm nhìn và ánh sáng không.
    pub opaque: bool,

    /// Đi xuyên qua được không — câu hỏi của tìm đường.
    pub walkable: bool,

    /// Nhãn phân loại, đã sắp xếp và khử trùng lặp.
    pub tags: Vec<String>,

    /// Đường dẫn tương đối tới script hành vi, nếu vật liệu này có hành vi riêng.
    pub script: Option<String>,
}

impl Definition for BlockDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl BlockDef {
    /// Vật liệu này có tag đó không.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Đọc một `metadata.yaml` đã có sẵn trong bộ nhớ.
    ///
    /// Tách khỏi việc đọc đĩa vì hai lý do. Thứ nhất, `mow-plugin` đã đọc toàn
    /// bộ cây thư mục của pack vào bộ nhớ để tính content hash — bắt đọc lại từ
    /// đĩa là đọc hai lần cùng một file và mở ra khe hở giữa thứ đã băm và thứ
    /// đã nạp. Thứ hai, mọi luật kiểm ở đây thử được mà không cần thư mục tạm.
    ///
    /// `path` chỉ dùng để dựng thông báo lỗi; nó không cần tồn tại thật.
    pub fn from_metadata(
        path: &Path,
        directory_name: &str,
        text: &str,
    ) -> Result<BlockDef, ContentError> {
        let raw: RawBlock = serde_yaml::from_str(text).map_err(|e| ContentError::Parse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

        check_schema(path, raw.schema.as_deref(), BLOCK_SCHEMA)?;
        check_id(path, &raw.id, directory_name)?;
        raw.name.validate(path, "name")?;

        if raw.hardness > MAX_HARDNESS {
            return Err(ContentError::OutOfRange {
                path: path.to_path_buf(),
                field: "hardness".to_owned(),
                value: i64::from(raw.hardness),
                min: 0,
                max: i64::from(MAX_HARDNESS),
            });
        }

        let color =
            crate::color::parse_hex_color(&raw.color).map_err(|reason| ContentError::BadField {
                path: path.to_path_buf(),
                field: "color".to_owned(),
                value: raw.color.clone(),
                reason,
            })?;

        let tags = normalize_tags(path, raw.tags)?;

        Ok(BlockDef {
            id: raw.id,
            name: raw.name,
            color,
            hardness: raw.hardness,
            liquid: raw.liquid,
            opaque: raw.opaque,
            walkable: raw.walkable,
            tags,
            script: raw.script,
        })
    }
}

/// Hình dạng thô của file, trước khi kiểm.
///
/// `deny_unknown_fields` để một trường gõ sai (`colour`) là lỗi chứ không phải
/// một giá trị bị bỏ qua trong im lặng.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBlock {
    #[serde(default)]
    schema: Option<String>,
    id: String,
    name: LocalizedText,
    color: String,
    hardness: u8,
    // Ba trường dưới **không** có `default`. Một vật liệu mới mà quên khai
    // `opaque` sẽ mặc định thành trong suốt, và một tảng đá nhìn xuyên qua được
    // là loại lỗi mất hàng giờ để lần ra. Bắt khai rẻ hơn nhiều.
    liquid: bool,
    opaque: bool,
    walkable: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    script: Option<String>,
}

/// Nạp mọi vật liệu từ một thư mục `blocks/`.
///
/// Mỗi thư mục con là một vật liệu và phải có `metadata.yaml`. Thư mục không tồn
/// tại là **lỗi**, không phải sổ rỗng: một đường dẫn gõ sai mà trả về "không có
/// vật liệu nào" là cách nhanh nhất để mất một buổi chiều.
pub fn load_blocks(dir: impl AsRef<Path>) -> Result<BlockRegistry, ContentError> {
    let map = load_directory(dir.as_ref(), BlockDef::from_metadata)?;
    Ok(DefRegistry::from_map(map))
}

#[cfg(test)]
mod tests {
    use super::{load_blocks, BlockDef};
    use crate::error::ContentError;
    use std::path::{Path, PathBuf};

    /// Mười một vật liệu của `mow-worldgen::strata::Material`, theo id tăng dần.
    const EXPECTED: [&str; 11] = [
        "air",
        "clay",
        "ice",
        "igneous",
        "magma",
        "metamorphic",
        "ore",
        "sand",
        "sedimentary",
        "topsoil",
        "water",
    ];

    fn blocks_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core/blocks")
    }

    fn sample(overrides: &str) -> String {
        format!(
            "id: topsoil\n\
             name: {{ en: \"Topsoil\", vi: \"Đất mặt\" }}\n\
             color: \"#6b5a3e\"\n\
             hardness: 20\n\
             liquid: false\n\
             opaque: true\n\
             walkable: false\n\
             tags: [soil, diggable]\n\
             script: null\n\
             {overrides}"
        )
    }

    fn fake_path() -> PathBuf {
        PathBuf::from("content/core/blocks/topsoil/metadata.yaml")
    }

    #[test]
    fn moi_vat_lieu_cua_worldgen_deu_co_dinh_nghia() {
        // Khẳng định **có mặt**, không khẳng định **đúng bằng**.
        //
        // Bản đầu viết `assert_eq!(r.len(), 11)`. Nó đỏ ngay lần đầu có người
        // thêm một vật liệu — tức là nó biến việc mở rộng bằng dữ liệu, đúng
        // thứ module này tồn tại để cho phép, thành một việc phải sửa test.
        //
        // Bất biến thật nằm ở chiều ngược lại: mọi vật liệu mà `mow-worldgen`
        // sinh ra được đều phải có định nghĩa, nếu không bản đồ hiện màu tím.
        // Pack **được phép** có thêm bao nhiêu tùy ý.
        let r = load_blocks(blocks_dir()).expect("content/core/blocks phải nạp được");
        for id in EXPECTED {
            assert!(
                r.contains(id),
                "thiếu vật liệu `{id}` mà worldgen sinh ra được"
            );
        }
        assert!(r.len() >= EXPECTED.len());
    }

    #[test]
    fn thu_tu_lap_la_thu_tu_id_khong_phai_thu_tu_thu_muc() {
        let r = load_blocks(blocks_dir()).expect("nạp được");
        let ids: Vec<&str> = r.ids().collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "lặp phải theo id tăng dần");
        let qua_iter: Vec<&str> = r.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(qua_iter, ids, "`iter` và `ids` phải cùng một thứ tự");
    }

    #[test]
    fn mau_khop_bang_cua_tang_ve() {
        let r = load_blocks(blocks_dir()).expect("nạp được");
        // Đây là những con số đúng trong `web/src/render/materials.ts`. Khi bảng
        // kia được sinh ra từ dữ liệu này, test này là chốt chặn cuối cùng.
        assert_eq!(r.get("topsoil").expect("có").color, 0x006b_5a3e);
        assert_eq!(r.get("magma").expect("có").color, 0x00d4_562a);
        assert_eq!(r.get("air").expect("có").color, 0x000d_1014);
        assert_eq!(r.get("ice").expect("có").color, 0x00cf_e6f0);
    }

    #[test]
    fn tra_cuu_theo_id() {
        let r = load_blocks(blocks_dir()).expect("nạp được");
        assert_eq!(r.get("water").expect("có").hardness, 0);
        assert!(r.get("khong_ton_tai").is_none());
        assert!(!r.is_empty());
    }

    #[test]
    fn lo_va_di_qua_duoc_la_hai_cau_hoi_khac_nhau() {
        let r = load_blocks(blocks_dir()).expect("nạp được");
        let magma = r.get("magma").expect("có");
        assert!(magma.liquid, "magma chảy");
        assert!(!magma.walkable, "nhưng đường tìm được không được đi qua nó");

        let water = r.get("water").expect("có");
        assert!(water.liquid && water.walkable, "nước thì cả hai");

        let air = r.get("air").expect("có");
        assert!(!air.liquid && air.walkable, "không khí thì ngược lại");
    }

    #[test]
    fn tag_da_duoc_sap_xep() {
        let r = load_blocks(blocks_dir()).expect("nạp được");
        for b in r.iter() {
            let mut sorted = b.tags.clone();
            sorted.sort_unstable();
            assert_eq!(b.tags, sorted, "tag của `{}` chưa sắp xếp", b.id);
        }
        assert!(r.get("topsoil").expect("có").has_tag("diggable"));
    }

    #[test]
    fn thieu_truong_bat_buoc_bao_loi_kem_ten_file() {
        let text = "id: topsoil\n\
                    name: { en: \"Topsoil\" }\n\
                    hardness: 20\n\
                    liquid: false\n\
                    opaque: true\n\
                    walkable: false\n";
        let e = BlockDef::from_metadata(&fake_path(), "topsoil", text).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(matches!(e, ContentError::Parse { .. }), "{s}");
        assert!(
            s.contains("topsoil/metadata.yaml"),
            "lỗi phải nói file: {s}"
        );
        assert!(s.contains("color"), "lỗi phải nói trường: {s}");
    }

    #[test]
    fn hex_sai_bao_loi_kem_ten_truong_va_gia_tri() {
        let text = sample("").replace("#6b5a3e", "6b5a3e");
        let e = BlockDef::from_metadata(&fake_path(), "topsoil", &text).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(matches!(e, ContentError::BadField { .. }), "{s}");
        assert!(s.contains("color") && s.contains("6b5a3e"), "{s}");
        assert!(s.contains("topsoil/metadata.yaml"), "{s}");
    }

    #[test]
    fn hex_do_dai_sai_bao_loi() {
        let text = sample("").replace("#6b5a3e", "#6b5a");
        let e = BlockDef::from_metadata(&fake_path(), "topsoil", &text).expect_err("phải lỗi");
        assert!(matches!(e, ContentError::BadField { .. }), "{e}");
    }

    #[test]
    fn id_lech_ten_thu_muc_la_loi() {
        // Lỗi chép thư mục rồi quên sửa `id`.
        let path = PathBuf::from("content/core/blocks/clay/metadata.yaml");
        let e = BlockDef::from_metadata(&path, "clay", &sample("")).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(matches!(e, ContentError::IdMismatch { .. }), "{s}");
        assert!(s.contains("topsoil") && s.contains("clay"), "{s}");
    }

    #[test]
    fn id_sai_bo_ky_tu_la_loi() {
        let text = sample("").replace("id: topsoil", "id: Topsoil");
        let e = BlockDef::from_metadata(&fake_path(), "Topsoil", &text).expect_err("phải lỗi");
        assert!(matches!(e, ContentError::BadField { .. }), "{e}");
    }

    #[test]
    fn hardness_ngoai_khoang_la_loi() {
        let text = sample("").replace("hardness: 20", "hardness: 150");
        let e = BlockDef::from_metadata(&fake_path(), "topsoil", &text).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(matches!(e, ContentError::OutOfRange { .. }), "{s}");
        assert!(s.contains("hardness") && s.contains("100"), "{s}");
    }

    #[test]
    fn schema_la_v2_thi_tu_choi() {
        let text = sample("schema: block_def/v2\n");
        let e = BlockDef::from_metadata(&fake_path(), "topsoil", &text).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(matches!(e, ContentError::UnknownSchema { .. }), "{s}");
        assert!(s.contains("block_def/v2"), "{s}");
    }

    #[test]
    fn truong_go_sai_la_loi_chu_khong_bi_bo_qua() {
        let text = sample("").replace("color:", "colour:");
        let e = BlockDef::from_metadata(&fake_path(), "topsoil", &text).expect_err("phải lỗi");
        assert!(matches!(e, ContentError::Parse { .. }), "{e}");
    }

    #[test]
    fn tag_duoc_sap_xep_va_khu_trung_lap() {
        let text = sample("").replace("tags: [soil, diggable]", "tags: [soil, diggable, soil]");
        let b = BlockDef::from_metadata(&fake_path(), "topsoil", &text).expect("hợp lệ");
        assert_eq!(b.tags, vec!["diggable".to_owned(), "soil".to_owned()]);
    }

    #[test]
    fn tag_sai_bo_ky_tu_la_loi() {
        let text = sample("").replace("tags: [soil, diggable]", "tags: [\"Soil\"]");
        let e = BlockDef::from_metadata(&fake_path(), "topsoil", &text).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(s.contains("tags"), "{s}");
    }

    #[test]
    fn thu_muc_khong_ton_tai_la_loi_chu_khong_phai_so_rong() {
        let e = load_blocks(blocks_dir().join("khong_co_that")).expect_err("phải lỗi");
        assert!(matches!(e, ContentError::Io { .. }), "{e}");
        assert!(e.to_string().contains("khong_co_that"), "{e}");
    }

    #[test]
    fn thu_muc_con_thieu_metadata_bao_loi_kem_ten_thu_muc() {
        // `content/core` có nhiều thư mục con không phải thư mục thực thể, nên
        // trỏ bộ nạp vào đó phải nổ ngay ở cái đầu tiên theo thứ tự id.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core");
        let e = load_blocks(dir).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(matches!(e, ContentError::MissingMetadata { .. }), "{s}");
        assert!(s.contains("metadata.yaml"), "{s}");
    }
}
