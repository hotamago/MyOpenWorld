//! Bus bền trên SQLite.
//!
//! Một bảng, ba trạng thái. Không có gì thông minh ở đây, và đó là tính năng.

use crate::{BusError, BusResult, Message, MessageBus};
use rusqlite::{params, Connection};
use std::path::Path;

/// Trạng thái của một thông điệp.
const READY: i64 = 0;
const LEASED: i64 = 1;
const DONE: i64 = 2;

const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS bus_message (
    seq            INTEGER PRIMARY KEY AUTOINCREMENT,
    subject        TEXT    NOT NULL,
    payload        BLOB    NOT NULL,
    state          INTEGER NOT NULL DEFAULT 0,
    delivery_count INTEGER NOT NULL DEFAULT 0
) STRICT;

-- Truy vấn nóng nhất là 'lấy N cái sẵn sàng của chủ đề này, theo thứ tự'.
CREATE INDEX IF NOT EXISTS bus_ready ON bus_message (subject, state, seq);
";

/// Bus trên SQLite.
pub struct SqliteBus {
    conn: Connection,
}

impl SqliteBus {
    /// Mở hoặc tạo file bus.
    ///
    /// File **riêng**, không phải file save. Bus ghi liên tục còn save thì
    /// không; để chung sẽ làm hai thứ tranh khóa nhau và làm chậm cả hai.
    pub fn open(path: impl AsRef<Path>) -> BusResult<SqliteBus> {
        SqliteBus::setup(Connection::open(path)?)
    }

    /// Bus trong bộ nhớ, cho test.
    pub fn in_memory() -> BusResult<SqliteBus> {
        SqliteBus::setup(Connection::open_in_memory()?)
    }

    fn setup(conn: Connection) -> BusResult<SqliteBus> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // `FULL` chứ không phải `NORMAL`: cả crate này tồn tại để hứa rằng khi
        // `publish` trả `Ok` thì dữ liệu đã ở trên đĩa. `NORMAL` cho phép mất
        // vài giao dịch cuối khi máy mất điện, và như thế lời hứa thành sai.
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.execute_batch(SCHEMA)?;
        Ok(SqliteBus { conn })
    }
}

impl MessageBus for SqliteBus {
    fn publish(&mut self, subject: &str, payload: &[u8]) -> BusResult<u64> {
        self.conn.execute(
            "INSERT INTO bus_message (subject, payload, state) VALUES (?1, ?2, ?3)",
            params![subject, payload, READY],
        )?;
        Ok(self.conn.last_insert_rowid() as u64)
    }

    fn fetch(&mut self, subject: &str, max: usize) -> BusResult<Vec<Message>> {
        let tx = self.conn.transaction()?;
        let seqs: Vec<i64> = {
            let mut stmt = tx.prepare(
                "SELECT seq FROM bus_message
                  WHERE subject = ?1 AND state = ?2
                  ORDER BY seq LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![subject, READY, max as i64], |r| r.get(0))?;
            rows.collect::<Result<_, _>>()?
        };

        let mut ra = Vec::with_capacity(seqs.len());
        for seq in seqs {
            tx.execute(
                "UPDATE bus_message
                    SET state = ?1, delivery_count = delivery_count + 1
                  WHERE seq = ?2",
                params![LEASED, seq],
            )?;
            let (payload, dc): (Vec<u8>, i64) = tx.query_row(
                "SELECT payload, delivery_count FROM bus_message WHERE seq = ?1",
                params![seq],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            ra.push(Message {
                seq: seq as u64,
                subject: subject.to_owned(),
                payload,
                delivery_count: dc as u32,
            });
        }
        tx.commit()?;
        Ok(ra)
    }

    fn ack(&mut self, seq: u64) -> BusResult<()> {
        let n = self.conn.execute(
            "UPDATE bus_message SET state = ?1 WHERE seq = ?2 AND state = ?3",
            params![DONE, seq as i64, LEASED],
        )?;
        if n == 0 {
            // Ack một thứ không đang được giữ là lỗi lập trình, không phải
            // chuyện bình thường. Nuốt nó đi sẽ giấu mất một consumer đang ack
            // hai lần, và consumer đó đang xử lý mọi thứ hai lần.
            return Err(BusError::NotLeased(seq));
        }
        Ok(())
    }

    fn nack(&mut self, seq: u64) -> BusResult<()> {
        let n = self.conn.execute(
            "UPDATE bus_message SET state = ?1 WHERE seq = ?2 AND state = ?3",
            params![READY, seq as i64, LEASED],
        )?;
        if n == 0 {
            return Err(BusError::NotLeased(seq));
        }
        Ok(())
    }

    fn recover(&mut self) -> BusResult<usize> {
        let n = self.conn.execute(
            "UPDATE bus_message SET state = ?1 WHERE state = ?2",
            params![READY, LEASED],
        )?;
        Ok(n)
    }

    fn pending(&self, subject: &str) -> BusResult<usize> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bus_message WHERE subject = ?1 AND state != ?2",
            params![subject, DONE],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }
}
