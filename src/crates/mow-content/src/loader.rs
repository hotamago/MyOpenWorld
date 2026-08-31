//! Duyệt thư mục, và sổ tra cứu có thứ tự xác định.
//!
//! ## Vì sao thứ tự thư mục của hệ điều hành không được lọt vào kết quả
//!
//! `read_dir` trả về thứ tự khác nhau trên NTFS, ext4 và APFS. Nếu thứ tự đó đi
//! tiếp vào kết quả thì cùng một pack cho hai kết quả trên hai máy — và vì nội
//! dung đi vào content hash của save (`§22.30`), save sẽ không chuyển máy được.
//!
//! Nên ở đây có hai lớp chặn, và cả hai đều cần:
//!
//! - Kết quả nằm trong `BTreeMap`, tức lặp theo id tăng dần.
//! - Danh sách thư mục được **sắp trước khi đọc**, nên khi có nhiều file hỏng
//!   thì file được báo lỗi trước cũng là một file cố định. Không có vế này, hai
//!   máy sẽ báo hai lỗi khác nhau cho cùng một pack, và người sửa sẽ tưởng mình
//!   đã sửa xong.

use crate::error::ContentError;
use crate::text::{validate_identifier, LocalizedText};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Tên file bắt buộc trong mỗi thư mục thực thể.
pub const METADATA_FILE: &str = "metadata.yaml";

/// Thứ mà một [`DefRegistry`] chứa được.
pub trait Definition {
    /// Id ổn định, cũng là tên thư mục chứa định nghĩa.
    fn id(&self) -> &str;
}

/// Sổ tra cứu theo id.
///
/// Lặp luôn theo **id tăng dần**, không theo thứ tự trên đĩa. Đây là kiểu chung
/// của [`crate::BlockRegistry`], [`crate::ItemRegistry`] và
/// [`crate::EventRegistry`]: ba loại nội dung khác nhau về trường nhưng giống
/// hệt nhau về cách tra cứu, và ba bản sao của cùng một `BTreeMap` là ba chỗ để
/// một lỗi nấp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DefRegistry<T> {
    by_id: BTreeMap<String, T>,
}

impl<T> Default for DefRegistry<T> {
    fn default() -> DefRegistry<T> {
        DefRegistry {
            by_id: BTreeMap::new(),
        }
    }
}

impl<T: Definition> DefRegistry<T> {
    /// Sổ rỗng.
    pub fn new() -> DefRegistry<T> {
        DefRegistry::default()
    }

    /// Tra một định nghĩa theo id.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.by_id.get(id)
    }

    /// Có id này không.
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// Lặp mọi định nghĩa theo **id tăng dần**.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.by_id.values()
    }

    /// Lặp mọi id theo thứ tự tăng dần.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.by_id.keys().map(String::as_str)
    }

    /// Số định nghĩa.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub(crate) fn from_map(by_id: BTreeMap<String, T>) -> DefRegistry<T> {
        DefRegistry { by_id }
    }
}

impl<'a, T: Definition> IntoIterator for &'a DefRegistry<T> {
    type Item = &'a T;
    type IntoIter = std::collections::btree_map::Values<'a, String, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.by_id.values()
    }
}

/// Nạp mọi thư mục con `<id>/metadata.yaml` của một thư mục.
///
/// `parse` nhận `(đường dẫn file, tên thư mục, nội dung)`. Tên thư mục được
/// truyền vào chứ không suy lại từ đường dẫn: nó là thứ `id` phải khớp, và người
/// kiểm phải nhìn thấy nó tường minh.
///
/// File lẻ nằm ngay trong `dir` (`README.md`, `.gitkeep`) bị bỏ qua. Chỉ thư mục
/// con mới là định nghĩa — đó là toàn bộ quy ước của bố cục này, và nó cần đúng
/// một câu để giải thích.
pub(crate) fn load_directory<T, F>(dir: &Path, parse: F) -> Result<BTreeMap<String, T>, ContentError>
where
    F: Fn(&Path, &str, &str) -> Result<T, ContentError>,
{
    if dir.exists() && !dir.is_dir() {
        return Err(ContentError::NotADirectory {
            path: dir.to_path_buf(),
        });
    }

    let entries = std::fs::read_dir(dir).map_err(|e| ContentError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;

    let mut names: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| ContentError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        // `.git`, `.DS_Store` và bạn bè không phải nội dung.
        if name.starts_with('.') {
            continue;
        }
        names.push(name);
    }
    names.sort_unstable();

    let mut out: BTreeMap<String, T> = BTreeMap::new();
    for name in names {
        let entity_dir: PathBuf = dir.join(&name);
        let file = entity_dir.join(METADATA_FILE);
        if !file.is_file() {
            return Err(ContentError::MissingMetadata { dir: entity_dir });
        }
        let text = std::fs::read_to_string(&file).map_err(|e| ContentError::Io {
            path: file.clone(),
            source: e,
        })?;
        out.insert(name.clone(), parse(&file, &name, &text)?);
    }
    Ok(out)
}

