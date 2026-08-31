//! Hiện thực thứ hai của [`Store`]: PostgreSQL (`plan.md §P3.4`, `PC-20`).
//!
//! ## Vì sao backend này tới ở Giai đoạn C chứ không phải Giai đoạn 0
//!
//! `§P3.4` nói thẳng lý do:
//!
//! > Viết thẳng vào Qdrant hay NATS rồi mới bọc lại là công việc gấp nhiều lần
//! > — nhưng **duy trì hai hiện thực trước khi có workload thật cũng vậy**, và
//! > tệ hơn: interface bị thiết kế dựa trên phỏng đoán.
//!
//! Nên trait được vẽ ở Giai đoạn 0 với đúng **một** hiện thực, rồi bị một
//! workload thật uốn nắn suốt hai giai đoạn, và chỉ tới đây mới có cái thứ hai.
//! Bộ test hợp đồng ở [`crate::contract`] được dùng **nguyên vẹn** — không sửa
//! một dòng — và đó chính là bằng chứng hai backend tương đương.
//!
//! ## Chỗ mà hai backend dễ lệch nhau nhất
//!
//! Không phải SQL. Là **kiểu số nguyên**. SQLite coi mọi số nguyên là `i64` và
//! không phàn nàn; Postgres phân biệt `INTEGER`/`BIGINT` và sẽ từ chối. Cả
//! `branch`, `seq`, `world`, `tick` ở đây đều là `u64` phía Rust, nên chúng
//! xuống `BIGINT` và đi qua `i64` — cùng cách ép mà `sqlite.rs` đang dùng.
//!
//! Cách hỏng nếu làm khác: một thế giới chạy đủ lâu để `tick` vượt `i32::MAX`
//! sẽ ghi hỏng, và nó xảy ra sau vài trăm giờ chứ không phải trong test.
//!
//! ## Chạy test hợp đồng
//!
//! ```bash
//! ./mow infra up          # dựng Postgres trong deploy/
//! MOW_POSTGRES_URL=postgres://mow:mow@localhost:5432/mow \
//!   cargo test -p mow-persist --features postgres -- --ignored
//! ```
//!
//! Không có biến môi trường thì test tự bỏ qua. Đó là chủ đích: một test cần
//! dịch vụ ngoài mà **fail** khi không có dịch vụ sẽ khiến `cargo test` đỏ trên
//! máy mọi người, và một bộ test đỏ thường xuyên là một bộ test không ai đọc.

use crate::error::{PersistError, PersistResult};
use crate::store::{BranchRecord, EventRecord, Snapshot, Store};
use mow_core::{BranchId, EventSeq, Tick, WorldId};
use mow_math::StateHash;
use postgres::{Client, NoTls};
use std::cell::RefCell;

/// Lược đồ. Cố ý **song song từng cột** với `sqlite.rs`.
///
/// Giữ hai lược đồ giống nhau tới mức có thể là cách rẻ nhất để một bug chỉ
/// xuất hiện ở một backend trở nên hiếm. Chỗ khác nhau duy nhất là kiểu, và
/// chúng được ghi chú ở đây chứ không để người đọc tự đối chiếu.
const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS event (
    branch           BIGINT  NOT NULL,
    seq              BIGINT  NOT NULL,
    world            BIGINT  NOT NULL,
    tick             BIGINT  NOT NULL,
    kind             TEXT    NOT NULL,
    actor            BIGINT  NOT NULL,
    subject          BIGINT  NOT NULL,
    -- `BYTEA` chứ không phải `TEXT`: payload là byte đục (`§P6.6`), và ép nó
    -- qua một kiểu văn bản sẽ làm hỏng những byte không hợp lệ UTF-8 một cách
    -- im lặng.
    payload          BYTEA   NOT NULL,
    cause            BIGINT,
    law_version      INTEGER,
    norm_set_version INTEGER,
    PRIMARY KEY (branch, seq)
);

CREATE TABLE IF NOT EXISTS snapshot (
    branch      BIGINT NOT NULL,
    world       BIGINT NOT NULL,
    tick        BIGINT NOT NULL,
    event_count BIGINT NOT NULL,
    state_hash  BYTEA  NOT NULL,
    blob        BYTEA  NOT NULL,
    PRIMARY KEY (branch, tick)
);

CREATE TABLE IF NOT EXISTS branch (
    id         BIGINT PRIMARY KEY,
    parent     BIGINT,
    fork_tick  BIGINT NOT NULL,
    label      TEXT   NOT NULL
);
";

