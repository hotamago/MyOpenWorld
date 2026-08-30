//! Miền 6 — **tọa độ**: `i64` có kiểm tra, trung gian `i128` (`idea.md §4.3`).
//!
//! Thế giới có thể lớn hơn `2^53`, tức lớn hơn khoảng số nguyên mà `f64` biểu
//! diễn chính xác được. Đó là lý do biên frontend phải dùng `BigInt` và tọa độ
//! camera-local (`§22.10`), và là lý do ở đây không có bất kỳ số thực nào.
//!
//! Tràn tọa độ là **lỗi xác định** chứ không phải wrap-around im lặng
//! (`§22.11`): một thực thể đi tới rìa vũ trụ phải nhận một lỗi có tên, không
//! phải đột ngột xuất hiện ở phía bên kia.

use crate::error::{overflow, MathResult};
use serde::{Deserialize, Serialize};

/// Điểm trong thế giới. `z` là tầng cao độ, cũng `i64` để phép toán đồng nhất.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct WorldPos {
    /// Trục đông–tây.
    pub x: i64,
    /// Trục bắc–nam.
    pub y: i64,
    /// Tầng cao độ.
    pub z: i64,
}

/// Vector dịch chuyển. Kiểu riêng với [`WorldPos`] vì "điểm cộng điểm" là vô
/// nghĩa còn "điểm cộng vector" thì có nghĩa; tách kiểu làm sai lầm đó không
/// biên dịch được.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct WorldVec {
    /// Thành phần theo `x`.
    pub dx: i64,
    /// Thành phần theo `y`.
    pub dy: i64,
    /// Thành phần theo `z`.
    pub dz: i64,
}

impl WorldPos {
    /// Gốc tọa độ.
    pub const ORIGIN: WorldPos = WorldPos { x: 0, y: 0, z: 0 };

    /// Dựng.
    #[inline]
    pub const fn new(x: i64, y: i64, z: i64) -> WorldPos {
        WorldPos { x, y, z }
    }

    /// Dời theo vector, tràn là lỗi.
    pub fn offset(self, v: WorldVec) -> MathResult<WorldPos> {
        Ok(WorldPos {
            x: self
                .x
                .checked_add(v.dx)
                .ok_or_else(|| overflow("WorldPos::offset.x", self.x, v.dx))?,
            y: self
                .y
                .checked_add(v.dy)
                .ok_or_else(|| overflow("WorldPos::offset.y", self.y, v.dy))?,
            z: self
                .z
                .checked_add(v.dz)
                .ok_or_else(|| overflow("WorldPos::offset.z", self.z, v.dz))?,
        })
    }

    /// Vector từ `other` tới `self`.
    pub fn delta(self, other: WorldPos) -> MathResult<WorldVec> {
        Ok(WorldVec {
            dx: self
                .x
                .checked_sub(other.x)
                .ok_or_else(|| overflow("WorldPos::delta.x", self.x, other.x))?,
            dy: self
                .y
                .checked_sub(other.y)
                .ok_or_else(|| overflow("WorldPos::delta.y", self.y, other.y))?,
            dz: self
                .z
                .checked_sub(other.z)
                .ok_or_else(|| overflow("WorldPos::delta.z", self.z, other.z))?,
        })
    }

    /// Khoảng cách Chebyshev trên mặt phẳng `xy`, tức số bước đi khi cho phép
    /// đi chéo. Đây là metric của lưới chiến thuật ở `§10.10`.
    ///
    /// Trả `i128` vì hiệu của hai `i64` ở hai cực có thể vượt `i64`.
    pub fn chebyshev_xy(self, other: WorldPos) -> i128 {
        let dx = (self.x as i128 - other.x as i128).abs();
        let dy = (self.y as i128 - other.y as i128).abs();
        dx.max(dy)
    }

    /// Khoảng cách Manhattan trên `xy`.
    pub fn manhattan_xy(self, other: WorldPos) -> i128 {
        (self.x as i128 - other.x as i128).abs() + (self.y as i128 - other.y as i128).abs()
    }

