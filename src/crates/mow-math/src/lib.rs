//! # `mow-math` — miền số học xác định của My Open World
//!
//! Crate này tồn tại vì một lý do rất cụ thể: **một fixed-point không đủ cho
//! mọi miền**. Đặc tả của chính dự án này chứa những đại lượng lệch nhau hàng
//! chục bậc độ lớn, và ép tất cả vào một kiểu duy nhất sẽ làm mất hẳn một số
//! trong chúng.
//!
//! Bằng chứng cụ thể (`plan.md §P10.2.1`):
//!
//! ```text
//! Q16.16 có bước nhỏ nhất  2^-16 = 1.5259e-05
//! idea.md §21.2 khai báo   mutation_rate_per_locus = 2.1e-08
//! 2.1e-8 * 65536 = 0.0014  ──►  lưu vào Q16.16 thành 0
//! ```
//!
//! Không phải "đột biến hiếm đi", mà là **đột biến không bao giờ xảy ra nữa**.
//! Một thế giới không có đột biến thì tiến hóa dừng lại, và không có test nào
//! của gameplay sẽ nói cho bạn biết vì sao.
//!
//! ## Bảng miền
//!
//! | Miền | Kiểu ở đây | Ví dụ trong thế giới |
//! |---|---|---|
//! | Tỉ lệ chuẩn hóa `[0,1]` | [`fixed::Unit`] trên [`fixed::Fx`] (Q16.16) | `focus`, `visibility`, `CraftQuality` |
//! | Xác suất nhỏ, tỉ lệ hiếm | [`prob::Prob`] (`u64` thang `2^64`) | tỉ lệ đột biến, lây bệnh, backfire |
//! | Tốc độ theo thời gian | [`rate::Rate`] (hữu tỉ + carry) | nhu cầu tụt, hao mòn, tỉ lệ đồng hồ |
//! | Đại lượng vật lý | [`units`] (nguyên có đơn vị) | `Mass`, `Energy`, `Volume`, `Temp` |
//! | Tiền tệ | [`units::Money`] | đơn vị nhỏ nhất, không chia lẻ |
//! | Tọa độ | [`coord::WorldPos`] (`i64` checked, `i128` trung gian) | vị trí, chunk |
//!
//! ## Ba luật bất di bất dịch
//!
//! 1. **Không có `f32`/`f64`** ở bất kỳ đâu trong crate này, kể cả để hiển thị.
//!    `mow-math` là crate duy nhất mà luật này được kiểm bằng test kiến trúc
//!    trong chính nó ([`tests/no_float.rs`]).
//! 2. **Tràn là lỗi có tên**, không phải wrap-around. Mọi phép trả
//!    [`error::MathResult`].
//! 3. **Chuyển miền là hàm tường minh.** Không có `From`/`Into` giữa hai miền;
//!    nếu bạn cần một tỉ lệ trở thành một xác suất thì phải viết ra là bạn đang
//!    làm gì.
//!
//! [`tests/no_float.rs`]: https://github.com/hotamago/MyOpenWorld

#![deny(missing_docs)]
#![deny(clippy::float_arithmetic)] // allow-float: đây chính là lint cấm số thực
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod coord;
pub mod error;
pub mod fixed;
pub mod hash;
pub mod prob;
pub mod rate;
pub mod rng;
pub mod units;

pub use coord::{ChunkPos, WorldPos, WorldVec};
pub use error::{MathError, MathResult};
pub use fixed::{Fx, Unit};
pub use hash::{CanonicalHash, StateHash, StateHasher};
pub use prob::Prob;
pub use rate::Rate;
pub use rng::{DetRng, RngStreams, StreamName, WorldSeed};
pub use units::{Energy, Food, Mass, Money, Temp, Ticks, Volume};
