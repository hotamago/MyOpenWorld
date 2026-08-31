//! Tri giác — **nguồn duy nhất** của observation (`idea.md §10.2`, `§22.4`).
//!
//! > Entity chỉ ra quyết định từ observation/belief hợp lệ; reference ngoài
//! > cognition context **không có hiệu lực**.
//!
//! ## Vì sao đây là ranh giới quan trọng nhất của toàn hệ thống
//!
//! Một thế giới mà NPC biết mọi thứ không phải một thế giới sống động — nó là
//! một bảng tính có kịch bản. Mọi thứ thú vị trong `idea.md` phụ thuộc vào việc
//! nhân vật **không biết**:
//!
//! - Tin đồn lan được vì người ta không kiểm chứng được.
//! - Trộm cắp có nghĩa vì không phải ai cũng thấy.
//! - Chẩn đoán sai xảy ra vì triệu chứng không nói ra nguyên nhân.
//! - Điều tra là một hoạt động vì sự thật phải được tìm ra.
//!
//! Nên [`Observation`] là kiểu **duy nhất** mà lớp quyết định nhận vào. Không
//! có hàm nào cho một thực thể đọc thẳng `Store`, và đó là ranh giới được thi
//! hành bằng chữ ký hàm chứ không bằng kỷ luật.
//!
//! ## Tri giác không phải là "thấy hay không thấy"
//!
//! Nó có **độ tin cậy**. Thấy loáng thoáng trong sương mù khác với nhìn rõ ban
//! ngày, và cả hai đều khác với nghe người khác kể lại. [`Observation::fidelity`]
//! mang thông tin đó, và lớp quyết định phải xử lý nó — nếu bỏ qua, thì một
//! bóng người trong sương sẽ được xử lý y hệt một khuôn mặt quen.

use mow_core::{EntityId, Store, Tick};
use mow_math::{CanonicalHash, StateHasher, Unit, WorldPos};
use serde::{Deserialize, Serialize};

/// Kênh giác quan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sense {
    /// Nhìn. Chặn bởi vật cản, giảm bởi bóng tối và sương.
    Sight,
    /// Nghe. Xuyên tường được, nhưng không định vị chính xác.
    Hearing,
    /// Ngửi. Chậm, dai, và đi theo gió.
    Smell,
    /// Chạm.
    Touch,
    /// Giác quan phép thuật. Loài nào có thì có, không phải mặc định.
    MagicSense,
}

impl Sense {
    /// Có xuyên qua vật cản được không.
    pub fn penetrates_cover(self) -> bool {
        matches!(self, Sense::Hearing | Sense::Smell | Sense::MagicSense)
    }

    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            Sense::Sight => "sight",
            Sense::Hearing => "hearing",
            Sense::Smell => "smell",
            Sense::Touch => "touch",
            Sense::MagicSense => "magic_sense",
        }
    }
}

impl CanonicalHash for Sense {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

/// Khả năng tri giác của một thực thể.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Senses {
    /// Kênh nào có, và tầm bao xa, tính bằng ô.
    pub ranges: Vec<(Sense, i64)>,
    /// Kỹ năng nhận biết dấu hiệu khó — dùng với `Effect::signs_for`.
    pub acuity: Unit,
}

impl Default for Senses {
    fn default() -> Self {
        Senses {
            ranges: vec![(Sense::Sight, 24), (Sense::Hearing, 40), (Sense::Touch, 1)],
            acuity: Unit::from_frac(1, 2).unwrap_or(Unit::ZERO),
        }
    }
}

impl Senses {
    /// Tầm của một kênh, `0` nếu không có kênh đó.
    pub fn range_of(&self, s: Sense) -> i64 {
        self.ranges
            .iter()
            .find(|(k, _)| *k == s)
            .map_or(0, |(_, r)| *r)
    }

    /// Có kênh này không.
    pub fn has(&self, s: Sense) -> bool {
        self.range_of(s) > 0
    }
}

impl CanonicalHash for Senses {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.ranges.iter(), |hh, (s, r)| {
            s.canonical_hash(hh);
            hh.write_i64(*r);
        });
        h.write_i64(self.acuity.get().raw());
    }
}

