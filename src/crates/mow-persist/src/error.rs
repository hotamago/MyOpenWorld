//! Loi cua tang luu tru.

use thiserror::Error;

/// Loi khi doc hoac ghi.
#[derive(Debug, Error)]
pub enum PersistError {
    /// Loi tu backend SQL.
    #[error("loi co so du lieu: {0}")]
    Backend(#[from] rusqlite::Error),

    /// Du lieu doc len khong dung hinh dang mong doi.
    ///
    /// Tach khoi `Backend` vi hai loai nay doi hoi hai phan ung khac han: loi
    /// backend thi thu lai duoc, con du lieu hong thi khong — no can mot bao
    /// cao ro rang va mot quyet dinh cua nguoi dung (§P10.6).
    #[error("du lieu hong: {0}")]
    Corrupt(String),

    /// Khong tim thay thu duoc yeu cau.
    #[error("khong tim thay: {0}")]
    NotFound(String),

    /// Loi tu mot backend khong phai `SQLite` (Postgres, ...).
    ///
    /// Tach khoi `Backend` vi `Backend` mang thang `rusqlite::Error`. Doi no
    /// thanh mot kieu chung se bat moi cho goi phai bo `?` va viet tay
    /// `map_err` — mot thay doi lan ra ca crate chi de phuc vu backend thu hai.
    #[error("loi backend: {0}")]
    External(String),
}

/// Ket qua cua thao tac luu tru.
pub type PersistResult<T> = Result<T, PersistError>;
