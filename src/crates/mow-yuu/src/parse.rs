//! Đọc câu trả lời của model thành một [`Answer`] đã kiểm chứng — hoặc cắt bỏ.
//!
//! ## Đây là chỗ lời hứa của `§1.2.4` được thi hành
//!
//! *"Một câu Yuu nói mà không truy được về một sự kiện có thật là một câu phải
//! bị cắt bỏ, không phải một câu để người chơi tự đánh giá."* Câu đó dễ gật đầu
//! và dễ quên viết. Nó được viết ở đúng một chỗ: [`read_answer`]. Mọi đường từ
//! văn bản model tới [`Answer`] đi qua hàm này.
//!
//! ## Hai tầng kiểm, không phải một
//!
//! 1. **Vỏ bọc** ([`parse_envelope`]): có phải JSON, có phải một object không.
//!    Qua được tầng này thì từng `line`/`proposal` mới được xét riêng; không
//!    qua được thì [`read_answer`] trả một [`Answer`] rỗng tuyệt đối — không
//!    phải một câu trả lời, mà là **tín hiệu "model trả rác hoàn toàn"** cho
//!    chỗ gọi (`crate::yuu::Yuu::ask`) biết cần rơi về [`crate::without_model`].
//!    `read_answer` không tự làm việc đó vì nó không có [`crate::Dossier`],
//!    chỉ có `known_events`/`known_powers` mà chỗ gọi đã tính sẵn.
//! 2. **Từng mục** ([`read_line`], [`read_proposal`]): một `line`/`proposal`
//!    thiếu `text`/`power`, thiếu `cites`, hay `cites` đọc không ra số, đều
//!    quy về đúng một kết cục — [`StripReason::NoCitation`]. Với Yuu, một
//!    trích dẫn không đọc được cũng là một trích dẫn không có; không có nhánh
//!    lỗi hình dạng riêng vì mục đích ở đây không phải bắt lỗi cú pháp, mà là
//!    bảo đảm tính truy vết — và cả hai đều dẫn tới cùng một hành động: cắt.
//!
//! ## Sửa chữa có giới hạn, và giới hạn nằm ở đâu
//!
//! Giống hệt `mow_mind::parse`: đúng một phép — cắt lấy đoạn từ dấu `{` đầu
//! tiên tới dấu `}` cuối cùng, để một câu trả lời bọc trong hàng rào mã hoặc
//! kèm một câu dẫn vẫn đọc được. Ngoài phép đó, sai hình dạng là sai hình dạng.
//!
//! ## Bí mật không được đi từ model, qua một câu bị cắt, tới màn hình
//!
//! `text`/`why`/`power` đều đến từ model — văn bản **không đáng tin**. Một
//! model có thể lặp lại nguyên văn một khóa API nó vừa nhìn thấy ở đâu đó
//! (`§20.10`), và câu chứa nó có thể vẫn qua được kiểm chứng trích dẫn (được
//! giữ) hoặc bị cắt (vào `stripped`) — cả hai đường đều cuối cùng tới giao
//! diện hoặc tới log của console True God. Nên mọi chuỗi lấy từ model đều đi
//! qua [`sanitize`], dùng lại `mow_llm::provider::che_bi_mat` thay vì viết lại
//! — một quy tắc che duy nhất trong cả workspace thì không lệch được.

use crate::answer::{Answer, Line, Proposal, StripReason, Stripped};
use serde_json::Value;
use std::collections::BTreeSet;

/// Độ dài tối đa của một đoạn văn bản đến từ model trước khi nó vào
/// [`Line::text`], [`Proposal::why`] hay [`Stripped::text`].
///
/// Cả ba cuối cùng đều có thể tới log của console True God — một câu trả lời
/// model dài mười nghìn ký tự không được phép biến một dòng log thành một
/// trang.
const MAX_TEXT_CHARS: usize = 480;

/// Che bí mật và cắt ngắn một đoạn văn bản **đến từ model, chưa được tin
/// cậy**. Xem "Bí mật không được..." trong tài liệu module.
fn sanitize(s: &str) -> String {
    let s = mow_llm::provider::che_bi_mat(s.trim());
    if s.chars().count() <= MAX_TEXT_CHARS {
        return s;
    }
    let head: String = s.chars().take(MAX_TEXT_CHARS).collect();
    format!("{head}… (còn {} ký tự)", s.chars().count() - MAX_TEXT_CHARS)
}

/// Cắt lấy đoạn trông như một đối tượng JSON. Phép sửa chữa **duy nhất**.
fn carve_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&text[start..=end])
}

/// Đọc phần bọc ngoài: có phải JSON, có phải một object. Đây là ranh giới
/// "rác hoàn toàn" — xem mục 1 trong tài liệu module.
fn parse_envelope(text: &str) -> Option<Value> {
    let carved = carve_json(text)?;
    let value: Value = serde_json::from_str(carved).ok()?;
    if value.is_object() {
        Some(value)
    } else {
        None
    }
}

