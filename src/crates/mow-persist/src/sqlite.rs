//! Hiện thực `SQLite` — hiện thực **duy nhất** cho tới `PC-20`.
//!
//! Đây là backend của bản desktop (`§P3.3`): một file, không tiến trình phụ,
//! không cấu hình. Nó cũng là backend của mọi bài test, vì một bài test tạo
//! world trong bộ nhớ và chạy 1000 tick không nên cần một container.
//!
//! Lược đồ ở đây theo đúng `plan.md §P6.6`, kể cả những chỗ khó chịu:
//! **không có cột `REAL`**. `SQLite` sẽ vui vẻ nhận số thực, và đó chính là vấn
//! đề — một cột `REAL` trên đường commit phá cả determinism lẫn tính nhất quán
//! giữa hai backend, và nó sẽ không báo lỗi cho tới khi hai máy khác nhau cho
//! hai kết quả khác nhau.

use crate::error::{PersistError, PersistResult};
use crate::store::{BranchRecord, EventRecord, Snapshot, Store};
use mow_core::{BranchId, EventSeq, Tick, WorldId};
use mow_math::StateHash;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// Kho trên `SQLite`.
pub struct SqliteStore {
    conn: Connection,
}

/// Lược đồ. Giữ nguyên văn ở một chỗ để `PC-20` đối chiếu với bản Postgres.
const SCHEMA: &str = r"
-- Chỉ ghi thêm. KHÔNG có UPDATE và KHÔNG có DELETE trên bảng này.
-- Trigger bên dưới biến điều đó từ quy ước thành ràng buộc.
CREATE TABLE IF NOT EXISTS event (
    branch      INTEGER NOT NULL,
    seq         INTEGER NOT NULL,
    world       INTEGER NOT NULL,
    tick        INTEGER NOT NULL,
    kind        TEXT    NOT NULL,
    actor       INTEGER NOT NULL DEFAULT 0,
    subject     INTEGER NOT NULL DEFAULT 0,
    payload     BLOB    NOT NULL,
    cause       INTEGER,
    law_version INTEGER,
    norm_set_version INTEGER,
    PRIMARY KEY (branch, seq)
) STRICT;

CREATE INDEX IF NOT EXISTS event_by_tick  ON event (branch, tick);
CREATE INDEX IF NOT EXISTS event_by_actor ON event (branch, actor, seq);

CREATE TRIGGER IF NOT EXISTS event_khong_sua
BEFORE UPDATE ON event
BEGIN
    SELECT RAISE(ABORT, 'nhat ky su kien chi ghi them: khong duoc UPDATE');
END;

CREATE TRIGGER IF NOT EXISTS event_khong_xoa
BEFORE DELETE ON event
BEGIN
    SELECT RAISE(ABORT, 'nhat ky su kien chi ghi them: khong duoc DELETE');
END;

CREATE TABLE IF NOT EXISTS snapshot (
    branch      INTEGER NOT NULL,
    tick        INTEGER NOT NULL,
    world       INTEGER NOT NULL,
    event_count INTEGER NOT NULL,
    state_hash  BLOB    NOT NULL,
    blob        BLOB    NOT NULL,
    PRIMARY KEY (branch, tick)
) STRICT;

CREATE TABLE IF NOT EXISTS branch (
    id         INTEGER PRIMARY KEY,
    parent     INTEGER,
    fork_tick  INTEGER NOT NULL,
    label      TEXT    NOT NULL,
    FOREIGN KEY (parent) REFERENCES branch(id)
) STRICT;
";

impl SqliteStore {
    /// Mở hoặc tạo một file save.
    pub fn open(path: impl AsRef<Path>) -> PersistResult<SqliteStore> {
        let conn = Connection::open(path)?;
        SqliteStore::setup(conn)
    }

    /// Kho trong bộ nhớ, cho test và cho chế độ thử nghiệm.
    pub fn in_memory() -> PersistResult<SqliteStore> {
        SqliteStore::setup(Connection::open_in_memory()?)
    }

    fn setup(conn: Connection) -> PersistResult<SqliteStore> {
        // WAL cho phép đọc song song lúc đang ghi — cần thiết vì giao diện đọc
        // timeline trong khi mô phỏng vẫn đang chạy.
        //
        // `foreign_keys` phải bật tường minh: SQLite tắt nó mặc định vì lý do
        // tương thích ngược, và một khóa ngoại không được thi hành thì tệ hơn
        // không có, vì lược đồ nói dối về thứ nó bảo đảm.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(SCHEMA)?;
        Ok(SqliteStore { conn })
    }

    /// Kiểm tra không có cột số thực nào trên đường commit.
    ///
    /// `plan.md §P10.2.1` cấm điều này, nhưng lệnh cấm chỉ có giá trị khi có
    /// thứ gì đó kiểm tra. Hàm này chạy trong bộ test hợp đồng, nên **mọi**
    /// backend đều phải vượt qua nó, không chỉ `SQLite`.
    pub fn kiem_tra_khong_co_cot_thuc(&self) -> PersistResult<Vec<String>> {
        let mut vi_pham = Vec::new();
        let bang = ["event", "snapshot", "branch"];
        for t in bang {
            let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({t})"))?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
            for row in rows {
                let (cot, kieu) = row?;
                let k = kieu.to_uppercase();
                if k.contains("REAL") || k.contains("DOUBLE") || k.contains("FLOAT") {
                    vi_pham.push(format!("{t}.{cot} kiểu {kieu}"));
                }
            }
        }
        Ok(vi_pham)
    }
}

