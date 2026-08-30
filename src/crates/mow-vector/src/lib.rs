//! # `mow-vector` — chỉ mục ký ức
//!
//! Hai quyết định ở crate này đáng giải thích, vì cả hai đều đi ngược trực giác
//! thông thường về vector search.
//!
//! ## 1. Vector được lượng tử hóa thành số nguyên
//!
//! Thư viện vector search bình thường dùng `f32` và điều đó hoàn toàn hợp lý —
//! khi kết quả tìm kiếm chỉ để hiển thị cho người. Ở đây thì không: ký ức được
//! truy xuất **chảy thẳng vào prompt**, prompt sinh ra hành động, hành động đổi
//! thế giới. Nghĩa là thứ tự xếp hạng của tìm kiếm là một phần của mô phỏng.
//!
//! Cộng dồn `f32` không giao hoán. Đổi thứ tự cộng — điều xảy ra khi số luồng
//! đổi, hay khi trình biên dịch vector hóa khác đi — sẽ đổi điểm số ở chữ số
//! cuối, và đôi khi đổi luôn thứ hạng của hai ký ức gần bằng nhau. Thế giới rẽ
//! nhánh, replay hỏng, và không ai tìm ra vì sao.
//!
//! Nên embedding được **lượng tử hóa sang `i16` ở biên** (xem [`quantize`]) và
//! mọi phép tính là số nguyên. Mất một chút độ chính xác ngữ nghĩa, đổi lấy
//! việc truy xuất trở thành hàm thuần. Đó là một cuộc trao đổi tốt, và nó chỉ
//! rẻ vì §P6.3 đã nói chỉ mục là thứ **dựng lại được**, không phải nguồn sự thật.
//!
//! ## 2. Lọc theo dòng dõi, không phải theo nhánh
//!
//! `plan.md §P6.3` viết ra điều kiện đầy đủ, và nó không phải `branch_id = ?`:
//!
//! ```text
//! created_branch_id ∈ ancestry(current_branch)
//! AND current_branch ∉ tombstoned_in_branches
//! AND created_tick <= fork_tick(nhánh con tương ứng trên đường đi)
//! ```
//!
//! Lọc phẳng theo `branch_id` diễn đạt được "ký ức của nhánh này", nhưng không
//! diễn đạt được **"thấy ký ức của cha tới điểm fork, không thấy sau đó"**.
//! Thiếu vế thứ ba, một nhánh con sẽ đọc được ký ức mà cha nó tạo ra *sau* khi
//! chúng đã tách — tức là đọc được tương lai của một thế giới song song.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod contract;
pub mod embedded;

use mow_core::{BranchId, Tick};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Lỗi của chỉ mục.
#[derive(Debug, Error)]
pub enum VectorError {
    /// Lỗi lưu trữ.
    #[error("lỗi lưu trữ chỉ mục: {0}")]
    Backend(#[from] rusqlite::Error),
    /// Số chiều không khớp với chiều đã khai báo lúc tạo chỉ mục.
    #[error("vector {got} chiều, chỉ mục khai báo {want} chiều")]
    Dimension {
        /// Số chiều nhận được.
        got: usize,
        /// Số chiều mong đợi.
        want: usize,
    },
}

/// Kết quả.
pub type VectorResult<T> = Result<T, VectorError>;

/// Định danh một điểm ký ức.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MemoryId(pub u64);

/// Lượng tử hóa một embedding sang `i16`, **chuẩn hóa L2**.
///
/// Đây là **biên duy nhất** của hệ thống mà số thực được phép xuất hiện, và nó
/// xuất hiện đúng một lần cho mỗi ký ức, ở phía nhận dữ liệu từ mô hình
/// embedding bên ngoài. Sau điểm này, mọi phép tính là số nguyên.
///
/// Phải là L2 chứ không phải chuẩn hóa theo giá trị lớn nhất. Chuẩn hóa theo
/// max nghe có vẻ tương đương — nó cũng đưa mọi vector về cùng thang — nhưng nó
/// vứt mất chính thông tin mà tích vô hướng dùng để xếp hạng:
///
/// ```text
/// chuẩn hóa theo max:   [1.0, 0.0] → [32767,     0]
///                       [0.9, 0.1] → [32767,  3641]     ← thành phần đầu bằng nhau!
///   ⇒ tích vô hướng với [1, 0] cho hai điểm BẰNG NHAU, dù rõ ràng cái đầu gần hơn
///
/// chuẩn hóa L2:         [1.0, 0.0] → [32767,     0]
///                       [0.9, 0.1] → [32586,  3620]
///   ⇒ xếp hạng đúng
/// ```
///
/// Lỗi này không làm chỉ mục hỏng hẳn — nó chỉ làm những ký ức *gần giống nhau*
/// hòa điểm, rồi vế phá hòa bằng `id` quyết định thay. Nghĩa là NPC sẽ nhớ ra
/// ký ức **cũ nhất** thay vì ký ức **liên quan nhất**, một cách im lặng.
pub fn quantize(v: &[f32]) -> Vec<i16> {
    // allow-float: biên nhận dữ liệu từ mô hình embedding, xem tài liệu module.
    let norm_sq: f32 = v.iter().map(|x| x * x).sum();
    if norm_sq <= 0.0 {
        return vec![0; v.len()];
    }
    let scale = f32::from(i16::MAX) / norm_sq.sqrt();
    v.iter()
        .map(|x| {
            (x * scale)
                .round()
                .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
        })
        .collect()
}

/// Một điểm trong chỉ mục.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPoint {
    /// Định danh.
    pub id: MemoryId,
    /// Không gian tên: ai sở hữu ký ức này (`§22.16`).
    pub namespace: String,
    /// Phiên bản persona lúc ký ức hình thành.
    pub persona_version: u32,
    /// Nhánh nơi ký ức được tạo.
    pub created_branch: BranchId,
    /// Tick lúc được tạo.
    pub created_tick: Tick,
    /// Embedding đã lượng tử hóa.
    pub vector: Vec<i16>,
    /// Nội dung, byte đục với tầng này.
    pub payload: Vec<u8>,
}

