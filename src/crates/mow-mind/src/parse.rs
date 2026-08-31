//! Đọc câu trả lời của model thành một [`Choice`] — hoặc thành một lý do rơi.
//!
//! ## Đây là chỗ `§10.5` được thi hành
//!
//! "Model chỉ chọn từ action mà engine công bố cho entity" là một câu dễ gật
//! đầu và dễ quên viết. Nó được viết ở đúng một chỗ: [`read_choice`]. Mọi đường
//! từ văn bản của model tới [`Choice`] đi qua hàm này, nên không có đường vòng.
//!
//! Và nó trả về **đúng phần tử trong registry**, không phải chuỗi model gõ. Nhờ
//! vậy `Eat`, `eat ` và `eat` quy về một giá trị, còn chỗ gọi không bao giờ phải
//! tự chuẩn hóa — mà việc chuẩn hóa lặp lại ở nhiều chỗ gọi là cách một biến thể
//! chính tả lọt qua được một chỗ trong số đó.
//!
//! ## Sửa chữa có giới hạn, và giới hạn nằm ở đâu
//!
//! `§20.10` cho phép "repair giới hạn hoặc bỏ; không lặp vô hạn". Giới hạn ở
//! đây là **đúng một phép**: cắt lấy đoạn từ dấu `{` đầu tiên tới dấu `}` cuối
//! cùng, để một câu trả lời bọc trong hàng rào mã hoặc kèm một câu dẫn vẫn đọc
//! được. Không gọi lại model, không đoán trường thiếu, không suy diễn hành động
//! từ văn xuôi. Ngoài phép đó ra, sai hình dạng là sai hình dạng.
//!
//! Ranh giới của phép đó đáng nói rõ vì nó không hiển nhiên: một đối tượng bọc
//! trong đúng một lớp vỏ (hàng rào mã, một câu dẫn, một cặp ngoặc vuông) vẫn đọc
//! được, còn một **kế hoạch nhiều bước** kiểu `§10.6` thì không — phép cắt sinh
//! ra một chuỗi không phải JSON và nó rơi. Đó là kết quả đúng: prompt này hỏi
//! một hành động, và lặng lẽ lấy bước đầu của một kế hoạch là trả lời một câu
//! hỏi không ai đặt ra.
//!
//! ## Nghiêm với `target`, dễ với `reason`
//!
//! Hai trường này không cùng hạng, nên không chịu cùng mức nghiêm khắc:
//!
//! - `target` **ảnh hưởng việc engine sẽ thử làm gì**. Một `target` sai kiểu là
//!   một câu trả lời sai hình dạng, và nó phải rơi.
//! - `reason` là lời kể, không có hiệu lực ở bất kỳ nhánh `if` nào. Thiếu nó
//!   hoặc sai kiểu chỉ mất một dòng log, và làm đứng một NPC vì thiếu một dòng
//!   log là một cái giá sai.
//!
//! ## Trường lạ bị bỏ, và đó chính là hai ràng buộc đầu
//!
//! Model có thể trả thêm `"observed"`, `"i_can_see"`, `"preconditions_met"`.
//! Không trường nào trong số đó có ô để đi vào [`Choice`], nên chúng biến mất
//! tại đây. Đó là cách `§10.4` bước 2 và bước 7 được giữ bằng **cấu trúc** chứ
//! không bằng kỷ luật: model không có kênh nào để dựng quan sát hay để khẳng
//! định điều kiện tiên quyết.

use crate::choice::{Choice, FallbackReason};
use crate::prompt::canonical_registry;
use serde_json::Value;

/// Độ dài tối đa của một đoạn văn bản đi vào lý do hoặc vào sổ.
///
/// Lý do rơi đi vào log, và log đi vào báo cáo sự cố. Một câu trả lời model dài
/// mười nghìn ký tự không được phép biến một dòng log thành một trang.
const MAX_DETAIL_CHARS: usize = 400;

/// Độ dài tối đa của [`Choice::reason`].
const MAX_REASON_CHARS: usize = 240;

/// Vì sao không đọc được, kèm chi tiết để dán vào log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    /// Nhánh fallback tương ứng.
    pub reason: FallbackReason,
    /// Văn bản gốc đã cắt ngắn và đã che bí mật.
    pub detail: String,
}

