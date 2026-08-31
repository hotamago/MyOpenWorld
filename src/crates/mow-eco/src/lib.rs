//! # `mow-eco` — hệ sinh thái thay đổi theo thời gian
//!
//! `§7.3` sinh hệ sinh thái ban đầu, `§8.3` mô phỏng quần thể. Crate này là
//! **phần ở giữa** mà `§9.10` nói là còn thiếu: hệ sinh thái thay đổi, và hành
//! động của nền văn minh có hậu quả sinh thái **đọc được**.
//!
//! - [`succession`] — diễn thế theo thời gian, bốn quá trình sinh thái.
//! - [`invasion`] — loài xâm lấn và mầm bệnh đi qua cổng (`§9.10.1`).
//!
//! Điểm chung của hai module: không có cờ `is_invasive`, không có
//! `disaster_chance`. Mọi hậu quả tính ra từ dữ liệu, và mọi hậu quả truy ngược
//! được về một hành động cụ thể — thường là một hành động mà người làm không
//! định gây ra hậu quả đó.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod invasion;
pub mod succession;

pub use invasion::{
    assess, outbreak, Ecosystem, FoodWeb, Immunity, InvasionRisk, Outbreak, Virulence,
    NGUONG_XOA_SO,
};
pub use succession::{dat_toi_thieu, Event, Patch, Process, Stage, NAM_MOI_GIAI_DOAN};