fn to_hash(bytes: Vec<u8>) -> PersistResult<StateHash> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| PersistError::Corrupt("state_hash không đủ 32 byte".into()))?;
    Ok(StateHash(arr))
}

impl Store for SqliteStore {
    fn append_events(&mut self, events: &[EventRecord]) -> PersistResult<()> {
        if events.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO event
                   (branch, seq, world, tick, kind, actor, subject, payload, cause, law_version,
                    norm_set_version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for e in events {
                stmt.execute(params![
                    e.branch.get() as i64,
                    e.seq.0 as i64,
                    e.world.get() as i64,
                    e.tick.0 as i64,
                    e.kind,
                    e.actor as i64,
                    e.subject as i64,
                    e.payload,
                    e.cause.map(|c| c.0 as i64),
                    e.law_version.map(i64::from),
                    e.norm_set_version.map(i64::from),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn read_events(
        &self,
        branch: BranchId,
        from: EventSeq,
        to: EventSeq,
    ) -> PersistResult<Vec<EventRecord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT seq, world, tick, kind, actor, subject, payload, cause, law_version,
                    norm_set_version
               FROM event
              WHERE branch = ?1 AND seq >= ?2 AND seq < ?3
              ORDER BY seq",
        )?;
        let rows = stmt.query_map(
            params![branch.get() as i64, from.0 as i64, to.0 as i64],
            |r| {
                Ok(EventRecord {
                    seq: EventSeq(r.get::<_, i64>(0)? as u64),
                    branch,
                    world: WorldId(r.get::<_, i64>(1)? as u64),
                    tick: Tick(r.get::<_, i64>(2)? as u64),
                    kind: r.get(3)?,
                    actor: r.get::<_, i64>(4)? as u64,
                    subject: r.get::<_, i64>(5)? as u64,
                    payload: r.get(6)?,
                    cause: r.get::<_, Option<i64>>(7)?.map(|v| EventSeq(v as u64)),
                    law_version: r.get::<_, Option<i64>>(8)?.map(|v| v as u32),
                    norm_set_version: r.get::<_, Option<i64>>(9)?.map(|v| v as u32),
                })
            },
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn next_seq(&self, branch: BranchId) -> PersistResult<EventSeq> {
        let n: Option<i64> = self.conn.query_row(
            "SELECT MAX(seq) FROM event WHERE branch = ?1",
            params![branch.get() as i64],
            |r| r.get(0),
        )?;
        Ok(EventSeq(n.map_or(0, |v| v as u64 + 1)))
    }

    fn put_snapshot(&mut self, snap: &Snapshot) -> PersistResult<()> {
        // `INSERT OR REPLACE` chứ không phải `INSERT`: chụp lại cùng một tick
        // là chuyện bình thường (chạy lại từ ảnh cũ), và ảnh chụp không phải
        // nguồn sự thật nên ghi đè nó không mất gì.
        self.conn.execute(
            "INSERT OR REPLACE INTO snapshot
               (branch, tick, world, event_count, state_hash, blob)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snap.branch.get() as i64,
                snap.tick.0 as i64,
                snap.world.get() as i64,
                snap.event_count as i64,
                &snap.state_hash.0[..],
                snap.blob,
            ],
        )?;
        Ok(())
    }

    fn latest_snapshot(&self, branch: BranchId, tick: Tick) -> PersistResult<Option<Snapshot>> {
        self.conn
            .query_row(
                "SELECT tick, world, event_count, state_hash, blob
                   FROM snapshot
                  WHERE branch = ?1 AND tick <= ?2
                  ORDER BY tick DESC LIMIT 1",
                params![branch.get() as i64, tick.0 as i64],
                |r| {
                    Ok((
                        Tick(r.get::<_, i64>(0)? as u64),
                        WorldId(r.get::<_, i64>(1)? as u64),
                        r.get::<_, i64>(2)? as u64,
                        r.get::<_, Vec<u8>>(3)?,
                        r.get::<_, Vec<u8>>(4)?,
                    ))
                },
            )
            .optional()?
            .map(|(tick, world, event_count, hash, blob)| {
                Ok(Snapshot {
                    branch,
                    world,
                    tick,
                    event_count,
                    state_hash: to_hash(hash)?,
                    blob,
                })
            })
            .transpose()
    }

    fn create_branch(&mut self, rec: &BranchRecord) -> PersistResult<()> {
        self.conn.execute(
            "INSERT INTO branch (id, parent, fork_tick, label) VALUES (?1, ?2, ?3, ?4)",
            params![
                rec.id.get() as i64,
                rec.parent.map(|p| p.get() as i64),
                rec.fork_tick.0 as i64,
                rec.label,
            ],
        )?;
        Ok(())
    }

    fn get_branch(&self, id: BranchId) -> PersistResult<Option<BranchRecord>> {
        self.conn
            .query_row(
                "SELECT parent, fork_tick, label FROM branch WHERE id = ?1",
                params![id.get() as i64],
                |r| {
                    Ok(BranchRecord {
                        id,
                        parent: r.get::<_, Option<i64>>(0)?.map(|v| BranchId(v as u64)),
                        fork_tick: Tick(r.get::<_, i64>(1)? as u64),
                        label: r.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn flush(&mut self) -> PersistResult<()> {
        self.conn
            .pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
        Ok(())
    }
}
