//! Chiến trường chiến thuật (`idea.md §10.10`, `PB-23`).
//!
//! Tám yếu tố, và mỗi yếu tố tồn tại để một quyết định nào đó có ý nghĩa:
//!
//! | Yếu tố | Quyết định nó tạo ra |
//! |---|---|
//! | hướng mặt | có nên vòng ra sau lưng không |
//! | tầm với | đứng gần hay xa |
//! | che chắn | có nên nấp không, và nấp sau cái gì |
//! | độ cao | có đáng leo lên không |
//! | mặt nền | đi đường nào |
//! | đội hình | đứng cạnh ai |
//! | bắn nhầm | có nên bắn khi đồng đội đang ở giữa không |
//! | vùng kiểm soát | có rút lui được không |
//!
//! Bỏ bất kỳ yếu tố nào thì một quyết định biến mất, và trận đánh nhạt đi đúng
//! bằng chừng đó.
//!
//! ## Trần vật lý của tốc độ
//!
//! `§10.10` yêu cầu một trần cứng. Không có nó, một nhân vật đủ nhanh sẽ đi hết
//! chiến trường trong một tick, và mọi yếu tố ở trên trở nên vô nghĩa — không
//! ai vòng ra sau lưng được một người dịch chuyển tức thời.

use mow_math::{CanonicalHash, StateHasher, Unit, WorldPos};
use serde::{Deserialize, Serialize};

/// Hướng mặt, tám hướng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    /// Bắc.
    N,
    /// Đông bắc.
    Ne,
    /// Đông.
    E,
    /// Đông nam.
    Se,
    /// Nam.
    S,
    /// Tây nam.
    Sw,
    /// Tây.
    W,
    /// Tây bắc.
    Nw,
}

impl Facing {
    /// Vector đơn vị.
    pub fn delta(self) -> (i64, i64) {
        match self {
            Facing::N => (0, -1),
            Facing::Ne => (1, -1),
            Facing::E => (1, 0),
            Facing::Se => (1, 1),
            Facing::S => (0, 1),
            Facing::Sw => (-1, 1),
            Facing::W => (-1, 0),
            Facing::Nw => (-1, -1),
        }
    }

    /// Hướng từ `from` tới `to`.
    pub fn towards(from: WorldPos, to: WorldPos) -> Facing {
        let dx = (to.x - from.x).signum();
        let dy = (to.y - from.y).signum();
        match (dx, dy) {
            (0, -1) => Facing::N,
            (1, -1) => Facing::Ne,
            (1, 0) => Facing::E,
            (1, 1) => Facing::Se,
            (0, 1) => Facing::S,
            (-1, 1) => Facing::Sw,
            (-1, 0) => Facing::W,
            (-1, -1) => Facing::Nw,
            // Cùng ô: giữ hướng bắc làm quy ước. Không có "hướng tới chính
            // mình", và trả về một `Option` ở đây sẽ làm mọi chỗ gọi phức tạp
            // hơn mà không giải quyết gì.
            _ => Facing::N,
        }
    }

    /// Số bậc lệch giữa hai hướng, `0`–`4`.
    pub fn arc_from(self, other: Facing) -> u8 {
        let a = self as i8;
        let b = other as i8;
        let d = (a - b).rem_euclid(8);
        u8::try_from(d.min(8 - d)).unwrap_or(4)
    }

    /// Đòn đánh từ hướng này có phải từ phía sau không.
    ///
    /// Lệch 3 hoặc 4 bậc, tức là hơn 90°. Đây là thứ khiến vòng ra sau lưng có
    /// giá trị chiến thuật thay vì chỉ là một chi tiết trang trí.
    pub fn is_behind(self, target_facing: Facing) -> bool {
        self.arc_from(target_facing) >= 3
    }
}

impl CanonicalHash for Facing {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(*self as i64);
    }
}

/// Mức che chắn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cover {
    /// Trống trải.
    None,
    /// Che một phần: bụi rậm, tường thấp.
    Partial,
    /// Che phần lớn: tường cao, ô cửa.
    Heavy,
    /// Che hoàn toàn — không bắn được.
    Full,
}

