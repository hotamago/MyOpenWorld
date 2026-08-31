//! [`Choice`], [`Decision`] và [`FallbackReason`] — đề xuất, kết quả, và lý do.
//!
//! ## Vì sao `Decision` không phải là `Result`
//!
//! `Result<Choice, Error>` sẽ đúng về kiểu và sai về ý. Nó nói với chỗ gọi rằng
//! "có thể không có quyết định nào", và chỗ gọi sẽ trả lời bằng một `unwrap`
//! hoặc một `?` — tức là một NPC đứng hình giữa lượt vì provider nghẽn mạng.
//! `§10.3` nói ngược lại: ba tầng dưới phải khiến thế giới chạy đúng kể cả khi
//! LLM chết hẳn.
//!
//! Nên [`Decision`] luôn mang một [`Choice`] dùng được. Cái nó **không** làm là
//! im lặng: một quyết định đến từ fallback mang theo [`FallbackReason`], và chỗ
//! gọi phân biệt được hai trường hợp bất cứ lúc nào nó muốn.
//!
//! ## Vì sao lý do là một enum chứ không phải một chuỗi
//!
//! `§20.10` đòi mỗi lần hạ cấp và mỗi lần rơi hẳn về policy là **một event có
//! lý do**. Một chuỗi tự do không đếm được: sáu cách viết khác nhau của cùng
//! một sự cố sẽ thành sáu dòng khác nhau trong báo cáo, và câu hỏi "vì sao hôm
//! đó cả vùng này hành xử ngờ nghệch" không có câu trả lời. [`FallbackReason`]
//! đếm được, và [`FallbackReason::label`] cho một khóa ổn định để gắn vào
//! metric.

use serde::{Deserialize, Serialize};

/// Một hành động **được đề xuất**, chưa được kiểm điều kiện.
///
/// Đây là toàn bộ những gì model được phép nói ra. Nó không chứng minh gì:
/// `§10.6` nói thẳng rằng lời giải thích của model không chứng minh hành động
/// thực hiện được, và engine vẫn tự kiểm đường đi, tầm nhìn, capability, tài
/// nguyên tại lúc commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    /// Hành động, **luôn là một phần tử của action registry**.
    ///
    /// Khi [`Choice`] đến từ [`crate::read_choice`], chuỗi này là bản sao của
    /// phần tử trong registry chứ không phải chuỗi model gõ — nên `Eat`, `eat `
    /// và `eat` đều quy về đúng một giá trị, và chỗ gọi không phải chuẩn hóa
    /// lại lần nữa.
    pub action: String,
    /// Đối tượng của hành động nếu có: một cái tên, chưa được phân giải.
    ///
    /// `None` nghĩa là "hành động này không cần đối tượng", hoặc "để engine tự
    /// chọn". Crate này **không** kiểm target có tồn tại hay có nhìn thấy được
    /// hay không — đó là bước 7 của `§10.4` và nó thuộc về engine, nơi có state
    /// thật để kiểm.
    pub target: Option<String>,
    /// Lời kể ngắn: nhân vật *nghĩ* rằng vì sao nó làm việc này.
    ///
    /// **Không có hiệu lực.** Nó đi vào log và vào giao diện, không đi vào một
    /// nhánh `if` nào. `§20.11.3`: văn bản của model không bao giờ ghi thẳng
    /// belief.
    pub reason: String,
}

impl Choice {
    /// Dựng một đề xuất.
    #[must_use]
    pub fn new(action: &str, target: Option<&str>, reason: &str) -> Choice {
        Choice {
            action: action.to_owned(),
            target: target.map(str::to_owned),
            reason: reason.to_owned(),
        }
    }
}

