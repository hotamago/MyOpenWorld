//! Lỗi khi nạp nội dung.
//!
//! ## Vì sao mọi biến thể đều mang đường dẫn
//!
//! Một pack của cộng đồng có thể có hàng trăm thư mục. Thông báo *"thiếu trường
//! `color`"* đúng nhưng vô dụng: người viết pack phải mở từng file để tìm. Nên
//! **mọi** biến thể ở đây mang đường dẫn file, và biến thể nào nói về một trường
//! cụ thể thì mang thêm tên trường.
//!
//! ## Vì sao có [`ContentError::IdMismatch`] riêng
//!
//! Chép một thư mục vật liệu rồi quên sửa `id` bên trong là lỗi hay gặp nhất
//! của bố cục "một thư mục một thực thể". Không kiểm thì nó không nổ ở đây — nó
//! nổ về sau, dưới dạng một vật liệu mất tích và một vật liệu bị ghi đè, cách
//! nguyên nhân rất xa.

use std::path::PathBuf;
use thiserror::Error;

/// Lỗi của bộ nạp nội dung.
#[derive(Debug, Error)]
pub enum ContentError {
    /// Không đọc được file hoặc thư mục.
    #[error("không đọc được `{}`: {source}", .path.display())]
    Io {
        /// Đường dẫn đang đọc.
        path: PathBuf,
        /// Nguyên nhân.
        #[source]
        source: std::io::Error,
    },

    /// Đường dẫn tồn tại nhưng không phải thư mục.
    #[error(
        "`{}` không phải thư mục — mỗi loại nội dung là một thư mục chứa các thư mục con \
         `<id>/metadata.yaml`",
        .path.display()
    )]
    NotADirectory {
        /// Đường dẫn.
        path: PathBuf,
    },

    /// Thư mục thực thể không có `metadata.yaml`.
    #[error(
        "thư mục `{}` không có `metadata.yaml` — một thư mục ở đây nghĩa là một định nghĩa, \
         nên thiếu file này là thiếu chính định nghĩa đó",
        .dir.display()
    )]
    MissingMetadata {
        /// Thư mục thực thể.
        dir: PathBuf,
    },

    /// YAML sai cú pháp, hoặc thiếu một trường bắt buộc.
    #[error("`{}`: không phân tích được YAML: {message}", .path.display())]
    Parse {
        /// File.
        path: PathBuf,
        /// Thông báo của `serde_yaml`, đã kèm tên trường và vị trí dòng.
        message: String,
    },

    /// Một trường có mặt nhưng giá trị không dùng được.
    #[error("`{}`: trường `{field}` = `{value}` không hợp lệ: {reason}", .path.display())]
    BadField {
        /// File.
        path: PathBuf,
        /// Tên trường, viết đúng như trong YAML.
        field: String,
        /// Giá trị đã đọc được.
        value: String,
        /// Vì sao nó sai.
        reason: String,
    },

    /// Số nguyên nằm ngoài khoảng cho phép.
    #[error(
        "`{}`: trường `{field}` = {value} nằm ngoài khoảng {min}..={max}",
        .path.display()
    )]
    OutOfRange {
        /// File.
        path: PathBuf,
        /// Tên trường.
        field: String,
        /// Giá trị đã đọc được.
        value: i64,
        /// Cận dưới, bao gồm.
        min: i64,
        /// Cận trên, bao gồm.
        max: i64,
    },

    /// `id` trong `metadata.yaml` khác tên thư mục chứa nó.
    #[error(
        "`{}`: `id` khai là `{declared}` nhưng thư mục tên `{directory}`. \
         Hai chỗ này phải khớp: tên thư mục là thứ người ta nhìn thấy, `id` là thứ mọi \
         tham chiếu chéo dùng, và để chúng lệch nhau là để một định nghĩa có hai tên",
        .path.display()
    )]
    IdMismatch {
        /// File.
        path: PathBuf,
        /// Id khai trong file.
        declared: String,
        /// Tên thư mục.
        directory: String,
    },

    /// File khai một phiên bản schema mà bộ nạp này không hiểu.
    #[error(
        "`{}`: `schema` là `{found}` nhưng bộ nạp này chỉ hiểu `{expected}`. \
         Từ chối đọc còn hơn đọc một file v2 theo luật của v1",
        .path.display()
    )]
    UnknownSchema {
        /// File.
        path: PathBuf,
        /// Giá trị khai trong file.
        found: String,
        /// Giá trị bộ nạp hiểu.
        expected: &'static str,
    },
}