impl Cover {
    /// Giảm xác suất trúng bao nhiêu, thang phần trăm.
    pub fn miss_bonus_percent(self) -> i64 {
        match self {
            Cover::None => 0,
            Cover::Partial => 25,
            Cover::Heavy => 50,
            Cover::Full => 100,
        }
    }
}

impl CanonicalHash for Cover {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(*self as i64);
    }
}

/// Loại mặt nền, ảnh hưởng tốc độ và độ vững.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Footing {
    /// Vững.
    Solid,
    /// Lỏng: cát, sỏi.
    Loose,
    /// Trơn: băng, bùn.
    Slippery,
    /// Ngập nước.
    Waterlogged,
}

impl Footing {
    /// Hệ số tốc độ, thang phần trăm.
    pub fn speed_percent(self) -> i64 {
        match self {
            Footing::Solid => 100,
            Footing::Loose => 75,
            Footing::Slippery => 60,
            Footing::Waterlogged => 45,
        }
    }

    /// Có nguy cơ ngã khi đổi hướng gấp không.
    pub fn risks_falling(self) -> bool {
        matches!(self, Footing::Slippery | Footing::Loose)
    }
}

impl CanonicalHash for Footing {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(*self as i64);
    }
}

/// **Trần vật lý của tốc độ di chuyển** (`§10.10`).
///
/// Ô mỗi tick, dạng hữu tỉ `num/den`. Không nhân vật nào — không buff nào,
/// không phép nào — vượt được. Không có trần này thì mọi yếu tố chiến thuật
/// khác trở nên vô nghĩa: không ai vòng ra sau lưng được một người dịch chuyển
/// tức thời.
pub const MAX_MOVE_CELLS_PER_100_TICKS: i64 = 400;

/// Kẹp tốc độ vào trần vật lý.
pub fn clamp_move_speed(cells_per_100_ticks: i64) -> i64 {
    cells_per_100_ticks.clamp(0, MAX_MOVE_CELLS_PER_100_TICKS)
}

/// Tình huống chiến thuật của một đòn đánh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Engagement {
    /// Vị trí người đánh.
    pub from: WorldPos,
    /// Vị trí mục tiêu.
    pub to: WorldPos,
    /// Hướng mặt của mục tiêu.
    pub target_facing: Facing,
    /// Che chắn của mục tiêu.
    pub cover: Cover,
    /// Tầm với của vũ khí, tính bằng ô.
    pub reach: i64,
    /// Chênh cao, mét. Dương là người đánh ở cao hơn.
    pub elevation_delta: i64,
    /// Mặt nền của người đánh.
    pub footing: Footing,
    /// Số đồng đội đứng liền kề mục tiêu — đội hình.
    pub flanking_allies: u32,
}

/// Kết quả đánh giá một tình huống.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// Có với tới được không.
    pub in_reach: bool,
    /// Đánh từ sau lưng không.
    pub from_behind: bool,
    /// Điều chỉnh xác suất trúng, thang phần trăm. Dương là dễ trúng hơn.
    pub hit_modifier: i64,
    /// Từng phần đóng góp, để giải thích (`§18.13`).
    pub factors: Vec<(&'static str, i64)>,
}

