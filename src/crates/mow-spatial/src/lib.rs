//! # `mow-spatial` — chunk, occupancy, tải lười
//!
//! Bất biến trung tâm là `§22.12`:
//!
//! > Chunk chưa materialize không chiếm storage tỉ lệ với thể tích world.
//!
//! Nghe hiển nhiên cho tới khi bạn thấy cách nó bị vi phạm. Cách phổ biến nhất
//! không phải là lưu hết các ô — không ai làm thế — mà là lưu một *bản ghi rằng
//! chunk đã được ghé qua*. Một `HashSet<ChunkPos>` các chunk đã tải nghe vô
//! hại, nhưng người chơi đi bộ mười giờ sẽ chạm hàng trăm nghìn chunk, và tập
//! đó lớn lên mãi mãi dù không có gì thay đổi trong chúng.
//!
//! Nguyên tắc ở đây: **chỉ ghi thứ khác với cái sinh ra được**. Một chunk chỉ
//! tồn tại trong save khi nó có delta. Đi ngang qua không tạo ra gì.
//!
//! ```text
//! ô hiện tại = base_cell(seed, profile, x, y)   nếu không có delta
//!            = delta[x, y]                       nếu có
//! ```
//!
//! Trạng thái tải (`Active`/`Near`/`Far`) là **bộ nhớ**, không phải lưu trữ.
//! Nó không bao giờ được ghi xuống đĩa; nếu ghi, nó lại trở thành một tập lớn
//! dần theo quãng đường đã đi.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod chunk;
pub mod lod;
pub mod occupancy;

pub use chunk::{Chunk, ChunkDelta, ChunkStore, Lod};
pub use lod::{lod_for_distance, transition, Aggregate, Conserved, Leak, LodError};
pub use occupancy::Occupancy;
