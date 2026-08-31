//! Vòng đời thông điệp và sự tiếp nhận (`idea.md §12.15`, `PD-14`).
//!
//! ## Tách hẳn "nghe được" khỏi "làm theo"
//!
//! `§12.15.2` là cả module này gói trong một câu, và lý do nó quan trọng nằm ở
//! câu tiếp theo:
//!
//! > Nhờ vậy thời trang, taboo, kỹ thuật canh tác và tín ngưỡng lan với **tốc độ
//! > khác nhau trên cùng một mạng lưới xã hội** — điều mà một hệ số lan truyền
//! > duy nhất không thể tạo ra.
//!
//! Với một hệ số `contagion: 0.3`, mọi thứ lan như nhau. Muốn thời trang lan
//! nhanh hơn tín ngưỡng thì phải cho chúng hai hệ số, rồi hai mạng lưới, rồi hai
//! hệ thống — và cuối cùng là hai hệ thống không nhất quán với nhau.
//!
//! Ở đây chỉ có **một** mạng lưới và **một** cơ chế lan. Cái khác nhau là
//! [`Bias`]: thời trang theo số đông, kỹ thuật canh tác theo thành công quan sát
//! được, tín ngưỡng theo hành động tốn kém khó giả mạo. Cùng một mạng, ba tốc độ.
//!
//! ## Nhiều phiên bản cạnh tranh, và Yuu **không** chọn bên
//!
//! `§12.15.1`: nhiều phiên bản của cùng một sự kiện lan song song và cạnh tranh.
//! Tuyên truyền, đính chính, hoảng loạn đạo đức và tin đồn tự chết đều rơi ra từ
//! đây.
//!
//! Nên [`Rumour`] không có trường `is_true`. Có `fidelity` — nó đã bị sửa bao
//! nhiêu so với bản đầu — và điều đó khác hẳn: một phiên bản trung thực với lời
//! kể gốc vẫn có thể sai, nếu người kể đầu tiên đã nhìn nhầm.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Người ta bắt chước theo cái gì (`§12.15.2`).
///
/// Đây là thứ làm cho **cùng một mạng lưới** cho ra nhiều tốc độ lan khác nhau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bias {
    /// Theo số đông. Thời trang lan kiểu này: nhanh, nông, và đảo chiều được.
    Conformity,
    /// Theo uy tín người làm. Lan chậm hơn nhưng bám lâu hơn.
    Prestige,
    /// Theo thành công **quan sát được**. Kỹ thuật canh tác lan kiểu này.
    Success,
    /// Theo quan hệ ingroup và huyết thống. Rất chậm ra ngoài nhóm, rất bền trong nhóm.
    Kinship,
    /// Theo chuyên môn của người hướng dẫn.
    Expertise,
    /// Theo **hành động tốn kém khó giả mạo**. Tín ngưỡng lan kiểu này (`§12.16`).
    CostlySignal,
}

/// Một phiên bản của một tin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rumour {
    /// Định danh phiên bản.
    pub id: u64,
    /// Nói về sự kiện nào.
    pub about_event: u64,
    /// Nội dung.
    pub content: String,
    /// Ai kể đầu tiên.
    pub origin: EntityId,
    /// **Trung thực với bản đầu** tới mức nào, `0`–`1000`.
    ///
    /// Cố ý **không phải** "đúng hay sai". Một phiên bản trung thực với lời kể
    /// gốc vẫn có thể sai, nếu người kể đầu tiên đã nhìn nhầm — và hai chuyện đó
    /// phải phân biệt được, vì đính chính chữa được cái thứ nhất mà không chữa
    /// được cái thứ hai.
    pub fidelity: u16,
    /// Đã qua bao nhiêu người.
    pub hops: u32,
    /// Người truyền có **động cơ sửa nội dung** không, `-1000`..`1000`.
    ///
    /// Dương là muốn phóng đại, âm là muốn giảm nhẹ. Đây là chỗ tuyên truyền
    /// khác với tam sao thất bản.
    pub distortion_motive: i16,
}