/// Lấy `text`, chấp nhận vắng mặt bằng một chuỗi hiển thị được thay vì rỗng —
/// một [`Stripped`] không có gì để đọc thì không ai debug được bằng nó.
fn extract_text(item: &Value) -> String {
    item.get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(sanitize)
        .unwrap_or_else(|| format!("(không đọc được `text`: {})", sanitize(&item.to_string())))
}

/// Lấy `cites`: chỉ giữ những phần tử đọc được là số nguyên không âm, khử
/// trùng lặp, giữ thứ tự xuất hiện đầu tiên.
///
/// Một phần tử không phải số bị bỏ qua thay vì làm hỏng cả danh sách — model
/// không đáng tin, nhưng một danh sách có ba trích dẫn đúng và một phần tử rác
/// thì đáng giữ ba trích dẫn đúng hơn là coi cả danh sách là không có gì.
fn extract_cites(item: &Value) -> Vec<u64> {
    let mut seen = BTreeSet::new();
    item.get("cites")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .filter(|c| seen.insert(*c))
        .collect()
}

/// Kiểm một danh sách trích dẫn theo đúng hai luật không thương lượng: rỗng
/// thì [`StripReason::NoCitation`]; có một trích dẫn ngoài `known_events` thì
/// [`StripReason::UnknownEvent`] — và **cả câu** rơi theo lỗi đầu tiên gặp
/// phải, không giữ lại phần trích dẫn đúng của một câu có trích dẫn sai.
fn check_cites(cites: &[u64], known_events: &BTreeSet<u64>) -> Result<(), StripReason> {
    if cites.is_empty() {
        return Err(StripReason::NoCitation);
    }
    for &seq in cites {
        if !known_events.contains(&seq) {
            return Err(StripReason::UnknownEvent(seq));
        }
    }
    Ok(())
}

fn read_line(item: &Value, known_events: &BTreeSet<u64>) -> Result<Line, Stripped> {
    let text = extract_text(item);
    let cites = extract_cites(item);
    match check_cites(&cites, known_events) {
        Ok(()) => Ok(Line { text, cites }),
        Err(reason) => Err(Stripped { text, reason }),
    }
}

fn read_proposal(
    item: &Value,
    known_events: &BTreeSet<u64>,
    known_powers: &BTreeSet<String>,
) -> Result<Proposal, Stripped> {
    let power = item
        .get("power")
        .and_then(Value::as_str)
        .map(str::trim)
        .map(sanitize)
        .unwrap_or_default();
    let why = item
        .get("why")
        .and_then(Value::as_str)
        .map(str::trim)
        .map(sanitize)
        .unwrap_or_default();
    let cites = extract_cites(item);
    let label = if power.is_empty() {
        why.clone()
    } else if why.is_empty() {
        power.clone()
    } else {
        format!("{power}: {why}")
    };

    if let Err(reason) = check_cites(&cites, known_events) {
        return Err(Stripped {
            text: label,
            reason,
        });
    }
    if !known_powers.contains(&power) {
        return Err(Stripped {
            text: label,
            reason: StripReason::UnknownPower(power),
        });
    }
    Ok(Proposal { power, why, cites })
}

/// Đọc trả lời model thành [`Answer`] — câu nào trích dẫn sai đã bị cắt.
///
/// # Rác hoàn toàn
///
/// Nếu không tìm được một đối tượng JSON hợp lệ, hàm trả về `Answer::default()`
/// — ba trường đều rỗng. Đây **không phải** là một câu trả lời hợp lệ có nội
/// dung rỗng; đó là tín hiệu để chỗ gọi rơi về [`crate::without_model`], vì
/// hàm này không có [`crate::Dossier`] để tự làm việc đó (xem tài liệu module).
///
/// `lines`/`proposals` rỗng nhưng `stripped` không rỗng cũng là một cách hợp
/// lệ để "trả lời rỗng" xảy ra: mọi mục model đưa ra đều trích dẫn sai. Chỗ
/// gọi coi hai trường hợp này như nhau (`lines` và `proposals` đều rỗng) và xử
/// lý giống nhau — xem `crate::yuu::Yuu::ask`.
#[must_use]
pub fn read_answer(
    text: &str,
    known_events: &BTreeSet<u64>,
    known_powers: &BTreeSet<String>,
) -> Answer {
    let Some(value) = parse_envelope(text) else {
        return Answer::default();
    };

    let mut lines = Vec::new();
    let mut stripped = Vec::new();
    for item in value
        .get("lines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match read_line(item, known_events) {
            Ok(line) => lines.push(line),
            Err(s) => stripped.push(s),
        }
    }

    let mut proposals = Vec::new();
    for item in value
        .get("proposals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match read_proposal(item, known_events, known_powers) {
            Ok(p) => proposals.push(p),
            Err(s) => stripped.push(s),
        }
    }

    Answer {
        lines,
        proposals,
        stripped,
    }
}
