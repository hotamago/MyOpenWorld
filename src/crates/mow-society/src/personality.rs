//! Tính cách năm lớp (`idea.md §9.9`, `PC-10`).
//!
//! ## Vì sao năm lớp chứ không phải một bảng chỉ số
//!
//! Một nhân vật với `hướng_ngoại: 7` là một con số. Một nhân vật với năm lớp là
//! một **người**, vì các lớp có thể mâu thuẫn nhau — và mâu thuẫn là nơi tính
//! cách trở nên thú vị.
//!
//! ```text
//! 5  tự sự      "tôi là người giữ lời"        ← có thể SAI về chính mình
//! 4  lâm sàng   ám ảnh, sang chấn, nghiện     ← có thể mâu thuẫn với 1 và 2
//! 3  cảm xúc    tâm trạng lúc này              ← thay đổi theo giờ
//! 2  giá trị    "trung thành quan trọng hơn thật thà"
//! 1  đặc điểm   hướng ngoại, tận tâm...        ← ổn định suốt đời
//! ```
//!
//! Một người *tin* mình giữ lời (lớp 5), *coi trọng* lòng trung thành hơn sự
//! thật (lớp 2), và *đang* hoảng sợ (lớp 3) sẽ nói dối để bảo vệ bạn — rồi
//! thành thật ngạc nhiên khi bị gọi là kẻ nói dối. Với một bảng chỉ số, cảnh đó
//! không diễn đạt được.
//!
//! ## Lấy mẫu **có tương quan**
//!
//! Đặc điểm không độc lập nhau trong thực tế. Một người rất tận tâm hiếm khi
//! rất bốc đồng. Lấy mẫu độc lập sẽ tạo ra những tổ hợp không tồn tại và làm
//! quần thể trông như nhiễu trắng thay vì như người.

use mow_math::{CanonicalHash, DetRng, StateHasher};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Năm đặc điểm nền, thang `0`–`1000`. Ổn định suốt đời.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Traits {
    /// Cởi mở với trải nghiệm mới.
    pub openness: u16,
    /// Tận tâm, có kỷ luật.
    pub conscientiousness: u16,
    /// Hướng ngoại.
    pub extraversion: u16,
    /// Dễ chịu, hợp tác.
    pub agreeableness: u16,
    /// Bất ổn cảm xúc.
    pub neuroticism: u16,
}

impl Traits {
    /// Lấy mẫu **có tương quan** từ một seed.
    ///
    /// Tương quan được cài bằng cách rút một trục chung rồi lệch quanh nó, thay
    /// vì rút năm số độc lập. Cụ thể: `conscientiousness` và `neuroticism`
    /// tương quan âm, `extraversion` và `agreeableness` tương quan dương nhẹ —
    /// đúng như quan sát thực nghiệm.
    pub fn sample(rng: &mut DetRng) -> Traits {
        let base = |rng: &mut DetRng| -> i32 {
            // Tổng ba lần rút cho phân phối hình chuông thay vì đều. Người ở
            // giữa phải nhiều hơn người ở cực, nếu không mọi nhân vật đều cực đoan.
            let a: i32 = rng.gen_range(0..334);
            let b: i32 = rng.gen_range(0..334);
            let c: i32 = rng.gen_range(0..334);
            a + b + c
        };

        let on_dinh = base(rng); // trục "ổn định – bất ổn"
        let xa_hoi = base(rng); // trục "hướng về người khác"

        let lech = |rng: &mut DetRng, quanh: i32, bien: i32| -> u16 {
            let d: i32 = rng.gen_range(-bien..=bien);
            u16::try_from((quanh + d).clamp(0, 1000)).unwrap_or(500)
        };

        Traits {
            openness: base(rng).clamp(0, 1000) as u16,
            conscientiousness: lech(rng, on_dinh, 200),
            extraversion: lech(rng, xa_hoi, 200),
            agreeableness: lech(rng, xa_hoi, 250),
            // Tương quan âm với tận tâm: lấy phần bù của trục ổn định.
            neuroticism: lech(rng, 1000 - on_dinh, 200),
        }
    }
}

impl CanonicalHash for Traits {
    fn canonical_hash(&self, h: &mut StateHasher) {
        for v in [
            self.openness,
            self.conscientiousness,
            self.extraversion,
            self.agreeableness,
            self.neuroticism,
        ] {
            h.write_u64(u64::from(v));
        }
    }
}

