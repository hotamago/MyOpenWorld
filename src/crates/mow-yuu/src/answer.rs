//! [`Answer`] và các phần của nó — có trích dẫn, hoặc không có gì.
//!
//! `§1.2.4`: Yuu là trình phiên dịch đồ thị nhân quả, không phải chatbot. Kiểu
//! ở đây là hợp đồng của lời hứa đó: [`Line`] và [`Proposal`] **không thể**
//! được dựng mà thiếu `cites`/`power` hợp lệ — [`crate::read_answer`] là cửa
//! duy nhất dựng chúng từ văn bản model, và nó từ chối thẳng mọi trường hợp
//! thiếu. [`Stripped`] tồn tại để "bị cắt" không bao giờ là "biến mất": mọi
//! câu, mọi đề xuất không qua được cửa kiểm chứng đều hiện ra ở đây, kèm lý do
//! có tên ([`StripReason`]) — một hệ thống lặng lẽ cắt bỏ là một hệ thống
//! không ai gỡ lỗi được.

use serde::{Deserialize, Serialize};

/// Một câu đã qua kiểm chứng: mọi `seq` trong `cites` là một sự kiện có thật
/// mà Yuu biết tại lúc trả lời — xem [`crate::read_answer`].
///
/// # Vì sao không re-export ở gốc crate
///
/// Tên này trùng với [`crate::audit::Line`] — một kiểu khác, phục vụ khâu
/// biên niên sử (`§18.11`) chứ không phải tư vấn. Trùng tên không phải tai
/// nạn cần sửa: hai kiểu bảo đảm hai thứ khác nhau (một cái là dòng đã xác
/// nhận từ sự kiện thật, một cái là dòng đã kiểm chứng từ lời model), và gộp
/// chúng làm một sẽ trộn hai bảo đảm đó vào một kiểu. Vì tên trùng ở cấp
/// module, kiểu này chỉ dùng được qua đường dẫn đủ `mow_yuu::answer::Line`,
/// không có mặt ở `mow_yuu::Line`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// Nội dung câu, đã che bí mật và cắt ngắn nếu nó tới từ model.
    pub text: String,
    /// `seq` của những sự kiện làm bằng chứng cho câu này. Không bao giờ rỗng
    /// — một `Line` với `cites` rỗng không được phép tồn tại, xem
    /// [`crate::read_answer`].
    pub cites: Vec<u64>,
}

/// Một đề xuất can thiệp, ánh xạ về đúng một quyền năng có thật.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// Tên quyền năng — phải nằm trong tập `known_powers` mà chỗ gọi
    /// [`crate::read_answer`] coi là có thật.
    pub power: String,
    /// Vì sao Yuu nghĩ nên dùng quyền năng này ngay lúc này.
    pub why: String,
    /// `seq` của những sự kiện làm bằng chứng. Cùng luật với [`Line::cites`]:
    /// không bao giờ rỗng.
    pub cites: Vec<u64>,
}

/// Vì sao một câu hoặc một đề xuất bị cắt khỏi [`Answer`].
///
/// Một enum có tên, không phải một chuỗi tự do — cùng lý do
/// `mow_mind::FallbackReason` là enum: một câu hỏi như "hôm nay Yuu cắt bao
/// nhiêu câu vì trích dẫn sai" chỉ trả lời được khi lý do đếm được.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StripReason {
    /// Câu hoặc đề xuất không kèm một trích dẫn nào.
    ///
    /// Đây cũng là nhánh dùng cho mọi trích dẫn đọc không được thành số
    /// nguyên không âm — với Yuu, một trích dẫn không đọc được cũng là một
    /// trích dẫn không có.
    NoCitation,
    /// Một trích dẫn trỏ tới `seq` không nằm trong tập sự kiện đã biết.
    ///
    /// Mang `seq` bị trỏ sai, và cắt **cả câu** — không giữ lại phần trích
    /// dẫn đúng của một câu có trích dẫn sai, vì việc đó vẫn để lọt một khẳng
    /// định không truy được về đâu.
    UnknownEvent(u64),
    /// Một đề xuất trỏ tới một quyền năng không nằm trong tập quyền năng có
    /// thật.
    ///
    /// Mang tên quyền năng Yuu đã bịa hoặc đọc sai, để chỗ vận hành biết nên
    /// thêm quyền năng đó vào danh sách thật hay nên coi đây là model bịa.
    UnknownPower(String),
}

impl core::fmt::Display for StripReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StripReason::NoCitation => f.write_str("không kèm trích dẫn nào"),
            StripReason::UnknownEvent(seq) => {
                write!(f, "trích dẫn sự kiện #{seq}, nhưng không có sự kiện đó")
            }
            StripReason::UnknownPower(power) => {
                write!(
                    f,
                    "đề xuất quyền năng `{power}`, nhưng quyền năng đó không có thật"
                )
            }
        }
    }
}

/// Một câu hoặc đề xuất đã bị cắt khỏi [`Answer`], kèm lý do.
///
/// **Không bao giờ bị giấu**: mọi lần cắt đều có mặt trong
/// [`Answer::stripped`], kể cả khi kết quả cuối cùng là rơi về
/// [`crate::without_model`] — xem `crate::yuu::Yuu::ask`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stripped {
    /// Nội dung đã bị cắt — vẫn giữ lại để chỗ vận hành đọc được Yuu *định*
    /// nói gì, dù nó không được phép nói ra.
    pub text: String,
    /// Vì sao.
    pub reason: StripReason,
}

/// Câu trả lời của Yuu sau khi đã lọc qua đúng một cửa kiểm chứng
/// ([`crate::read_answer`] hoặc [`crate::without_model`]).
///
/// **Không có "một phần đáng tin, một phần không"**: mọi phần tử trong `lines`
/// và `proposals` đều đã qua kiểm chứng như nhau. Cái gì không qua được thì
/// nằm trong `stripped`, không nằm lẫn vào hai trường kia.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    /// Các câu đã qua kiểm chứng.
    pub lines: Vec<Line>,
    /// Đề xuất can thiệp đã qua kiểm chứng.
    pub proposals: Vec<Proposal>,
    /// Những gì đã bị cắt, và vì sao.
    pub stripped: Vec<Stripped>,
}
