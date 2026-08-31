//! Chỗ tầng nhận thức gặp tầng Routine (`§10.3`).
//!
//! ## Vì sao fallback phải là lịch sinh hoạt, không phải một hằng số
//!
//! Cám dỗ khi viết fallback là chọn một hành động vô hại: `idle`. Nó không bao
//! giờ sai, và đó chính là vấn đề — nó cũng không bao giờ đúng. Một tuần
//! provider chập chờn sẽ cho ra một ngôi làng đứng như tượng, và người xem đọc
//! nó thành "trò chơi hỏng" chứ không đọc thành "mạng chập".
//!
//! `§10.3` đã có sẵn câu trả lời: tháp điều khiển có bốn tầng dưới LLM, và
//! [`mow_society::routine::decide`] là tầng thứ hai. Khi model im lặng, cư dân
//! quay về nhịp của vai nó — sáng ra giếng, ngày ra đồng, đói thì bỏ việc mà về
//! ăn. Không ai nhận ra là LLM đang chết, và đó đúng là điều `§20.10` muốn:
//! "entity dùng fallback plan hợp lý... không nhận quyền năng mới vì model lỗi".
//!
//! ## Vì sao đổi enum sang tên nằm ở đây
//!
//! [`crate::Observation`] toàn chuỗi, còn [`Situation`] toàn enum. Chỗ nối giữa
//! hai thứ đó phải nằm ở đúng một nơi, nếu không mỗi chỗ gọi sẽ tự đặt tên cho
//! `Place::Well` và một chỗ trong số đó sẽ gõ `"wells"`. Đặt ở đây thì
//! [`intent_of`] và [`choice_of_intent`] là hai chiều của cùng một bảng, và có
//! một bài test khứ hồi giữ chúng khớp nhau.

use crate::choice::Choice;
use crate::observation::Observation;
use mow_society::routine::{day_percent, decide, Intent, Place, Role, Situation};

/// Action registry cho một cư dân làng (`§10.5`).
///
/// Sáu hành động này là **giao của hai tập**: những gì `§10.5` liệt kê, và
/// những gì [`Intent`] thật sự thực hiện được. Công bố một hành động mà engine
/// chưa làm được sẽ cho model một cái bẫy — nó sẽ chọn, sẽ qua validate, rồi
/// [`intent_of`] trả `None` và cư dân đứng hình mà không ai biết vì sao.
///
/// Đã sắp sẵn theo thứ tự [`crate::canonical_registry`].
pub const ACTIONS_VILLAGE: [&str; 6] = ["eat", "go_to", "idle", "sleep", "socialize", "work"];

/// [`ACTIONS_VILLAGE`] dạng `Vec<String>` để đưa thẳng vào [`crate::Mind::new`].
#[must_use]
pub fn village_registry() -> Vec<String> {
    ACTIONS_VILLAGE.iter().map(|s| (*s).to_owned()).collect()
}

/// Tên của một địa điểm. Trùng với `serde` `snake_case` của [`Place`].
#[must_use]
pub const fn place_name(p: Place) -> &'static str {
    match p {
        Place::Home => "home",
        Place::Workplace => "workplace",
        Place::Well => "well",
        Place::Square => "square",
        Place::Field => "field",
    }
}

/// Địa điểm từ tên, không phân biệt hoa thường.
///
/// `None` nghĩa là model gọi tên một nơi **không tồn tại**. Đó không phải lý do
/// để đoán: đoán ở đây là dựng ra một nơi chốn thay cho thế giới, đúng thứ
/// `§10.4` bước 2 cấm.
#[must_use]
pub fn place_from_name(s: &str) -> Option<Place> {
    let s = s.trim();
    [
        Place::Home,
        Place::Workplace,
        Place::Well,
        Place::Square,
        Place::Field,
    ]
    .into_iter()
    .find(|p| place_name(*p).eq_ignore_ascii_case(s))
}

/// Tên của một vai. Trùng với `serde` `snake_case` của [`Role`].
#[must_use]
pub const fn role_name(r: Role) -> &'static str {
    match r {
        Role::Farmer => "farmer",
        Role::Smith => "smith",
        Role::Hunter => "hunter",
        Role::Elder => "elder",
        Role::Child => "child",
    }
}

/// Nhãn buổi trong ngày từ một [`Situation`].
///
/// Nhãn chứ không phải tick, và đó là yêu cầu của `§20.8`: cache phải theo
/// "situation abstraction", không theo prompt thô có dấu thời gian. Một tick
/// tuyệt đối trong prompt nghĩa là **mọi** lời gọi đều trượt cache và đều sinh
/// một dòng bản ghi mới.
///
/// Mốc là phần trăm nguyên của ngày (`§P10.2.1`: không số thực).
#[must_use]
pub fn time_of_day(s: &Situation) -> &'static str {
    match day_percent(s.tick, s.ticks_per_day) {
        20..=34 => "dawn",
        35..=49 => "morning",
        50..=64 => "midday",
        65..=79 => "afternoon",
        80..=91 => "evening",
        // `0..=19` và `92..`: đêm là khoảng **bọc qua nửa đêm**, nên nó là hai
        // đoạn ở hai đầu chứ không phải một đoạn ở giữa.
        _ => "night",
    }
}

