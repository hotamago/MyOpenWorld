//! Hộ gia đình và địa điểm (`idea.md §12.9`, `§12.18.2`, `PB-14`).
//!
//! ## Vì sao địa điểm phải có **hàng đợi thật**
//!
//! Một cái giếng không phải một chỗ đánh dấu trên bản đồ. Nó là chỗ mà mỗi
//! sáng cả làng đi qua, một người một lượt, và **đứng chờ cạnh nhau**.
//!
//! Điều đó tạo ra thứ mà không cơ chế nào khác tạo được: một **contact graph**
//! nổi lên từ hành vi thay vì được khai báo. Ai gặp ai, bao lâu, ở đâu — và từ
//! đó:
//!
//! - Bệnh lây theo đúng những đường mà người ta thật sự đi.
//! - Tin đồn lan theo cùng những đường đó, nên nó tới nơi có nghĩa.
//! - Người mới tới bị nhận ra vì họ không có trong đồ thị.
//! - Một người tránh giếng suốt một tuần là một dữ kiện, và nó có thể là manh mối.
//!
//! Nếu chỉ mô hình hóa "làng có một cái giếng" mà không có hàng đợi, mọi thứ ở
//! trên phải được **giả lập bằng xác suất**, và xác suất không kể được chuyện.

use mow_math::{CanonicalHash, StateHasher, WorldPos};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Quan hệ huyết thống hoặc hôn nhân.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kinship {
    /// Cha hoặc mẹ.
    Parent,
    /// Con.
    Child,
    /// Anh chị em.
    Sibling,
    /// Bạn đời.
    Spouse,
    /// Nhận nuôi — quan hệ xã hội, không huyết thống.
    ///
    /// Tách riêng vì `PD-22` cần phân biệt: nhận nuôi không truyền gen, nên hệ
    /// số cận huyết không tính nó.
    Adopted,
}

impl CanonicalHash for Kinship {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(match self {
            Kinship::Parent => "parent",
            Kinship::Child => "child",
            Kinship::Sibling => "sibling",
            Kinship::Spouse => "spouse",
            Kinship::Adopted => "adopted",
        });
    }
}

/// Giai đoạn vòng đời của một hộ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseholdStage {
    /// Mới lập.
    Forming,
    /// Đang nuôi con nhỏ.
    Rearing,
    /// Con đã lớn, sắp tách ra.
    Mature,
    /// Chỉ còn người già.
    Contracting,
    /// Đã tan.
    Dissolved,
}

/// Một hộ gia đình.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Household {
    /// Định danh.
    pub id: u64,
    /// Thành viên.
    members: BTreeSet<u64>,
    /// Nơi ở.
    pub home: WorldPos,
    /// Giai đoạn.
    pub stage: HouseholdStage,
    /// Kho chung.
    pub stores: BTreeMap<String, i64>,
}

impl Household {
    /// Hộ mới với một nhóm thành viên.
    pub fn new(id: u64, home: WorldPos, members: impl IntoIterator<Item = u64>) -> Household {
        Household {
            id,
            members: members.into_iter().collect(),
            home,
            stage: HouseholdStage::Forming,
            stores: BTreeMap::new(),
        }
    }

    /// Thành viên, theo thứ tự định danh.
    pub fn members(&self) -> impl Iterator<Item = u64> + '_ {
        self.members.iter().copied()
    }

    /// Số thành viên.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Thêm thành viên.
    pub fn add(&mut self, who: u64) {
        self.members.insert(who);
    }

    /// Bớt thành viên. Hộ rỗng thì tan.
    ///
    /// Tan **tự động** chứ không đợi ai gọi: một hộ không còn ai mà vẫn tồn tại
    /// sẽ giữ kho chung mãi mãi, và của cải trong đó biến mất khỏi kinh tế mà
    /// không ai nhận ra.
    pub fn remove(&mut self, who: u64) {
        self.members.remove(&who);
        if self.members.is_empty() {
            self.stage = HouseholdStage::Dissolved;
        }
    }

    /// Đã tan chưa.
    pub fn is_dissolved(&self) -> bool {
        self.stage == HouseholdStage::Dissolved
    }
}

impl CanonicalHash for Household {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.id);
        h.write_seq(self.members.iter().copied(), |hh, m| {
            hh.write_u64(m);
        });
        self.home.canonical_hash(h);
        h.write_i64(self.stage as i64);
        h.write_seq(self.stores.iter(), |hh, (k, v)| {
            hh.write_str(k);
            hh.write_i64(*v);
        });
    }
}

/// Loại địa điểm sinh hoạt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    /// Giếng nước.
    Well,
    /// Chợ.
    Market,
    /// Quán.
    Tavern,
    /// Nơi thờ tự.
    Shrine,
    /// Lò rèn.
    Forge,
}

/// Một địa điểm có sức chứa và hàng đợi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    /// Định danh.
    pub id: String,
    /// Loại.
    pub kind: PlaceKind,
    /// Vị trí.
    pub at: WorldPos,
    /// Bao nhiêu người dùng được cùng lúc.
    pub capacity: u32,
    /// Số tick mỗi lượt.
    pub service_ticks: u64,
    /// Đang dùng: `(ai, tick xong)`.
    serving: Vec<(u64, u64)>,
    /// Đang chờ, **theo thứ tự tới**.
    queue: VecDeque<u64>,
}

