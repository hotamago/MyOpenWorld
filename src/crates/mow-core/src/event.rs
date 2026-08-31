//! Nhật ký sự kiện **chỉ ghi thêm** (`idea.md §8.4`, `plan.md §P6.6`).
//!
//! Bảng `event` không có `UPDATE` và không có `DELETE`. Đó không phải sở thích
//! kiến trúc mà là điều kiện để ba thứ khác tồn tại: replay bit-perfect, chuỗi
//! nhân quả truy ngược được (`§18.10`), và biên niên sử hai lớp phân biệt
//! "đã xảy ra" với "người ta tin là đã xảy ra" (`§18.11`).
//!
//! Nếu một sự kiện sửa được sau khi ghi thì cả ba thứ đó đều thành lời nói
//! suông. Sửa lịch sử là tạo **nhánh mới** (`§4.4`), không phải ghi đè.

use crate::clock::Tick;
use crate::ids::{BranchId, EntityId, WorldId};
use crate::value::Value;
use mow_math::{CanonicalHash, StateHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Số thứ tự sự kiện trong một nhánh. Đơn điệu tăng, không có lỗ hổng.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct EventSeq(pub u64);

impl CanonicalHash for EventSeq {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.0);
    }
}

/// Loại sự kiện, ví dụ `entity.spawned` hay `item.crafted`.
///
/// Chuỗi có namespace chứ không phải enum, vì content pack của cộng đồng phải
/// định nghĩa được loại sự kiện mới mà không phải sửa engine (`§19.7`). Namespace
/// là bắt buộc: `core.entity.spawned`, `mypack.ritual.completed`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventKind(pub String);

impl EventKind {
    /// Dựng từ chuỗi tĩnh.
    pub fn of(s: &str) -> EventKind {
        EventKind(s.to_owned())
    }

    /// Namespace, tức phần trước dấu chấm đầu tiên.
    pub fn namespace(&self) -> &str {
        self.0.split('.').next().unwrap_or("")
    }
}

impl CanonicalHash for EventKind {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.0);
    }
}

/// Một sự kiện đã xảy ra. Bất biến sau khi ghi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Thứ tự trong nhánh.
    pub seq: EventSeq,
    /// Nhánh lịch sử.
    pub branch: BranchId,
    /// Thế giới nơi sự kiện xảy ra.
    pub world: WorldId,
    /// Tick địa phương lúc xảy ra.
    pub tick: Tick,
    /// Loại.
    pub kind: EventKind,
    /// Ai gây ra, nếu có chủ thể.
    pub actor: Option<EntityId>,
    /// Chịu tác động lên ai, nếu có đối tượng.
    pub subject: Option<EntityId>,
    /// Dữ liệu.
    pub payload: Value,
    /// Sự kiện nào đã dẫn tới sự kiện này.
    ///
    /// Đây là cạnh của đồ thị nhân quả mà `§18.10` cần. Nó phải được ghi *lúc
    /// tạo* sự kiện; suy ngược lại sau đó là bất khả thi, và một chuỗi nhân quả
    /// được đoán ra thì tệ hơn không có, vì người xem sẽ tin nó.
    pub cause: Option<EventSeq>,
    /// Phiên bản luật đang hiệu lực lúc sự kiện xảy ra (`§13.9.5`, `§22.49`).
    ///
    /// Không có trường này thì sửa một luật sẽ hồi tố lên toàn bộ lịch sử: một
    /// hành vi hợp pháp năm xưa bỗng thành phạm pháp khi ta chỉnh `norm_set`
    /// hôm nay.
    pub law_version: Option<u32>,
    /// Phiên bản **bộ chuẩn mực** đang hiệu lực lúc đó (`§18.10`, `§22.49`).
    ///
    /// Tách khỏi [`Event::law_version`] vì luật và chuẩn mực là hai thứ khác
    /// nhau, và chúng đổi độc lập: luật là thứ engine cưỡng chế, chuẩn mực là
    /// thứ một nền văn hóa tán thành. Cùng một hành vi có thể hợp pháp mà bị
    /// khinh, hoặc phạm pháp mà được nể — và khung xem nhân quả phải nói được
    /// điều đó, nếu không nó chỉ trả lời được "chuyện gì đã xảy ra" chứ không
    /// trả lời được "vì sao cả làng phản ứng như thế".
    pub norm_set_version: Option<u32>,
}

impl CanonicalHash for Event {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.seq.canonical_hash(h);
        self.branch.canonical_hash(h);
        self.world.canonical_hash(h);
        self.tick.canonical_hash(h);
        self.kind.canonical_hash(h);
        self.actor.canonical_hash(h);
        self.subject.canonical_hash(h);
        self.payload.canonical_hash(h);
        h.write_option(self.cause, |hh, c| c.canonical_hash(hh));
        h.write_option(self.norm_set_version, |hh, v| {
            hh.write_u64(u64::from(v));
        });
        h.write_option(self.law_version, |hh, v| {
            hh.write_u64(u64::from(v));
        });
    }
}

/// Bản nháp một sự kiện, trước khi log gán `seq` cho nó.
///
/// Handler tạo `EventDraft`; chỉ [`EventLog::append`] mới gán được số thứ tự.
/// Nhờ vậy không handler nào tự chọn được vị trí của mình trong lịch sử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDraft {
    /// Loại.
    pub kind: EventKind,
    /// Chủ thể.
    pub actor: Option<EntityId>,
    /// Đối tượng.
    pub subject: Option<EntityId>,
    /// Dữ liệu.
    pub payload: Value,
    /// Nguyên nhân.
    pub cause: Option<EventSeq>,
    /// Phiên bản luật.
    pub law_version: Option<u32>,
    /// Phiên bản bộ chuẩn mực lúc đó.
    pub norm_set_version: Option<u32>,
}

