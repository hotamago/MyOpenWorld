//! Chrono-turn: `ready_at`, bốn loại tốc độ, ba pha (`idea.md §10.7`, `§10.8`).
//!
//! ## Vì sao bốn loại tốc độ chứ không phải một
//!
//! Một chỉ số "nhanh" duy nhất gộp bốn thứ độc lập, và gộp chúng lại xóa mất
//! bốn kiểu nhân vật khác nhau:
//!
//! | Tốc độ | Nghĩa | Nhân vật điển hình |
//! |---|---|---|
//! | [`Speeds::cognition`] | bao lâu nghĩ một lần | người từng trải quyết đoán |
//! | [`Speeds::action`] | ra đòn nhanh cỡ nào | tay kiếm luyện tập |
//! | [`Speeds::movement`] | đi nhanh cỡ nào | người chạy bộ |
//! | [`Speeds::recovery`] | hồi lại sau đòn nhanh cỡ nào | người khỏe |
//!
//! Một lão già thông thái nghĩ nhanh nhưng ra đòn chậm. Một chiến binh trẻ
//! ngược lại. Với một chỉ số duy nhất, cả hai chỉ là "nhanh 7" và "nhanh 4".
//!
//! `cognition` đặc biệt quan trọng: `§20.2.2` dùng nó để tính độ trễ nhận thức
//! `D`. Muốn nhân vật phản ứng nhanh hơn thì tăng chỉ số này — **không phải**
//! hy vọng mô hình trả lời nhanh.
//!
//! ## Ba pha, và chỉ `impact` phát proposal
//!
//! ```text
//! wind_up ──► impact ──► recovery
//!  vung tay    chạm      thu tay về
//!  (thấy được) (kết quả)  (hở sườn)
//! ```
//!
//! `wind_up` **quan sát được**: đó là thứ cho phép né và đỡ. Một hệ thống mà
//! đòn đánh xảy ra tức thời thì không có phòng thủ có ý nghĩa, chỉ có xác suất.
//!
//! `recovery` là lúc hở sườn. Nó biến "ra đòn" thành một quyết định có giá,
//! chứ không phải một thứ luôn nên làm.

use mow_core::{EntityId, StableKey, Tick, WorldId};
use mow_math::{CanonicalHash, MathResult, Rate, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Bốn loại tốc độ của một thực thể.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Speeds {
    /// Số tick giữa hai lần suy nghĩ. Nhỏ hơn là nghĩ thường xuyên hơn.
    ///
    /// `§20.2.2` suy `D` từ đây. Đây là **thuộc tính của thế giới**; tốc độ
    /// đường truyền thì không.
    pub cognition: u32,
    /// Hệ số tốc độ ra đòn, thang phần trăm. `100` là chuẩn.
    pub action: u32,
    /// Hệ số tốc độ di chuyển, thang phần trăm.
    pub movement: u32,
    /// Hệ số tốc độ hồi phục sau đòn, thang phần trăm.
    pub recovery: u32,
}

impl Default for Speeds {
    fn default() -> Self {
        Speeds {
            cognition: 10,
            action: 100,
            movement: 100,
            recovery: 100,
        }
    }
}

impl CanonicalHash for Speeds {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(u64::from(self.cognition));
        h.write_u64(u64::from(self.action));
        h.write_u64(u64::from(self.movement));
        h.write_u64(u64::from(self.recovery));
    }
}

/// Pha của một hành động đang diễn ra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Đang vung tay. **Quan sát được** — đây là cửa sổ để né và đỡ.
    WindUp,
    /// Chạm. Điểm **duy nhất** phát proposal.
    Impact,
    /// Thu tay về. Hở sườn.
    Recovery,
    /// Đã xong.
    Done,
}

impl Phase {
    /// Pha kế tiếp.
    pub fn next(self) -> Phase {
        match self {
            Phase::WindUp => Phase::Impact,
            Phase::Impact => Phase::Recovery,
            Phase::Recovery | Phase::Done => Phase::Done,
        }
    }

    /// Có quan sát được không.
    pub fn is_observable(self) -> bool {
        matches!(self, Phase::WindUp | Phase::Recovery)
    }

    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::WindUp => "wind_up",
            Phase::Impact => "impact",
            Phase::Recovery => "recovery",
            Phase::Done => "done",
        }
    }
}

