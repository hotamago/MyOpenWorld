//! Tên hiển thị và quy tắc đặt định danh.

use crate::error::ContentError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Tên hiển thị theo ngôn ngữ.
///
/// ## Vì sao tiếng Anh bắt buộc còn tiếng Việt thì không
///
/// Bản địa hóa là việc của người dịch, không phải điều kiện để nội dung nạp
/// được. Bắt mọi pack phải có đủ mọi ngôn ngữ nghĩa là một pack tiếng Anh không
/// cài được, và kết quả thực tế là người ta điền chuỗi tiếng Anh vào ô tiếng
/// Việt cho qua — tệ hơn hẳn việc thiếu bản dịch một cách trung thực.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizedText {
    /// Tiếng Anh. Bắt buộc, và là chuỗi quay về khi thiếu bản dịch.
    pub en: String,
    /// Tiếng Việt, nếu có.
    #[serde(default)]
    pub vi: Option<String>,
}

impl LocalizedText {
    /// Chuỗi cho một ngôn ngữ, quay về `en` khi chưa có bản dịch.
    ///
    /// Quay về chứ không trả rỗng: một ô không có tên trong giao diện là một lỗi
    /// người dùng không mô tả được, còn một cái tên sai ngôn ngữ thì người ta
    /// vẫn đọc ra và vẫn báo được.
    pub fn get(&self, language: &str) -> &str {
        match language {
            "vi" => self.vi.as_deref().unwrap_or(&self.en),
            _ => &self.en,
        }
    }

    /// Kiểm chuỗi bắt buộc không rỗng.
    pub(crate) fn validate(&self, path: &Path, field: &str) -> Result<(), ContentError> {
        if self.en.trim().is_empty() {
            return Err(ContentError::BadField {
                path: path.to_path_buf(),
                field: format!("{field}.en"),
                value: self.en.clone(),
                reason: "tên tiếng Anh không được rỗng — nó là chuỗi quay về của mọi ngôn ngữ"
                    .to_owned(),
            });
        }
        Ok(())
    }
}

/// Kiểm một định danh: chữ thường ASCII, chữ số và `_`.
///
/// Cùng bộ ký tự với namespace của pack (`§22.29`). Hẹp là chủ đích: định danh
/// đi vào tên thư mục, vào khóa của bảng tra, vào chuỗi truyền đi và vào content
/// hash. Cho phép chữ hoa nghĩa là `Topsoil` và `topsoil` là hai id trên Linux
/// nhưng một id trên Windows, và một pack sẽ chạy ở một nửa số máy.
pub(crate) fn validate_identifier(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("định danh không được rỗng".to_owned());
    }
    if let Some(bad) = text
        .chars()
        .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_'))
    {
        return Err(format!(
            "ký tự `{bad}` không dùng được — chỉ chữ thường ASCII, chữ số và `_`"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_identifier, LocalizedText};

    #[test]
    fn thieu_tieng_viet_thi_quay_ve_tieng_anh() {
        let t = LocalizedText {
            en: "Topsoil".to_owned(),
            vi: None,
        };
        assert_eq!(t.get("vi"), "Topsoil");
        assert_eq!(t.get("en"), "Topsoil");
        // Ngôn ngữ chưa hỗ trợ cũng quay về `en` chứ không rỗng.
        assert_eq!(t.get("ja"), "Topsoil");
    }

    #[test]
    fn co_tieng_viet_thi_dung_tieng_viet() {
        let t = LocalizedText {
            en: "Topsoil".to_owned(),
            vi: Some("Đất mặt".to_owned()),
        };
        assert_eq!(t.get("vi"), "Đất mặt");
        assert_eq!(t.get("en"), "Topsoil");
    }

    #[test]
    fn dinh_danh_chu_hoa_bi_tu_choi() {
        assert!(validate_identifier("topsoil").is_ok());
        assert!(validate_identifier("iron_ingot_2").is_ok());
        let e = validate_identifier("Topsoil").expect_err("phải lỗi");
        assert!(e.contains('T'), "{e}");
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("top-soil").is_err());
    }
}