/// Vì sao một lượt suy nghĩ phải rơi về fallback (`§20.10`).
///
/// Mỗi biến thể là **một nhánh có tên**, không phải một khối `catch`. Thêm một
/// cách hỏng mới nghĩa là thêm một biến thể ở đây, và trình biên dịch sẽ chỉ ra
/// mọi chỗ cần xử lý nó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackReason {
    /// Không có mô hình nào trả lời được lời gọi này.
    ///
    /// Gom ba tình huống vì chúng cùng một nghĩa với thế giới — *phía sau cổng
    /// không có ai*: `STUB` không có stub cho prompt, `LIVE`/`RECORD` chưa cắm
    /// provider, `REPLAY` thiếu bản ghi. Chúng khác nhau ở cách sửa, và cách
    /// sửa nằm nguyên vẹn trong [`FallbackNote::detail`].
    NoProvider,
    /// Lời gọi không mang về được câu trả lời nào.
    ///
    /// Lỗi vận chuyển (DNS, TLS, quá hạn) và lỗi `5xx` của provider đều rơi vào
    /// đây: xét từ phía thế giới, cả hai là "hỏi rồi mà không có gì về".
    Timeout,
    /// Có câu trả lời nhưng sai hình dạng đã hứa: không phải JSON, không phải
    /// đối tượng, thiếu `action`, hoặc `target` không phải chuỗi.
    ///
    /// Mang theo đoạn văn bản đã cắt ngắn và đã che bí mật, vì "model trả sai
    /// hình dạng" mà không kèm hình dạng nó đã trả thì không sửa được prompt.
    BadShape(String),
    /// Model chọn một hành động **ngoài action registry** (`§10.5`).
    ///
    /// Mang theo đúng chuỗi model đã gõ. Đây là một lỗi validate, không phải
    /// một hành động lạ: giá trị này không bao giờ được thực hiện, nó chỉ được
    /// ghi lại.
    NotInRegistry(String),
    /// Hết ngân sách gọi model.
    ///
    /// Hai đường tới đây: [`crate::Mind`] đã tiêu hết số lời gọi cho phép, hoặc
    /// provider trả `402`/`429`. Cả hai đều là "còn muốn hỏi nhưng không được
    /// phép hỏi nữa", và cả hai đều phải hiện ra chứ không được đội lốt một
    /// lượt suy nghĩ bình thường.
    BudgetSpent,
    /// Registry rỗng: engine không công bố hành động hợp lệ nào cho thực thể này.
    ///
    /// Tách riêng khỏi [`FallbackReason::NotInRegistry`] vì hai thứ này đòi hai
    /// cách sửa khác hẳn nhau — một bên là prompt hoặc model sai, bên kia là
    /// **chỗ gọi** sai. Gộp chúng lại sẽ cho ra một báo cáo nói rằng model chọn
    /// nhầm, trong khi model chưa từng được hỏi.
    EmptyRegistry,
}

impl FallbackReason {
    /// Khóa ổn định để gắn vào metric và event log (`§20.10`).
    ///
    /// Ổn định nghĩa là: đổi chuỗi này là một thay đổi phá vỡ tương thích của
    /// bảng thống kê, không phải một lần sửa chính tả.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            FallbackReason::NoProvider => "no_provider",
            FallbackReason::Timeout => "timeout",
            FallbackReason::BadShape(_) => "bad_shape",
            FallbackReason::NotInRegistry(_) => "not_in_registry",
            FallbackReason::BudgetSpent => "budget_spent",
            FallbackReason::EmptyRegistry => "empty_registry",
        }
    }
}

impl core::fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FallbackReason::NoProvider => f.write_str("không có mô hình nào trả lời được"),
            FallbackReason::Timeout => f.write_str("lời gọi không mang về câu trả lời"),
            FallbackReason::BadShape(s) => write!(f, "trả lời sai hình dạng: {s}"),
            FallbackReason::NotInRegistry(s) => {
                write!(f, "hành động `{s}` không có trong action registry")
            }
            FallbackReason::BudgetSpent => f.write_str("hết ngân sách gọi model"),
            FallbackReason::EmptyRegistry => {
                f.write_str("action registry rỗng: engine chưa công bố hành động nào")
            }
        }
    }
}

/// Kết quả của một lượt suy nghĩ. **Luôn** có một hành động dùng được.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Model đã chọn, và lựa chọn đó qua được validate.
    Chose(Choice),
    /// Đã rơi về fallback.
    Fell {
        /// Hành động thật sự sẽ dùng — của fallback, **không** của model.
        to: Choice,
        /// Vì sao.
        reason: FallbackReason,
    },
}

impl Decision {
    /// Hành động chỗ gọi phải thực hiện, bất kể nó đến từ đâu.
    ///
    /// Tồn tại để chỗ gọi không viết `match` chỉ để lấy ra một [`Choice`] — và
    /// vì cái `match` đó là nơi người ta vô tình thực hiện hành động model đã
    /// đề xuất trong nhánh `Fell`.
    #[must_use]
    pub fn choice(&self) -> &Choice {
        match self {
            Decision::Chose(c) | Decision::Fell { to: c, .. } => c,
        }
    }

    /// Lượt này có phải là một lần rơi về fallback không.
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        matches!(self, Decision::Fell { .. })
    }

    /// Lý do rơi, nếu có.
    #[must_use]
    pub fn reason(&self) -> Option<&FallbackReason> {
        match self {
            Decision::Chose(_) => None,
            Decision::Fell { reason, .. } => Some(reason),
        }
    }
}

/// Một dòng trong sổ của [`crate::Mind`]: một lần rơi về fallback đã xảy ra.
///
/// `§20.10` đòi request/result có trace, và đòi mỗi lần rơi là một event chứ
/// không phải một sự im lặng. Đây là dạng tối thiểu của event đó, đủ để chỗ gọi
/// đẩy thẳng vào event log mà không phải tự dựng lại ngữ cảnh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackNote {
    /// Ai đã rơi.
    pub self_name: String,
    /// Vì sao.
    pub reason: FallbackReason,
    /// Chi tiết thô đã che bí mật và đã cắt ngắn: thông báo lỗi gốc, hoặc đoạn
    /// văn bản model đã trả.
    pub detail: String,
    /// Hành động đã dùng thay thế.
    pub used: Choice,
}
