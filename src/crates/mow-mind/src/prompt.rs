//! Dựng prompt từ một [`Observation`] — xác định, và không lấy gì ngoài nó.
//!
//! ## Vì sao tính xác định ở đây là bắt buộc chứ không phải đáng có
//!
//! `mow_llm::Request::hash` gồm cả chuỗi đã render, và hash đó là **khóa của
//! bản ghi `REPLAY`**. Nếu prompt đổi giữa hai lần chạy — vì một `HashSet` duyệt
//! khác thứ tự, vì một dấu thời gian, vì một danh sách chưa sắp — thì:
//!
//! - bộ ghi không bao giờ trúng, và `REPLAY` trả [`mow_llm::LlmError::NoCassette`];
//! - `RECORD` sinh một dòng mới mỗi lần chạy, nên bộ ghi phình ra vô hạn;
//! - CI hoặc phải gọi mạng thật, hoặc phải đỏ. Cả hai đều là hỏng.
//!
//! Nên [`prompt_of`] là hàm thuần của đúng hai đối số của nó. Nó không đọc biến
//! môi trường, không đọc tệp, không đọc đồng hồ, không dùng RNG. Có một bài test
//! đặt một chuỗi bí mật vào biến môi trường và khẳng định nó không xuất hiện.
//!
//! ## Ba việc chuẩn hóa, và vì sao đúng ba
//!
//! 1. **Registry được sắp và khử trùng lặp.** Nó là một *tập*; thứ tự chỗ gọi
//!    khai ra không phải là thông tin, và để nó lọt vào prompt là để một chi
//!    tiết ngẫu nhiên quyết định khóa bản ghi.
//! 2. **`nearby` được sắp và khử trùng lặp.** Cùng lập luận: "ai đang đứng
//!    quanh đây" là một tập, và engine thường gom nó từ một container không có
//!    thứ tự ổn định.
//! 3. **`recent` giữ nguyên thứ tự, chỉ cắt bớt.** Đây là ngoại lệ có chủ ý:
//!    dòng thời gian *có* thứ tự, sắp nó là nói dối về thứ tự sự kiện. Cắt về
//!    [`MAX_RECENT`] mục cuối là để prompt có trần độ dài (`§20.9`), và phép
//!    cắt đó tất định nên không phá tính xác định.

use crate::observation::Observation;

/// Định danh prompt, dùng làm khóa stub và tên tệp bản ghi.
pub const PROMPT_ID: &str = "npc.mind.decide";

/// Phiên bản prompt.
///
/// Tăng số này **mỗi lần đổi một ký tự** trong prompt. `§20.10` đòi ghi lại
/// prompt version cùng với kết quả: không có nó thì một bộ ghi cũ và một prompt
/// mới sẽ trông giống nhau trong log, và không ai giải thích được vì sao hành vi
/// đổi.
pub const PROMPT_VERSION: u32 = 1;

/// Số mục [`Observation::recent`] tối đa đưa vào prompt.
///
/// Trần chứ không phải toàn bộ lịch sử (`§20.9`): một nhân vật sống lâu có hàng
/// nghìn sự kiện, và nhét hết vào prompt là cách chắc chắn nhất để hóa đơn tăng
/// mà chất lượng quyết định thì không.
pub const MAX_RECENT: usize = 8;

/// Sắp, cắt khoảng trắng, bỏ mục rỗng và khử trùng lặp một action registry.
///
/// Trả về **dạng chuẩn**: cùng một tập hành động luôn cho cùng một `Vec`, bất kể
/// chỗ gọi khai theo thứ tự nào. [`crate::Mind`] giữ đúng dạng này, và
/// [`crate::read_choice`] so khớp trên đúng dạng này — nên prompt và validator
/// không bao giờ nhìn thấy hai tập khác nhau.
#[must_use]
pub fn canonical_registry(registry: &[String]) -> Vec<String> {
    canonicalize(registry.to_vec())
}