/// Giá trị mà nhân vật coi trọng, **theo thứ tự ưu tiên**.
///
/// Thứ tự là toàn bộ nội dung: ai cũng coi trọng cả trung thành lẫn thật thà,
/// và tính cách nằm ở chỗ khi hai thứ đó xung đột thì bỏ cái nào.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Values {
    ordered: Vec<String>,
}

impl Values {
    /// Dựng từ danh sách đã sắp theo ưu tiên.
    pub fn new(ordered: impl IntoIterator<Item = String>) -> Values {
        Values {
            ordered: ordered.into_iter().collect(),
        }
    }

    /// Giá trị nào thắng khi hai giá trị xung đột.
    ///
    /// Trả `None` nếu nhân vật không coi trọng cái nào trong hai — và khi đó họ
    /// sẽ quyết định bằng thứ khác, không phải bằng giá trị.
    pub fn resolve<'a>(&self, a: &'a str, b: &'a str) -> Option<&'a str> {
        let ia = self.ordered.iter().position(|v| v == a);
        let ib = self.ordered.iter().position(|v| v == b);
        match (ia, ib) {
            (Some(x), Some(y)) => Some(if x <= y { a } else { b }),
            (Some(_), None) => Some(a),
            (None, Some(_)) => Some(b),
            (None, None) => None,
        }
    }

    /// Mức ưu tiên; nhỏ hơn là quan trọng hơn.
    pub fn rank(&self, v: &str) -> Option<usize> {
        self.ordered.iter().position(|x| x == v)
    }

    /// Danh sách theo thứ tự.
    pub fn list(&self) -> &[String] {
        &self.ordered
    }
}

impl CanonicalHash for Values {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.ordered.iter(), |hh, v| {
            hh.write_str(v);
        });
    }
}

/// Trạng thái cảm xúc lúc này. Thay đổi theo giờ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Affect {
    /// Dễ chịu – khó chịu, `-1000`..`1000`.
    pub valence: i16,
    /// Kích thích, `0`..`1000`.
    pub arousal: i16,
    /// Cảm giác kiểm soát được tình hình, `0`..`1000`.
    pub control: i16,
}

impl CanonicalHash for Affect {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(i64::from(self.valence));
        h.write_i64(i64::from(self.arousal));
        h.write_i64(i64::from(self.control));
    }
}

/// Tình trạng lâm sàng — thứ **mâu thuẫn** với các lớp khác.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Clinical {
    /// Các ám ảnh, sang chấn, nghiện đang có.
    pub conditions: Vec<String>,
}

impl Clinical {
    /// Có tình trạng này không.
    pub fn has(&self, c: &str) -> bool {
        self.conditions.iter().any(|x| x == c)
    }
}

impl CanonicalHash for Clinical {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.conditions.iter(), |hh, c| {
            hh.write_str(c);
        });
    }
}

/// Nhân vật **tự kể về mình** — và có thể sai.
///
/// Đây là lớp mà một bảng chỉ số không có, và là lớp làm nên phần lớn kịch
/// tính: khoảng cách giữa "tôi là người thế nào" và "tôi hành xử thế nào".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SelfNarrative {
    /// Những điều nhân vật tin về bản thân.
    pub claims: Vec<String>,
}

impl CanonicalHash for SelfNarrative {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.claims.iter(), |hh, c| {
            hh.write_str(c);
        });
    }
}

/// Năm đặc điểm, dưới dạng tên trường — để một thay đổi chỉ vào được đúng chỗ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitField {
    /// [`Traits::openness`].
    Openness,
    /// [`Traits::conscientiousness`].
    Conscientiousness,
    /// [`Traits::extraversion`].
    Extraversion,
    /// [`Traits::agreeableness`].
    Agreeableness,
    /// [`Traits::neuroticism`].
    Neuroticism,
}

impl TraitField {
    /// Tên ổn định, dùng trong event và UI.
    pub fn as_str(self) -> &'static str {
        match self {
            TraitField::Openness => "openness",
            TraitField::Conscientiousness => "conscientiousness",
            TraitField::Extraversion => "extraversion",
            TraitField::Agreeableness => "agreeableness",
            TraitField::Neuroticism => "neuroticism",
        }
    }
}

