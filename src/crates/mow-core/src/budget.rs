//! Ngân sách nhận thức (`idea.md §20.2`, `§20.2.1`, `§22.9`, `PC-08`).
//!
//! > **Selection deterministic trong Rust**, throttling ở gateway; hai thứ
//! > không được trộn.
//!
//! ## Hai thứ trông giống nhau nhưng khác hẳn
//!
//! | | Selection | Throttling |
//! |---|---|---|
//! | Câu hỏi | *ai được nghĩ ở tick này* | *gửi bao nhiêu request mỗi giây* |
//! | Ở đâu | **`mow-core`**, trong đường commit | LLM Gateway |
//! | Đầu vào | state của thế giới | hạn mức API, độ trễ đo được, chi phí |
//! | Xác định? | **bắt buộc** | không, và không cần |
//! | Vào state hash? | **có** | không |
//!
//! Trộn chúng lại là lỗi mà `§P5.2.1` cảnh báo bằng một câu rất thẳng:
//!
//! > Nếu Cognition Scheduler nằm ở Python, thứ tự và số lượng request LLM sẽ
//! > phụ thuộc timing của tiến trình và replay sẽ hỏng. Đây là **lý do kỹ
//! > thuật**, không phải sở thích.
//!
//! Cụ thể: nếu gateway quyết định ai được nghĩ dựa trên "còn hạn mức không",
//! thì cùng một thế giới chạy vào giờ cao điểm và giờ thấp điểm sẽ diễn tiến
//! khác nhau. Người chơi không thấy hạn mức API; họ chỉ thấy một ngôi làng
//! hành xử khác nhau vào hai buổi tối khác nhau.
//!
//! ## Chọn theo cái gì
//!
//! Không phải "ai đói nhất". Ba yếu tố, và yếu tố thứ ba là quan trọng nhất:
//!
//! 1. **Cấp bách** — nhu cầu sắp chạm ngưỡng chết, hoặc có kẻ thù trong tầm.
//! 2. **Người chơi đang chú ý** — nhân vật trong tầm nhìn đáng nghĩ hơn nhân
//!    vật ở nửa bên kia bản đồ.
//! 3. **Đã chờ bao lâu** — không có yếu tố này thì một nhân vật ít cấp bách
//!    **không bao giờ** được nghĩ, và nó đứng ngây ra mãi mãi. Đây là điều mà
//!    một hàng đợi ưu tiên thuần túy luôn làm sai.

use crate::clock::Tick;
use crate::ids::{EntityId, StableKey};
use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Một yêu cầu nhận thức đang chờ được chọn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    /// Ai muốn nghĩ.
    pub entity: EntityId,
    /// Mức cấp bách, `0`–`1000`. Suy từ state, không phải do ai đặt.
    pub urgency: u32,
    /// Người chơi có đang nhìn không.
    pub in_focus: bool,
    /// Tick mà thực thể này bắt đầu chờ.
    pub waiting_since: Tick,
    /// Khóa ổn định để phá hòa.
    pub key: StableKey,
}

impl CanonicalHash for Pending {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.entity.canonical_hash(h);
        h.write_u64(u64::from(self.urgency));
        h.write_bool(self.in_focus);
        self.waiting_since.canonical_hash(h);
        self.key.canonical_hash(h);
    }
}

/// Trọng số của ba yếu tố.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weights {
    /// Nhân với `urgency`.
    pub urgency: i64,
    /// Cộng thêm nếu đang trong tầm chú ý.
    pub focus_bonus: i64,
    /// Nhân với số tick đã chờ.
    ///
    /// **Không được bằng 0.** Bằng 0 nghĩa là một nhân vật ít cấp bách không
    /// bao giờ tới lượt, và nó sẽ đứng ngây ra suốt đời — trong khi ngôi làng
    /// xung quanh vẫn sống. Người chơi sẽ nhận ra, và họ sẽ gọi đó là bug.
    ///
    /// Nhưng "khác 0" chưa đủ. Với `starvation = 1` và `urgency = 10`, một
    /// người ở đáy thang cấp bách phải chờ `1000 * 10 / 1 = 10 000` tick mới
    /// đuổi kịp — hàng giờ đồng hồ. Về mặt kỹ thuật thì không chết đói; về mặt
    /// người chơi nhìn thấy thì vẫn là đứng ngây. Xem [`Weights::max_wait`].
    pub starvation: i64,
}

