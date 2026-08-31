//! Trao đổi xã hội và volition (`idea.md §10.12`, `PC-12`).
//!
//! > Volition tính bằng **quy tắc trên social state**; LLM chọn ý định, engine
//! > tính kết quả.
//!
//! ## Ranh giới này là gì và vì sao nó ở đúng chỗ đó
//!
//! Cám dỗ tự nhiên: hỏi mô hình *"Aren có đồng ý cho Bram mượn rìu không?"* và
//! làm theo câu trả lời. Nó cho ra đối thoại hay và hỏng theo bốn cách:
//!
//! 1. **Không nhất quán.** Cùng một tình huống, hỏi hai lần, hai câu trả lời.
//!    Người chơi không học được luật nào cả.
//! 2. **Không replay được.** Kết quả phụ thuộc mô hình, và mô hình thay đổi.
//! 3. **Không dùng được ở LOD thấp.** Chín nghìn dân không thể mỗi người một
//!    lời gọi mỗi lần có ai hỏi mượn gì.
//! 4. **Thao túng được bằng lời.** Một người chơi viết đúng câu là được mọi
//!    thứ, bất kể quan hệ thật ra sao.
//!
//! Nên ranh giới là: **LLM chọn *làm gì*, engine tính *có được không*.** Aren
//! quyết định *hỏi mượn*; việc Bram có cho hay không là một phép tính trên
//! quan hệ, nợ nần, và rủi ro — thứ mà cả hai bên đều nhìn thấy được.

use crate::reputation::Reputation;
use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Loại trao đổi xã hội.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExchangeKind {
    /// Xin giúp đỡ.
    Request,
    /// Tặng cho.
    Gift,
    /// Đe dọa.
    Threat,
    /// Chia sẻ thông tin.
    Confide,
    /// Nhờ vả có đi có lại.
    Bargain,
    /// Xin lỗi.
    Apologize,
}

impl ExchangeKind {
    /// Nó thay đổi quan hệ theo hướng nào nếu **thành công**.
    pub fn on_success(self) -> i16 {
        match self {
            ExchangeKind::Request => 5,
            ExchangeKind::Gift => 40,
            // Đe dọa **thành công** vẫn làm hỏng quan hệ. Đó là toàn bộ cái giá
            // của nó, và một hệ thống chỉ tính "được việc hay không" sẽ khiến đe
            // dọa luôn là lựa chọn tốt nhất.
            ExchangeKind::Threat => -60,
            ExchangeKind::Confide => 30,
            ExchangeKind::Bargain => 10,
            ExchangeKind::Apologize => 25,
        }
    }

    /// Nó thay đổi quan hệ theo hướng nào nếu **bị từ chối**.
    pub fn on_refusal(self) -> i16 {
        match self {
            ExchangeKind::Request => -5,
            ExchangeKind::Gift => -20,
            ExchangeKind::Threat => -80,
            ExchangeKind::Confide => -15,
            ExchangeKind::Bargain => -3,
            // Xin lỗi bị từ chối không làm tệ hơn — nó vốn đã tệ rồi.
            ExchangeKind::Apologize => 0,
        }
    }

    /// **Ai trả cái giá.**
    ///
    /// Trường này tồn tại vì một mô hình chỉ có một con số `cost` sẽ hiểu sai
    /// món quà: nếu cái giá luôn do người nhận trả, thì tặng một món càng quý
    /// càng dễ bị từ chối — càng hào phóng càng bị cự tuyệt. Đó là điều ngược
    /// đời, và nó không lộ ra cho tới khi có ai đó thử tặng quà.
    ///
    /// Xin, mặc cả, đe dọa: **người nhận** trả. Tặng, tâm sự, xin lỗi: **người
    /// đề nghị** trả, và cái giá đó trở thành lý do để người nhận đồng ý.
    pub fn payer(self) -> Payer {
        match self {
            ExchangeKind::Request | ExchangeKind::Bargain | ExchangeKind::Threat => {
                Payer::Responder
            }
            ExchangeKind::Gift | ExchangeKind::Confide | ExchangeKind::Apologize => Payer::Proposer,
        }
    }
}

/// Bên nào chịu cái giá của một trao đổi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Payer {
    /// Người đề nghị trả — tặng quà, tâm sự, xin lỗi.
    Proposer,
    /// Người nhận trả — xin xỏ, mặc cả, đe dọa.
    Responder,
}

impl CanonicalHash for ExchangeKind {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(*self as i64);
    }
}