/// Kho trên `PostgreSQL`.
///
/// ## Vì sao `RefCell`
///
/// `postgres::Client` đòi `&mut self` cho **cả** truy vấn đọc, còn [`Store`] thì
/// khai `&self` cho đọc. Hai lựa chọn:
///
/// 1. Đổi trait thành `&mut self` khắp nơi. Như vậy là bắt `SQLite` — backend mặc
///    định, dùng ở mọi chỗ — phải mượn khả biến cho một thao tác thuần đọc, chỉ
///    vì một backend thứ hai. Đuôi vẫy con chó.
/// 2. `RefCell` ở đây.
///
/// Chọn (2), và nó **an toàn**: [`Store`] chỉ đòi `Send`, không đòi `Sync`, nên
/// `RefCell` hợp lệ. Không có `unsafe` nào trong file này, và không nên có: một
/// khối `unsafe` để lách một mượn kiểu này là chỗ mà lỗi aliasing sẽ nằm im
/// hàng tháng rồi hiện ra dưới dạng dữ liệu hỏng.
pub struct PostgresStore {
    client: RefCell<Client>,
}

impl PostgresStore {
    /// Kết nối và dựng lược đồ nếu chưa có.
    pub fn connect(url: &str) -> PersistResult<PostgresStore> {
        let mut client = Client::connect(url, NoTls).map_err(|e| loi(&e))?;
        client.batch_execute(SCHEMA).map_err(|e| loi(&e))?;
        Ok(PostgresStore {
            client: RefCell::new(client),
        })
    }

    /// Xóa sạch — chỉ dùng cho test hợp đồng, vốn đòi một kho **rỗng, độc lập**.
    pub fn truncate_all(&mut self) -> PersistResult<()> {
        self.client
            .borrow_mut()
            .batch_execute("TRUNCATE event, snapshot, branch")
            .map_err(|e| loi(&e))
    }
}

fn loi(e: &postgres::Error) -> PersistError {
    PersistError::External(e.to_string())
}

impl Store for PostgresStore {
    fn append_events(&mut self, events: &[EventRecord]) -> PersistResult<()> {
        if events.is_empty() {
            return Ok(());
        }
        // Nguyên tử **theo lô**: một giao dịch sinh nhiều sự kiện, và một nửa
        // nằm trong nhật ký còn nửa kia thì không là trạng thái vô nghĩa.
        let mut c = self.client.borrow_mut();
        let mut tx = c.transaction().map_err(|e| loi(&e))?;
        for e in events {
            tx.execute(
                "INSERT INTO event
                   (branch, seq, world, tick, kind, actor, subject, payload, cause,
                    law_version, norm_set_version)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
                &[
                    &(e.branch.get() as i64),
                    &(e.seq.0 as i64),
                    &(e.world.get() as i64),
                    &(e.tick.0 as i64),
                    &e.kind,
                    &(e.actor as i64),
                    &(e.subject as i64),
                    &e.payload,
                    &e.cause.map(|c| c.0 as i64),
                    &e.law_version.map(|v| v as i32),
                    &e.norm_set_version.map(|v| v as i32),
                ],
            )
            .map_err(|e| loi(&e))?;
        }
        tx.commit().map_err(|e| loi(&e))
    }

    fn read_events(
        &self,
        branch: BranchId,
        from: EventSeq,
        to: EventSeq,
    ) -> PersistResult<Vec<EventRecord>> {
        // `[from, to)` — nửa mở, đúng như hợp đồng. Một backend hiểu thành
        // `[from, to]` sẽ trả thừa đúng một sự kiện mỗi lần đọc, và lỗi đó biểu
        // hiện thành replay lệch một bước.
        let rows = self
            .client
            .borrow_mut()
            .query(
                "SELECT seq, world, tick, kind, actor, subject, payload, cause,
                        law_version, norm_set_version
                   FROM event
                  WHERE branch = $1 AND seq >= $2 AND seq < $3
                  ORDER BY seq",
                // Cùng lý do với `sqlite.rs`: ép `u64::MAX` sang `i64` cho ra
                // `-1`, và truy vấn trả rỗng thay vì trả tất cả.
                &[
                    &(branch.get() as i64),
                    &i64::try_from(from.0).unwrap_or(i64::MAX),
                    &i64::try_from(to.0).unwrap_or(i64::MAX),
                ],
            )
            .map_err(|e| loi(&e))?;

        rows.into_iter()
            .map(|r| {
                Ok(EventRecord {
                    branch,
                    seq: EventSeq(r.get::<_, i64>(0) as u64),
                    world: WorldId(r.get::<_, i64>(1) as u64),
                    tick: Tick(r.get::<_, i64>(2) as u64),
                    kind: r.get(3),
                    actor: r.get::<_, i64>(4) as u64,
                    subject: r.get::<_, i64>(5) as u64,
                    payload: r.get(6),
                    cause: r.get::<_, Option<i64>>(7).map(|v| EventSeq(v as u64)),
                    law_version: r.get::<_, Option<i32>>(8).map(|v| v as u32),
                    norm_set_version: r.get::<_, Option<i32>>(9).map(|v| v as u32),
                })
            })
            .collect()
    }

