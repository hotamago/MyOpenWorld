//! Loi cau hinh.

use thiserror::Error;

/// Loi khi nap hoac kiem tra cau hinh.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Khong tim thay file bat buoc.
    #[error("khong tim thay file cau hinh: {0}")]
    Missing(String),

    /// Loi doc hoac phan tich.
    #[error("khong doc duoc cau hinh: {0}")]
    Read(#[from] figment::Error),

    /// Cau hinh doc duoc nhung vi pham rang buoc.
    ///
    /// Giu **danh sach** chu khong dung lai o loi dau tien: sua tung loi mot
    /// roi khoi dong lai la vong lap cham va buc boi. Bao het mot lan.
    #[error("{}", .0.iter().map(|(k, v)| format!("  {k}: {v}")).collect::<Vec<_>>().join("\n"))]
    Invalid(Vec<(&'static str, String)>),
}

/// Ket qua.
pub type ConfigResult<T> = Result<T, ConfigError>;
