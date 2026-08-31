//! # `mow-obs` — quan sát khi chạy
//!
//! Một quy tắc, và nó không có ngoại lệ (`plan.md §P8`):
//!
//! > **Mọi dòng log đều kèm `branch`, `world`, `tick`.**
//!
//! Nghe như chuyện nhỏ. Nó không nhỏ. Một hệ thống chạy nhiều thế giới trên
//! nhiều nhánh, và một dòng log ghi *"vật phẩm biến mất"* mà không nói ở đâu,
//! lúc nào, trên nhánh nào thì hoàn toàn vô dụng — không tái hiện được, không
//! đối chiếu được với nhật ký sự kiện, không bisect được.
//!
//! Cách thi hành: [`SimContext`] không có `Default`, và [`log_event`] đòi nó.
//! Không thể ghi một dòng log của mô phỏng mà quên ngữ cảnh, vì không có hàm
//! nào cho phép làm thế.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]

pub mod causechain;

pub use causechain::{Chain, ChainView, Link};

use mow_core::{BranchId, Tick, WorldId};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

/// Ngữ cảnh bắt buộc của mọi dòng log thuộc mô phỏng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SimContext {
    /// Nhánh lịch sử.
    pub branch: u64,
    /// Thế giới.
    pub world: u64,
    /// Tick địa phương.
    pub tick: u64,
}

impl SimContext {
    /// Dựng từ các kiểu của `mow-core`.
    pub fn new(branch: BranchId, world: WorldId, tick: Tick) -> SimContext {
        SimContext {
            branch: branch.get(),
            world: world.get(),
            tick: tick.0,
        }
    }
}

/// Mức log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Chi tiết cho lúc gỡ lỗi.
    Debug,
    /// Diễn tiến bình thường.
    Info,
    /// Đáng chú ý nhưng chưa hỏng.
    Warn,
    /// Hỏng.
    Error,
}

/// Một trace span đang mở.
///
/// `§P8` yêu cầu trace được cả đường `command → event`. Span ở đây tối giản —
/// nó có `trace_id` và `parent`, đủ để dựng lại cây nhân quả — và khi
/// OpenTelemetry được nối vào ở Giai đoạn C, `trace_id` này chính là thứ được
/// truyền sang.
#[derive(Debug, Clone)]
pub struct Span {
    /// Định danh trace.
    pub trace_id: u64,
    /// Span cha, nếu có.
    pub parent: Option<u64>,
    /// Tên.
    pub name: String,
    /// Ngữ cảnh mô phỏng.
    pub ctx: SimContext,
}

static NEXT_TRACE: AtomicU64 = AtomicU64::new(1);

impl Span {
    /// Mở một span gốc.
    pub fn root(name: &str, ctx: SimContext) -> Span {
        Span {
            trace_id: NEXT_TRACE.fetch_add(1, Ordering::Relaxed),
            parent: None,
            name: name.to_owned(),
            ctx,
        }
    }

    /// Mở một span con, giữ nguyên `trace_id`.
    pub fn child(&self, name: &str) -> Span {
        Span {
            trace_id: self.trace_id,
            parent: Some(self.trace_id),
            name: name.to_owned(),
            ctx: self.ctx,
        }
    }
}

/// Một bản ghi log đã định dạng.
#[derive(Debug, Clone, Serialize)]
pub struct LogRecord {
    /// Mức.
    pub level: Level,
    /// Thông điệp.
    pub message: String,
    /// Nhánh.
    pub branch: u64,
    /// Thế giới.
    pub world: u64,
    /// Tick.
    pub tick: u64,
    /// Trace, nếu dòng này nằm trong một span.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<u64>,
    /// Trường phụ, đã sắp theo khóa.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fields: std::collections::BTreeMap<String, String>,
}

/// Ghi một dòng log của mô phỏng.
///
/// Không có phiên bản nào của hàm này thiếu [`SimContext`]. Đó là toàn bộ cơ
/// chế thi hành quy tắc ở đầu module.
pub fn log_event(level: Level, ctx: SimContext, message: impl Into<String>) -> LogRecord {
    LogRecord {
        level,
        message: message.into(),
        branch: ctx.branch,
        world: ctx.world,
        tick: ctx.tick,
        trace_id: None,
        fields: std::collections::BTreeMap::new(),
    }
}

impl LogRecord {
    /// Gắn span.
    #[must_use]
    pub fn in_span(mut self, s: &Span) -> LogRecord {
        self.trace_id = Some(s.trace_id);
        self
    }

    /// Thêm một trường.
    #[must_use]
    pub fn field(mut self, k: &str, v: impl core::fmt::Display) -> LogRecord {
        self.fields.insert(k.to_owned(), v.to_string());
        self
    }

    /// Ra JSON một dòng, để thu thập.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            // Log không được làm sập tiến trình. Nếu tuần tự hóa hỏng thì ghi
            // ra một dòng vẫn đọc được, còn hơn mất hẳn dòng đó.
            format!(
                r#"{{"level":"error","message":"log hỏng: {e}","branch":{},"world":{},"tick":{}}}"#,
                self.branch, self.world, self.tick
            )
        })
    }

    /// Ra dạng người đọc.
    pub fn to_pretty(&self) -> String {
        let muc = match self.level {
            Level::Debug => "DEBUG",
            Level::Info => "INFO ",
            Level::Warn => "WARN ",
            Level::Error => "ERROR",
        };
        let phu: String = self
            .fields
            .iter()
            .map(|(k, v)| " ".to_owned() + k + "=" + v)
            .collect();
        format!(
            "{muc} [b{} w{} t{}] {}{phu}",
            self.branch, self.world, self.tick, self.message
        )
    }
}

/// Ngân sách hiệu năng (`plan.md §P8.1`).
///
/// Vượt ngân sách làm CI fail. Kiểu riêng thay vì vài hằng số rời rạc, vì báo
/// cáo phải nói **vượt bao nhiêu phần trăm**, không chỉ "vượt" — một mức vượt
/// 3% là nhiễu đo đạc, còn 300% là một hồi quy cần chặn ngay.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    /// Tên phép đo.
    pub name: &'static str,
    /// Giới hạn.
    pub limit: u64,
    /// Đơn vị, để in ra.
    pub unit: &'static str,
}

impl Budget {
    /// Kiểm một giá trị đo được.
    pub fn check(self, measured: u64) -> Result<(), String> {
        if measured <= self.limit {
            return Ok(());
        }
        let phan_tram = (measured.saturating_sub(self.limit) * 100) / self.limit.max(1);
        Err(format!(
            "{}: {} {} vượt ngân sách {} {} ({}% quá)",
            self.name, measured, self.unit, self.limit, self.unit, phan_tram
        ))
    }
}
