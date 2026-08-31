//! Học, truyền dạy có hao hụt, và sách (`idea.md §13.3`, `§8.8`, `PD-16`, `PD-24`).
//!
//! ## Người dạy **có thể truyền sai**
//!
//! `§13.3` liệt kê năm thứ quyết định hiệu quả, và cái cuối cùng là cái quan
//! trọng nhất: **độ chính xác của kiến thức nguồn**. Một người dạy rất giỏi
//! truyền một kiến thức sai sẽ tạo ra một học trò rất tự tin và rất sai.
//!
//! Nên [`teach`] trả về cả bậc **lẫn** `fidelity`, và fidelity đi theo tri thức
//! suốt đời. Không có nó, mọi tri thức trong world đều đúng như nhau, và cả
//! `§13.10` — "chép sai sinh ra trường phái mới" — không có chỗ để xảy ra.
//!
//! ## Đọc sách **là một lần truyền dạy có hao hụt**
//!
//! `§8.8` quy tắc 1 nói thẳng: dùng đúng cơ chế `§13.3`. Nên [`read`] gọi lại
//! [`teach`], với "người dạy" là văn bản. Điều đó cho ra ngay một hệ quả đúng
//! mà không phải viết riêng:
//!
//! > *"Một cuốn sách phép cao cấp trong tay người thiếu nền tảng chỉ cho ra
//! > trạng thái `HEARD_OF`, không phải `PRACTICED`."*
//!
//! ## Sao chép sinh lỗi tích lũy
//!
//! `§8.8` quy tắc 3. [`Text::copy`] tăng `transcription_errors` và giảm
//! `fidelity` mỗi thế hệ. Từ đó có phê bình văn bản, bản gốc thất lạc, và
//! **những dị giáo sinh ra từ một lỗi dịch** — cả ba đều là hệ quả của một hàm
//! bốn dòng.
//!
//! ## Tri thức **mất thật được**
//!
//! `§8.8` quy tắc 4. [`Corpus::knowledge_survives`] trả lời *"đốt hết sách thì
//! tri thức này còn không"*, và nó nhìn cả sách lẫn người. Đốt sách là một hành
//! động có hậu quả đo được, không phải một sự kiện trang trí.

use crate::graph::{Level, Node, Understanding};
use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Người dạy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Teacher {
    /// Ai.
    pub who: EntityId,
    /// Bậc của chính người dạy.
    pub level: Level,
    /// Kỹ năng sư phạm, `0`–`1000`. Khác hẳn với việc giỏi nghề.
    pub pedagogy: u16,
    /// **Độ chính xác** của kiến thức mà người này đang mang, `0`–`1000`.
    pub fidelity: u16,
}

/// Người học.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Learner {
    /// Ai.
    pub who: EntityId,
    /// Trí nhớ, `0`–`1000`.
    pub memory: u16,
    /// Mức tập trung, `0`–`1000`.
    pub attention: u16,
    /// Động lực, `0`–`1000`.
    pub motivation: u16,
}

/// Bối cảnh buổi dạy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    /// Hai bên có chung ngôn ngữ tới mức nào, `0`–`1000`.
    pub shared_language: u16,
    /// Người học tin người dạy tới mức nào, `0`–`1000`.
    pub trust: u16,
    /// Có công cụ và tài liệu không, `0`–`1000`.
    pub tools: u16,
    /// Thời gian thực hành, `0`–`1000`.
    pub practice_time: u16,
}

/// Kết quả một lần truyền dạy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taught {
    /// Bậc đạt được.
    pub level: Level,
    /// Độ chính xác của thứ vừa học, `0`–`1000`.
    ///
    /// **Không bao giờ cao hơn nguồn.** Truyền dạy chỉ mất mát, không tạo ra
    /// độ chính xác — muốn tăng thì phải nghiên cứu lại, không phải học lại.
    pub fidelity: u16,
    /// Điểm hiệu quả, để giải thích.
    pub effectiveness: u16,
    /// Vì sao ra bậc đó.
    pub reasons: Vec<String>,
}

