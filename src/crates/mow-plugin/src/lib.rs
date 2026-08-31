//! # `mow-plugin` — sổ đăng ký content pack
//!
//! `plan.md §P10.7` giải thích vì sao thứ này phải có **từ Giai đoạn 0**, dù UI
//! quản lý pack tới tận Giai đoạn F mới làm:
//!
//! > `content/core/` chính là một pack dùng đúng cơ chế mà cộng đồng sẽ dùng —
//! > không có đường đặc quyền cho nội dung chính thức.
//!
//! Nếu để muộn, các giai đoạn giữa sẽ chạy bằng một loader đặc quyền tạm thời.
//! Rồi khi chuyển sang cơ chế thật, **id, lockfile và content hash của mọi save
//! cũ đều đổi** — tức là mọi thế giới đã tạo trong sáu tháng phát triển đều
//! không mở lại được. Cái giá của việc làm sớm là vài trăm dòng; cái giá của
//! việc làm muộn là toàn bộ dữ liệu thử nghiệm.
//!
//! Ba bất biến crate này giữ:
//!
//! - `§22.29` — **mọi id đăng ký phải có namespace**. Ghi đè phải khai báo
//!   tường minh, và xung đột là **lỗi**, không phải "ai load sau thì thắng".
//! - `§22.30` — save ghi pack set, version và content hash; lệch thì **từ chối
//!   load** thay vì load một phần.
//! - Thứ tự nạp **xác định**, suy ra từ đồ thị phụ thuộc, không phải từ thứ tự
//!   thư mục trên đĩa (thứ khác nhau giữa Windows và Linux).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

pub mod capability;
pub mod hotreload;
pub mod manifest;
pub mod registry;

pub use capability::{Capability, ContentKind, Grants, Violation};
pub use hotreload::{
    plan_reload, snapshot, BuildKind, PackSnapshot, ReloadError, ReloadPlan, ReloadStep, TestReport,
};
pub use manifest::{PackManifest, PackRef};
pub use registry::{LoadOrder, PackSet, Registry, RegistryError, RegistryResult};