impl Weights {
    /// **Trần thời gian chờ trong trường hợp xấu nhất**, tính bằng tick.
    ///
    /// Đây là con số mà cả tài liệu lẫn test đều nói tới, thay vì một lời hứa
    /// mơ hồ kiểu "cuối cùng thì cũng tới lượt". Suy ra trực tiếp: một người ở
    /// đáy thang cấp bách bị một người ở đỉnh thang chặn, và chỉ có `starvation`
    /// kéo họ lên.
    ///
    /// ```text
    /// max_wait = urgency_max * urgency / starvation  +  1  +  1
    ///            └── đuổi kịp khoảng cách cấp bách ─┘   │     │
    ///            kẻ chặn đường cũng chờ ít nhất 1 tick ─┘     │
    ///            hòa thì thua (phá hòa bằng khóa) ────────────┘
    /// ```
    ///
    /// Hai số hạng cuối trông như số hạng vụn, nhưng chúng có thật và bỏ đi thì
    /// trần sai: kẻ đang chặn cũng tích `starvation` mỗi tick nó chờ, và người
    /// chờ lâu phải **vượt hẳn** chứ không được hòa — hòa thì [`StableKey`] xử,
    /// và khóa không quan tâm ai chờ lâu hơn (`§22.43`).
    ///
    /// Trả `None` nếu `starvation == 0` — nghĩa là **không có trần**, và đó
    /// chính là lỗi mà trường này tồn tại để tránh.
    ///
    /// [`StableKey`]: crate::ids::StableKey
    pub fn max_wait(&self, urgency_max: u32) -> Option<u64> {
        if self.starvation <= 0 {
            return None;
        }
        let tu = i64::from(urgency_max).checked_mul(self.urgency)?;
        u64::try_from(tu / self.starvation).ok()?.checked_add(2)
    }
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            urgency: 10,
            // Chú ý của người chơi đáng giá 200 điểm cấp bách — một ngón tay
            // đè lên cán cân, không phải một cái công tắc. Một người sắp chết
            // đói ở nửa bên kia bản đồ vẫn thắng một người đang no ngay trước
            // mặt, và đó là điều đúng: thế giới không diễn kịch cho camera.
            focus_bonus: 2_000,
            // Trần chờ = 1000 * 10 / 20 + 2 = 502 tick. Xem `Weights::max_wait`.
            starvation: 20,
        }
    }
}

impl CanonicalHash for Weights {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.urgency);
        h.write_i64(self.focus_bonus);
        h.write_i64(self.starvation);
    }
}

/// Điểm ưu tiên và các phần đóng góp, để giải thích (`§18.13`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Priority {
    /// Thực thể.
    pub entity: EntityId,
    /// Tổng điểm.
    pub score: i64,
    /// Từng phần: `(tên, điểm)`.
    pub parts: Vec<(&'static str, i64)>,
}

/// Bộ lập lịch nhận thức.
///
/// **Không có** trường nào về hạn mức API, độ trễ mạng, hay chi phí. Đó là
/// ranh giới với gateway, dưới dạng một struct không có chỗ để nhét chúng vào.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionScheduler {
    /// Số lời gọi tối đa mỗi tick. Đây là **ngân sách của thế giới**, không phải
    /// hạn mức của nhà cung cấp — nó nằm trong config đã version hóa, và đổi nó
    /// phải ghi vào event log (`§P6.1`).
    pub max_per_tick: u32,
    /// Trọng số.
    pub weights: Weights,
    /// Lần gần nhất mỗi thực thể được chọn.
    last_served: BTreeMap<EntityId, Tick>,
}

impl CognitionScheduler {
    /// Bộ lập lịch mới.
    pub fn new(max_per_tick: u32) -> CognitionScheduler {
        CognitionScheduler {
            max_per_tick,
            weights: Weights::default(),
            last_served: BTreeMap::new(),
        }
    }

