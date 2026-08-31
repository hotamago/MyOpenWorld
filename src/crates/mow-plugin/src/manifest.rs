//! Manifest của một content pack (`pack.yaml`).

use crate::capability::Capability;
use mow_math::{CanonicalHash, StateHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    /// Định danh, cũng là **namespace** của mọi id trong pack.
    ///
    /// Chữ thường, gạch dưới, không dấu chấm. Dấu chấm là dấu phân cấp của id
    /// (`core.apple`), nên cho phép nó trong namespace sẽ làm `a.b` và `a` với
    /// id `b` không phân biệt được.
    pub id: String,

    /// Phiên bản, `major.minor.patch`.
    pub version: String,

    /// Tên hiển thị.
    #[serde(default)]
    pub name: String,

    /// Mô tả một dòng.
    #[serde(default)]
    pub description: String,

    /// Pack này cần pack nào nạp trước.
    #[serde(default)]
    pub requires: Vec<PackRef>,

    /// Những id của pack khác mà pack này **cố ý** ghi đè.
    ///
    /// Phải khai báo tường minh (`§22.29`). Ghi đè không khai báo là xung đột,
    /// và xung đột là lỗi — vì "ai load sau thì thắng" biến thứ tự nạp thành
    /// một phần của luật chơi, một phần vô hình và không ai gỡ được.
    #[serde(default)]
    pub overrides: Vec<String>,

    /// Kịch bản test mà pack này khai báo, chạy bởi `mow-cli pack test`.
    #[serde(default)]
    pub tests: Vec<String>,

    /// Quyền pack xin (`§19.7`, `PF-01`).
    ///
    /// Mặc định **rỗng**, nghĩa là chỉ khai được dữ liệu tĩnh. Một pack muốn
    /// khai luật, module, prompt hoặc generator phải nói ra ở đây — và
    /// [`crate::capability::Grants::audit`] kiểm lại bằng nội dung thật trên
    /// đĩa, không tin lời khai.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

/// Tham chiếu tới một pack khác.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackRef {
    /// Định danh pack.
    pub id: String,
    /// Yêu cầu phiên bản, ví dụ `>=1.0`.
    #[serde(default)]
    pub version: String,
}

impl PackManifest {
    /// Kiểm tra hình dạng của manifest.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut loi = Vec::new();

        if self.id.is_empty() {
            loi.push("`id` không được rỗng".to_owned());
        }
        if self.id.contains('.') {
            loi.push(format!(
                "`id` = `{}` chứa dấu chấm. Dấu chấm là dấu phân cấp của id nội dung, \
                 nên namespace không được chứa nó",
                self.id
            ));
        }
        if !self
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            loi.push(format!(
                "`id` = `{}` chỉ được dùng chữ thường ASCII, số, `_` và `-`",
                self.id
            ));
        }
        if self.version.is_empty() {
            loi.push("`version` không được rỗng".to_owned());
        }

        // Ghi đè phải trỏ tới namespace khác. Một pack "ghi đè" chính nó chỉ có
        // nghĩa là nó định nghĩa trùng id hai lần, và đó luôn là lỗi soạn thảo.
        for o in &self.overrides {
            let ns = o.split('.').next().unwrap_or("");
            if ns == self.id {
                loi.push(format!(
                    "`overrides` chứa `{o}` thuộc chính pack này — ghi đè chính mình \
                     nghĩa là định nghĩa trùng id"
                ));
            }
            if !o.contains('.') {
                loi.push(format!(
                    "`overrides` chứa `{o}` không có namespace — mọi id phải có dạng \
                     `<pack>.<tên>` (§22.29)"
                ));
            }
        }

        if loi.is_empty() {
            Ok(())
        } else {
            Err(loi)
        }
    }

    /// Đọc từ YAML.
    pub fn from_yaml(s: &str) -> Result<PackManifest, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }
}

impl CanonicalHash for PackManifest {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_str(&self.version);
        h.write_seq(self.requires.iter(), |hh, r| {
            hh.write_str(&r.id);
            hh.write_str(&r.version);
        });
        h.write_seq(self.overrides.iter(), |hh, o| {
            hh.write_str(o);
        });
    }
}

/// Băm nội dung một pack thành một giá trị ổn định.
///
/// `files` ánh xạ đường dẫn tương đối → nội dung. `BTreeMap` nên thứ tự duyệt
/// là thứ tự đường dẫn, không phải thứ tự hệ thống tệp trả về — thứ khác nhau
/// giữa Windows, ext4 và APFS, và sẽ làm cùng một pack cho ba hash khác nhau
/// trên ba máy.
///
/// Đường dẫn được chuẩn hóa về dấu `/` vì lý do y hệt.
pub fn content_hash(manifest: &PackManifest, files: &BTreeMap<String, Vec<u8>>) -> StateHash {
    let mut h = StateHasher::with_domain("mow.pack.v1");
    manifest.canonical_hash(&mut h);
    h.write_seq(files.iter(), |hh, (path, bytes)| {
        hh.write_str(&path.replace('\\', "/"));
        hh.write_bytes(bytes);
    });
    h.finish()
}