impl CanonicalHash for Phase {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

/// Thời lượng ba pha của một loại hành động, tính bằng tick ở tốc độ chuẩn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseDurations {
    /// Vung tay.
    pub wind_up: u32,
    /// Chạm.
    pub impact: u32,
    /// Thu tay.
    pub recovery: u32,
}

impl PhaseDurations {
    /// Thời lượng một pha, đã điều chỉnh theo tốc độ.
    ///
    /// `wind_up` và `impact` chịu `action`; `recovery` chịu `recovery`. Tách ra
    /// vì đó là hai khả năng khác nhau: ra đòn nhanh và thu tay nhanh không đi
    /// cùng nhau, và một nhân vật mạnh mà chậm phục hồi là một loại nhân vật có
    /// thật.
    pub fn ticks_for(self, phase: Phase, s: Speeds) -> u64 {
        let (co_ban, he_so) = match phase {
            Phase::WindUp => (self.wind_up, s.action),
            Phase::Impact => (self.impact, s.action),
            Phase::Recovery => (self.recovery, s.recovery),
            Phase::Done => return 0,
        };
        // Hệ số cao thì nhanh, nên chia. `max(1)` để một pha không bao giờ dài 0
        // tick — pha 0 tick nghĩa là không quan sát được, và `wind_up` không
        // quan sát được thì phòng thủ biến mất.
        (u64::from(co_ban) * 100 / u64::from(he_so.max(1))).max(1)
    }
}

impl CanonicalHash for PhaseDurations {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(u64::from(self.wind_up));
        h.write_u64(u64::from(self.impact));
        h.write_u64(u64::from(self.recovery));
    }
}

/// Một hành động đang diễn ra trên dòng thời gian.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scheduled {
    /// Ai làm.
    pub actor: EntityId,
    /// Loại hành động.
    pub action: String,
    /// Thế giới.
    pub world: WorldId,
    /// Pha hiện tại.
    pub phase: Phase,
    /// Tick mà pha hiện tại kết thúc.
    pub ready_at: Tick,
    /// Khóa ổn định để phá hòa.
    pub key: StableKey,
    /// Thời lượng các pha.
    pub durations: PhaseDurations,
    /// Tốc độ của người làm, chốt lúc bắt đầu.
    ///
    /// Chốt chứ không tra lại mỗi pha: nếu tốc độ đổi giữa chừng (bị thương,
    /// hết buff), một đòn đang vung dở sẽ đột ngột nhanh lên hoặc chậm đi. Chốt
    /// lúc bắt đầu làm hành động thành một cam kết, và cam kết là thứ tạo ra
    /// chiến thuật.
    pub speeds: Speeds,
}

impl CanonicalHash for Scheduled {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.actor.canonical_hash(h);
        h.write_str(&self.action);
        self.world.canonical_hash(h);
        self.phase.canonical_hash(h);
        self.ready_at.canonical_hash(h);
        self.key.canonical_hash(h);
        self.durations.canonical_hash(h);
        self.speeds.canonical_hash(h);
    }
}

/// Khóa sắp xếp trong hàng đợi: `(ready_at, world, stable_key)` (`plan.md §P6.5`).
///
/// Ba tầng, và mỗi tầng phá hòa cho tầng trên. Thiếu tầng cuối thì hai hành
/// động cùng tick cùng thế giới sẽ xếp theo thứ tự chèn — và thứ tự chèn là
/// lịch sử, không phải luật.
type QueueKey = (u64, u64, StableKey);

/// Dòng thời gian hành động.
#[derive(Debug, Default)]
pub struct Timeline {
    /// `BTreeSet` chứ không phải `BinaryHeap`: heap không cho duyệt theo thứ tự
    /// và không cho gỡ một phần tử ở giữa. Cả hai đều cần — gỡ khi hành động bị
    /// hủy, duyệt khi hiển thị dòng thời gian cho người chơi (`§18.8`).
    queue: BTreeSet<QueueKey>,
    items: std::collections::BTreeMap<QueueKey, Scheduled>,
}

impl Timeline {
    /// Rỗng.
    pub fn new() -> Timeline {
        Timeline::default()
    }