    /// Chấm điểm một yêu cầu.
    pub fn score(&self, p: &Pending, now: Tick) -> Priority {
        let w = self.weights;
        let cho = i64::try_from(now.since(p.waiting_since).unwrap_or(0)).unwrap_or(i64::MAX);

        let mut parts = vec![("cấp bách", i64::from(p.urgency) * w.urgency)];
        if p.in_focus {
            parts.push(("đang được nhìn", w.focus_bonus));
        }
        parts.push(("đã chờ", cho.saturating_mul(w.starvation)));

        Priority {
            entity: p.entity,
            score: parts.iter().map(|(_, v)| v).sum(),
            parts,
        }
    }

    /// Chọn ai được nghĩ ở tick này.
    ///
    /// **Hàm thuần của `(pending, now, self)`.** Không đọc đồng hồ hệ thống,
    /// không hỏi mạng, không có ngẫu nhiên. Đó là điều kiện để `§22.9` giữ
    /// được: cùng seed, cùng command, cùng lịch sử LLM ⇒ cùng checkpoint hash.
    pub fn select(&mut self, pending: &[Pending], now: Tick) -> Vec<Priority> {
        let mut cham: Vec<Priority> = pending.iter().map(|p| self.score(p, now)).collect();

        // Sắp theo điểm giảm dần, phá hòa bằng khóa ổn định. Vế phá hòa là chỗ
        // duy nhất `EntityId` được dùng, và nó chỉ được dùng khi mọi thứ luật
        // quan tâm đã bằng nhau (`§22.43`).
        let khoa: BTreeMap<EntityId, StableKey> =
            pending.iter().map(|p| (p.entity, p.key)).collect();
        cham.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| khoa.get(&a.entity).cmp(&khoa.get(&b.entity)))
        });

        cham.truncate(self.max_per_tick as usize);
        for p in &cham {
            self.last_served.insert(p.entity, now);
        }
        cham
    }

    /// Lần gần nhất một thực thể được chọn.
    pub fn last_served(&self, e: EntityId) -> Option<Tick> {
        self.last_served.get(&e).copied()
    }

    /// Thực thể chờ lâu nhất mà chưa được phục vụ.
    ///
    /// Công cụ chẩn đoán: nếu con số này lớn dần không giới hạn, thì trọng số
    /// `starvation` quá nhỏ và có nhân vật đang đứng ngây.
    pub fn longest_wait(&self, pending: &[Pending], now: Tick) -> Option<(EntityId, u64)> {
        pending
            .iter()
            .map(|p| (p.entity, now.since(p.waiting_since).unwrap_or(0)))
            .max_by_key(|(_, w)| *w)
    }

    /// Dọn bản ghi của những thực thể không còn tồn tại.
    ///
    /// `last_served` nằm trong state hash, nên để nó lớn lên vô hạn là vừa rò
    /// rỉ bộ nhớ vừa làm hash phình theo lịch sử thay vì theo trạng thái.
    pub fn prune(&mut self, alive: &std::collections::BTreeSet<EntityId>) -> usize {
        let truoc = self.last_served.len();
        self.last_served.retain(|e, _| alive.contains(e));
        truoc - self.last_served.len()
    }
}

impl CanonicalHash for CognitionScheduler {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(u64::from(self.max_per_tick));
        self.weights.canonical_hash(h);
        h.write_seq(self.last_served.iter(), |hh, (e, t)| {
            e.canonical_hash(hh);
            t.canonical_hash(hh);
        });
    }
}

/// Vì sao một yêu cầu không được phục vụ.
///
/// Ghi vào event để `§20.10` giữ được: mọi lần hạ cấp hay bỏ qua đều để lại
/// dấu vết. Không có nó, một ngôi làng im lặng suốt một giờ là một bí ẩn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Deferred {
    /// Hết ngân sách của tick này.
    BudgetExhausted,
    /// Điểm quá thấp so với những người khác.
    Outranked,
}

impl Deferred {
    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            Deferred::BudgetExhausted => "budget_exhausted",
            Deferred::Outranked => "outranked",
        }
    }
}
