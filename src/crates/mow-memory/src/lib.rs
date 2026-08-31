//! # `mow-memory` — ký ức và quan hệ suy ra từ ký ức
//!
//! Ba mắt viết hoa của vòng lặp ở `idea.md §1.2`, `§3.2` mà crate này lấp:
//!
//! ```text
//! ... → hành động hợp lệ → tác động
//!         → NGƯỜI KHÁC QUAN SÁT/DIỄN GIẢI → KÝ ỨC, QUAN HỆ → điều kiện mới → ...
//! ```
//!
//! Trước crate này, cư dân đi làm, ăn, ngủ đúng lịch
//! (`mow_society::routine`) nhưng không ai để ý tới ai: không ký ức, không
//! quan hệ, không lý do để hai cư dân đối xử khác nhau với hai người khác
//! nhau. Thế giới **đúng** mà không **sống**, đúng như tài liệu nhiệm vụ mô
//! tả.
//!
//! ## Trụ cột `§1.2.2` — tri thức cục bộ
//!
//! > Một cá thể chỉ biết điều nó cảm nhận, được dạy, suy luận hoặc nghe kể.
//! > Dữ liệu thật của thế giới không tự động trở thành kiến thức của nhân
//! > vật.
//!
//! Đây là lý do toàn bộ crate xoay quanh [`Recollection`] thay vì một hàm đọc
//! thẳng sự thật thế giới. Không có hàm nào ở đây nhận `&Store` hay `&Sim`.
//! Thứ duy nhất một hàm công khai được đọc là một [`Recollection`] cụ thể —
//! ký ức của **một** người — giống hệt cách
//! `mow_action::perception::CognitionContext` là cửa duy nhất vào tri giác:
//! ranh giới được thi hành bằng chữ ký hàm, không bằng kỷ luật của người gọi.
//!
//! ## Bốn module, đúng bốn mắt của vòng lặp
//!
//! - [`memory`] — [`Memory`]/[`MemoryKind`]: một điều **một** người tin là đã
//!   xảy ra, và nó tương ứng `EventKind` thật nào của engine (không bịa loại
//!   ký ức mà engine chưa từng sinh sự kiện).
//! - [`recollection`] — [`Recollection`]: sổ ký ức có trần, cách nó mờ dần
//!   không đều, và cách nghe kể làm ký ức méo đi mà không bịa nội dung.
//! - [`bond`] — [`Bond`]/[`bond_of`]: quan hệ **là một hàm của ký ức**, không
//!   phải trạng thái lưu riêng.
//! - [`behavior`] — [`preferred_company`]/[`would_help`]: ký ức đổi hành vi,
//!   khép vòng lặp lại thành "điều kiện mới".
//!
//! ## Vì sao đây là một thư viện thuần
//!
//! Không `Sim`, không mạng, không thời gian thực, không số thực trên đường
//! commit (`§P10.2.1`). Mọi hàm công khai là hàm thuần của dữ liệu được
//! truyền vào — kể cả [`Recollection::hear`], nơi "ngẫu nhiên" của việc méo
//! tin đồn thật ra là một hàm tất định của nội dung câu chuyện (xem tài liệu
//! module `recollection`). Crate chỉ phụ thuộc `mow-core` (cho `EntityId`/
//! `EventSeq` — kiểu định danh, không phải để đọc trạng thái) và `mow-math`
//! (cho tính xác định: `CanonicalHash`, `StateHasher`, `Rate`, `RngStreams`).
//! Không phụ thuộc `mow-action` hay `mow-society` dù cả hai được đọc để bắt
//! giọng và để tái dùng ý tưởng — xem tài liệu [`MemoryKind::Met`] và
//! [`MemoryKind::Saw`] về lý do cụ thể.
//!
//! Việc nối crate này vào [`mow_core::Store`]/`game.rs` — quyết định khi nào
//! gọi [`Recollection::witness`], `EventKind` nào sinh loại ký ức nào — nằm
//! ngoài phạm vi crate này, có chủ đích.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

pub mod behavior;
pub mod bond;
pub mod memory;
pub mod recollection;

pub use behavior::{preferred_company, would_help, HELP_THRESHOLD};
pub use bond::{bond_of, Bond};
pub use memory::{source_event, Memory, MemoryKind, STRENGTH_MAX, STRENGTH_MIN};
pub use recollection::{Recollection, MEMORY_CAP};