/// Dựng một [`Observation`] từ trạng thái mô phỏng.
///
/// `nearby` và `recent` do **chỗ gọi** cung cấp và chỗ gọi phải lọc chúng theo
/// giác quan trước (`§10.4` bước 2). [`Situation::nearby`] chỉ là một con số nên
/// nó không dùng được ở đây — và cố suy ra tên từ một con số sẽ là bịa.
#[must_use]
pub fn observation_of(
    s: &Situation,
    self_name: &str,
    nearby: &[String],
    recent: &[String],
) -> Observation {
    Observation {
        self_name: self_name.to_owned(),
        role: role_name(s.role).to_owned(),
        hunger: s.hunger,
        time_of_day: time_of_day(s).to_owned(),
        at: place_name(s.at).to_owned(),
        nearby: nearby.to_vec(),
        recent: recent.to_vec(),
    }
}

/// Đổi một [`Intent`] của bộ lập lịch thành một [`Choice`].
#[must_use]
pub fn choice_of_intent(intent: Intent, reason: &str) -> Choice {
    match intent {
        Intent::GoTo { place } => Choice::new("go_to", Some(place_name(place)), reason),
        Intent::Sleep => Choice::new("sleep", None, reason),
        Intent::Eat => Choice::new("eat", None, reason),
        Intent::Work => Choice::new("work", None, reason),
        // `with: None` là hợp lệ và có nghĩa: "nói chuyện với ai đó quanh đây",
        // để lớp mô phỏng tự chọn người.
        Intent::Socialize { with } => Choice::new(
            "socialize",
            with.map(|id| format!("entity:{id}")).as_deref(),
            reason,
        ),
        Intent::Idle => Choice::new("idle", None, reason),
    }
}

/// Fallback có tên: quyết định của tầng Routine tại đúng tick này.
///
/// Đây là thứ nên truyền vào [`crate::Mind::think_with`] mỗi tick.
#[must_use]
pub fn routine_fallback(s: &Situation) -> Choice {
    choice_of_intent(decide(s), "theo nhịp sinh hoạt của vai")
}