    /// Bình phương khoảng cách Euclid trong không gian 3 chiều, **bão hòa**.
    ///
    /// Trả bình phương chứ không phải căn bậc hai: căn là phép không đóng trên
    /// số nguyên, và mọi so sánh khoảng cách đều làm được trên bình phương.
    /// Chỉ tầng hiển thị mới cần căn, và ở đó số thực đã hết nguy hiểm.
    ///
    /// Bão hòa ở `u128::MAX` chứ không trả lỗi: hiệu hai tọa độ có thể tới
    /// `2^65`, và bình phương của nó vượt cả `i128`. Nhưng mọi câu hỏi thật sự
    /// hỏi bằng hàm này — "ai gần hơn", "có trong tầm không" — vẫn trả lời đúng
    /// khi giá trị bão hòa, vì ngưỡng bão hòa lớn hơn mọi bán kính có nghĩa
    /// nhiều bậc độ lớn. Một lỗi ở đây sẽ chỉ làm hai điểm xa vô tận trở nên
    /// "xa bằng nhau", điều vốn đúng.
    pub fn dist_sq(self, other: WorldPos) -> u128 {
        let comp = |a: i64, b: i64| -> u128 {
            let d = (a as i128 - b as i128).unsigned_abs();
            d.checked_mul(d).unwrap_or(u128::MAX)
        };
        comp(self.x, other.x)
            .saturating_add(comp(self.y, other.y))
            .saturating_add(comp(self.z, other.z))
    }

    /// Tọa độ chunk chứa điểm này, với chunk vuông cạnh `size`.
    ///
    /// Dùng `div_euclid` chứ không phải `/`: với `x = -1` và `size = 32`, phép
    /// chia cắt-về-0 cho chunk `0`, tức là ô `-1` và ô `0` rơi vào cùng chunk
    /// còn ô `-32` thì không. Lưới sẽ lệch đúng một ô quanh gốc và mọi seam
    /// test sẽ chạy qua mà không thấy gì.
    pub fn chunk_of(self, size: i64) -> MathResult<ChunkPos> {
        if size <= 0 {
            return Err(crate::error::MathError::BadDenominator { den: size });
        }
        Ok(ChunkPos {
            cx: self.x.div_euclid(size),
            cy: self.y.div_euclid(size),
            cz: self.z,
        })
    }

    /// Vị trí trong chunk, luôn nằm trong `[0, size)`.
    pub fn local_in_chunk(self, size: i64) -> MathResult<(i64, i64)> {
        if size <= 0 {
            return Err(crate::error::MathError::BadDenominator { den: size });
        }
        Ok((self.x.rem_euclid(size), self.y.rem_euclid(size)))
    }
}

impl WorldVec {
    /// Vector không.
    pub const ZERO: WorldVec = WorldVec {
        dx: 0,
        dy: 0,
        dz: 0,
    };

    /// Dựng.
    #[inline]
    pub const fn new(dx: i64, dy: i64, dz: i64) -> WorldVec {
        WorldVec { dx, dy, dz }
    }
}

/// Tọa độ chunk. `cz` là tầng, không chia nhỏ theo chiều cao.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct ChunkPos {
    /// Chỉ số chunk theo `x`.
    pub cx: i64,
    /// Chỉ số chunk theo `y`.
    pub cy: i64,
    /// Tầng.
    pub cz: i64,
}

impl ChunkPos {
    /// Góc gốc của chunk trong tọa độ thế giới.
    pub fn origin(self, size: i64) -> MathResult<WorldPos> {
        Ok(WorldPos {
            x: self
                .cx
                .checked_mul(size)
                .ok_or_else(|| overflow("ChunkPos::origin.x", self.cx, size))?,
            y: self
                .cy
                .checked_mul(size)
                .ok_or_else(|| overflow("ChunkPos::origin.y", self.cy, size))?,
            z: self.cz,
        })
    }
}

impl core::fmt::Display for WorldPos {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl core::fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "chunk({}, {}, {})", self.cx, self.cy, self.cz)
    }
}
