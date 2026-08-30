//! Chỉ mục nhúng trên SQLite — hiện thực duy nhất cho tới `PC-20` (Qdrant).
//!
//! Quét tuyến tính, không có HNSW. Với vài chục nghìn ký ức của một bản
//! single-player thì quét tuyến tính trên số nguyên nhanh hơn người ta tưởng,
//! và nó có một tính chất mà chỉ mục xấp xỉ không có: **kết quả chính xác**.
//! HNSW là xấp xỉ, và một xấp xỉ có thể trả kết quả khác nhau giữa hai lần dựng
//! chỉ mục — tức là phá replay. Khi Qdrant vào ở `PC-20`, đây là ràng buộc phải
//! giải quyết tường minh, không phải bật lên rồi hy vọng.
//!
//! File **riêng**, không nằm trong file save (`P0-09`). Tiến trình mô phỏng ghi
//! save liên tục; nếu chỉ mục nằm chung file thì hai bên tranh khóa nhau, và
//! `PC-06` "xóa sạch chỉ mục rồi rebuild" sẽ phải đụng vào file save của người
//! chơi — điều không bao giờ nên xảy ra.

use crate::{dot, Hit, MemoryId, MemoryPoint, Query, VectorError, VectorIndex, VectorResult};
use mow_core::{BranchId, Tick};
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use std::path::Path;

const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS point (
    id              INTEGER PRIMARY KEY,
    namespace       TEXT    NOT NULL,
    persona_version INTEGER NOT NULL,
    created_branch  INTEGER NOT NULL,
    created_tick    INTEGER NOT NULL,
    vector          BLOB    NOT NULL,
    payload         BLOB    NOT NULL
) STRICT;

CREATE INDEX IF NOT EXISTS point_by_ns ON point (namespace, created_branch, created_tick);

-- Bia mo: mot ky uc bi quen tren MOT nhanh, khong phai moi nhanh.
CREATE TABLE IF NOT EXISTS tombstone (
    id     INTEGER NOT NULL,
    branch INTEGER NOT NULL,
    PRIMARY KEY (id, branch)
) STRICT;
";

/// Chỉ mục nhúng.
pub struct EmbeddedIndex {
    conn: Connection,
    dim: usize,
}

impl EmbeddedIndex {
    /// Mở hoặc tạo file chỉ mục.
    pub fn open(path: impl AsRef<Path>, dim: usize) -> VectorResult<EmbeddedIndex> {
        EmbeddedIndex::setup(Connection::open(path)?, dim)
    }

    /// Chỉ mục trong bộ nhớ, cho test.
    pub fn in_memory(dim: usize) -> VectorResult<EmbeddedIndex> {
        EmbeddedIndex::setup(Connection::open_in_memory()?, dim)
    }

    fn setup(conn: Connection, dim: usize) -> VectorResult<EmbeddedIndex> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(SCHEMA)?;
        Ok(EmbeddedIndex { conn, dim })
    }

    fn ma_hoa(v: &[i16]) -> Vec<u8> {
        // Little-endian tường minh: chỉ mục có thể được chép giữa các máy, và
        // endianness của máy không được là một phần của định dạng file.
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn giai_ma(b: &[u8]) -> Vec<i16> {
        b.chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect()
    }
}

impl VectorIndex for EmbeddedIndex {
    fn dimension(&self) -> usize {
        self.dim
    }

    fn upsert(&mut self, p: &MemoryPoint) -> VectorResult<()> {
        if p.vector.len() != self.dim {
            return Err(VectorError::Dimension {
                got: p.vector.len(),
                want: self.dim,
            });
        }
        self.conn.execute(
            "INSERT OR REPLACE INTO point
               (id, namespace, persona_version, created_branch, created_tick, vector, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                p.id.0 as i64,
                p.namespace,
                i64::from(p.persona_version),
                p.created_branch.get() as i64,
                p.created_tick.0 as i64,
                Self::ma_hoa(&p.vector),
                p.payload,
            ],
        )?;
        Ok(())
    }

    fn tombstone(&mut self, id: MemoryId, branch: BranchId) -> VectorResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tombstone (id, branch) VALUES (?1, ?2)",
            params![id.0 as i64, branch.get() as i64],
        )?;
        Ok(())
    }

    fn search(&self, q: &Query) -> VectorResult<Vec<Hit>> {
        if q.vector.len() != self.dim {
            return Err(VectorError::Dimension {
                got: q.vector.len(),
                want: self.dim,
            });
        }
        // Mặc định an toàn: không namespace nào thì không thấy gì. Xem tài liệu
        // của `Query::namespaces` về lý do.
        if q.namespaces.is_empty() || q.lineage.is_empty() || q.limit == 0 {
            return Ok(Vec::new());
        }

        // Nhánh hiện tại là mắt xích đầu tiên của dòng dõi.
        let hien_tai = q.lineage[0].branch;

        // Mốc cắt theo từng nhánh tổ tiên. Đây là vế thứ ba của điều kiện
        // §P6.3, vế mà lọc phẳng theo `branch_id` không diễn đạt được.
        let cutoff: BTreeMap<u64, Tick> = q
            .lineage
            .iter()
            .map(|s| (s.branch.get(), s.cutoff))
            .collect();

        let bia_mo: std::collections::BTreeSet<u64> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM tombstone WHERE branch = ?1")?;
            let rows = stmt.query_map(params![hien_tai.get() as i64], |r| r.get::<_, i64>(0))?;
            rows.map(|r| r.map(|v| v as u64))
                .collect::<Result<_, _>>()?
        };

        let mut stmt = self.conn.prepare(
            "SELECT id, namespace, persona_version, created_branch, created_tick, vector, payload
               FROM point ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(MemoryPoint {
                id: MemoryId(r.get::<_, i64>(0)? as u64),
                namespace: r.get(1)?,
                persona_version: r.get::<_, i64>(2)? as u32,
                created_branch: BranchId(r.get::<_, i64>(3)? as u64),
                created_tick: Tick(r.get::<_, i64>(4)? as u64),
                vector: Self::giai_ma(&r.get::<_, Vec<u8>>(5)?),
                payload: r.get(6)?,
            })
        })?;

        let mut hits: Vec<Hit> = Vec::new();
        for row in rows {
            let p = row?;
            if !q.namespaces.iter().any(|n| *n == p.namespace) {
                continue;
            }
            if bia_mo.contains(&p.id.0) {
                continue;
            }
            let Some(&cut) = cutoff.get(&p.created_branch.get()) else {
                continue; // không thuộc dòng dõi
            };
            if p.created_tick > cut {
                continue; // cha đã tạo ra nó SAU khi ta tách ra
            }
            let score = dot(&q.vector, &p.vector);
            hits.push(Hit { point: p, score });
        }

        // Sắp theo điểm giảm dần, **phá hòa bằng id tăng dần**. Không có vế
        // phá hòa thì hai ký ức cùng điểm sẽ xếp theo thứ tự nào đó của backend,
        // và thứ tự đó đi vào prompt. `sort_by` của Rust ổn định, nhưng dựa vào
        // tính ổn định của thứ tự đầu vào là dựa vào thứ tự chèn — thứ sẽ khác
        // sau một lần rebuild chỉ mục.
        hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.point.id.cmp(&b.point.id)));
        hits.truncate(q.limit);
        Ok(hits)
    }

    fn clear(&mut self) -> VectorResult<()> {
        self.conn
            .execute_batch("DELETE FROM point; DELETE FROM tombstone;")?;
        Ok(())
    }

    fn len(&self) -> VectorResult<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM point", [], |r| r.get(0))?;
        Ok(n as usize)
    }
}