/// Đổi một [`Choice`] đã kiểm thành [`Intent`] để mô phỏng thực hiện.
///
/// `None` nghĩa là **engine từ chối**: hành động nằm trong registry nhưng đề
/// xuất không dùng được (`go_to` không có nơi đến, hoặc trỏ tới một nơi không
/// tồn tại). Đó là `§10.4` bước 7 đang làm việc, và trả `None` thay vì đoán một
/// nơi gần đúng là điểm khác nhau giữa một engine và một máy chiều lòng model.
#[must_use]
pub fn intent_of(c: &Choice) -> Option<Intent> {
    match c.action.as_str() {
        "go_to" => {
            let place = place_from_name(c.target.as_deref()?)?;
            Some(Intent::GoTo { place })
        }
        "sleep" => Some(Intent::Sleep),
        "eat" => Some(Intent::Eat),
        "work" => Some(Intent::Work),
        "socialize" => Some(Intent::Socialize {
            with: c
                .target
                .as_deref()
                .and_then(|t| t.strip_prefix("entity:"))
                .and_then(|id| id.parse::<u64>().ok()),
        }),
        "idle" => Some(Intent::Idle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        choice_of_intent, intent_of, observation_of, place_from_name, place_name, role_name,
        routine_fallback, time_of_day, village_registry, ACTIONS_VILLAGE,
    };
    use crate::choice::Choice;
    use crate::prompt::canonical_registry;
    use mow_society::routine::{decide, Intent, Place, Role, Situation};
    use std::collections::BTreeSet;

    const DAY: u64 = 100;

    const ROLES: [Role; 5] = [
        Role::Farmer,
        Role::Smith,
        Role::Hunter,
        Role::Elder,
        Role::Child,
    ];

    const PLACES: [Place; 5] = [
        Place::Home,
        Place::Workplace,
        Place::Well,
        Place::Square,
        Place::Field,
    ];

    fn situation(role: Role, percent: u64, at: Place) -> Situation {
        Situation {
            tick: percent,
            ticks_per_day: DAY,
            role,
            hunger: 5,
            fatigue: 5,
            at,
            nearby: 1,
            nearest: Some(7),
        }
    }

    #[test]
    fn the_village_registry_is_already_canonical() {
        assert_eq!(village_registry(), canonical_registry(&village_registry()));
        assert_eq!(village_registry().len(), ACTIONS_VILLAGE.len());
    }

    /// Bảng tên phải khứ hồi được, nếu không một chỗ nào đó sẽ gõ `"wells"`.
    #[test]
    fn place_names_round_trip() {
        for p in PLACES {
            assert_eq!(place_from_name(place_name(p)), Some(p));
            assert_eq!(place_from_name(&place_name(p).to_uppercase()), Some(p));
        }
        assert_eq!(place_from_name("atlantis"), None);
        assert_eq!(place_from_name(""), None);
    }

    #[test]
    fn role_names_are_distinct() {
        let names: BTreeSet<&str> = ROLES.iter().map(|r| role_name(*r)).collect();
        assert_eq!(names.len(), ROLES.len());
    }

    /// Bài quan trọng nhất của module: mọi ý định của bộ lập lịch đi qua
    /// [`Choice`] rồi quay về **đúng chính nó**. Nếu không, fallback sẽ lặng lẽ
    /// đổi nghĩa trên đường đi.
    #[test]
    fn every_routine_intent_survives_the_round_trip() {
        for role in ROLES {
            for percent in 0..DAY {
                for at in PLACES {
                    for hunger in [0_i64, 60, 90] {
                        let mut s = situation(role, percent, at);
                        s.hunger = hunger;
                        let want = decide(&s);
                        let choice = routine_fallback(&s);
                        assert!(
                            village_registry().contains(&choice.action),
                            "`{}` khong nam trong registry",
                            choice.action
                        );
                        assert_eq!(
                            intent_of(&choice),
                            Some(want),
                            "{role:?} tai {percent}% o {at:?} khong khu hoi duoc"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn socialize_carries_the_partner_when_the_engine_knows_one() {
        let with_id = choice_of_intent(Intent::Socialize { with: Some(42) }, "chao");
        assert_eq!(with_id.target.as_deref(), Some("entity:42"));
        assert_eq!(
            intent_of(&with_id),
            Some(Intent::Socialize { with: Some(42) })
        );

        // Một cái tên người thay vì một id vẫn hợp lệ: nghĩa là "ai đó quanh
        // đây", và lớp mô phỏng tự chọn.
        let by_name = Choice::new("socialize", Some("Doran"), "");
        assert_eq!(intent_of(&by_name), Some(Intent::Socialize { with: None }));
    }

    /// `§10.4` bước 7: engine có quyền từ chối, và từ chối là `None` chứ không
    /// phải một nơi gần đúng.
    #[test]
    fn the_engine_refuses_instead_of_guessing() {
        assert_eq!(intent_of(&Choice::new("go_to", None, "")), None);
        assert_eq!(intent_of(&Choice::new("go_to", Some("atlantis"), "")), None);
        assert_eq!(intent_of(&Choice::new("open_portal", None, "")), None);
        assert_eq!(intent_of(&Choice::new("", None, "")), None);
    }

    #[test]
    fn time_of_day_is_a_label_not_a_clock() {
        let mut labels = BTreeSet::new();
        for percent in 0..DAY {
            let s = situation(Role::Farmer, percent, Place::Home);
            labels.insert(time_of_day(&s));
        }
        assert!(labels.len() >= 5, "chi co {labels:?} trong ca ngay");
        // Cùng một buổi, hai tick khác nhau: cùng một nhãn, nên cache ở `§20.8`
        // còn trúng được.
        let a = situation(Role::Farmer, 40, Place::Home);
        let b = situation(Role::Farmer, 45, Place::Home);
        assert_eq!(time_of_day(&a), time_of_day(&b));
    }

    #[test]
    fn ticks_per_day_zero_does_not_panic() {
        let mut s = situation(Role::Farmer, 0, Place::Home);
        s.ticks_per_day = 0;
        s.tick = u64::MAX;
        assert_eq!(time_of_day(&s), "night");
        assert!(intent_of(&routine_fallback(&s)).is_some());
    }

    #[test]
    fn observation_of_copies_the_situation_and_nothing_more() {
        let mut s = situation(Role::Hunter, 40, Place::Field);
        s.hunger = 71;
        let obs = observation_of(
            &s,
            "Mara",
            &["Doran".to_owned()],
            &["gieng gan can".to_owned()],
        );
        assert_eq!(obs.self_name, "Mara");
        assert_eq!(obs.role, "hunter");
        assert_eq!(obs.hunger, 71);
        assert_eq!(obs.at, "field");
        assert_eq!(obs.time_of_day, time_of_day(&s));
        assert_eq!(obs.nearby, vec!["Doran".to_owned()]);
        assert_eq!(obs.recent, vec!["gieng gan can".to_owned()]);
    }
}
