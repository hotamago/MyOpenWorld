//! Chunk, delta và mức chi tiết.

use mow_core::Value;
use mow_math::{CanonicalHash, ChunkPos, StateHash, StateHasher, WorldPos};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Mức chi tiết của một chunk (`§8.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lod {
    /// Mô phỏng đầy đủ theo từng ô và từng thực thể.
    Active,
    /// Mô phỏng rút gọn: thực thể còn tồn tại nhưng hành vi gộp lại.
    Near,
    /// Chỉ còn thống kê gộp. Dân số, tài nguyên, quan hệ được **bảo toàn**
    /// (`§22.14`), nhưng không có thực thể riêng lẻ nào chạy.
    ///
    /// **Mặc định.** Mọi chunk bắt đầu ở mức rẻ nhất và chỉ được nâng lên khi có
    /// lý do. Mặc định ngược lại — `Active` — nghĩa là một thế giới vừa nạp sẽ
    /// mô phỏng đầy đủ mọi chunk từng được ghi, và nó sẽ chết đứng.
    #[default]
    Far,
}

/// Thay đổi so với ô nguyên thủy.
///
/// Chỉ chứa **hiệu** với thứ worldgen sinh ra. Một chunk mà người chơi chỉ đi
/// ngang qua không có delta nào, nên nó không tồn tại trong save.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDelta {
    /// Ô đã đổi: `(x_cục_bộ, y_cục_bộ)` → thuộc tính đã ghi đè.
    ///
    /// `BTreeMap` để thứ tự duyệt xác định — delta đi vào state hash.
    cells: BTreeMap<(u32, u32), BTreeMap<String, Value>>,
}

impl ChunkDelta {
    /// Delta rỗng.
    pub fn new() -> ChunkDelta {
        ChunkDelta::default()
    }

    /// Không có thay đổi nào.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Số ô đã đổi.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Ghi đè một thuộc tính của một ô.
    pub fn set(&mut self, lx: u32, ly: u32, key: &str, v: Value) {
        self.cells
            .entry((lx, ly))
            .or_default()
            .insert(key.to_owned(), v);
    }

    /// Đọc một thuộc tính đã ghi đè.
    pub fn get(&self, lx: u32, ly: u32, key: &str) -> Option<&Value> {
        self.cells.get(&(lx, ly))?.get(key)
    }

    /// Xóa một ghi đè, trả ô về đúng thứ worldgen sinh ra.
    ///
    /// Dọn sạch ô rỗng để delta không giữ lại những `BTreeMap` trống — chúng
    /// vẫn chiếm chỗ trong save và vẫn làm `is_empty` trả `false`, tức là một
    /// chunk "đã hoàn nguyên" vẫn bị lưu mãi mãi.
    pub fn clear(&mut self, lx: u32, ly: u32, key: &str) {
        if let Some(o) = self.cells.get_mut(&(lx, ly)) {
            o.remove(key);
            if o.is_empty() {
                self.cells.remove(&(lx, ly));
            }
        }
    }
}

impl CanonicalHash for ChunkDelta {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.cells.iter(), |hh, ((lx, ly), attrs)| {
            hh.write_u64(u64::from(*lx));
            hh.write_u64(u64::from(*ly));
            hh.write_seq(attrs.iter(), |h3, (k, v)| {
                h3.write_str(k);
                v.canonical_hash(h3);
            });
        });
    }
}

/// Một chunk đang nằm trong bộ nhớ.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Vị trí.
    pub pos: ChunkPos,
    /// Mức chi tiết hiện tại.
    pub lod: Lod,
    /// Thay đổi so với ô nguyên thủy.
    pub delta: ChunkDelta,
}

/// Kho chunk.
///
/// Hai cấu trúc, và ranh giới giữa chúng là ranh giới giữa **lưu trữ** và **bộ
/// nhớ** — trộn chúng lại là cách `§22.12` bị vi phạm:
///
/// - `deltas` là **lưu trữ**: chỉ chứa chunk *đã thay đổi*, được ghi vào save.
/// - `resident` là **bộ nhớ**: chunk đang tải, **không bao giờ** được ghi.
#[derive(Debug, Default)]
pub struct ChunkStore {
    deltas: BTreeMap<ChunkPos, ChunkDelta>,
    resident: BTreeMap<ChunkPos, Lod>,
    chunk_size: i64,
}

impl ChunkStore {
    /// Kho rỗng với cạnh chunk cho trước.
    pub fn new(chunk_size: i64) -> ChunkStore {
        assert!(chunk_size > 0, "cạnh chunk phải dương");
        ChunkStore {
            deltas: BTreeMap::new(),
            resident: BTreeMap::new(),
            chunk_size,
        }
    }

    /// Cạnh chunk.
    pub fn chunk_size(&self) -> i64 {
        self.chunk_size
    }

    /// Số chunk **được ghi vào save**.
    ///
    /// Đây là con số mà `§22.12` nói tới. Nó phải bằng 0 với một thế giới chưa
    /// ai đụng vào, bất kể người chơi đã đi qua bao nhiêu chunk.
    pub fn stored_chunks(&self) -> usize {
        self.deltas.len()
    }

    /// Số chunk đang nằm trong bộ nhớ.
    pub fn resident_chunks(&self) -> usize {
        self.resident.len()
    }

    /// Tải một chunk ở mức chi tiết cho trước. Chỉ đụng vào bộ nhớ.
    pub fn load(&mut self, pos: ChunkPos, lod: Lod) {
        self.resident.insert(pos, lod);
    }

    /// Bỏ tải. Delta **không** mất — nó là lưu trữ, không phải bộ nhớ.
    pub fn unload(&mut self, pos: ChunkPos) {
        self.resident.remove(&pos);
    }

    /// Mức chi tiết hiện tại của một chunk, `None` nếu chưa tải.
    pub fn lod_of(&self, pos: ChunkPos) -> Option<Lod> {
        self.resident.get(&pos).copied()
    }

    /// Ghi đè một ô. Đây là **cách duy nhất** một chunk lọt vào save.
    pub fn write_cell(&mut self, at: WorldPos, key: &str, v: Value) -> Result<(), String> {
        let pos = at.chunk_of(self.chunk_size).map_err(|e| e.to_string())?;
        let (lx, ly) = at
            .local_in_chunk(self.chunk_size)
            .map_err(|e| e.to_string())?;
        self.deltas
            .entry(pos)
            .or_default()
            .set(lx as u32, ly as u32, key, v);
        Ok(())
    }

    /// Đọc ghi đè của một ô, `None` nghĩa là "dùng ô nguyên thủy".
    pub fn read_cell(&self, at: WorldPos, key: &str) -> Option<&Value> {
        let pos = at.chunk_of(self.chunk_size).ok()?;
        let (lx, ly) = at.local_in_chunk(self.chunk_size).ok()?;
        self.deltas.get(&pos)?.get(lx as u32, ly as u32, key)
    }

    /// Hoàn nguyên một ô về đúng thứ worldgen sinh ra.
    pub fn revert_cell(&mut self, at: WorldPos, key: &str) -> Result<(), String> {
        let pos = at.chunk_of(self.chunk_size).map_err(|e| e.to_string())?;
        let (lx, ly) = at
            .local_in_chunk(self.chunk_size)
            .map_err(|e| e.to_string())?;
        if let Some(d) = self.deltas.get_mut(&pos) {
            d.clear(lx as u32, ly as u32, key);
            // Chunk không còn delta nào thì biến mất khỏi save hoàn toàn.
            if d.is_empty() {
                self.deltas.remove(&pos);
            }
        }
        Ok(())
    }

    /// Delta của một chunk.
    pub fn delta(&self, pos: ChunkPos) -> Option<&ChunkDelta> {
        self.deltas.get(&pos)
    }

    /// Mọi chunk có delta, theo thứ tự vị trí.
    pub fn stored(&self) -> impl Iterator<Item = (&ChunkPos, &ChunkDelta)> {
        self.deltas.iter()
    }

    /// Hash của phần lưu trữ.
    ///
    /// **Chỉ** delta, không có `resident`. Nếu trạng thái tải lọt vào hash thì
    /// hai người chơi cùng một save nhưng đứng ở hai chỗ sẽ có hai hash khác
    /// nhau, và determinism harness sẽ báo lệch ở mọi lần chạy.
    pub fn storage_hash(&self) -> StateHash {
        let mut h = StateHasher::with_domain("mow.spatial.v1");
        h.write_i64(self.chunk_size);
        h.write_seq(self.deltas.iter(), |hh, (pos, d)| {
            pos.canonical_hash(hh);
            d.canonical_hash(hh);
        });
        h.finish()
    }
}