    fn next_seq(&self, branch: BranchId) -> PersistResult<EventSeq> {
        let row = self
            .client
            .borrow_mut()
            .query_one(
                "SELECT COALESCE(MAX(seq) + 1, 0) FROM event WHERE branch = $1",
                &[&(branch.get() as i64)],
            )
            .map_err(|e| loi(&e))?;
        Ok(EventSeq(row.get::<_, i64>(0) as u64))
    }

    fn put_snapshot(&mut self, snap: &Snapshot) -> PersistResult<()> {
        // Ghi đè cùng `(branch, tick)`: chụp lại cùng một thời điểm phải thay
        // bản cũ, không đẻ ra hai bản khác nhau cho cùng một lúc.
        self.client
            .borrow_mut()
            .execute(
                "INSERT INTO snapshot (branch, world, tick, event_count, state_hash, blob)
                 VALUES ($1,$2,$3,$4,$5,$6)
                 ON CONFLICT (branch, tick) DO UPDATE
                   SET world = EXCLUDED.world,
                       event_count = EXCLUDED.event_count,
                       state_hash = EXCLUDED.state_hash,
                       blob = EXCLUDED.blob",
                &[
                    &(snap.branch.get() as i64),
                    &(snap.world.get() as i64),
                    &(snap.tick.0 as i64),
                    &(snap.event_count as i64),
                    &snap.state_hash.0.to_vec(),
                    &snap.blob,
                ],
            )
            .map_err(|e| loi(&e))?;
        Ok(())
    }

    fn latest_snapshot(&self, branch: BranchId, tick: Tick) -> PersistResult<Option<Snapshot>> {
        let rows = self
            .client
            .borrow_mut()
            .query(
                "SELECT world, tick, event_count, state_hash, blob
                   FROM snapshot
                  WHERE branch = $1 AND tick <= $2
                  ORDER BY tick DESC LIMIT 1",
                &[&(branch.get() as i64), &(tick.0 as i64)],
            )
            .map_err(|e| loi(&e))?;

        let Some(r) = rows.into_iter().next() else {
            return Ok(None);
        };
        let hash: Vec<u8> = r.get(3);
        Ok(Some(Snapshot {
            branch,
            world: WorldId(r.get::<_, i64>(0) as u64),
            tick: Tick(r.get::<_, i64>(1) as u64),
            event_count: r.get::<_, i64>(2) as u64,
            state_hash: StateHash(
                <[u8; 32]>::try_from(&hash[..])
                    .map_err(|_| PersistError::Corrupt("state_hash không đủ 32 byte".into()))?,
            ),
            blob: r.get(4),
        }))
    }

    fn create_branch(&mut self, rec: &BranchRecord) -> PersistResult<()> {
        self.client
            .borrow_mut()
            .execute(
                "INSERT INTO branch (id, parent, fork_tick, label) VALUES ($1,$2,$3,$4)",
                &[
                    &(rec.id.get() as i64),
                    &rec.parent.map(|p| p.get() as i64),
                    &(rec.fork_tick.0 as i64),
                    &rec.label,
                ],
            )
            .map_err(|e| loi(&e))?;
        Ok(())
    }

    fn flush(&mut self) -> PersistResult<()> {
        // Postgres đã `fsync` theo `synchronous_commit` của chính nó, và mỗi
        // `execute` ở đây là một giao dịch tự đóng. Không có gì đang nằm trong
        // bộ đệm của tiến trình này để mà đẩy xuống.
        //
        // Trả `Ok(())` chứ **không** phải `unimplemented!()`: hợp đồng của
        // `flush` là *"khi hàm này trả về, dữ liệu đã bền"*, và ở đây điều đó
        // đã đúng từ trước khi gọi. Một `todo!()` ở đây sẽ làm bộ test hợp đồng
        // nổ ở một chỗ vốn không có gì sai.
        Ok(())
    }

    fn get_branch(&self, id: BranchId) -> PersistResult<Option<BranchRecord>> {
        let rows = self
            .client
            .borrow_mut()
            .query(
                "SELECT parent, fork_tick, label FROM branch WHERE id = $1",
                &[&(id.get() as i64)],
            )
            .map_err(|e| loi(&e))?;
        let Some(r) = rows.into_iter().next() else {
            return Ok(None);
        };
        Ok(Some(BranchRecord {
            id,
            parent: r.get::<_, Option<i64>>(0).map(|v| BranchId(v as u64)),
            fork_tick: Tick(r.get::<_, i64>(1) as u64),
            label: r.get(2),
        }))
    }
}