/// Cắt ngắn và che bí mật một đoạn văn bản trước khi cho nó vào log.
///
/// Dùng lại [`mow_llm::provider::che_bi_mat`] thay vì viết lại: văn bản model
/// trả về có thể chứa bất cứ thứ gì, kể cả một khóa API mà nó vừa được cho xem ở
/// đâu đó, và một quy tắc che duy nhất trong cả workspace thì không lệch được.
#[must_use]
pub fn trim_for_log(s: &str) -> String {
    let s = mow_llm::provider::che_bi_mat(s.trim());
    if s.chars().count() <= MAX_DETAIL_CHARS {
        return s;
    }
    let head: String = s.chars().take(MAX_DETAIL_CHARS).collect();
    format!(
        "{head}… (còn {} ký tự)",
        s.chars().count() - MAX_DETAIL_CHARS
    )
}

/// Cắt lấy đoạn trông như một đối tượng JSON. Phép sửa chữa **duy nhất**.
fn carve_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end < start {
        return None;
    }
    // `{` và `}` là ASCII một byte nên `..=end` luôn rơi đúng biên ký tự.
    Some(&text[start..=end])
}

/// Đọc văn bản model thành một [`Choice`] đã kiểm theo action registry.
///
/// `registry` được chuẩn hóa ngay tại đây bằng [`canonical_registry`], nên chỗ
/// gọi truyền vào theo thứ tự nào cũng cho cùng kết quả.
///
/// # Errors
///
/// - [`FallbackReason::EmptyRegistry`] — registry không có hành động nào, nên
///   không có gì để hợp lệ.
/// - [`FallbackReason::BadShape`] — không tìm thấy JSON, JSON hỏng, không phải
///   đối tượng, thiếu `action`, `action` không phải chuỗi hoặc rỗng, `target`
///   có mặt nhưng không phải chuỗi và không phải `null`.
/// - [`FallbackReason::NotInRegistry`] — `action` không nằm trong registry.
pub fn read_choice(text: &str, registry: &[String]) -> Result<Choice, ReadError> {
    let allowed = canonical_registry(registry);
    if allowed.is_empty() {
        return Err(ReadError {
            reason: FallbackReason::EmptyRegistry,
            detail: trim_for_log(text),
        });
    }

    let bad_shape = |what: &str| ReadError {
        reason: FallbackReason::BadShape(trim_for_log(what)),
        detail: trim_for_log(text),
    };

    let carved = carve_json(text)
        .ok_or_else(|| bad_shape(&format!("không tìm thấy đối tượng JSON trong: {text}")))?;
    let value: Value = serde_json::from_str(carved)
        .map_err(|e| bad_shape(&format!("JSON hỏng ({e}): {carved}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| bad_shape(&format!("không phải một đối tượng JSON: {carved}")))?;

    let raw_action = object
        .get("action")
        .ok_or_else(|| bad_shape(&format!("thiếu trường `action`: {carved}")))?
        .as_str()
        .ok_or_else(|| bad_shape(&format!("`action` không phải chuỗi: {carved}")))?
        .trim();
    if raw_action.is_empty() {
        return Err(bad_shape(&format!("`action` rỗng: {carved}")));
    }

    // So khớp không phân biệt hoa thường, nhưng **trả về chuỗi của registry**.
    let action = allowed
        .iter()
        .find(|a| a.eq_ignore_ascii_case(raw_action))
        .ok_or_else(|| ReadError {
            reason: FallbackReason::NotInRegistry(trim_for_log(raw_action)),
            detail: trim_for_log(carved),
        })?
        .clone();

    let target = match object.get("target") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(trim_for_log(s))
            }
        }
        Some(other) => {
            return Err(bad_shape(&format!(
                "`target` phải là chuỗi hoặc null, nhận được: {other}"
            )))
        }
    };

    // `reason` không có hiệu lực, nên hình dạng sai của nó không được phép làm
    // đứng một NPC. Thiếu hoặc sai kiểu thì mất lời kể, không mất quyết định.
    let reason = object
        .get("reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .map(|s| {
            let s = mow_llm::provider::che_bi_mat(s);
            s.chars().take(MAX_REASON_CHARS).collect::<String>()
        })
        .unwrap_or_default();

    Ok(Choice {
        action,
        target,
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::{read_choice, MAX_REASON_CHARS};
    use crate::choice::FallbackReason;

    fn registry() -> Vec<String> {
        ["eat", "go_to", "idle", "sleep", "socialize", "work"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    #[test]
    fn a_well_formed_answer_inside_the_registry_reads() {
        let c = read_choice(
            r#"{"action": "eat", "target": null, "reason": "doi qua"}"#,
            &registry(),
        )
        .expect("phai doc duoc");
        assert_eq!(c.action, "eat");
        assert_eq!(c.target, None);
        assert_eq!(c.reason, "doi qua");
    }

    /// Chuỗi trả về phải là **chuỗi của registry**, không phải chuỗi model gõ.
    #[test]
    fn spelling_variants_collapse_onto_the_registry_entry() {
        for raw in ["eat", "EAT", "  Eat  ", "eAt"] {
            let text = format!(r#"{{"action": "{raw}"}}"#);
            let c = read_choice(&text, &registry()).expect("phai doc duoc");
            assert_eq!(
                c.action, "eat",
                "`{raw}` khong quy ve dung phan tu registry"
            );
        }
    }

    /// `§10.5`: ngoài registry là **lỗi validate**, không phải một hành động lạ.
    #[test]
    fn an_action_outside_the_registry_is_a_validation_error() {
        let e = read_choice(
            r#"{"action": "fly", "reason": "vi sao khong"}"#,
            &registry(),
        )
        .expect_err("`fly` khong duoc phep");
        assert_eq!(e.reason, FallbackReason::NotInRegistry("fly".to_owned()));
        assert_eq!(e.reason.label(), "not_in_registry");
    }

    #[test]
    fn plain_prose_is_a_bad_shape() {
        let e = read_choice("Toi se di an com bay gio.", &registry())
            .expect_err("van xuoi khong phai JSON");
        assert!(matches!(e.reason, FallbackReason::BadShape(_)), "{e:?}");
    }

    #[test]
    fn broken_json_is_a_bad_shape() {
        let e = read_choice(r#"{"action": "eat""#, &registry()).expect_err("JSON hong");
        assert!(matches!(e.reason, FallbackReason::BadShape(_)), "{e:?}");
    }

    #[test]
    fn a_missing_action_is_a_bad_shape_not_an_empty_action() {
        let e = read_choice(r#"{"target": "home", "reason": "ve nha"}"#, &registry())
            .expect_err("thieu `action`");
        match &e.reason {
            FallbackReason::BadShape(msg) => assert!(msg.contains("action"), "{msg}"),
            other => panic!("sai nhanh: {other:?}"),
        }
    }

    #[test]
    fn a_non_string_action_is_a_bad_shape() {
        let e = read_choice(r#"{"action": 7}"#, &registry()).expect_err("`action` la so");
        assert!(matches!(e.reason, FallbackReason::BadShape(_)), "{e:?}");
        let e = read_choice(r#"{"action": "   "}"#, &registry()).expect_err("`action` rong");
        assert!(matches!(e.reason, FallbackReason::BadShape(_)), "{e:?}");
    }

    /// Prompt này hỏi **một** hành động. Một kế hoạch nhiều bước kiểu `§10.6`
    /// là câu trả lời cho một câu hỏi khác, nên nó phải rơi chứ không được im
    /// lặng lấy bước đầu.
    #[test]
    fn a_multi_step_plan_is_a_bad_shape() {
        let e = read_choice(r#"[{"action": "eat"}, {"action": "work"}]"#, &registry())
            .expect_err("ke hoach nhieu buoc khong phai mot lua chon");
        assert!(matches!(e.reason, FallbackReason::BadShape(_)), "{e:?}");
    }

    /// Ranh giới của phép sửa chữa duy nhất, viết ra để nó là một quyết định
    /// chứ không phải một tai nạn: một đối tượng bọc trong đúng một lớp vỏ vẫn
    /// đọc được.
    #[test]
    fn a_single_object_wrapped_in_an_array_still_reads() {
        let c = read_choice(r#"[{"action": "eat"}]"#, &registry()).expect("phai doc duoc");
        assert_eq!(c.action, "eat");
    }

    #[test]
    fn target_absent_null_or_blank_all_mean_none() {
        for text in [
            r#"{"action": "sleep"}"#,
            r#"{"action": "sleep", "target": null}"#,
            r#"{"action": "sleep", "target": "   "}"#,
        ] {
            let c = read_choice(text, &registry()).expect("phai doc duoc");
            assert_eq!(c.target, None, "{text}");
        }
    }

    /// Nghiêm với `target` vì nó quyết định engine sẽ **thử làm gì**.
    #[test]
    fn a_non_string_target_is_a_bad_shape() {
        let e = read_choice(
            r#"{"action": "go_to", "target": {"place": "home"}}"#,
            &registry(),
        )
        .expect_err("`target` phai la chuoi hoac null");
        assert!(matches!(e.reason, FallbackReason::BadShape(_)), "{e:?}");
    }

    /// Dễ với `reason` vì nó không có hiệu lực ở bất kỳ nhánh `if` nào.
    #[test]
    fn a_missing_or_mistyped_reason_never_stops_the_world() {
        for text in [
            r#"{"action": "work"}"#,
            r#"{"action": "work", "reason": 12}"#,
            r#"{"action": "work", "reason": null}"#,
        ] {
            let c = read_choice(text, &registry()).expect("phai doc duoc");
            assert_eq!(c.action, "work");
            assert_eq!(c.reason, "", "{text}");
        }
    }

    #[test]
    fn a_long_reason_is_capped() {
        let long = "a".repeat(MAX_REASON_CHARS * 3);
        let text = format!(r#"{{"action": "idle", "reason": "{long}"}}"#);
        let c = read_choice(&text, &registry()).expect("phai doc duoc");
        assert_eq!(c.reason.chars().count(), MAX_REASON_CHARS);
    }

    /// Ràng buộc 1 và 2 giữ bằng cấu trúc: model không có ô nào để dựng quan
    /// sát hay để khẳng định điều kiện tiên quyết.
    #[test]
    fn extra_fields_are_dropped_so_there_is_no_channel_for_them() {
        let text = r#"{
            "action": "eat",
            "observed": "toi thay mot con rong o phia bac",
            "i_can_see": ["cong thanh", "bao vat"],
            "preconditions_met": true,
            "capability": "bay"
        }"#;
        let c = read_choice(text, &registry()).expect("phai doc duoc");
        assert_eq!(c.action, "eat");
        assert_eq!(c.target, None);
        assert_eq!(c.reason, "");
        let dump = format!("{c:?}");
        assert!(
            !dump.contains("rong"),
            "quan sat model bia ra lot vao: {dump}"
        );
        assert!(!dump.contains("preconditions"), "{dump}");
    }

    /// Phép sửa chữa **duy nhất** được phép (`§20.10`: repair có giới hạn).
    #[test]
    fn one_bounded_repair_strips_a_code_fence_and_a_preamble() {
        let text = "Duoc thoi, day la lua chon:\n```json\n{\"action\": \"work\"}\n```\n";
        let c = read_choice(text, &registry()).expect("phai doc duoc");
        assert_eq!(c.action, "work");
    }

    #[test]
    fn an_empty_registry_is_its_own_branch() {
        let e = read_choice(r#"{"action": "eat"}"#, &[]).expect_err("registry rong");
        assert_eq!(e.reason, FallbackReason::EmptyRegistry);
        assert_eq!(e.reason.label(), "empty_registry");
    }

    #[test]
    fn a_key_in_the_model_text_never_reaches_the_log() {
        let text = "khoa cua ban la sk-or-v1-KHOATHAT-abc123 nhe";
        let e = read_choice(text, &registry()).expect_err("van xuoi");
        assert!(
            !e.detail.contains("KHOATHAT"),
            "khoa lot vao so: {}",
            e.detail
        );
        assert!(e.detail.contains("sk-***"), "{}", e.detail);
    }

    #[test]
    fn registry_order_does_not_change_the_verdict() {
        let mut reversed = registry();
        reversed.reverse();
        let a = read_choice(r#"{"action": "go_to", "target": "well"}"#, &registry());
        let b = read_choice(r#"{"action": "go_to", "target": "well"}"#, &reversed);
        assert_eq!(a, b);
    }
}