    /// Số hành động đang chờ.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn key_of(s: &Scheduled) -> QueueKey {
        (s.ready_at.0, s.world.get(), s.key)
    }

    /// Bắt đầu một hành động: xếp lịch pha `wind_up`.
    pub fn begin(&mut self, mut s: Scheduled, now: Tick) -> MathResult<()> {
        s.phase = Phase::WindUp;
        let d = s.durations.ticks_for(Phase::WindUp, s.speeds);
        s.ready_at = now.plus(d).unwrap_or(now);
        let k = Self::key_of(&s);
        self.queue.insert(k);
        self.items.insert(k, s);
        Ok(())
    }

    /// Hủy mọi hành động của một thực thể.
    ///
    /// Trả số hành động đã hủy. Cần thiết khi thực thể chết hoặc bị choáng —
    /// nếu không, một xác chết vẫn hoàn thành đòn đánh của nó ba tick sau.
    pub fn cancel(&mut self, actor: EntityId) -> usize {
        let can_go: Vec<QueueKey> = self
            .items
            .iter()
            .filter(|(_, s)| s.actor == actor)
            .map(|(k, _)| *k)
            .collect();
        for k in &can_go {
            self.queue.remove(k);
            self.items.remove(k);
        }
        can_go.len()
    }

    /// Mọi hành động tới hạn tại `now`, **theo thứ tự khóa**.
    ///
    /// Gỡ chúng khỏi hàng đợi. Chỗ gọi phải xử lý rồi gọi [`Timeline::advance`]
    /// để đưa chúng sang pha kế.
    pub fn due(&mut self, now: Tick) -> Vec<Scheduled> {
        let toi_han: Vec<QueueKey> = self
            .queue
            .iter()
            .take_while(|(t, _, _)| *t <= now.0)
            .copied()
            .collect();
        toi_han
            .into_iter()
            .filter_map(|k| {
                self.queue.remove(&k);
                self.items.remove(&k)
            })
            .collect()
    }

    /// Đưa một hành động sang pha kế và xếp lại lịch.
    ///
    /// Trả `None` khi hành động đã xong.
    pub fn advance(&mut self, mut s: Scheduled, now: Tick) -> Option<Scheduled> {
        s.phase = s.phase.next();
        if s.phase == Phase::Done {
            return None;
        }
        let d = s.durations.ticks_for(s.phase, s.speeds);
        s.ready_at = now.plus(d).unwrap_or(now);
        let k = Self::key_of(&s);
        self.queue.insert(k);
        self.items.insert(k, s.clone());
        Some(s)
    }

    /// Duyệt mọi hành động đang chờ, theo thứ tự. Dùng cho `§18.8`.
    pub fn iter(&self) -> impl Iterator<Item = &Scheduled> {
        self.queue.iter().filter_map(|k| self.items.get(k))
    }

    /// Tick sớm nhất có hành động tới hạn.
    ///
    /// Cho phép nhảy thẳng tới đó thay vì tiến từng tick một — cùng ý tưởng với
    /// đánh thức theo ngưỡng của homeostasis.
    pub fn next_due(&self) -> Option<Tick> {
        self.queue.iter().next().map(|(t, _, _)| Tick(*t))
    }
}

impl CanonicalHash for Timeline {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.iter(), |hh, s| s.canonical_hash(hh));
    }
}

/// Tick mà một thực thể sẽ suy nghĩ lần tới (`§20.2.2`).
///
/// Hàm của `cognition_rate` — **không** của tốc độ mạng.
pub fn next_cognition_tick(now: Tick, s: Speeds) -> Option<Tick> {
    now.plus(u64::from(s.cognition.max(1)))
}

/// Độ trễ nhận thức `D` suy từ `cognition_rate`.
///
/// `§20.2.2`: kết quả LLM được áp tại `T + D`, bất kể mô hình trả lời nhanh hay
/// chậm. Nhân vật nghĩ nhanh có `D` nhỏ; điều đó là một thuộc tính của nhân
/// vật, quan sát được từ trong thế giới.
pub fn cognitive_latency(s: Speeds) -> u64 {
    u64::from(s.cognition.max(1))
}

/// Tốc độ di chuyển thực tế, ô mỗi tick, dưới dạng hữu tỉ.
///
/// Hữu tỉ chứ không phải số nguyên: một nhân vật đi `2/3` ô mỗi tick là chuyện
/// bình thường, và làm tròn xuống 0 sẽ khiến người chậm đứng yên vĩnh viễn.
pub fn movement_rate(base_cells_per_100_ticks: i64, s: Speeds) -> MathResult<Rate> {
    Rate::new(base_cells_per_100_ticks * i64::from(s.movement), 100 * 100)
}
