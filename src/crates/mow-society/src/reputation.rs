//! Danh tiếng và trật tự chuẩn mực (`idea.md §9.9.3`, `§9.9.4`, `PC-11`).
//!
//! ## Danh tiếng là **belief theo bộ ba**, không phải một con số toàn cục
//!
//! Bộ ba là `(ai tin, về ai, về chuyện gì)`. Ba trục, và bỏ bất kỳ trục nào
//! cũng xóa mất một thứ:
//!
//! - Bỏ *ai tin*: thành một con số toàn cục, và không ai có thể **nhầm** về ai.
//!   Không có tin đồn sai, không có phục hồi danh dự, không có hai phe đánh giá
//!   một người khác nhau.
//! - Bỏ *về chuyện gì*: một người "tốt" hoặc "xấu" nói chung. Nhưng một tay
//!   trộm giỏi giữ lời hứa là một nhân vật, còn "tay trộm 3 điểm" thì không.
//!
//! ## Và nó **tách khỏi trait thật**
//!
//! `§9.9.3` nói rõ. Một người trung thực bị đồn là kẻ trộm vẫn **là** người
//! trung thực; điều thay đổi là cách người khác đối xử với họ. Nếu danh tiếng
//! và tính cách là cùng một trường, thì tin đồn sẽ **biến đổi con người**, và
//! đó là một thế giới rất khác — một thế giới không có oan sai.

