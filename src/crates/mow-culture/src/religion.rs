//! Tôn giáo như một **thể chế** (`idea.md §12.16`, `PD-15`).
//!
//! ## Hai điểm thiết kế, và cả hai đều là điều dễ làm sai
//!
//! **1. Belief của tín đồ tách khỏi việc vị thần có thật hay không.**
//!
//! > Một giáo hội hoàn toàn có thể hiểu sai chính vị thần mình thờ, thờ một vị
//! > thần **đã chết**, hoặc thờ một thứ **chưa bao giờ tồn tại** — và vẫn vận
//! > hành, vẫn có quyền lực thật.
//!
//! Nên [`Religion`] không có trường trỏ tới một `deity: EntityId` bắt buộc. Nó
//! có `worships: String` — một cái *tên*. Việc cái tên đó có ứng với ai không là
//! chuyện của `§14`, và giáo hội không có cách nào hỏi.
//!
//! Nếu để giáo hội trỏ thẳng vào thực thể thần, thì một vị thần chết đi sẽ làm
//! con trỏ đó hỏng, và ai đó sẽ "sửa" bằng cách cho giáo hội biết thần đã chết
//! — xóa mất đúng cái tình huống thú vị nhất.
//!
//! **2. Nghi lễ tốn kém là bằng chứng, không phải điểm số.**
//!
//! > Giảng đạo chỉ tạo ra thông điệp ở `§12.15`. Hy sinh tài sản, giữ lời thề
//! > khó giữ, hành hương hay sống khổ hạnh mới tạo ra **bằng chứng** về mức cam
//! > kết, và chính bằng chứng đó làm người khác tin theo — thay vì cộng một biến
//! > `faith_point`.
//!
//! Nên ở đây không có `faith_point`. Có [`Rite`] với `cost` và `hard_to_fake`,
//! và [`credibility`] tính ra một con số mà **người ngoài quan sát được**. Một
//! nghi lễ rẻ tiền không thuyết phục được ai, dù làm bao nhiêu lần.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Một điều trong giáo lý.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Doctrine {
    /// Định danh.
    pub id: String,
    /// Điều này suy ra từ những điều nào.
    pub derives_from: Vec<String>,
    /// **Ai có quyền diễn giải.**
    ///
    /// Đây là trường mà ly giáo xoay quanh: khi hai người cùng nói mình có quyền
    /// diễn giải một điều, giáo hội tách làm đôi. Không cần cơ chế "ly giáo"
    /// riêng — chỉ cần trường này bị tranh chấp.
    pub interpreters: Vec<EntityId>,
}

/// Một nghi lễ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rite {
    /// Định danh.
    pub id: String,
    /// **Cái giá thật** phải trả: tài sản, thời gian, đau đớn, rủi ro.
    pub cost: i64,
    /// Có **khó giả mạo** không.
    ///
    /// Đây là chỗ phân biệt hy sinh thật với biểu diễn. Quỳ lạy trước đám đông
    /// thì rẻ và dễ giả; đi bộ ba tháng tới thánh địa thì không.
    pub hard_to_fake: bool,
    /// Có **công khai** không. Bằng chứng không ai thấy thì không thuyết phục ai.
    pub public: bool,
}

/// Một tôn giáo, dưới dạng **tổ chức**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Religion {
    /// Định danh.
    pub id: String,
    /// **Tên** của thứ được thờ.
    ///
    /// Chỉ là một chuỗi, cố ý. Xem docstring của module: giáo hội không có cách
    /// nào hỏi xem cái tên này có ứng với ai không.
    pub worships: String,
    /// Đồ thị giáo lý.
    pub doctrines: Vec<Doctrine>,
    /// Lịch nghi lễ.
    pub rites: Vec<Rite>,
    /// Thánh địa.
    pub holy_sites: Vec<String>,
    /// Hàng giáo sĩ.
    pub clergy: Vec<EntityId>,
}

impl Religion {
    /// Một điều giáo lý.
    pub fn doctrine(&self, id: &str) -> Option<&Doctrine> {
        self.doctrines.iter().find(|d| d.id == id)
    }

    /// **Quyền diễn giải một điều có đang bị tranh chấp không.**
    ///
    /// Nhiều hơn một người diễn giải nghĩa là nhiều hơn một câu trả lời có thẩm
    /// quyền — và đó chính là điều kiện của ly giáo.
    pub fn contested(&self, doctrine: &str) -> bool {
        self.doctrine(doctrine)
            .is_some_and(|d| d.interpreters.len() > 1)
    }