impl CanonicalHash for TraitField {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(*self as i64);
    }
}

/// Vì sao một tính cách đổi (`§20.11.4`).
///
/// Danh sách này **đóng** có chủ đích. Nó là toàn bộ tập những lý do hợp lệ để
/// một người thay đổi, và một thay đổi không thuộc tập này thì theo định nghĩa
/// là trôi persona — tức là bug, chứ không phải nhân vật đang phát triển.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CauseKind {
    /// Sang chấn.
    Trauma,
    /// Cải đạo, đổi niềm tin.
    Conversion,
    /// Nghiện.
    Addiction,
    /// Lời thề.
    Oath,
    /// Effect điều khiển tâm trí.
    MindControl,
    /// Tuổi tác.
    Aging,
}

/// Con trỏ tới sự kiện đã gây ra thay đổi.
///
/// Không có trường nào cho "lý do dạng văn bản tự do". Một chuỗi mô tả nghe thì
/// đủ, nhưng nó không truy ngược được: `§20.11.4` đòi *cause chain*, và cause
/// chain cần một `event_seq` có thật để nhảy tới.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CauseRef {
    /// Sự kiện trong event log.
    pub event_seq: u64,
    /// Loại nguyên nhân.
    pub kind: CauseKind,
}

impl CanonicalHash for CauseRef {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.event_seq);
        h.write_i64(self.kind as i64);
    }
}

/// Một lần tính cách đổi, đã ghi nhận nguyên nhân.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitChange {
    /// Lúc nào.
    pub at_tick: u64,
    /// Đặc điểm nào.
    pub field: TraitField,
    /// Đổi bao nhiêu.
    pub delta: i16,
    /// Vì sao.
    pub cause: CauseRef,
}

impl CanonicalHash for TraitChange {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.at_tick);
        self.field.canonical_hash(h);
        h.write_i64(i64::from(self.delta));
        self.cause.canonical_hash(h);
    }
}

/// Tính cách đầy đủ, năm lớp.
///
/// ## Vì sao `traits` không phải trường công khai
///
/// `§20.11.4` đặt ra một bất biến nghe rất gọn:
///
/// > **Mọi thay đổi tính cách phải truy được về một sự kiện.**
///
/// Một bất biến kiểu đó, viết trong tài liệu, sẽ bị vi phạm ở tuần thứ ba — vì
/// ai đó cần "chỉnh nhanh một chỉ số cho cảnh này". Nên nó được thực thi bằng
/// trình biên dịch, giống hệt cách `§22.1` bảo vệ đường ghi state: đặc điểm chỉ
/// đọc được qua [`Personality::traits`], và chỉ đổi được qua
/// [`Personality::apply_change`], hàm này **bắt buộc** có [`CauseRef`].
///
/// Nhờ vậy [`crate::drift`] kiểm được một điều mạnh hơn hẳn "hành vi có vẻ lệch
/// tính cách": nó kiểm rằng **đặc điểm lúc sinh cộng toàn bộ thay đổi đã ghi
/// bằng đúng đặc điểm hiện tại**. Lệch một đơn vị nghĩa là có người đã ghi tắt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Personality {
    /// Lớp 1 lúc sinh ra — **không bao giờ đổi**. Đây là mỏ neo mà `§20.11.1`
    /// nói tới, dưới dạng một trường.
    birth: Traits,
    /// Lớp 1 hiện tại.
    traits: Traits,
    /// Nhật ký thay đổi, mỗi mục có nguyên nhân.
    history: Vec<TraitChange>,
    /// Lớp 2.
    pub values: Values,
    /// Lớp 3.
    pub affect: Affect,
    /// Lớp 4.
    pub clinical: Clinical,
    /// Lớp 5.
    pub narrative: SelfNarrative,
}

impl Personality {
    /// Lấy mẫu một tính cách.
    pub fn sample(rng: &mut DetRng) -> Personality {
        Personality::from_traits(Traits::sample(rng))
    }

    /// Dựng từ đặc điểm cho sẵn — dùng cho worldseed và cho test.
    pub fn from_traits(traits: Traits) -> Personality {
        Personality {
            birth: traits,
            traits,
            history: Vec::new(),
            values: Values::default(),
            affect: Affect::default(),
            clinical: Clinical::default(),
            narrative: SelfNarrative::default(),
        }
    }