/// Một thứ mà thực thể **đã nhận biết được**.
///
/// Chú ý những gì **không** có ở đây: không có `EntityId` của thứ được quan sát
/// khi người quan sát chưa nhận ra đó là ai. Một bóng người trong sương là một
/// `Observation` không có danh tính, và lớp quyết định không có cách nào lấy
/// được danh tính đó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// Kênh nào bắt được.
    pub sense: Sense,
    /// Vị trí **ước lượng**, không phải vị trí thật.
    ///
    /// Nghe thấy tiếng động thì biết hướng, không biết chính xác ô nào. Ước
    /// lượng chứ không phải sự thật là điều khiến rình rập và trốn có ý nghĩa.
    pub at: WorldPos,
    /// Danh tính, **nếu nhận ra được**.
    ///
    /// `None` nghĩa là "có ai đó" chứ không phải "không có ai".
    pub identity: Option<EntityId>,
    /// Dấu hiệu quan sát được — triệu chứng, không phải nguyên nhân.
    pub signs: Vec<String>,
    /// Độ tin cậy, `[0,1]`.
    pub fidelity: Unit,
    /// Tick lúc quan sát.
    pub at_tick: Tick,
}

impl CanonicalHash for Observation {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.sense.canonical_hash(h);
        self.at.canonical_hash(h);
        self.identity.canonical_hash(h);
        h.write_seq(self.signs.iter(), |hh, s| {
            hh.write_str(s);
        });
        h.write_i64(self.fidelity.get().raw());
        self.at_tick.canonical_hash(h);
    }
}

/// Điều kiện môi trường ảnh hưởng tri giác.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conditions {
    /// Mức sáng, `[0,1]`.
    pub light: Unit,
    /// Mức che khuất do sương, khói, mưa, `[0,1]`.
    pub obscurity: Unit,
    /// Mức ồn nền, `[0,1]`.
    pub noise: Unit,
}

impl Default for Conditions {
    fn default() -> Self {
        Conditions {
            light: Unit::ONE,
            obscurity: Unit::ZERO,
            noise: Unit::ZERO,
        }
    }
}

/// Ngưỡng độ tin cậy để nhận ra danh tính.
///
/// Dưới ngưỡng này thì quan sát được là "có ai đó", không phải "là Aren". Con
/// số này là một quyết định gameplay: cao quá thì cải trang vô dụng, thấp quá
/// thì không ai nhận ra ai.
pub const IDENTIFY_THRESHOLD_PERCENT: i64 = 60;

/// Quan sát thế giới từ vị trí của một thực thể.
///
/// **Hàm duy nhất** biến `Store` thành thứ mà lớp quyết định đọc được. Nó nhận
/// `&Store` nhưng trả về `Vec<Observation>` — nên một khi đã qua đây, không còn
/// đường nào quay lại world truth.
pub fn observe(
    store: &Store,
    observer: EntityId,
    senses: &Senses,
    cond: Conditions,
    now: Tick,
) -> Vec<Observation> {
    let Some(tu) = position_of(store, observer) else {
        return Vec::new();
    };

    let mut ra = Vec::new();
    for id in store.ids() {
        if id == observer {
            continue;
        }
        let Some(den) = position_of(store, id) else {
            continue;
        };
        if den.z != tu.z {
            // Khác tầng: chỉ nghe và ngửi xuyên được, và cả hai đều rất kém.
            continue;
        }
        let d = tu.chebyshev_xy(den);

        for (sense, tam) in &senses.ranges {
            if d > i128::from(*tam) {
                continue;
            }
            let Some(o) = qua_kenh(store, id, den, *sense, d, *tam, cond, senses.acuity, now)
            else {
                continue;
            };
            ra.push(o);
        }
    }

    // Sắp xếp để kết quả xác định — nó chảy thẳng vào prompt, nên thứ tự là một
    // phần của thế giới.
    ra.sort_by(|a, b| {
        a.sense
            .cmp(&b.sense)
            .then((a.at.x, a.at.y).cmp(&(b.at.x, b.at.y)))
            .then(a.identity.cmp(&b.identity))
    });
    ra
}

fn position_of(store: &Store, id: EntityId) -> Option<WorldPos> {
    Some(WorldPos::new(
        store.attr_int(id, "core.pos.x")?,
        store.attr_int(id, "core.pos.y")?,
        store.attr_int(id, "core.pos.z").unwrap_or(0),
    ))
}

