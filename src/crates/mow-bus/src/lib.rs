//! # `mow-bus` — bus thông điệp bền
//!
//! `P0-08` nói rõ một điều dễ làm sai: **"Không tự dựng lại JetStream."**
//!
//! Cám dỗ là viết một bus in-process "có cùng ngữ nghĩa durable như JetStream"
//! — at-least-once, consumer group, replay theo sequence, redelivery có
//! backoff. Nhưng đó chính là viết một hàng đợi thứ hai, và nguyên tắc 2 của
//! `plan.md §P1` cấm điều đó.
//!
//! Cái được dựng ở đây hẹp hơn nhiều, và cố ý hẹp:
//!
//! > Một proposal đã nhận **không được mất khi tiến trình chết**.
//!
//! Chỉ vậy. Không phân phối, không nhiều consumer tranh nhau, không backoff
//! tinh vi. Đủ để bản desktop chạy đúng, và đủ để interface không phải đổi khi
//! NATS JetStream thay thế nó ở `PC-20`.
//!
//! Điều còn lại quan trọng hơn: bộ test hợp đồng ở [`contract`] định nghĩa
//! **ngữ nghĩa nào là bắt buộc**. Ngữ nghĩa nào không có trong đó thì code gọi
//! không được phép dựa vào — kể cả khi hiện thực SQLite tình cờ cung cấp nó.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod contract;
pub mod sqlite;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Lỗi của bus.
#[derive(Debug, Error)]
pub enum BusError {
    /// Lỗi lưu trữ.
    #[error("lỗi lưu trữ bus: {0}")]
    Backend(#[from] rusqlite::Error),
    /// Ack một thông điệp không đang được giữ.
    #[error("ack {0} nhưng nó không đang được giữ")]
    NotLeased(u64),
}

/// Kết quả.
pub type BusResult<T> = Result<T, BusError>;

/// Tên chủ đề, có namespace: `cognition.request`, `memory.proposal`.
pub type Subject = String;

/// Một thông điệp đã nhận.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Số thứ tự bền, đơn điệu tăng trên toàn bus.
    pub seq: u64,
    /// Chủ đề.
    pub subject: Subject,
    /// Nội dung, byte đục.
    pub payload: Vec<u8>,
    /// Đã được giao bao nhiêu lần.
    ///
    /// Có trường này thì code gọi phân biệt được "lần đầu" với "thử lại lần thứ
    /// tư", và một thông điệp độc (`poison message`) không quay vòng mãi mãi.
    pub delivery_count: u32,
}

/// Bus thông điệp bền.
///
/// Ngữ nghĩa **at-least-once**: một thông điệp có thể được giao lại nếu tiến
/// trình chết trước khi ack. Vì vậy mọi consumer phải idempotent — và đó chính
/// là lý do [`mow_core::Command`] có `request_id` (`§20.2.2`).
///
/// [`mow_core::Command`]: https://docs.rs/mow-core
pub trait MessageBus: Send + 'static {
    /// Gửi. Trả về số thứ tự đã cấp.
    ///
    /// Khi hàm này trả `Ok`, thông điệp **đã nằm trên đĩa**. Đó là toàn bộ lời
    /// hứa của crate này.
    fn publish(&mut self, subject: &str, payload: &[u8]) -> BusResult<u64>;

    /// Lấy tối đa `max` thông điệp chưa xử lý của một chủ đề và **giữ** chúng.
    ///
    /// Giữ chứ không xóa: nếu tiến trình chết giữa chừng, [`MessageBus::recover`]
    /// sẽ trả chúng về hàng đợi.
    fn fetch(&mut self, subject: &str, max: usize) -> BusResult<Vec<Message>>;

    /// Xác nhận đã xử lý xong. Sau đó thông điệp không bao giờ được giao lại.
    fn ack(&mut self, seq: u64) -> BusResult<()>;

    /// Trả một thông điệp về hàng đợi mà không xử lý.
    fn nack(&mut self, seq: u64) -> BusResult<()>;

    /// Trả **mọi** thông điệp đang bị giữ về hàng đợi.
    ///
    /// Gọi lúc khởi động. Đây là chỗ lời hứa "không mất proposal khi crash"
    /// được thực hiện: những gì đang được giữ lúc tiến trình chết sẽ được giao
    /// lại. Trả về số thông điệp đã khôi phục.
    fn recover(&mut self) -> BusResult<usize>;

    /// Số thông điệp chưa ack của một chủ đề, gồm cả đang giữ.
    fn pending(&self, subject: &str) -> BusResult<usize>;
}