/// Đánh giá một tình huống chiến thuật.
///
/// Trả về **từng phần đóng góp**, không chỉ tổng. Người chơi hỏi "vì sao tôi
/// trượt" và câu trả lời phải là một danh sách chứ không phải một con số.
pub fn assess(e: Engagement) -> Assessment {
    let khoang_cach = e.from.chebyshev_xy(e.to);
    let in_reach = khoang_cach <= i128::from(e.reach);

    let huong_danh = Facing::towards(e.from, e.to);
    // Đòn từ sau lưng: hướng mà người đánh tới **ngược** với hướng mục tiêu
    // đang nhìn.
    let from_behind = huong_danh.is_behind(e.target_facing);

    let mut factors: Vec<(&'static str, i64)> = Vec::new();

    if from_behind {
        factors.push(("từ sau lưng", 30));
    }
    let che = e.cover.miss_bonus_percent();
    if che > 0 {
        factors.push(("che chắn", -che));
    }
    if e.elevation_delta > 0 {
        // Cao hơn thì lợi, nhưng lợi ích bão hòa: đứng trên đồi tốt hơn đứng
        // dưới, nhưng đứng trên núi không tốt hơn đứng trên đồi bao nhiêu.
        factors.push(("cao hơn", (e.elevation_delta / 2).min(15)));
    } else if e.elevation_delta < 0 {
        factors.push(("thấp hơn", (e.elevation_delta / 2).max(-15)));
    }
    let nen = e.footing.speed_percent() - 100;
    if nen != 0 {
        factors.push(("mặt nền", nen / 4));
    }
    if e.flanking_allies > 0 {
        // Vây kín thì lợi, nhưng chỉ tới một mức: người thứ tư không chen vào
        // được nữa.
        factors.push(("vây", i64::from(e.flanking_allies.min(3)) * 10));
    }

    Assessment {
        in_reach,
        from_behind,
        hit_modifier: factors.iter().map(|(_, v)| v).sum(),
        factors,
    }
}

/// Có nguy cơ bắn nhầm đồng đội không (`§10.10`).
///
/// Đồng đội nằm **trên đường bắn** và gần người bắn hơn mục tiêu. Kiểm bằng
/// khoảng cách tới đoạn thẳng, không phải bằng "có trong hình nón" — hình nón
/// sẽ báo động giả với đồng đội đứng cạnh nhưng không cản đường.
pub fn friendly_fire_risk(from: WorldPos, to: WorldPos, allies: &[WorldPos]) -> Vec<WorldPos> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let do_dai_sq = dx * dx + dy * dy;
    if do_dai_sq == 0 {
        return Vec::new();
    }

    let mut ra: Vec<WorldPos> = allies
        .iter()
        .copied()
        .filter(|a| {
            let ax = a.x - from.x;
            let ay = a.y - from.y;
            // Chiếu lên đường bắn; phải nằm giữa người bắn và mục tiêu.
            let t = ax * dx + ay * dy;
            if t <= 0 || t >= do_dai_sq {
                return false;
            }
            // Khoảng cách vuông góc tới đường, bình phương, nhân độ dài².
            let cheo = ax * dy - ay * dx;
            // Trong vòng một ô: `cheo² <= do_dai_sq`.
            cheo * cheo <= do_dai_sq
        })
        .collect();
    // Sắp xếp để kết quả xác định — nó đi vào quyết định của AI.
    ra.sort_by_key(|p| (p.x, p.y, p.z));
    ra
}

/// Vùng kiểm soát: rút lui khỏi ô kề địch thì bị đòn miễn phí.
///
/// Đây là thứ khiến "rút lui" thành một quyết định có giá thay vì một nút bấm.
pub fn zone_of_control(at: WorldPos, enemies: &[WorldPos]) -> Vec<WorldPos> {
    let mut ra: Vec<WorldPos> = enemies
        .iter()
        .copied()
        .filter(|e| e.z == at.z && at.chebyshev_xy(*e) == 1)
        .collect();
    ra.sort_by_key(|p| (p.x, p.y, p.z));
    ra
}

/// Xác suất trúng cuối cùng, `[0,1]`.
///
/// Kẹp vào `[5%, 95%]`: không có đòn nào chắc chắn trúng và không có đòn nào
/// chắc chắn trượt. Cả hai cực đều làm chiến đấu mất tính bất định, và tính bất
/// định là thứ khiến người ta phải cân nhắc.
pub fn hit_chance(base_percent: i64, a: &Assessment) -> Unit {
    if !a.in_reach {
        return Unit::ZERO;
    }
    let p = (base_percent + a.hit_modifier).clamp(5, 95);
    Unit::from_frac(p, 100).unwrap_or(Unit::ZERO)
}
