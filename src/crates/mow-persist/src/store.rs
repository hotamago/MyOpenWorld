//! Trait lưu trữ và các kiểu bản ghi.

use crate::error::PersistResult;
use mow_core::{BranchId, EventSeq, Tick, WorldId};
use mow_math::StateHash;
use serde::{Deserialize, Serialize};

/// Một sự kiện đã tuần tự hóa, ở dạng lưu trữ.
///
/// Cố tình **không** dùng thẳng [`mow_core::Event`]: tầng lưu trữ không được
/// biết cấu trúc payload, vì payload sẽ thay đổi theo content pack còn schema
/// bảng thì không được đổi theo. `payload` là byte đục.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    /// Thứ tự trong nhánh.
    pub seq: EventSeq,
    /// Nhánh.
    pub branch: BranchId,
    /// Thế giới.
    pub world: WorldId,
    /// Tick địa phương.
    pub tick: Tick,
    /// Loại, có namespace.
    pub kind: String,
    /// Chủ thể, `0` là không có.
    pub actor: u64,
    /// Đối tượng, `0` là không có.
    pub subject: u64,
    /// Nội dung đã tuần tự hóa.
    pub payload: Vec<u8>,
    /// Sự kiện nguyên nhân, nếu có.
    pub cause: Option<EventSeq>,
    /// Phiên bản luật lúc xảy ra.
    pub law_version: Option<u32>,
    /// Phiên bản bộ chuẩn mực lúc xảy ra (`§18.10`, `§22.49`).
    pub norm_set_version: Option<u32>,
}

/// Ảnh chụp state tại một tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Nhánh.
    pub branch: BranchId,
    /// Thế giới.
    pub world: WorldId,
    /// Tick đã chụp.
    pub tick: Tick,
    /// Số sự kiện đã áp tính tới ảnh này.
    pub event_count: u64,
    /// Hash state tại đúng thời điểm này. Đây là thứ harness so sánh.
    pub state_hash: StateHash,
    /// Dữ liệu đã tuần tự hóa.
    pub blob: Vec<u8>,
}

/// Bản ghi một nhánh lịch sử (`§4.4`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchRecord {
    /// Định danh.
    pub id: BranchId,
    /// Nhánh cha. `None` chỉ với nhánh gốc.
    pub parent: Option<BranchId>,
    /// Tick trên nhánh cha mà nhánh này tách ra.
    ///
    /// Đây là trường mà mọi truy vấn theo dòng dõi cần (`§P6.3`): nhánh con
    /// thấy dữ liệu của cha **tới điểm này**, không thấy sau đó. Lọc phẳng
    /// theo `branch_id` không diễn đạt được điều đó.
    pub fork_tick: Tick,
    /// Nhãn người đọc được.
    pub label: String,
}

/// Kho lưu trữ bền.
///
/// Bốn nhóm phép, và ranh giới giữa chúng là ranh giới của những thứ có thể
/// hỏng độc lập:
///
/// - **Sự kiện** — chỉ ghi thêm. Không có `update`, không có `delete`. Đây là
///   nguồn sự thật; mọi thứ khác dựng lại được từ đây.
/// - **Ảnh chụp** — tăng tốc, không phải sự thật. Xóa hết ảnh chụp thì thế giới
///   vẫn dựng lại được, chỉ chậm hơn.
/// - **Nhánh** — cấu trúc DAG của lịch sử.
/// - **Bảo trì** — flush, kiểm tra toàn vẹn.
pub trait Store: Send + 'static {
    // ── Sự kiện ─────────────────────────────────────────────────────────────

    /// Ghi thêm một loạt sự kiện, **nguyên tử**.
    ///
    /// Nguyên tử theo lô chứ không theo từng cái: một giao dịch sinh nhiều sự
    /// kiện, và một nửa số đó nằm trong nhật ký còn nửa kia thì không là trạng
    /// thái không có ý nghĩa nào cả.
    fn append_events(&mut self, events: &[EventRecord]) -> PersistResult<()>;

    /// Đọc sự kiện của một nhánh trong khoảng `[from, to)`.
    fn read_events(
        &self,
        branch: BranchId,
        from: EventSeq,
        to: EventSeq,
    ) -> PersistResult<Vec<EventRecord>>;

    /// Số thứ tự tiếp theo của một nhánh.
    fn next_seq(&self, branch: BranchId) -> PersistResult<EventSeq>;

    // ── Ảnh chụp ────────────────────────────────────────────────────────────

    /// Lưu một ảnh chụp.
    fn put_snapshot(&mut self, snap: &Snapshot) -> PersistResult<()>;

    /// Ảnh chụp gần nhất **không vượt quá** `tick`.
    fn latest_snapshot(&self, branch: BranchId, tick: Tick) -> PersistResult<Option<Snapshot>>;

    // ── Nhánh ───────────────────────────────────────────────────────────────

    /// Tạo một nhánh.
    fn create_branch(&mut self, rec: &BranchRecord) -> PersistResult<()>;

    /// Đọc một nhánh.
    fn get_branch(&self, id: BranchId) -> PersistResult<Option<BranchRecord>>;

    /// Dòng dõi của một nhánh, từ chính nó ngược về gốc.
    ///
    /// Có sẵn ở tầng này thay vì để mỗi chỗ gọi tự lần ngược, vì **mọi** truy
    /// vấn ký ức và belief đều cần nó (`§P6.3`), và một chỗ quên lọc theo dòng
    /// dõi là một chỗ nhánh con đọc được ký ức mà nó đã quên.
    fn ancestry(&self, id: BranchId) -> PersistResult<Vec<BranchRecord>> {
        let mut ra = Vec::new();
        let mut cur = Some(id);
        let mut da_tham = std::collections::BTreeSet::new();
        while let Some(b) = cur {
            if !da_tham.insert(b) {
                break; // DAG hỏng; thà cắt còn hơn treo.
            }
            let Some(rec) = self.get_branch(b)? else {
                break;
            };
            cur = rec.parent;
            ra.push(rec);
        }
        Ok(ra)
    }

    // ── Bảo trì ─────────────────────────────────────────────────────────────

    /// Đẩy mọi thứ xuống đĩa.
    fn flush(&mut self) -> PersistResult<()>;
}