use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Một belief về danh tiếng: `(người tin, người được nói tới, chuyện gì)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReputationKey {
    /// Ai tin.
    pub holder: u64,
    /// Về ai.
    pub about: u64,
    /// Về chuyện gì: `honesty`, `courage`, `craft.smithing`.
    pub trait_id: String,
}

/// Một belief kèm độ chắc chắn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Belief {
    /// Đánh giá, `-1000`..`1000`.
    pub value: i16,
    /// Độ chắc chắn, `0`..`1000`.
    ///
    /// Tách khỏi `value` vì "tôi chắc chắn hắn lương thiện" và "tôi hơi nghi
    /// hắn lương thiện" dẫn tới hành vi khác nhau, dù cùng dấu.
    pub confidence: u16,
    /// Tick cập nhật gần nhất.
    pub updated_at: u64,
    /// Dựa trên bao nhiêu lần quan sát trực tiếp.
    ///
    /// Phân biệt "tôi thấy tận mắt" với "tôi nghe kể". Belief dựa trên tin đồn
    /// dễ đổi hơn, và đó là cách một lời vu khống bị lật lại được.
    pub firsthand: u16,
}

impl Belief {
    /// Belief mới từ một lần quan sát trực tiếp.
    pub fn observed(value: i16, at_tick: u64) -> Belief {
        Belief {
            value,
            confidence: 400,
            updated_at: at_tick,
            firsthand: 1,
        }
    }

    /// Belief mới từ nghe kể.
    pub fn hearsay(value: i16, at_tick: u64) -> Belief {
        Belief {
            value,
            // Nghe kể thì tin ít hơn hẳn — và **không có** lần quan sát nào.
            confidence: 150,
            updated_at: at_tick,
            firsthand: 0,
        }
    }

    /// Có dựa trên quan sát trực tiếp không.
    pub fn is_firsthand(self) -> bool {
        self.firsthand > 0
    }
}

impl CanonicalHash for Belief {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(i64::from(self.value));
        h.write_u64(u64::from(self.confidence));
        h.write_u64(self.updated_at);
        h.write_u64(u64::from(self.firsthand));
    }
}

/// Kho belief về danh tiếng.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reputation {
    beliefs: BTreeMap<ReputationKey, Belief>,
}

impl Reputation {
    /// Rỗng.
    pub fn new() -> Reputation {
        Reputation::default()
    }

    /// Cập nhật một belief từ quan sát trực tiếp.
    ///
    /// Belief cũ và mới **trộn theo trọng số độ chắc chắn**, không ghi đè. Ghi
    /// đè sẽ khiến một lần gặp gỡ duy nhất xóa sạch mười năm quen biết.
    pub fn observe(&mut self, holder: u64, about: u64, trait_id: &str, value: i16, at_tick: u64) {
        let k = ReputationKey {
            holder,
            about,
            trait_id: trait_id.to_owned(),
        };
        let moi = Belief::observed(value, at_tick);
        let hop = match self.beliefs.get(&k) {
            Some(cu) => tron(*cu, moi),
            None => moi,
        };
        self.beliefs.insert(k, hop);
    }

    /// Cập nhật từ nghe kể.
    pub fn hear(&mut self, holder: u64, about: u64, trait_id: &str, value: i16, at_tick: u64) {
        let k = ReputationKey {
            holder,
            about,
            trait_id: trait_id.to_owned(),
        };
        let moi = Belief::hearsay(value, at_tick);
        let hop = match self.beliefs.get(&k) {
            Some(cu) => tron(*cu, moi),
            None => moi,
        };
        self.beliefs.insert(k, hop);
    }

    /// Một người tin gì về một người khác.
    pub fn get(&self, holder: u64, about: u64, trait_id: &str) -> Option<Belief> {
        self.beliefs
            .get(&ReputationKey {
                holder,
                about,
                trait_id: trait_id.to_owned(),
            })
            .copied()
    }

    /// Mọi belief mà một người giữ về một người khác.
    pub fn about(&self, holder: u64, about: u64) -> Vec<(&str, Belief)> {
        self.beliefs
            .iter()
            .filter(|(k, _)| k.holder == holder && k.about == about)
            .map(|(k, b)| (k.trait_id.as_str(), *b))
            .collect()
    }

    /// Số belief.
    pub fn len(&self) -> usize {
        self.beliefs.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.beliefs.is_empty()
    }

    /// Những người **bất đồng** về một người.
    ///
    /// Đây là dữ liệu mà kịch tính xã hội đọc: hai phe đánh giá một người khác
    /// nhau là mầm của xung đột, và một con số toàn cục không diễn đạt được nó.
    pub fn disagreement(&self, about: u64, trait_id: &str) -> Option<(i16, i16)> {
        let ds: Vec<i16> = self
            .beliefs
            .iter()
            .filter(|(k, _)| k.about == about && k.trait_id == trait_id)
            .map(|(_, b)| b.value)
            .collect();
        if ds.len() < 2 {
            return None;
        }
        Some((*ds.iter().min()?, *ds.iter().max()?))
    }
}

/// Trộn belief cũ và mới theo trọng số độ chắc chắn.
fn tron(cu: Belief, moi: Belief) -> Belief {
    let wc = i32::from(cu.confidence);

    // **Nghe kể bị chiết khấu theo số lần đã thấy tận mắt.** Một người đã tự
    // chứng kiến thì nghe đồn ngược lại sẽ nghi lời đồn trước, chứ không nghi
    // mắt mình. Không có bước này, một lời vu khống nói đủ to là xóa sạch mọi
    // thứ nhân vật tự thấy.
    let wm = if moi.firsthand == 0 {
        i32::from(moi.confidence) >> i32::from(cu.firsthand).min(4)
    } else {
        i32::from(moi.confidence)
    };
    let tong = (wc + wm).max(1);

    // Bằng chứng **mâu thuẫn** làm giảm độ chắc chắn, chứ không chỉ kéo giá trị.
    //
    // Đây là chỗ mà một mô hình chỉ có một con số sẽ sai. Nghe chín lời đồn
    // ngược với điều mình đã thấy thì kết quả đúng là *"tôi không biết phải
    // nghĩ sao"* — chứ không phải *"tôi tin chắc điều ngược lại"*. Với hai
    // trường tách nhau, trạng thái đó diễn đạt được: `value` trôi về phía lời
    // đồn, còn `confidence` tụt xuống, và một belief độ chắc chắn thấp gần như
    // không ảnh hưởng gì tới quyết định (`social::volition` nhân với nó).
    let mau_thuan = cu.value.signum() != moi.value.signum() && cu.value != 0 && moi.value != 0;
    let chac = if mau_thuan && moi.firsthand == 0 {
        // Chỉ **nghe kể** ngược lại mới làm lung lay. Thấy tận mắt một điều
        // ngược với những gì mình vẫn nghĩ thì không làm ta bớt chắc chắn — nó
        // làm ta đổi ý, và ta tin điều mới đó.
        (wc - wm / 2).clamp(50, 950)
    } else {
        // Nhiều bằng chứng đồng thuận thì chắc hơn, nhưng bão hòa — không ai
        // chắc 100%, và một nhân vật chắc 100% là một nhân vật không học được.
        (wc + wm / 2).min(950)
    };

    let mut gia_tri = i16::try_from((i32::from(cu.value) * wc + i32::from(moi.value) * wm) / tong)
        .unwrap_or(cu.value);

    // **Tin đồn không được kéo một belief đã thấy tận mắt qua bên kia số 0.**
    //
    // Đây là bất biến chứ không phải một hệ số chỉnh cho vừa. Lời đồn có thể
    // làm ta bớt chắc về điều mình đã thấy, nhưng không thể làm ta tin điều
    // ngược lại — chỉ nhìn thấy mới làm được điều đó. Không có ràng buộc này,
    // đủ số lần lặp là mọi lời vu khống đều thắng, và một nhân vật có thể bị
    // thuyết phục rằng người bạn nó vừa được cứu mạng là kẻ phản bội.
    if cu.firsthand > 0 && moi.firsthand == 0 && gia_tri.signum() != cu.value.signum() {
        gia_tri = cu.value.signum();
    }

    Belief {
        value: gia_tri,
        confidence: u16::try_from(chac).unwrap_or(950),
        updated_at: moi.updated_at,
        firsthand: cu.firsthand.saturating_add(moi.firsthand),
    }
}

impl CanonicalHash for Reputation {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.beliefs.iter(), |hh, (k, b)| {
            hh.write_u64(k.holder);
            hh.write_u64(k.about);
            hh.write_str(&k.trait_id);
            b.canonical_hash(hh);
        });
    }
}

/// Bậc của một chuẩn mực (`§9.9.4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormOrder {
    /// Bậc một: "đừng ăn trộm".
    First,
    /// Bậc hai: **"hãy trừng phạt kẻ không trừng phạt kẻ ăn trộm"**.
    ///
    /// Đây là bậc khiến hợp tác bền vững được. Không có nó, trừng phạt là một
    /// hành động tốn kém mà không ai có động cơ làm, và mọi chuẩn mực bậc một
    /// sẽ sụp — đó là kết quả kinh điển của lý thuyết trò chơi tiến hóa.
    Second,
}