/// Quan hệ giữa hai người.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bond {
    /// Thiện cảm, `-1000`..`1000`.
    pub affinity: i16,
    /// **Nợ nghĩa**: dương nghĩa là `a` nợ `b`.
    ///
    /// Đây là trường làm nên có đi có lại. Không có nó, giúp đỡ là một hành
    /// động mất mát thuần túy và không ai có lý do làm — trừ khi thiện cảm đủ
    /// cao, và lúc đó xã hội chỉ có bạn thân với người dưng.
    pub obligation: i16,
    /// Tin tưởng, `0`..`1000`.
    pub trust: u16,
}

impl CanonicalHash for Bond {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(i64::from(self.affinity));
        h.write_i64(i64::from(self.obligation));
        h.write_u64(u64::from(self.trust));
    }
}

/// Trạng thái xã hội của một thế giới.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialState {
    /// `(a, b)` → quan hệ **có hướng** từ `a` tới `b`.
    ///
    /// Có hướng vì quan hệ thường không đối xứng: Aren quý Bram nhiều hơn Bram
    /// quý Aren là một tình huống rất thường, và là nguồn của kịch tính.
    bonds: BTreeMap<(u64, u64), Bond>,
}

impl SocialState {
    /// Rỗng.
    pub fn new() -> SocialState {
        SocialState::default()
    }

    /// Quan hệ từ `a` tới `b`.
    pub fn bond(&self, a: u64, b: u64) -> Bond {
        self.bonds.get(&(a, b)).copied().unwrap_or_default()
    }

    /// Đặt quan hệ.
    pub fn set_bond(&mut self, a: u64, b: u64, bond: Bond) {
        self.bonds.insert((a, b), bond);
    }

    /// Điều chỉnh quan hệ.
    pub fn adjust(&mut self, a: u64, b: u64, d_affinity: i16, d_obligation: i16) {
        let e = self.bonds.entry((a, b)).or_default();
        e.affinity = e.affinity.saturating_add(d_affinity).clamp(-1000, 1000);
        e.obligation = e.obligation.saturating_add(d_obligation).clamp(-1000, 1000);
    }

    /// Số quan hệ.
    pub fn len(&self) -> usize {
        self.bonds.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.bonds.is_empty()
    }

    /// Quan hệ có đối xứng không — dùng cho chẩn đoán và cho kịch tính.
    pub fn asymmetry(&self, a: u64, b: u64) -> i16 {
        self.bond(a, b)
            .affinity
            .saturating_sub(self.bond(b, a).affinity)
    }
}

impl CanonicalHash for SocialState {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.bonds.iter(), |hh, ((a, b), bond)| {
            hh.write_u64(*a);
            hh.write_u64(*b);
            bond.canonical_hash(hh);
        });
    }
}

/// Một đề nghị trao đổi. **LLM chọn cái này.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    /// Ai đề nghị.
    pub from: u64,
    /// Với ai.
    pub to: u64,
    /// Loại.
    pub kind: ExchangeKind,
    /// Độ lớn của cái được trao, `0`–`1000`. Xin một ngụm nước thì nhỏ; xin
    /// một căn nhà thì lớn.
    ///
    /// **Ai chịu nó là do [`ExchangeKind::payer`] quyết**, không phải do trường
    /// này. Cùng một con số 600 nghĩa là "đòi hỏi quá đáng" với một lời xin và
    /// "món quà hậu hĩnh" với một món quà.
    pub cost: u16,
}

/// Ý chí đáp lại. **Engine tính cái này.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volition {
    /// Có đồng ý không.
    pub accepts: bool,
    /// Điểm sẵn lòng; dương là đồng ý.
    pub score: i64,
    /// Từng phần đóng góp, để giải thích (`§18.13`).
    ///
    /// Đây là thứ khiến trao đổi xã hội **học được**: người chơi thấy *"thiện
    /// cảm +40, nợ nghĩa +30, cái giá −80"* và hiểu ra rằng làm ơn trước sẽ
    /// giúp lần sau được đồng ý.
    pub factors: Vec<(&'static str, i64)>,
}