    /// Đặc điểm hiện tại. **Chỉ đọc.**
    pub fn traits(&self) -> &Traits {
        &self.traits
    }

    /// Đặc điểm lúc sinh ra.
    pub fn birth_traits(&self) -> &Traits {
        &self.birth
    }

    /// Toàn bộ thay đổi đã ghi nhận.
    pub fn history(&self) -> &[TraitChange] {
        &self.history
    }

    /// **Đường duy nhất** để một tính cách thay đổi.
    ///
    /// `cause` không phải tham số tùy chọn và sẽ không bao giờ trở thành tùy
    /// chọn: nó là thứ phân biệt "nhân vật đang phát triển" với "model đã quên
    /// nhân vật là ai", và không có nó thì hai trường hợp đó không phân biệt
    /// được — kể cả bằng mắt người.
    pub fn apply_change(&mut self, at_tick: u64, field: TraitField, delta: i16, cause: CauseRef) {
        let o = |v: u16| -> u16 { (i32::from(v) + i32::from(delta)).clamp(0, 1000) as u16 };
        match field {
            TraitField::Openness => self.traits.openness = o(self.traits.openness),
            TraitField::Conscientiousness => {
                self.traits.conscientiousness = o(self.traits.conscientiousness);
            }
            TraitField::Extraversion => self.traits.extraversion = o(self.traits.extraversion),
            TraitField::Agreeableness => self.traits.agreeableness = o(self.traits.agreeableness),
            TraitField::Neuroticism => self.traits.neuroticism = o(self.traits.neuroticism),
        }
        self.history.push(TraitChange {
            at_tick,
            field,
            delta,
            cause,
        });
    }

    /// Đặc điểm lúc sinh **cộng toàn bộ thay đổi đã ghi** có bằng đặc điểm hiện
    /// tại không.
    ///
    /// `false` nghĩa là có ai đó đã ghi tắt vào `traits` mà không đi qua
    /// [`Personality::apply_change`] — với `traits` là trường riêng, điều đó chỉ
    /// xảy ra được từ trong crate này, hoặc qua một bản deserialize đã bị sửa.
    /// Cả hai đều là thứ `§20.11.4` gọi là bug, và cả hai đều im lặng nếu không
    /// có phép kiểm này.
    pub fn history_explains_current(&self) -> bool {
        let mut t = self.birth;
        for c in &self.history {
            let o = |v: u16| -> u16 { (i32::from(v) + i32::from(c.delta)).clamp(0, 1000) as u16 };
            match c.field {
                TraitField::Openness => t.openness = o(t.openness),
                TraitField::Conscientiousness => t.conscientiousness = o(t.conscientiousness),
                TraitField::Extraversion => t.extraversion = o(t.extraversion),
                TraitField::Agreeableness => t.agreeableness = o(t.agreeableness),
                TraitField::Neuroticism => t.neuroticism = o(t.neuroticism),
            }
        }
        t == self.traits
    }

    /// Những chỗ mà **lời tự kể mâu thuẫn với đặc điểm thật**.
    ///
    /// Đây là dữ liệu cho `PC-13` (chống trôi persona): một nhân vật tin mình
    /// dũng cảm mà có `neuroticism` rất cao là một mâu thuẫn có thật và có
    /// nghĩa. Auditor dùng nó để phân biệt "nhân vật đang mâu thuẫn nội tâm"
    /// với "mô hình đã quên nhân vật là ai".
    pub fn self_contradictions(&self) -> Vec<&str> {
        let mut ra = Vec::new();
        for c in &self.narrative.claims {
            let mau_thuan = match c.as_str() {
                "dũng cảm" => self.traits.neuroticism > 750,
                "giữ lời" => self.traits.conscientiousness < 250,
                "hòa nhã" => self.traits.agreeableness < 250,
                "cởi mở" => self.traits.openness < 250,
                _ => false,
            };
            if mau_thuan {
                ra.push(c.as_str());
            }
        }
        ra
    }
}

impl CanonicalHash for Personality {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.birth.canonical_hash(h);
        self.traits.canonical_hash(h);
        h.write_seq(self.history.iter(), |hh, c| c.canonical_hash(hh));
        self.values.canonical_hash(h);
        self.affect.canonical_hash(h);
        self.clinical.canonical_hash(h);
        self.narrative.canonical_hash(h);
    }
}