/// Một kết quả tìm kiếm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// Điểm ký ức.
    pub point: MemoryPoint,
    /// Điểm tương đồng, số nguyên. Lớn hơn là gần hơn.
    pub score: i64,
}

/// Một mắt xích dòng dõi: nhánh, và mốc cắt hiệu lực khi nhìn từ nhánh hiện tại.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineageStep {
    /// Nhánh tổ tiên.
    pub branch: BranchId,
    /// Ký ức của nhánh này chỉ nhìn thấy được nếu `created_tick <= cutoff`.
    ///
    /// Với chính nhánh hiện tại, `cutoff` là [`Tick`] lớn nhất — nó thấy hết
    /// những gì nó tạo ra. Với mỗi tổ tiên, `cutoff` là `fork_tick` của nhánh
    /// con nằm trên đường đi xuống nhánh hiện tại.
    pub cutoff: Tick,
}

/// Truy vấn có lọc dòng dõi.
#[derive(Debug, Clone)]
pub struct Query {
    /// Vector cần tìm gần.
    pub vector: Vec<i16>,
    /// Chỉ lấy ký ức thuộc các namespace này. Rỗng nghĩa là **không lấy gì**.
    ///
    /// Mặc định an toàn là "không thấy gì" chứ không phải "thấy tất cả".
    /// Mặc định ngược lại sẽ biến một lỗi quên truyền namespace thành một vụ rò
    /// rỉ toàn bộ ký ức của mọi nhân vật vào một prompt.
    pub namespaces: Vec<String>,
    /// Nhánh hiện tại và toàn bộ dòng dõi của nó, từ chính nó ngược về gốc.
    pub lineage: Vec<LineageStep>,
    /// Số kết quả tối đa.
    pub limit: usize,
}

/// Chỉ mục vector.
pub trait VectorIndex: Send + 'static {
    /// Số chiều của chỉ mục.
    fn dimension(&self) -> usize;

    /// Thêm hoặc thay một điểm.
    fn upsert(&mut self, point: &MemoryPoint) -> VectorResult<()>;

    /// Đánh dấu một ký ức là đã bị quên **trên một nhánh cụ thể** (`§11.5`).
    ///
    /// Đánh dấu chứ không xóa: nhánh chị em vẫn phải thấy ký ức đó. Xóa thật sự
    /// sẽ làm một thao tác "quên" ở nhánh này bốc hơi ký ức ở nhánh khác — và
    /// vì hai nhánh thường được so sánh với nhau, lỗi đó sẽ trông như một sự
    /// khác biệt có ý nghĩa.
    fn tombstone(&mut self, id: MemoryId, branch: BranchId) -> VectorResult<()>;

    /// Tìm kiếm.
    ///
    /// Kết quả **phải xác định**: cùng chỉ mục, cùng truy vấn, cùng thứ tự trả
    /// về, kể cả khi các điểm được chèn theo thứ tự khác.
    fn search(&self, q: &Query) -> VectorResult<Vec<Hit>>;

    /// Xóa sạch để dựng lại từ nhật ký sự kiện (`PC-06`).
    fn clear(&mut self) -> VectorResult<()>;

    /// Số điểm đang có.
    fn len(&self) -> VectorResult<usize>;

    /// Rỗng hay không.
    fn is_empty(&self) -> VectorResult<bool> {
        Ok(self.len()? == 0)
    }
}

/// Tích vô hướng số nguyên. Cộng dồn trong `i64`, không thể tràn với `i16` và
/// số chiều thực tế (tối đa `2^15 · 2^15 · n`, tức `n` phải vượt `2^33` mới
/// tràn).
pub fn dot(a: &[i16], b: &[i16]) -> i64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| i64::from(*x) * i64::from(*y))
        .sum()
}
