//! Mức chi tiết mô phỏng (`idea.md §8.3`, `§22.14`, `PB-15`).
//!
//! > Chuyển LOD **không làm mất** entity quan trọng, project, relationship,
//! > knowledge hoặc casualty.
//!
//! ## Vì sao đây là bất biến khó nhất trong dự án
//!
//! LOD là thứ khiến thế giới lớn được. Không có nó, một thành phố mười nghìn
//! dân đòi mười nghìn lượt mô phỏng mỗi tick và mọi thứ dừng lại.
//!
//! Nhưng LOD cũng là chỗ dữ liệu **biến mất một cách im lặng**. Hạ một khu định
//! cư xuống `Far` là gộp mười nghìn cá thể thành vài con số; nâng nó lên lại là
//! dựng lại mười nghìn cá thể từ vài con số đó. Ở giữa hai bước, thông tin mất
//! đi — và nó mất theo cách không ai để ý, vì con số tổng vẫn trông hợp lý.
//!
//! Ba lớp phòng thủ ở đây, và cả ba đều cần:
//!
//! 1. **Đại lượng bảo toàn được khai báo tường minh** ([`Conserved`]), và
//!    [`Aggregate::verify_against`] so trước và sau.
//! 2. **Thực thể quan trọng không bao giờ bị gộp** ([`Aggregate::pinned`]).
//!    Một vị vua không được biến thành "một phần của dân số 10 000".
//! 3. **Trạng thái là hàm của thời gian, không phải biến cập nhật mỗi tick.**
//!    Đó là lý do `mow-life` và `mow-effect` được viết như thế — với tích phân
//!    đóng, LOD **không xuất hiện trong công thức** và bất biến này gần như
//!    miễn phí.

use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use crate::chunk::Lod;

/// Đại lượng phải bảo toàn qua mọi lần chuyển mức.
///
/// Danh sách này **là** bất biến `§22.14`, dưới dạng dữ liệu. Thêm một trường ở
/// đây là thêm một thứ mà LOD không được phép làm mất.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Conserved {
    /// Số cá thể còn sống.
    pub population: u64,
    /// Số cá thể đã chết — **cũng phải bảo toàn**.
    ///
    /// Người chết là một sự kiện đã xảy ra. Bỏ họ ra khỏi phép đếm sẽ khiến một
    /// trận dịch trông như thể dân số "bốc hơi", và phép kiểm bảo toàn sẽ báo
    /// động giả sau mỗi thảm họa.
    pub casualties: u64,
    /// Tổng tài nguyên, theo loại.
    pub resources: u64,
    /// Số quan hệ xã hội.
    pub relationships: u64,
    /// Số dự án đang dở.
    pub projects: u64,
    /// Số nút tri thức mà cộng đồng nắm.
    pub knowledge: u64,
}

impl Conserved {
    /// Cộng hai bộ đại lượng.
    pub fn merge(self, other: Conserved) -> Conserved {
        Conserved {
            population: self.population + other.population,
            casualties: self.casualties + other.casualties,
            resources: self.resources + other.resources,
            relationships: self.relationships + other.relationships,
            projects: self.projects + other.projects,
            knowledge: self.knowledge + other.knowledge,
        }
    }
}

impl CanonicalHash for Conserved {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.population);
        h.write_u64(self.casualties);
        h.write_u64(self.resources);
        h.write_u64(self.relationships);
        h.write_u64(self.projects);
        h.write_u64(self.knowledge);
    }
}

/// Một đại lượng bị lệch khi chuyển mức.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leak {
    /// Tên đại lượng.
    pub quantity: &'static str,
    /// Trước khi chuyển.
    pub before: u64,
    /// Sau khi chuyển.
    pub after: u64,
}

impl core::fmt::Display for Leak {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let dau = if self.after > self.before { "+" } else { "-" };
        let lech = self.before.abs_diff(self.after);
        write!(
            f,
            "{}: {} → {} ({dau}{lech})",
            self.quantity, self.before, self.after
        )
    }
}

/// Dạng gộp của một vùng ở mức `Near` hoặc `Far`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aggregate {
    /// Mức chi tiết hiện tại.
    pub lod: Lod,
    /// Đại lượng bảo toàn.
    pub conserved: Conserved,
    /// Thực thể **không bao giờ bị gộp**.
    ///
    /// Một vị vua, một nhân vật người chơi quen, một người mang một tri thức
    /// độc nhất. Chúng giữ nguyên định danh qua mọi lần chuyển mức; nếu không,
    /// hạ LOD sẽ là một cách giết người mà không ai để ý.
    pinned: BTreeSet<u64>,
    /// Thống kê gộp theo loại, cho những gì không được ghim.
    pub buckets: BTreeMap<String, u64>,
}

impl Aggregate {
    /// Dạng gộp mới.
    pub fn new(lod: Lod, conserved: Conserved) -> Aggregate {
        Aggregate {
            lod,
            conserved,
            pinned: BTreeSet::new(),
            buckets: BTreeMap::new(),
        }
    }