impl Place {
    /// Địa điểm mới.
    pub fn new(
        id: &str,
        kind: PlaceKind,
        at: WorldPos,
        capacity: u32,
        service_ticks: u64,
    ) -> Place {
        Place {
            id: id.to_owned(),
            kind,
            at,
            capacity,
            service_ticks,
            serving: Vec::new(),
            queue: VecDeque::new(),
        }
    }

    /// Xếp hàng. Trả vị trí trong hàng, hoặc `0` nếu được phục vụ ngay.
    pub fn arrive(&mut self, who: u64, now: u64) -> usize {
        if (self.serving.len() as u32) < self.capacity {
            self.serving.push((who, now + self.service_ticks));
            self.serving.sort_by_key(|(w, _)| *w);
            return 0;
        }
        if !self.queue.contains(&who) {
            self.queue.push_back(who);
        }
        self.queue
            .iter()
            .position(|w| *w == who)
            .map_or(0, |i| i + 1)
    }

    /// Tiến thời gian: ai xong thì rời, ai đang chờ thì vào.
    ///
    /// Trả về những người vừa xong.
    pub fn tick(&mut self, now: u64) -> Vec<u64> {
        let xong: Vec<u64> = self
            .serving
            .iter()
            .filter(|(_, t)| *t <= now)
            .map(|(w, _)| *w)
            .collect();
        self.serving.retain(|(_, t)| *t > now);

        while (self.serving.len() as u32) < self.capacity {
            let Some(ke_tiep) = self.queue.pop_front() else {
                break;
            };
            self.serving.push((ke_tiep, now + self.service_ticks));
        }
        self.serving.sort_by_key(|(w, _)| *w);
        xong
    }

    /// Những ai đang có mặt — đang dùng **hoặc** đang chờ.
    ///
    /// Cả hai, vì người đứng chờ cũng đứng cạnh nhau. Chỉ đếm người đang dùng
    /// sẽ bỏ sót phần lớn tiếp xúc — và ở một cái giếng đông thì hàng chờ dài
    /// hơn chỗ múc nước nhiều lần.
    pub fn present(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self.serving.iter().map(|(w, _)| *w).collect();
        v.extend(self.queue.iter().copied());
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Số người đang chờ.
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Rời hàng mà không được phục vụ.
    pub fn leave(&mut self, who: u64) -> bool {
        let truoc = self.queue.len();
        self.queue.retain(|w| *w != who);
        self.serving.retain(|(w, _)| *w != who);
        self.queue.len() != truoc
    }
}

impl CanonicalHash for Place {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_i64(self.kind as i64);
        self.at.canonical_hash(h);
        h.write_u64(u64::from(self.capacity));
        h.write_u64(self.service_ticks);
        h.write_seq(self.serving.iter(), |hh, (w, t)| {
            hh.write_u64(*w);
            hh.write_u64(*t);
        });
        h.write_seq(self.queue.iter().copied(), |hh, w| {
            hh.write_u64(w);
        });
    }
}

/// Đồ thị tiếp xúc — **nổi lên từ hành vi**, không được khai báo.
///
/// Đây là cấu trúc mà bệnh, tin đồn và quan hệ xã hội cùng đọc. Nó không được
/// dựng bằng cách nói "hai người này quen nhau"; nó được dựng bằng cách ghi lại
/// **ai đã thật sự đứng cạnh ai**.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactGraph {
    /// `(a, b)` với `a < b` → số tick đã ở cạnh nhau.
    edges: BTreeMap<(u64, u64), u64>,
}

impl ContactGraph {
    /// Rỗng.
    pub fn new() -> ContactGraph {
        ContactGraph::default()
    }

    /// Ghi nhận một nhóm người ở cùng chỗ trong `ticks` tick.
    ///
    /// Mọi cặp trong nhóm đều tiếp xúc. Chi phí là `O(n²)` theo số người ở một
    /// địa điểm — chấp nhận được vì `capacity` của một địa điểm là nhỏ, và đó
    /// chính là lý do địa điểm có sức chứa.
    pub fn record(&mut self, present: &[u64], ticks: u64) {
        for i in 0..present.len() {
            for j in (i + 1)..present.len() {
                let (a, b) = (present[i].min(present[j]), present[i].max(present[j]));
                *self.edges.entry((a, b)).or_default() += ticks;
            }
        }
    }

    /// Tổng thời gian hai người đã ở cạnh nhau.
    pub fn contact_ticks(&self, a: u64, b: u64) -> u64 {
        self.edges.get(&(a.min(b), a.max(b))).copied().unwrap_or(0)
    }

    /// Những người mà `who` đã tiếp xúc, kèm thời lượng, **nhiều nhất trước**.
    ///
    /// Đây là thứ mà điều tra dịch tễ đọc: *"ai đã ở gần bệnh nhân số 0 lâu
    /// nhất"* — và câu trả lời có thể sai, vì đồ thị chỉ ghi những nơi có mô
    /// hình hóa hàng đợi.
    pub fn contacts_of(&self, who: u64) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self
            .edges
            .iter()
            .filter_map(|((a, b), t)| {
                if *a == who {
                    Some((*b, *t))
                } else if *b == who {
                    Some((*a, *t))
                } else {
                    None
                }
            })
            .collect();
        v.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
        v
    }

    /// Số cạnh.
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Quên dần những tiếp xúc cũ.
    ///
    /// Không có bước này thì đồ thị lớn lên vô hạn và mọi người cuối cùng đều
    /// "quen" mọi người. Quên là một phần của mô hình, không phải một tối ưu.
    pub fn decay(&mut self, amount: u64) {
        self.edges.retain(|_, t| {
            *t = t.saturating_sub(amount);
            *t > 0
        });
    }
}