/// Phần chung của một định nghĩa tối giản: định danh, tên, tag, script.
///
/// [`crate::ItemDef`] và [`crate::EventDef`] hiện có đúng bằng đây trường. Chúng
/// vẫn là hai kiểu riêng vì chúng sẽ rẽ nhánh — vật phẩm sẽ có công thức chế
/// tạo, sự kiện sẽ có điều kiện kích hoạt — nhưng phần **kiểm tra** thì không
/// nên chép hai lần, vì hai bản chép sẽ lệch nhau ở lần sửa thứ ba.
pub(crate) struct SimpleParts {
    pub(crate) id: String,
    pub(crate) name: LocalizedText,
    pub(crate) tags: Vec<String>,
    pub(crate) script: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSimple {
    #[serde(default)]
    schema: Option<String>,
    id: String,
    name: LocalizedText,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    script: Option<String>,
}

/// Đọc và kiểm phần chung.
pub(crate) fn parse_simple(
    path: &Path,
    directory_name: &str,
    text: &str,
    expected_schema: &'static str,
) -> Result<SimpleParts, ContentError> {
    let raw: RawSimple = serde_yaml::from_str(text).map_err(|e| ContentError::Parse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    check_schema(path, raw.schema.as_deref(), expected_schema)?;
    check_id(path, &raw.id, directory_name)?;
    raw.name.validate(path, "name")?;
    let tags = normalize_tags(path, raw.tags)?;

    Ok(SimpleParts {
        id: raw.id,
        name: raw.name,
        tags,
        script: raw.script,
    })
}

/// Kiểm `schema`, cho phép vắng mặt.
///
/// Vắng mặt nghĩa là "phiên bản đầu tiên". Bắt khai sẽ làm mọi file mẫu trong
/// tài liệu dài thêm một dòng mà chưa đổi được gì; nhưng khi có v2 thật thì một
/// file v2 **phải** khai, và dòng dưới đây là chỗ chặn nó.
pub(crate) fn check_schema(
    path: &Path,
    found: Option<&str>,
    expected: &'static str,
) -> Result<(), ContentError> {
    match found {
        None => Ok(()),
        Some(s) if s == expected => Ok(()),
        Some(s) => Err(ContentError::UnknownSchema {
            path: path.to_path_buf(),
            found: s.to_owned(),
            expected,
        }),
    }
}

/// Kiểm `id`: đúng bộ ký tự, và khớp tên thư mục.
pub(crate) fn check_id(
    path: &Path,
    declared: &str,
    directory_name: &str,
) -> Result<(), ContentError> {
    if let Err(reason) = validate_identifier(declared) {
        return Err(ContentError::BadField {
            path: path.to_path_buf(),
            field: "id".to_owned(),
            value: declared.to_owned(),
            reason,
        });
    }
    if declared != directory_name {
        return Err(ContentError::IdMismatch {
            path: path.to_path_buf(),
            declared: declared.to_owned(),
            directory: directory_name.to_owned(),
        });
    }
    Ok(())
}

/// Kiểm và chuẩn hóa danh sách tag.
///
/// Sắp xếp và khử trùng lặp vì tag là một **tập**, không phải một dãy. Hai file
/// viết `[soil, diggable]` và `[diggable, soil]` nói cùng một điều, nên chúng
/// phải cho cùng một kết quả — nếu không, thứ tự gõ phím sẽ đi vào content hash.
pub(crate) fn normalize_tags(
    path: &Path,
    mut tags: Vec<String>,
) -> Result<Vec<String>, ContentError> {
    for t in &tags {
        if let Err(reason) = validate_identifier(t) {
            return Err(ContentError::BadField {
                path: path.to_path_buf(),
                field: "tags".to_owned(),
                value: t.clone(),
                reason,
            });
        }
    }
    tags.sort_unstable();
    tags.dedup();
    Ok(tags)
}