    /// Ghim một thực thể để nó không bị gộp.
    pub fn pin(&mut self, entity: u64) {
        self.pinned.insert(entity);
    }

    /// Bỏ ghim.
    pub fn unpin(&mut self, entity: u64) -> bool {
        self.pinned.remove(&entity)
    }

    /// Những thực thể được ghim, theo thứ tự.
    pub fn pinned(&self) -> impl Iterator<Item = u64> + '_ {
        self.pinned.iter().copied()
    }

    /// Có được ghim không.
    pub fn is_pinned(&self, entity: u64) -> bool {
        self.pinned.contains(&entity)
    }

    /// So với một bộ đại lượng khác và trả về mọi chỗ lệch.
    ///
    /// Gọi **trước và sau** mỗi lần chuyển mức. Trả về danh sách rỗng nghĩa là
    /// `§22.14` được giữ; bất kỳ mục nào cũng là một bug của engine.
    pub fn verify_against(&self, other: &Conserved) -> Vec<Leak> {
        let a = self.conserved;
        let b = *other;
        [
            ("population", a.population, b.population),
            ("casualties", a.casualties, b.casualties),
            ("resources", a.resources, b.resources),
            ("relationships", a.relationships, b.relationships),
            ("projects", a.projects, b.projects),
            ("knowledge", a.knowledge, b.knowledge),
        ]
        .into_iter()
        .filter(|(_, x, y)| x != y)
        .map(|(quantity, before, after)| Leak {
            quantity,
            before,
            after,
        })
        .collect()
    }
}

impl CanonicalHash for Aggregate {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.lod as i64);
        self.conserved.canonical_hash(h);
        h.write_seq(self.pinned.iter().copied(), |hh, e| {
            hh.write_u64(e);
        });
        h.write_seq(self.buckets.iter(), |hh, (k, v)| {
            hh.write_str(k);
            hh.write_u64(*v);
        });
    }
}

/// Lỗi khi chuyển mức.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LodError {
    /// Đại lượng bảo toàn bị lệch.
    #[error(
        "chuyển {from:?} → {to:?} làm lệch đại lượng bảo toàn:\n{}\n\
         §22.14 cấm điều này. Đây LUÔN là bug của engine, không phải lỗi dữ liệu.",
        .leaks.iter().map(|l| format!("  {l}")).collect::<Vec<_>>().join("\n")
    )]
    NotConserved {
        /// Mức cũ.
        from: Lod,
        /// Mức mới.
        to: Lod,
        /// Các chỗ lệch.
        leaks: Vec<Leak>,
    },

    /// Thực thể được ghim bị mất khi gộp.
    #[error(
        "thực thể được ghim {0} biến mất khi chuyển mức. Hạ LOD không được là \
         một cách giết người."
    )]
    PinnedLost(u64),
}

/// Chuyển một vùng sang mức khác.
///
/// `to_conserved` là đại lượng đo được **sau** khi chuyển. Hàm này so nó với
/// đại lượng trước, và từ chối nếu lệch — chứ không im lặng chấp nhận và để lỗi
/// trôi vào lịch sử.
pub fn transition(
    agg: &mut Aggregate,
    to: Lod,
    to_conserved: Conserved,
    surviving_entities: &BTreeSet<u64>,
) -> Result<(), LodError> {
    let leaks = agg.verify_against(&to_conserved);
    if !leaks.is_empty() {
        return Err(LodError::NotConserved {
            from: agg.lod,
            to,
            leaks,
        });
    }

    // Thực thể ghim phải sống sót qua phép chuyển.
    if let Some(mat) = agg.pinned.iter().find(|e| !surviving_entities.contains(e)) {
        return Err(LodError::PinnedLost(*mat));
    }

    agg.lod = to;
    agg.conserved = to_conserved;
    Ok(())
}

/// Chọn mức chi tiết theo khoảng cách tới tiêu điểm.
///
/// `§8.4`: đổi camera **không** đổi mức mô phỏng — chỉ `SetSimulationFocus` mới
/// đổi. Hàm này nhận khoảng cách tới **tiêu điểm mô phỏng**, không tới camera,
/// và phân biệt đó là toàn bộ nội dung của `PA-07`.
pub fn lod_for_distance(chunks_from_focus: u32, active_radius: u32) -> Lod {
    if chunks_from_focus <= active_radius {
        Lod::Active
    } else if chunks_from_focus <= active_radius * 4 {
        Lod::Near
    } else {
        Lod::Far
    }
}

/// Chi phí mô phỏng tương đối của một mức, để lập ngân sách.
///
/// Con số thật không quan trọng bằng **tỉ lệ**: `Far` phải rẻ hơn `Active` vài
/// bậc độ lớn, nếu không LOD không giải quyết được gì.
pub fn relative_cost(lod: Lod) -> u32 {
    match lod {
        Lod::Active => 1_000,
        Lod::Near => 50,
        Lod::Far => 1,
    }
}