/// Một chuẩn mực trong một nền văn hóa.
///
/// **Là dữ liệu văn hóa**, không phải hằng số engine (`§9.9.4`). Hai nền văn
/// hóa có thể có hai bộ chuẩn mực trái ngược nhau, và cả hai đều đúng trong
/// phạm vi của mình.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Norm {
    /// Định danh có namespace.
    pub id: String,
    /// Bậc.
    pub order: NormOrder,
    /// Hành vi bị điều chỉnh.
    pub act: String,
    /// Mức phản đối, `0`–`1000`.
    pub disapproval: u16,
    /// Có bao nhiêu phần dân số thật sự tuân thủ, `0`–`1000`.
    ///
    /// Tách khỏi `disapproval` vì hai thứ này thường lệch nhau, và khoảng lệch
    /// **là** thông tin: một chuẩn mực bị phản đối mạnh nhưng ai cũng vi phạm
    /// là một chuẩn mực sắp sụp.
    pub compliance: u16,
}

impl Norm {
    /// Chuẩn mực này có đang sụp không.
    ///
    /// Phản đối cao mà tuân thủ thấp: ai cũng nói nó sai, ai cũng làm. Đó là
    /// trạng thái ngay trước khi một chuẩn mực biến mất.
    pub fn is_collapsing(&self) -> bool {
        self.disapproval > 600 && self.compliance < 400
    }
}

impl CanonicalHash for Norm {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_i64(self.order as i64);
        h.write_str(&self.act);
        h.write_u64(u64::from(self.disapproval));
        h.write_u64(u64::from(self.compliance));
    }
}

/// Bộ chuẩn mực của một nền văn hóa.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormSet {
    /// Định danh.
    pub id: String,
    norms: Vec<Norm>,
}

impl NormSet {
    /// Bộ mới.
    pub fn new(id: &str) -> NormSet {
        NormSet {
            id: id.to_owned(),
            norms: Vec::new(),
        }
    }

    /// Thêm một chuẩn mực.
    pub fn add(&mut self, n: Norm) {
        self.norms.push(n);
        self.norms.sort_by(|a, b| a.id.cmp(&b.id));
    }

    /// Chuẩn mực điều chỉnh một hành vi.
    pub fn for_act(&self, act: &str) -> Vec<&Norm> {
        self.norms.iter().filter(|n| n.act == act).collect()
    }

    /// Có chuẩn mực bậc hai bảo vệ một chuẩn mực bậc một không.
    ///
    /// Không có nó thì chuẩn mực bậc một kia sẽ sụp — trừng phạt tốn kém, và
    /// không ai có động cơ trả cái giá đó một mình.
    pub fn has_second_order_for(&self, act: &str) -> bool {
        self.norms
            .iter()
            .any(|n| n.order == NormOrder::Second && n.act.contains(act))
    }

    /// Những chuẩn mực đang sụp.
    pub fn collapsing(&self) -> Vec<&Norm> {
        self.norms.iter().filter(|n| n.is_collapsing()).collect()
    }

    /// Mọi chuẩn mực.
    pub fn all(&self) -> &[Norm] {
        &self.norms
    }
}

impl CanonicalHash for NormSet {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_seq(self.norms.iter(), |hh, n| n.canonical_hash(hh));
    }
}