/// Tính ý chí đáp lại một đề nghị.
///
/// **Hàm thuần** của social state, danh tiếng và đề nghị. Không có ngẫu nhiên,
/// không có lời gọi mô hình — nên cùng tình huống luôn cho cùng kết quả, và
/// người chơi học được luật.
pub fn volition(ex: &Exchange, social: &SocialState, rep: &Reputation) -> Volition {
    let b = social.bond(ex.to, ex.from);
    let mut factors: Vec<(&'static str, i64)> = Vec::new();

    factors.push(("thiện cảm", i64::from(b.affinity) / 10));

    // Nợ nghĩa: nếu người nhận đang nợ người đề nghị thì họ dễ đồng ý hơn.
    // `bond(to, from).obligation > 0` nghĩa là `to` nợ `from`.
    if b.obligation > 0 {
        factors.push(("đang mắc nợ", i64::from(b.obligation) / 5));
    } else if b.obligation < 0 {
        factors.push(("đã được nợ", i64::from(b.obligation) / 20));
    }

    factors.push(("tin tưởng", i64::from(b.trust) / 20));
    match ex.kind.payer() {
        // Người nhận phải móc túi ra: càng lớn càng khó đồng ý.
        Payer::Responder => factors.push(("cái giá", -i64::from(ex.cost) / 5)),
        // Người đề nghị móc túi ra: càng lớn càng dễ được nhận. Nhưng chỉ tính
        // một nửa hệ số — một món quà quá hậu từ người lạ làm người ta nghi,
        // chứ không làm người ta mừng gấp đôi.
        Payer::Proposer => factors.push(("được nhận", i64::from(ex.cost) / 10)),
    }

    // Danh tiếng về sự đáng tin: nếu người nhận tin người đề nghị là kẻ lừa
    // đảo thì họ từ chối, dù thiện cảm cao.
    if let Some(honest) = rep.get(ex.to, ex.from, "honesty") {
        // Nhân với độ chắc chắn: một nghi ngờ mơ hồ ảnh hưởng ít hơn một điều
        // đã thấy tận mắt.
        let anh_huong = i64::from(honest.value) * i64::from(honest.confidence) / 1000 / 10;
        factors.push(("danh tiếng", anh_huong));
    }

    // Đe dọa hoạt động theo một logic khác hẳn: nó không hỏi thiện cảm mà hỏi
    // sợ hãi. Ở đây, sợ hãi xấp xỉ bằng "thiếu tin tưởng cộng cái giá thấp".
    if ex.kind == ExchangeKind::Threat {
        factors.push(("bị đe dọa", 200 - i64::from(ex.cost) / 3));
    }

    let score: i64 = factors.iter().map(|(_, v)| v).sum();
    Volition {
        accepts: score > 0,
        score,
        factors,
    }
}

/// Áp kết quả một trao đổi lên social state.
///
/// Đây là chỗ vòng lặp khép: đề nghị → tính ý chí → áp kết quả → quan hệ đổi →
/// đề nghị lần sau khác đi. Không có bước cuối thì mọi cuộc trò chuyện đều là
/// lần đầu tiên.
pub fn apply_outcome(ex: &Exchange, v: &Volition, social: &mut SocialState) {
    if !v.accepts {
        // Bị từ chối: người đề nghị nguội đi với người đã từ chối mình.
        social.adjust(ex.from, ex.to, ex.kind.on_refusal(), 0);
        return;
    }

    if ex.kind == ExchangeKind::Threat {
        // **Đe dọa là trao đổi duy nhất mà phục tùng không tạo ra quan hệ.**
        //
        // Nạn nhân ghét kẻ ép mình, và không mang ơn gì hết. Nếu việc chịu ép
        // cũng sinh nợ nghĩa như việc được cho, thì bắt nạt trở thành một cách
        // xây dựng quan hệ — rẻ hơn tặng quà, và hiệu quả ngang. Cả thế giới sẽ
        // thành côn đồ, và không phải vì ai đó muốn thế.
        social.adjust(ex.to, ex.from, ex.kind.on_success(), 0);
        return;
    }

    // **Nợ nghĩa chảy về phía người đã trả**, quy đổi **1:1 với cái giá**.
    //
    // Tỉ lệ 1:1 không phải để cho tròn. Nó là điều khiến có đi có lại *thật sự*
    // hoạt động: [`volition`] cân nợ nghĩa và cái giá bằng cùng một hệ số, nên
    // một ân huệ cỡ `X` trả đúng cho một lời nhờ cỡ `X`. Đổi thành `cost / 4`
    // thì phải tặng quà gấp bốn lần mới xin lại được bằng ấy, và người chơi sẽ
    // kết luận — đúng — rằng giúp người khác chẳng được gì.
    //
    // `bond(x, y).obligation > 0` nghĩa là `x` nợ `y`.
    let no = i16::try_from(ex.cost).unwrap_or(i16::MAX);
    let (mang_no, duoc_no) = match ex.kind.payer() {
        Payer::Proposer => (ex.to, ex.from),
        Payer::Responder => (ex.from, ex.to),
    };
    // Người được nhận vừa ấm lên với người đã cho, vừa mang nợ nghĩa với họ.
    social.adjust(mang_no, duoc_no, ex.kind.on_success(), no);
    social.adjust(duoc_no, mang_no, 0, -no);
}