/// Truyền dạy một node.
///
/// **Hàm thuần và xác định.** Cùng người dạy, người học, bối cảnh thì luôn cùng
/// kết quả — nên người chơi học được rằng thuê thầy giỏi hơn thì trò khá hơn,
/// thay vì học được rằng nên lưu game trước khi học.
pub fn teach(
    node: &Node,
    teacher: &Teacher,
    learner: &Learner,
    setting: &Setting,
    current: Level,
) -> Taught {
    let mut reasons = Vec::new();

    if !teacher.level.can_teach() {
        reasons.push(format!(
            "người dạy mới ở bậc {:?}, chưa dạy được",
            teacher.level
        ));
        return Taught {
            level: current,
            fidelity: 0,
            effectiveness: 0,
            reasons,
        };
    }

    // Không ai dạy được cái mình không biết, và không ai học vượt thầy trong
    // một buổi. Trần này là chỗ "học trò hơn thầy" phải đi đường khác: tự
    // nghiên cứu, hoặc học nhiều thầy.
    let tran = teacher.level;

    let phia_thay = i64::midpoint(
        i64::from(teacher.pedagogy),
        i64::from(setting.shared_language),
    );
    let phia_tro =
        (i64::from(learner.memory) + i64::from(learner.attention) + i64::from(learner.motivation))
            / 3;
    let hoan_canh = i64::midpoint(i64::from(setting.tools), i64::from(setting.practice_time));

    // Tin tưởng **nhân vào**, không cộng: không tin thầy thì nghe cũng như không.
    let tin = i64::from(setting.trust);
    let kho = i64::from(node.teaching_difficulty);

    let hieu_qua =
        ((phia_thay + phia_tro + hoan_canh) / 3 * tin / 1_000) * (1_000 - kho / 2) / 1_000;
    let hieu_qua = hieu_qua.clamp(0, 1_000);

    reasons.push(format!("sư phạm và ngôn ngữ: {phia_thay}"));
    reasons.push(format!("nền tảng người học: {phia_tro}"));
    reasons.push(format!("công cụ và thời gian: {hoan_canh}"));
    if tin < 300 {
        reasons.push("không tin thầy nên nghe cũng như không".to_owned());
    }

    // Bậc đạt được, theo hiệu quả. Một buổi dạy tốt lên một bậc; rất tốt lên hai.
    let mut moi = current;
    if hieu_qua >= 300 {
        moi = moi.next().unwrap_or(moi);
    }
    if hieu_qua >= 700 {
        moi = moi.next().unwrap_or(moi);
    }
    if moi > tran {
        moi = tran;
        reasons.push("chạm trần: không học vượt thầy trong một buổi".to_owned());
    }

    // **Độ chính xác không bao giờ vượt nguồn.**
    let fidelity = u16::try_from(i64::from(teacher.fidelity) * hieu_qua / 1_000)
        .unwrap_or(0)
        .min(teacher.fidelity);

    Taught {
        level: moi,
        fidelity,
        effectiveness: u16::try_from(hieu_qua).unwrap_or(0),
        reasons,
    }
}

/// Một vật phẩm mang thông tin (`§8.8`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    /// Định danh vật phẩm.
    pub id: u64,
    /// Ngôn ngữ.
    pub language: String,
    /// Hệ chữ.
    pub script: String,
    /// Có mã hóa không, và có cần khóa không.
    pub cipher: Option<String>,
    /// Node tri thức mà nó chứa.
    pub node: String,
    /// **Độ chính xác** của nội dung, `0`–`1000`.
    pub fidelity: u16,
    /// Đời bản sao. `0` là bản gốc.
    pub generation: u32,
    /// Số lỗi chép tích lũy.
    pub transcription_errors: u32,
    /// Chép từ bản nào.
    pub copied_from: Option<u64>,
}

impl Text {
    /// Một người có **đọc được** không.
    ///
    /// Ba điều kiện tách rời, vì chúng thất bại theo ba kiểu khác nhau: không
    /// biết tiếng thì cần người dịch; không biết chữ thì cần người đọc hộ;
    /// không có khóa thì cần trộm khóa.
    pub fn legible_to(&self, languages: &[String], scripts: &[String], keys: &[String]) -> bool {
        languages.contains(&self.language)
            && scripts.contains(&self.script)
            && self.cipher.as_ref().is_none_or(|c| keys.contains(c))
    }