impl Rumour {
    /// Truyền cho một người nữa. **Luôn mất một ít.**
    pub fn retell(&self, new_id: u64, teller_motive: i16) -> Rumour {
        // Mất mát tự nhiên, cộng thêm phần cố ý bẻ.
        let mat = 20 + i64::from(teller_motive.abs()) / 20;
        Rumour {
            id: new_id,
            about_event: self.about_event,
            content: self.content.clone(),
            origin: self.origin,
            fidelity: u16::try_from((i64::from(self.fidelity) - mat).max(0)).unwrap_or(0),
            hops: self.hops + 1,
            distortion_motive: teller_motive,
        }
    }
}

/// Một người đã nghe gì, và có làm theo không.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Reception {
    /// `(người, tin)` → đã nghe chưa.
    heard: BTreeMap<(u64, u64), bool>,
    /// `(người, tin)` → đã làm theo chưa.
    adopted: BTreeMap<(u64, u64), bool>,
}

impl Reception {
    /// Rỗng.
    pub fn new() -> Reception {
        Reception::default()
    }

    /// Ghi nhận đã nghe.
    pub fn hear(&mut self, who: EntityId, rumour: u64) {
        self.heard.insert((who.0, rumour), true);
    }

    /// Đã nghe chưa.
    pub fn has_heard(&self, who: EntityId, rumour: u64) -> bool {
        self.heard.get(&(who.0, rumour)).copied().unwrap_or(false)
    }

    /// Ghi nhận đã làm theo.
    pub fn adopt(&mut self, who: EntityId, rumour: u64) {
        self.adopted.insert((who.0, rumour), true);
    }

    /// Đã làm theo chưa.
    pub fn has_adopted(&self, who: EntityId, rumour: u64) -> bool {
        self.adopted.get(&(who.0, rumour)).copied().unwrap_or(false)
    }

    /// Bao nhiêu người đã nghe.
    pub fn heard_count(&self, rumour: u64) -> usize {
        self.heard
            .iter()
            .filter(|((_, r), v)| *r == rumour && **v)
            .count()
    }

    /// Bao nhiêu người đã làm theo.
    pub fn adopted_count(&self, rumour: u64) -> usize {
        self.adopted
            .iter()
            .filter(|((_, r), v)| *r == rumour && **v)
            .count()
    }
}

/// Bối cảnh xã hội quanh một người, để quyết định họ có làm theo không.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SocialEvidence {
    /// Bao nhiêu phần nghìn người xung quanh đã làm theo.
    pub peers_adopting: u16,
    /// Uy tín của người đang làm, `0`–`1000`.
    pub adopter_prestige: u16,
    /// Thành công **quan sát được** của người đã làm, `0`–`1000`.
    pub observed_success: u16,
    /// Người làm có cùng nhóm với mình không, `0`–`1000`.
    pub ingroup: u16,
    /// Chuyên môn của người hướng dẫn, `0`–`1000`.
    pub instructor_expertise: u16,
    /// Người làm đã **trả giá** bao nhiêu để chứng tỏ, `0`–`1000`.
    pub costly_display: u16,
}

/// Có làm theo không, và **vì sao**.
///
/// Không `Deserialize` được: `factors` mang `&'static str`, và một nhãn đọc từ
/// file thì không còn là `'static`. Đây là kiểu **kết quả tính toán**, không
/// phải state — nó không cần đi vào save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Adoption {
    /// Có làm theo không.
    pub adopts: bool,
    /// Điểm.
    pub score: i64,
    /// Xu hướng nào đã quyết định.
    pub bias: Bias,
    /// Phân rã.
    pub factors: Vec<(&'static str, i64)>,
}