    /// Tách giáo hội theo một điều đang tranh chấp.
    ///
    /// Trả `None` nếu điều đó không bị tranh chấp — không có cách nào ly giáo mà
    /// không có bất đồng về quyền diễn giải, và đó là điểm.
    pub fn schism(&self, over: &str, breakaway: EntityId, new_id: &str) -> Option<Religion> {
        let d = self.doctrine(over)?;
        if d.interpreters.len() <= 1 || !d.interpreters.contains(&breakaway) {
            return None;
        }

        // Nhánh mới mang theo giáo lý, nhưng **chỉ một người diễn giải** — nên
        // nó nhất quán hơn giáo hội cũ, và đó là lý do các nhánh ly khai thường
        // nghiêm ngặt hơn bản gốc.
        let doctrines = self
            .doctrines
            .iter()
            .map(|x| Doctrine {
                id: x.id.clone(),
                derives_from: x.derives_from.clone(),
                interpreters: if x.id == over {
                    vec![breakaway]
                } else {
                    x.interpreters.clone()
                },
            })
            .collect();

        Some(Religion {
            id: new_id.to_owned(),
            worships: self.worships.clone(),
            doctrines,
            rites: self.rites.clone(),
            holy_sites: self.holy_sites.clone(),
            clergy: vec![breakaway],
        })
    }
}

/// Một lần ai đó thực hiện nghi lễ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observance {
    /// Ai làm.
    pub who: EntityId,
    /// Nghi lễ nào.
    pub rite: String,
    /// Đã thật sự trả bao nhiêu.
    pub paid: i64,
    /// Có bao nhiêu người thấy.
    pub witnesses: u32,
}

/// **Bằng chứng cam kết** mà người ngoài quan sát được, `0`–`1000`.
///
/// Đây là thứ thay cho `faith_point`. Ba điều kiện, và thiếu cái nào cũng làm
/// nó gần bằng 0:
///
/// - **Trả giá thật**: hứa suông không tính.
/// - **Khó giả mạo**: một hành động ai cũng làm được không chứng minh gì.
/// - **Có người thấy**: bằng chứng không ai chứng kiến không thuyết phục ai.
///
/// Nhân với nhau chứ không cộng: đó là lý do giảng đạo — rẻ, dễ giả, đông người
/// nghe — vẫn cho ra gần 0.
pub fn credibility(rite: &Rite, obs: &Observance) -> u16 {
    // Chưa trả đủ cái giá mà nghi lễ đòi thì **chưa làm nghi lễ đó**. Hứa hành
    // hương rồi đi được nửa đường không phải là một nửa bằng chứng.
    if obs.paid < rite.cost {
        return 0;
    }
    // Bằng chứng không ai thấy thì không phải bằng chứng cho ai cả.
    if !rite.public || obs.witnesses == 0 {
        return 0;
    }

    // **Cái giá thật, không phải tỉ lệ đã trả.**
    //
    // Bản đầu tính `paid / cost`, và nó cho ra một kết quả sai một cách rất êm:
    // một buổi giảng giá 1 mà trả đủ 1 cũng đạt tỉ lệ 100%, y hệt một chuyến
    // hành hương giá 900 trả đủ 900. Cộng năm mươi buổi giảng lại là vượt một
    // chuyến hành hương — tức là đúng cái `faith_point` mà `§12.16` bác bỏ, chỉ
    // đổi tên.
    //
    // Henrich nói về **chi phí tuyệt đối**: thứ khiến người ngoài tin là bạn đã
    // mất bao nhiêu, không phải bạn đã giữ đúng bao nhiêu phần lời hứa.
    let tra = obs.paid.clamp(0, 1_000);
    let kho_gia = if rite.hard_to_fake { 1_000 } else { 100 };

    let d = tra * kho_gia / 1_000;
    u16::try_from(d.clamp(0, 1_000)).unwrap_or(1_000)
}

/// Một người tin theo tới mức nào, dựa trên **những gì họ đã thấy người khác trả giá**.
///
/// Không cộng dồn số buổi giảng. Cộng dồn bằng chứng — và bằng chứng thì tốn kém.
pub fn conviction(rites: &[Rite], observed: &[Observance]) -> u16 {
    let tong: i64 = observed
        .iter()
        .filter_map(|o| {
            rites
                .iter()
                .find(|r| r.id == o.rite)
                .map(|r| i64::from(credibility(r, o)))
        })
        .sum();
    u16::try_from(tong.clamp(0, 1_000)).unwrap_or(1_000)
}