/// Bản nuốt luôn `Vec` của [`canonical_registry`], cho [`crate::Mind::new`].
///
/// Hai hàm, **một** luật: nếu chuẩn hóa được chép ra làm hai bản thì một ngày
/// nào đó chúng sẽ lệch nhau, và triệu chứng sẽ là một registry mà prompt in ra
/// khác registry mà validator so khớp — thứ không ai nghĩ tới khi đi tìm lý do
/// một hành động hợp lệ bị từ chối.
pub(crate) fn canonicalize(registry: Vec<String>) -> Vec<String> {
    let mut v: Vec<String> = registry
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

const HEADER: &str =
    "Bạn là một cư dân trong một thế giới mô phỏng. Hãy chọn đúng một việc để làm ngay bây giờ.

== QUAN SÁT ==
Phần dưới do engine dựng từ giác quan của nhân vật. Đây là toàn bộ những gì
nhân vật biết lúc này, không có gì thêm.
";

const ACTIONS_HEAD: &str = "
== HÀNH ĐỘNG ĐƯỢC PHÉP ==
Chỉ được chọn đúng một giá trị trong danh sách này.
";

const RULES: &str = "
== LUẬT ==
1. Không mô tả thêm bất cứ thứ gì nhân vật thấy, nghe hay biết. Quan sát ở trên
   là đủ và là tất cả. Một dữ kiện không có ở trên là một dữ kiện nhân vật không
   có.
2. Không khẳng định rằng việc bạn chọn thực hiện được. Bạn chỉ đề xuất; engine
   tự kiểm điều kiện tiên quyết và có quyền từ chối.
3. Trả về đúng một đối tượng JSON, không lời dẫn, không hàng rào mã.

== HÌNH DẠNG TRẢ LỜI ==
{\"action\": \"<một giá trị trong danh sách trên>\", \"target\": \"<tên, hoặc null>\", \"reason\": \"<một câu ngắn>\"}
";

/// Ghi một dòng `khóa: giá trị`.
fn field(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value);
    out.push('\n');
}

/// Prompt cho một quan sát và một action registry.
///
/// Công khai để test khẳng định về prompt mà **không cần model**: hình dạng của
/// prompt là một hợp đồng, và một hợp đồng chỉ kiểm được qua mạng thì trên thực
/// tế là không kiểm.
///
/// Thứ tự các trường bám đúng thứ tự khai báo trong [`Observation`].
#[must_use]
pub fn prompt_of(obs: &Observation, registry: &[String]) -> String {
    let actions = canonical_registry(registry);

    let mut nearby: Vec<&str> = obs
        .nearby
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    nearby.sort_unstable();
    nearby.dedup();

    let mut out = String::with_capacity(1024);
    out.push_str(HEADER);

    field(&mut out, "self_name", &obs.self_name);
    field(&mut out, "role", &obs.role);
    field(&mut out, "hunger", &obs.hunger.to_string());
    field(&mut out, "time_of_day", &obs.time_of_day);
    field(&mut out, "at", &obs.at);
    let nearby_line = if nearby.is_empty() {
        "(không có ai)".to_owned()
    } else {
        nearby.join(", ")
    };
    field(&mut out, "nearby", &nearby_line);

    out.push_str("recent:\n");
    // Giữ `MAX_RECENT` mục **cuối**: `recent` xếp cũ trước mới sau, nên phần
    // đáng giữ khi phải cắt là phần mới nhất.
    let recent: Vec<&String> = obs.recent.iter().rev().take(MAX_RECENT).rev().collect();
    if recent.is_empty() {
        out.push_str("  (không có gì)\n");
    } else {
        for (i, item) in recent.iter().enumerate() {
            out.push_str("  ");
            out.push_str(&(i + 1).to_string());
            out.push_str(". ");
            out.push_str(item);
            out.push('\n');
        }
    }

    out.push_str(ACTIONS_HEAD);
    if actions.is_empty() {
        // Không xảy ra qua `Mind` — nó chặn registry rỗng từ trước và không tốn
        // một lời gọi nào. Nhưng `prompt_of` là hàm công khai, và một hàm công
        // khai không được sinh ra một danh sách trống trông như một lỗi in ấn.
        out.push_str("- (không có hành động nào được phép)\n");
    } else {
        for a in &actions {
            out.push_str("- ");
            out.push_str(a);
            out.push('\n');
        }
    }

    out.push_str(RULES);
    out
}

#[cfg(test)]
mod tests {
    use super::{canonical_registry, prompt_of, MAX_RECENT};
    use crate::observation::Observation;

    fn registry() -> Vec<String> {
        ["work", "eat", "go_to", "idle", "sleep", "socialize"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    fn mara() -> Observation {
        Observation {
            self_name: "Mara".to_owned(),
            role: "farmer".to_owned(),
            hunger: 62,
            time_of_day: "morning".to_owned(),
            at: "well".to_owned(),
            nearby: vec!["Ila".to_owned(), "Doran".to_owned()],
            recent: vec!["gieng gan can".to_owned(), "Doran chao".to_owned()],
        }
    }

    #[test]
    fn prompt_is_deterministic_across_a_hundred_calls() {
        let obs = mara();
        let reg = registry();
        let first = prompt_of(&obs, &reg);
        for _ in 0..100 {
            assert_eq!(prompt_of(&obs, &reg), first, "prompt doi giua hai lan goi");
        }
    }

    /// Registry là một *tập*. Thứ tự chỗ gọi khai ra không được lọt vào prompt,
    /// vì nó sẽ lọt vào khóa của bản ghi `REPLAY`.
    #[test]
    fn registry_order_and_duplicates_do_not_change_the_prompt() {
        let obs = mara();
        let a = prompt_of(&obs, &registry());
        let mut shuffled = registry();
        shuffled.reverse();
        shuffled.push("eat".to_owned());
        shuffled.push("  work  ".to_owned());
        shuffled.push(String::new());
        assert_eq!(prompt_of(&obs, &shuffled), a);
    }

    #[test]
    fn canonical_registry_sorts_trims_and_dedups() {
        let raw = vec![
            "work".to_owned(),
            " eat ".to_owned(),
            "eat".to_owned(),
            String::new(),
            "  ".to_owned(),
        ];
        assert_eq!(canonical_registry(&raw), vec!["eat", "work"]);
    }

    #[test]
    fn nearby_is_sorted_and_deduplicated() {
        let mut obs = mara();
        obs.nearby = vec!["Ila".to_owned(), "Doran".to_owned(), "Ila".to_owned()];
        let a = prompt_of(&obs, &registry());
        obs.nearby = vec!["Doran".to_owned(), "Ila".to_owned()];
        assert_eq!(prompt_of(&obs, &registry()), a);
        assert!(a.contains("nearby: Doran, Ila"), "{a}");
    }

    #[test]
    fn empty_nearby_and_recent_read_as_absence_not_as_a_blank() {
        let obs = Observation::new("Mara", "farmer", "home");
        let p = prompt_of(&obs, &registry());
        assert!(p.contains("nearby: (không có ai)"), "{p}");
        assert!(p.contains("(không có gì)"), "{p}");
    }

    /// `recent` là một dòng thời gian: sắp nó lại là nói dối về thứ tự sự kiện.
    #[test]
    fn recent_keeps_the_order_it_was_given() {
        let mut obs = mara();
        obs.recent = vec!["z cuoi".to_owned(), "a dau".to_owned()];
        let p = prompt_of(&obs, &registry());
        let first = p.find("z cuoi").expect("phai co muc dau");
        let second = p.find("a dau").expect("phai co muc sau");
        assert!(first < second, "recent bi sap lai:\n{p}");
    }

    #[test]
    fn recent_is_capped_and_keeps_the_newest() {
        let mut obs = mara();
        obs.recent = (0..MAX_RECENT + 5)
            .map(|i| format!("su kien {i}"))
            .collect();
        let p = prompt_of(&obs, &registry());
        assert!(!p.contains("su kien 0"), "muc cu nhat phai bi cat:\n{p}");
        assert!(
            p.contains(&format!("su kien {}", MAX_RECENT + 4)),
            "muc moi nhat phai duoc giu:\n{p}"
        );
        let kept = p.matches("su kien ").count();
        assert_eq!(kept, MAX_RECENT, "giu {kept} muc thay vi {MAX_RECENT}");
    }

    /// Ràng buộc 1: prompt chỉ chứa những gì có trong quan sát. Một bí mật đặt
    /// trong biến môi trường không có đường nào vào đây.
    #[test]
    fn prompt_holds_nothing_beyond_the_observation() {
        const SECRET: &str = "MOW_MIND_SECRET_KHONG_DUOC_LOT_1a2b3c";
        std::env::set_var("MOW_MIND_TEST_SECRET", SECRET);
        let obs = mara();
        let p = prompt_of(&obs, &registry());
        assert!(!p.contains(SECRET), "bi mat lot vao prompt:\n{p}");
        assert!(!p.contains("MOW_MIND_TEST_SECRET"), "{p}");
        // Và giá trị của biến không ảnh hưởng chuỗi sinh ra.
        std::env::set_var("MOW_MIND_TEST_SECRET", "mot gia tri khac han");
        assert_eq!(prompt_of(&obs, &registry()), p);
        std::env::remove_var("MOW_MIND_TEST_SECRET");
    }

    #[test]
    fn prompt_lists_every_allowed_action_and_nothing_else() {
        let p = prompt_of(&mara(), &registry());
        for a in registry() {
            assert!(p.contains(&format!("- {a}\n")), "thieu `{a}`:\n{p}");
        }
        assert!(!p.contains("- fly\n"), "{p}");
    }

    /// Hai ràng buộc đầu của `§10` phải nói thẳng trong prompt, không chỉ nằm
    /// trong tài liệu.
    #[test]
    fn prompt_states_both_constraints() {
        let p = prompt_of(&mara(), &registry());
        assert!(p.contains("Không mô tả thêm"), "thieu rang buoc 1:\n{p}");
        assert!(p.contains("Không khẳng định"), "thieu rang buoc 2:\n{p}");
        assert!(p.contains("engine"), "{p}");
    }

    #[test]
    fn every_observation_field_reaches_the_prompt_in_declaration_order() {
        let p = prompt_of(&mara(), &registry());
        let order = [
            "self_name: Mara",
            "role: farmer",
            "hunger: 62",
            "time_of_day: morning",
            "at: well",
            "nearby: ",
            "recent:",
        ];
        let mut last = 0;
        for key in order {
            let at = p.find(key).unwrap_or_else(|| panic!("thieu `{key}`:\n{p}"));
            assert!(at > last || last == 0, "`{key}` sai thu tu:\n{p}");
            last = at;
        }
    }

    #[test]
    fn empty_registry_does_not_panic() {
        let p = prompt_of(&mara(), &[]);
        assert!(p.contains("(không có hành động nào được phép)"), "{p}");
    }
}