impl EventDraft {
    /// Bản nháp tối thiểu.
    pub fn new(kind: &str, payload: Value) -> EventDraft {
        EventDraft {
            kind: EventKind::of(kind),
            actor: None,
            subject: None,
            payload,
            cause: None,
            law_version: None,
            norm_set_version: None,
        }
    }

    /// Gắn chủ thể.
    #[must_use]
    pub fn by(mut self, actor: EntityId) -> EventDraft {
        self.actor = Some(actor);
        self
    }

    /// Gắn đối tượng.
    #[must_use]
    pub fn on(mut self, subject: EntityId) -> EventDraft {
        self.subject = Some(subject);
        self
    }

    /// Gắn nguyên nhân.
    #[must_use]
    pub fn caused_by(mut self, cause: EventSeq) -> EventDraft {
        self.cause = Some(cause);
        self
    }

    /// Gắn phiên bản luật.
    #[must_use]
    pub fn under_law(mut self, v: u32) -> EventDraft {
        self.law_version = Some(v);
        self
    }

    /// Gắn phiên bản bộ chuẩn mực.
    #[must_use]
    pub fn under_norms(mut self, v: u32) -> EventDraft {
        self.norm_set_version = Some(v);
        self
    }
}

/// Nhật ký chỉ ghi thêm.
///
/// Cấu trúc này **cố ý không có** `remove`, `truncate`, `get_mut` hay
/// `IndexMut`. Nếu một ngày nào đó ai đó thêm một trong số đó, bài test
/// `nhat_ky_khong_the_sua` sẽ không bắt được — nhưng review sẽ, vì lý do nằm
/// ngay đây.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<Event>,
    /// Hash tích lũy của toàn bộ nhật ký.
    ///
    /// Cập nhật tăng dần: `h_n = H(h_{n-1} ‖ event_n)`. Nhờ vậy so hai nhánh
    /// chỉ tốn một phép so 32 byte thay vì duyệt lại cả lịch sử, và bisect
    /// theo tick (`§P7.5`) trở nên khả thi trên nhật ký dài.
    running: StateHash,
}

impl EventLog {
    /// Nhật ký rỗng.
    pub fn new() -> EventLog {
        EventLog {
            events: Vec::new(),
            running: StateHash::ZERO,
        }
    }

    /// Số sự kiện.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Số thứ tự sẽ cấp cho sự kiện tiếp theo.
    pub fn next_seq(&self) -> EventSeq {
        EventSeq(self.events.len() as u64)
    }

    /// Hash tích lũy hiện tại.
    pub fn running_hash(&self) -> StateHash {
        self.running
    }

    /// Ghi thêm một sự kiện. Đây là **cách duy nhất** đưa sự kiện vào nhật ký.
    pub fn append(
        &mut self,
        draft: EventDraft,
        branch: BranchId,
        world: WorldId,
        tick: Tick,
    ) -> EventSeq {
        let seq = self.next_seq();
        let ev = Event {
            seq,
            branch,
            world,
            tick,
            kind: draft.kind,
            actor: draft.actor,
            subject: draft.subject,
            payload: draft.payload,
            cause: draft.cause,
            law_version: draft.law_version,
            norm_set_version: draft.norm_set_version,
        };
        let mut h = StateHasher::with_domain("mow.eventlog.v1");
        h.write_hash(self.running);
        ev.canonical_hash(&mut h);
        self.running = h.finish();
        self.events.push(ev);
        seq
    }

    /// Đọc một sự kiện.
    pub fn get(&self, seq: EventSeq) -> Option<&Event> {
        self.events.get(seq.0 as usize)
    }

    /// Duyệt toàn bộ theo thứ tự.
    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }

    /// Duyệt các sự kiện trong một khoảng tick, dùng cho timeline và replay.
    pub fn in_range(&self, from: Tick, to: Tick) -> impl Iterator<Item = &Event> {
        self.events
            .iter()
            .filter(move |e| e.tick >= from && e.tick <= to)
    }

    /// Truy ngược chuỗi nhân quả từ một sự kiện về gốc (`§18.10`).
    ///
    /// Trả theo thứ tự từ sự kiện đã cho ngược về nguyên nhân xa nhất. Có chặn
    /// độ sâu vì một chuỗi nhân quả hỏng (tự trỏ về mình) sẽ treo giao diện, và
    /// một giao diện treo thì không ai gỡ được lỗi bằng nó nữa.
    pub fn cause_chain(&self, from: EventSeq, max_depth: usize) -> Vec<&Event> {
        let mut ra = Vec::new();
        let mut cur = Some(from);
        let mut da_tham = std::collections::BTreeSet::new();
        while let Some(seq) = cur {
            if ra.len() >= max_depth || !da_tham.insert(seq) {
                break;
            }
            let Some(ev) = self.get(seq) else { break };
            ra.push(ev);
            cur = ev.cause;
        }
        ra
    }
}

impl CanonicalHash for EventLog {
    fn canonical_hash(&self, h: &mut StateHasher) {
        // Hash tích lũy đã tóm tắt toàn bộ nội dung, nên không cần duyệt lại.
        h.write_hash(self.running);
        h.write_u64(self.events.len() as u64);
    }
}