#[allow(clippy::too_many_arguments)]
fn qua_kenh(
    store: &Store,
    target: EntityId,
    at: WorldPos,
    sense: Sense,
    d: i128,
    tam: i64,
    cond: Conditions,
    acuity: Unit,
    now: Tick,
) -> Option<Observation> {
    // Độ tin cậy giảm theo khoảng cách: gần thì rõ, xa thì mờ.
    let gan = Unit::from_frac(
        (i64::try_from(i128::from(tam) - d).ok()?).max(0),
        tam.max(1),
    )
    .ok()?;

    let fidelity = match sense {
        // Nhìn: cần sáng, bị sương cản.
        Sense::Sight => gan.and(cond.light).and(cond.obscurity.complement()),
        // Nghe: bị ồn nền át, không cần sáng. Xuyên tường được nhưng định vị kém.
        Sense::Hearing => gan.and(cond.noise.complement()),
        // Ngửi: không phụ thuộc sáng tối; đây là lý do chó dẫn đường có giá trị.
        // Khứu giác và cảm ứng ma thuật cùng suy giảm theo khoảng cách, và
        // chúng giống nhau **vì cùng một lý do vật lý** chứ không phải ngẫu
        // nhiên trùng — nên gộp một nhánh là đúng, không phải là mất thông tin.
        Sense::Smell | Sense::MagicSense => gan,
        Sense::Touch => Unit::ONE,
    };

    if fidelity == Unit::ZERO {
        return None;
    }

    // Nhận ra danh tính chỉ khi đủ rõ. Dưới ngưỡng là "có ai đó".
    let phan_tram = fidelity.get().raw() * 100 / mow_math::fixed::ONE_RAW;
    let identity = if phan_tram >= IDENTIFY_THRESHOLD_PERCENT && sense == Sense::Sight {
        Some(target)
    } else {
        None
    };

    // Vị trí ước lượng: nghe thì lệch, nhìn thì chính xác.
    let at = if sense == Sense::Hearing {
        // Làm tròn về lưới 4 ô — biết hướng, không biết chính xác ô nào.
        WorldPos::new(at.x.div_euclid(4) * 4, at.y.div_euclid(4) * 4, at.z)
    } else {
        at
    };

    // Dấu hiệu: chỉ những gì kênh này bắt được và kỹ năng đủ để nhận ra.
    let signs = store
        .attrs(target)
        .map(|a| {
            a.keys()
                .filter_map(|k| k.strip_prefix(&format!("sign.{}.", sense.as_str())))
                .filter(|_| acuity > Unit::ZERO)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    Some(Observation {
        sense,
        at,
        identity,
        signs,
        fidelity,
        at_tick: now,
    })
}

/// Ngữ cảnh nhận thức: **tất cả** những gì một thực thể được phép dùng để quyết định.
///
/// `§22.4`: reference ngoài ngữ cảnh này không có hiệu lực. Struct này là ranh
/// giới đó, dưới dạng một kiểu — và nó **không chứa** `&Store`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CognitionContext {
    /// Ai đang nghĩ.
    pub self_id: EntityId,
    /// Tick hiện tại.
    pub now: Tick,
    /// Những gì quan sát được ngay lúc này.
    pub observations: Vec<Observation>,
    /// Những hành động mà thực thể **biết** làm.
    pub known_actions: Vec<String>,
    /// Trạng thái bên trong: đói, đau, mệt.
    pub internal: Vec<(String, i64)>,
}

impl CognitionContext {
    /// Một quan sát có nằm trong ngữ cảnh này không.
    ///
    /// Validator gọi hàm này để từ chối mọi tham chiếu bịa ra (`§22.4`). Một mô
    /// hình sẽ nhắc tới những thứ nó chưa thấy, một cách thuyết phục.
    pub fn contains_observation(&self, o: &Observation) -> bool {
        self.observations.contains(o)
    }

    /// Thực thể có biết hành động này không.
    pub fn knows_action(&self, id: &str) -> bool {
        self.known_actions.iter().any(|a| a == id)
    }

    /// Những thực thể mà người này **nhận ra được** ngay lúc này.
    ///
    /// Chỉ những quan sát đủ rõ để có danh tính. Một bóng người trong sương
    /// không xuất hiện ở đây, và đó là điểm mấu chốt.
    pub fn identified(&self) -> Vec<EntityId> {
        let mut v: Vec<EntityId> = self
            .observations
            .iter()
            .filter_map(|o| o.identity)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

impl CanonicalHash for CognitionContext {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.self_id.canonical_hash(h);
        self.now.canonical_hash(h);
        h.write_seq(self.observations.iter(), |hh, o| o.canonical_hash(hh));
        h.write_seq(self.known_actions.iter(), |hh, a| {
            hh.write_str(a);
        });
        h.write_seq(self.internal.iter(), |hh, (k, v)| {
            hh.write_str(k);
            hh.write_i64(*v);
        });
    }
}