    /// Chép ra một bản mới. **Sinh lỗi.**
    ///
    /// `scribe_skill` `0`–`1000`: thợ chép giỏi mất ít hơn, nhưng **không bao
    /// giờ mất 0**. Một bản sao hoàn hảo tuyệt đối sẽ làm cả `§12.3` trôi dạt
    /// văn bản biến mất, và cùng với nó là phê bình văn bản và mọi dị giáo sinh
    /// ra từ một lỗi dịch.
    pub fn copy(&self, new_id: u64, scribe_skill: u16) -> Text {
        let mat = (1_000 - i64::from(scribe_skill)) / 20 + 1;
        Text {
            id: new_id,
            language: self.language.clone(),
            script: self.script.clone(),
            cipher: self.cipher.clone(),
            node: self.node.clone(),
            fidelity: u16::try_from((i64::from(self.fidelity) - mat).max(0)).unwrap_or(0),
            generation: self.generation + 1,
            transcription_errors: self.transcription_errors + u32::try_from(mat).unwrap_or(1),
            copied_from: Some(self.id),
        }
    }
}

/// Đọc một văn bản — **là một lần truyền dạy có hao hụt**.
///
/// Trả `None` khi người đọc không giải mã nổi: không biết tiếng, không biết chữ,
/// hoặc không có khóa. Đó không phải thất bại của việc học mà là chưa bắt đầu
/// học được, và hai chuyện đó khác nhau.
pub fn read(
    text: &Text,
    node: &Node,
    reader: &Learner,
    reader_understanding: &Understanding,
    languages: &[String],
    scripts: &[String],
    keys: &[String],
) -> Option<Taught> {
    if !text.legible_to(languages, scripts, keys) {
        return None;
    }

    // "Người dạy" là văn bản: nó không có kỹ năng sư phạm, không trả lời câu
    // hỏi, và không nhận ra khi người học hiểu sai. Đó là lý do sách dạy kém hơn
    // người, và là lý do một cuốn sách phép cao cấp trong tay người thiếu nền
    // tảng chỉ cho ra `HEARD_OF`.
    let sach = Teacher {
        who: EntityId(0),
        level: Level::Mastered,
        pedagogy: 300,
        fidelity: text.fidelity,
    };
    let boi_canh = Setting {
        shared_language: 1_000,
        trust: 1_000,
        tools: 200,
        // Sách không thực hành thay người đọc được.
        practice_time: 0,
    };

    Some(teach(
        node,
        &sach,
        reader,
        &boi_canh,
        reader_understanding.level(&text.node),
    ))
}

/// Kho sách và người biết — dùng để trả lời "tri thức này còn không".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Corpus {
    /// Sách còn tồn tại.
    pub texts: Vec<Text>,
    /// Ai còn biết node nào, ở bậc nào.
    pub minds: Vec<(EntityId, String, Level)>,
}

impl Corpus {
    /// **Tri thức này còn tồn tại trong world không** (`§8.8` quy tắc 4).
    ///
    /// Còn khi có ít nhất một bản sao đọc được, **hoặc** một người còn biết ở
    /// bậc làm được. Cả hai đều mất thì tri thức biến mất khỏi world cho tới khi
    /// có người khám phá lại từ đầu.
    pub fn knowledge_survives(&self, node: &str) -> bool {
        self.texts.iter().any(|t| t.node == node)
            || self
                .minds
                .iter()
                .any(|(_, n, l)| n == node && l.can_practise())
    }

    /// Đốt sách. Trả về số bản đã mất.
    ///
    /// Có hàm này để "đốt thư viện" là một thao tác có tên, đo được, và ghi
    /// event được — chứ không phải một dòng `retain` nằm đâu đó.
    pub fn burn(&mut self, predicate: impl Fn(&Text) -> bool) -> usize {
        let truoc = self.texts.len();
        self.texts.retain(|t| !predicate(t));
        truoc - self.texts.len()
    }

    /// Bản có độ chính xác cao nhất còn lại của một node.
    ///
    /// Đây là thứ phê bình văn bản đi tìm: giữa mười bản chép tay, bản nào gần
    /// nguyên tác nhất. Đời bản sao nhỏ hơn thường đúng hơn, nhưng không phải
    /// luôn luôn — một thợ chép giỏi ở đời thứ năm có thể hơn một thợ vụng ở
    /// đời thứ hai.
    pub fn best_copy(&self, node: &str) -> Option<&Text> {
        self.texts
            .iter()
            .filter(|t| t.node == node)
            .max_by_key(|t| (t.fidelity, std::cmp::Reverse(t.generation)))
    }
}