/// Một người **nghe được** thì có **làm theo** không.
///
/// `threshold` là ngưỡng riêng của từng người — bảo thủ thì cao, cả tin thì
/// thấp. Nó ở đây chứ không phải một hằng số toàn cục, vì `§12.11` đã cho thấy
/// phân bố ngưỡng quyết định kết cục chứ không phải giá trị trung bình.
pub fn consider(bias: Bias, ev: &SocialEvidence, threshold: i64) -> Adoption {
    let mut factors: Vec<(&'static str, i64)> = Vec::new();

    // Mọi xu hướng đều nhìn cùng một bối cảnh; chúng chỉ **cân** khác nhau. Đó
    // là lý do một mạng lưới cho ra nhiều tốc độ lan.
    let (ten, diem): (&'static str, i64) = match bias {
        Bias::Conformity => ("số đông đang làm", i64::from(ev.peers_adopting)),
        Bias::Prestige => ("người có uy tín đang làm", i64::from(ev.adopter_prestige)),
        Bias::Success => ("thấy nó có hiệu quả", i64::from(ev.observed_success)),
        Bias::Kinship => ("người cùng nhóm đang làm", i64::from(ev.ingroup)),
        Bias::Expertise => ("người có nghề chỉ dạy", i64::from(ev.instructor_expertise)),
        // Hành động tốn kém khó giả mạo: chính **cái giá** là bằng chứng, không
        // phải lời nói. Xem `§12.16`.
        Bias::CostlySignal => ("người ta đã trả giá thật", i64::from(ev.costly_display)),
    };
    factors.push((ten, diem));

    // Số đông luôn có một chút ảnh hưởng, dù xu hướng chính là gì khác — nhưng
    // chỉ một phần tư. Bỏ hẳn thì không giải thích được vì sao một kỹ thuật tốt
    // vẫn lan chậm ở nơi chưa ai dùng.
    if bias != Bias::Conformity {
        factors.push(("xung quanh cũng đang làm", i64::from(ev.peers_adopting) / 4));
    }

    let score: i64 = factors.iter().map(|(_, v)| v).sum();
    Adoption {
        adopts: score >= threshold,
        score,
        bias,
        factors,
    }
}

/// Hai phiên bản cùng nói về một sự kiện — cái nào **đang thắng**.
///
/// Không phải cái nào đúng. Yuu không quyết định phiên bản nào thắng
/// (`§12.15.1`); nó chỉ là kết quả của việc bao nhiêu người đã làm theo.
pub fn dominant_version<'a>(versions: &'a [Rumour], reception: &Reception) -> Option<&'a Rumour> {
    versions.iter().max_by_key(|r| {
        (
            reception.adopted_count(r.id),
            reception.heard_count(r.id),
            std::cmp::Reverse(r.id),
        )
    })
}

/// Một bản dịch, và chỗ nó có thể hỏng (`§12.15.3`).
///
/// > Một hiệp ước, một câu thần chú, một lời tiên tri hay một bài giảng có thể
/// > hỏng vì dịch sai **mà không ai cố tình nói dối**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Translation {
    /// Từ ngôn ngữ nào.
    pub from: String,
    /// Sang ngôn ngữ nào.
    pub to: String,
    /// Hai ngôn ngữ hiểu lẫn nhau tới mức nào, `0`–`1000`.
    pub mutual_intelligibility: u16,
    /// Kỹ năng người dịch, `0`–`1000`.
    pub translator_skill: u16,
}

impl Translation {
    /// Độ chính xác còn lại sau khi dịch, `0`–`1000`.
    pub fn fidelity_after(&self, before: u16) -> u16 {
        let giu = i64::midpoint(
            i64::from(self.mutual_intelligibility),
            i64::from(self.translator_skill),
        );
        u16::try_from(i64::from(before) * giu / 1_000).unwrap_or(0)
    }

    /// Bản dịch này có hỏng tới mức nguy hiểm không.
    ///
    /// Với một câu thần chú, "hỏng" nghĩa là một tai nạn hoàn toàn hợp lý theo
    /// `§8.10.5` — không phải một lỗi engine.
    pub fn is_dangerous(&self, before: u16, danger_threshold: u16) -> bool {
        self.fidelity_after(before) < danger_threshold
    }
}
