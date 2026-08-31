//! Test tư vấn của Yuu: `Dossier`, `prompt_of`, `read_answer`, `without_model`,
//! `Yuu` (`idea.md §3.1` bước 2, `§1.2.4`).

use mow_llm::{Gateway, LlmError, LlmResult, Mode, ModelClient, Request, Response};
use mow_yuu::dossier::{Dossier, EventBrief, FolkBrief};
use mow_yuu::prompt::MAX_EVENTS;
use mow_yuu::{
    prompt_of, read_answer, suggested_questions, without_model, Answer, Proposal, StripReason, Yuu,
    PROMPT_ID, PROMPT_VERSION, ROUTE_ROLE,
};
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

// ═══════════════════ Đồ đạc dùng chung ═══════════════════

fn ev(
    seq: u64,
    tick: u64,
    kind: &str,
    actor: Option<u64>,
    cause: Option<u64>,
    summary: &str,
) -> EventBrief {
    EventBrief {
        seq,
        tick,
        kind: kind.to_owned(),
        actor,
        cause,
        summary: summary.to_owned(),
    }
}

fn folk(id: u64, name: &str, role: &str, intent: &str, hunger: i64) -> FolkBrief {
    FolkBrief {
        id,
        name: name.to_owned(),
        role: role.to_owned(),
        intent: intent.to_owned(),
        hunger,
    }
}

/// Một ngôi làng: kho grain đang cạn vì một chuỗi nhân quả ba mắt xích, và
/// Doran chỉ có một sự kiện đơn lẻ không liên quan.
fn village() -> Dossier {
    Dossier {
        tick: 42,
        stock: vec![("grain".to_owned(), 3), ("wood".to_owned(), 120)],
        folk: vec![
            folk(1, "Mara", "farmer", "forage", 70),
            folk(2, "Doran", "guard", "patrol", 20),
            folk(3, "Ila", "child", "play", 10),
        ],
        events: vec![
            ev(
                1,
                10,
                "econ.harvest.failed",
                None,
                None,
                "Mùa gặt thất bát vì hạn hán.",
            ),
            ev(
                2,
                20,
                "econ.grain.consumed",
                Some(1),
                Some(1),
                "Mara ăn phần grain cuối cùng của nhà.",
            ),
            ev(
                3,
                30,
                "society.hunger.rising",
                Some(1),
                Some(2),
                "Mara bắt đầu đói vì kho grain cạn.",
            ),
            ev(
                4,
                35,
                "society.patrol.reported",
                Some(2),
                None,
                "Doran báo cáo tuần tra bình thường.",
            ),
        ],
    }
}

/// Một câu trả lời đã lên kịch bản, giống hệt khuôn của `mow_mind::mind::tests`.
enum Scripted {
    Say(String),
    Fail(LlmError),
}

#[derive(Default)]
struct Seen {
    prompts: Vec<String>,
    models: Vec<String>,
}

/// Client giả: **không mạng**, trả lời theo kịch bản, ghi lại mọi lời gọi.
struct Fake {
    plan: VecDeque<Scripted>,
    seen: Arc<Mutex<Seen>>,
}

impl Fake {
    fn scripted(plan: Vec<Scripted>, seen: &Arc<Mutex<Seen>>) -> Box<dyn ModelClient> {
        Box::new(Fake {
            plan: plan.into(),
            seen: Arc::clone(seen),
        })
    }

    fn saying(text: &str, seen: &Arc<Mutex<Seen>>) -> Box<dyn ModelClient> {
        Fake::scripted(vec![Scripted::Say(text.to_owned())], seen)
    }

    fn failing(e: LlmError, seen: &Arc<Mutex<Seen>>) -> Box<dyn ModelClient> {
        Fake::scripted(vec![Scripted::Fail(e)], seen)
    }
}

impl ModelClient for Fake {
    fn mode(&self) -> Mode {
        Mode::Stub
    }

    fn call(&mut self, req: &Request) -> LlmResult<Response> {
        {
            let mut seen = self.seen.lock().expect("khoa hong");
            seen.prompts.push(req.rendered.clone());
            seen.models.push(req.model.clone());
        }
        match self.plan.pop_front() {
            Some(Scripted::Say(text)) => Ok(Response {
                text,
                model: "fake".to_owned(),
                input_tokens: 0,
                output_tokens: 0,
            }),
            Some(Scripted::Fail(e)) => Err(e),
            None => Err(LlmError::NoStub(req.prompt_id.clone())),
        }
    }
}

fn seen() -> Arc<Mutex<Seen>> {
    Arc::new(Mutex::new(Seen::default()))
}

// ═══════════════════ Dossier: chuẩn hoá ═══════════════════

#[test]
fn canonical_stock_sorts_trims_and_dedups_last_wins() {
    let d = Dossier {
        stock: vec![
            (" grain ".to_owned(), 1),
            ("wood".to_owned(), 5),
            ("grain".to_owned(), 9),
            (String::new(), 100),
        ],
        ..Dossier::default()
    };
    assert_eq!(
        d.canonical_stock(),
        vec![("grain".to_owned(), 9), ("wood".to_owned(), 5)]
    );
}

#[test]
fn canonical_folk_sorts_and_dedups_by_id_last_wins() {
    let d = Dossier {
        folk: vec![
            folk(2, "Doran", "guard", "patrol", 20),
            folk(1, "Mara-cu", "farmer", "idle", 0),
            folk(1, "Mara-moi", "farmer", "forage", 70),
        ],
        ..Dossier::default()
    };
    let f = d.canonical_folk();
    assert_eq!(f.len(), 2);
    assert_eq!(f[0].id, 1);
    assert_eq!(f[0].name, "Mara-moi");
    assert_eq!(f[1].id, 2);
}

#[test]
fn known_events_is_exactly_the_set_of_seqs() {
    let d = village();
    let known = d.known_events();
    assert_eq!(known.len(), 4);
    assert!(known.contains(&1) && known.contains(&4));
    assert!(!known.contains(&99));
}

#[test]
fn recent_events_keeps_order_and_caps_to_the_newest() {
    let d = Dossier {
        events: (0..10).map(|i| ev(i, i, "k", None, None, "s")).collect(),
        ..Dossier::default()
    };
    let recent: Vec<u64> = d.recent_events(3).iter().map(|e| e.seq).collect();
    assert_eq!(recent, vec![7, 8, 9]);
}

#[test]
fn recent_events_dedupes_by_seq_keeping_the_first_occurrence() {
    let d = Dossier {
        events: vec![
            ev(1, 1, "k", None, None, "a"),
            ev(2, 2, "k", None, None, "b"),
            ev(1, 1, "k", None, None, "c"),
        ],
        ..Dossier::default()
    };
    let seqs: Vec<u64> = d.recent_events(10).iter().map(|e| e.seq).collect();
    assert_eq!(seqs, vec![1, 2]);
}

// ═══════════════════ prompt_of ═══════════════════

#[test]
fn prompt_is_deterministic_across_a_hundred_calls() {
    let d = village();
    let first = prompt_of(&d, "Vì sao kho lương đang cạn?");
    for _ in 0..100 {
        assert_eq!(prompt_of(&d, "Vì sao kho lương đang cạn?"), first);
    }
}

/// Ràng buộc `§1.2.4`: prompt chỉ chứa những gì có trong `Dossier` và câu hỏi.
#[test]
fn prompt_holds_nothing_beyond_the_dossier_and_question() {
    const SECRET: &str = "MOW_YUU_TEST_SECRET_KHONG_DUOC_LOT_9x8y7z";
    std::env::set_var("MOW_YUU_TEST_SECRET", SECRET);
    let d = village();
    let p = prompt_of(&d, "cau hoi");
    assert!(!p.contains(SECRET), "bi mat lot vao prompt:\n{p}");
    std::env::set_var("MOW_YUU_TEST_SECRET", "gia tri khac");
    assert_eq!(prompt_of(&d, "cau hoi"), p);
    std::env::remove_var("MOW_YUU_TEST_SECRET");
}

#[test]
fn prompt_contains_the_question_verbatim() {
    let d = village();
    let p = prompt_of(&d, "Nếu ta không làm gì, chuyện gì sẽ tới?");
    assert!(p.contains("Nếu ta không làm gì, chuyện gì sẽ tới?"), "{p}");
}

#[test]
fn prompt_reads_empty_dossier_as_absence_not_blank() {
    let p = prompt_of(&Dossier::default(), "cau hoi");
    assert!(p.contains("(trống)"), "{p}");
    assert!(p.contains("(không có ai)"), "{p}");
    assert!(p.contains("(không có gì)"), "{p}");
}

/// `events` là một dòng thời gian: sắp lại nó là nói dối thứ tự nhân quả.
#[test]
fn prompt_keeps_events_in_given_order_not_sorted() {
    let d = Dossier {
        events: vec![
            ev(
                5,
                50,
                "k",
                None,
                None,
                "z cuoi cung theo seq nhung dua vao truoc",
            ),
            ev(
                1,
                10,
                "k",
                None,
                None,
                "a dau tien theo seq nhung dua vao sau",
            ),
        ],
        ..Dossier::default()
    };
    let p = prompt_of(&d, "cau hoi");
    let first = p.find("z cuoi cung").expect("phai co muc dau");
    let second = p.find("a dau tien").expect("phai co muc sau");
    assert!(first < second, "thu tu su kien bi sap lai:\n{p}");
}

#[test]
fn prompt_caps_events_to_max_and_keeps_the_newest() {
    let d = Dossier {
        events: (0..MAX_EVENTS + 5)
            .map(|i| ev(i as u64, i as u64, "k", None, None, &format!("su kien {i}")))
            .collect(),
        ..Dossier::default()
    };
    let p = prompt_of(&d, "cau hoi");
    assert!(!p.contains("su kien 0\n"), "muc cu nhat phai bi cat:\n{p}");
    assert!(p.contains(&format!("su kien {}", MAX_EVENTS + 4)));
    assert_eq!(p.matches("su kien ").count(), MAX_EVENTS);
}

#[test]
fn prompt_id_and_version_are_stable() {
    assert_eq!(PROMPT_ID, "yuu.advise");
    assert_eq!(PROMPT_VERSION, 1);
}

#[test]
fn stock_reordering_and_duplicates_do_not_change_the_prompt() {
    let a = prompt_of(&village(), "cau hoi");
    let mut shuffled = village();
    shuffled.stock.reverse();
    shuffled.stock.push(("grain".to_owned(), 3));
    assert_eq!(prompt_of(&shuffled, "cau hoi"), a);
}

// ═══════════════════ read_answer ═══════════════════

#[test]
fn a_line_without_citation_is_cut_with_no_citation() {
    let known: BTreeSet<u64> = [1, 2].into_iter().collect();
    let a = read_answer(
        r#"{"lines":[{"text":"khong co bang chung","cites":[]}]}"#,
        &known,
        &BTreeSet::new(),
    );
    assert!(a.lines.is_empty());
    assert_eq!(a.stripped.len(), 1);
    assert_eq!(a.stripped[0].reason, StripReason::NoCitation);
    assert_eq!(a.stripped[0].text, "khong co bang chung");
}

#[test]
fn a_line_citing_an_unknown_event_is_cut_with_unknown_event() {
    let known: BTreeSet<u64> = [1, 2].into_iter().collect();
    let a = read_answer(
        r#"{"lines":[{"text":"kho da can","cites":[99]}]}"#,
        &known,
        &BTreeSet::new(),
    );
    assert!(a.lines.is_empty());
    assert_eq!(a.stripped[0].reason, StripReason::UnknownEvent(99));
}

#[test]
fn a_well_formed_line_is_kept() {
    let known: BTreeSet<u64> = [1, 2].into_iter().collect();
    let a = read_answer(
        r#"{"lines":[{"text":"kho da can vi mua that bat","cites":[1,2]}]}"#,
        &known,
        &BTreeSet::new(),
    );
    assert_eq!(a.lines.len(), 1);
    assert_eq!(a.lines[0].cites, vec![1, 2]);
    assert!(a.stripped.is_empty());
}

#[test]
fn non_numeric_citations_are_filtered_but_valid_ones_still_count() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let a = read_answer(
        r#"{"lines":[{"text":"mot phan dung","cites":[1,"rac",null,2.5]}]}"#,
        &known,
        &BTreeSet::new(),
    );
    assert_eq!(a.lines.len(), 1);
    assert_eq!(a.lines[0].cites, vec![1]);
}

#[test]
fn all_non_numeric_citations_count_as_no_citation() {
    let a = read_answer(
        r#"{"lines":[{"text":"toan rac","cites":["a","b"]}]}"#,
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert_eq!(a.stripped[0].reason, StripReason::NoCitation);
}

#[test]
fn duplicate_citations_are_deduplicated_in_a_kept_line() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let a = read_answer(
        r#"{"lines":[{"text":"lap lai","cites":[1,1,1]}]}"#,
        &known,
        &BTreeSet::new(),
    );
    assert_eq!(a.lines[0].cites, vec![1]);
}

#[test]
fn a_proposal_with_no_citation_is_cut_before_power_is_even_checked() {
    let known_powers: BTreeSet<String> = ["grant_food".to_owned()].into_iter().collect();
    let a = read_answer(
        r#"{"proposals":[{"power":"grant_food","why":"cuu doi","cites":[]}]}"#,
        &BTreeSet::new(),
        &known_powers,
    );
    assert!(a.proposals.is_empty());
    assert_eq!(a.stripped[0].reason, StripReason::NoCitation);
}

#[test]
fn a_proposal_citing_an_unknown_event_is_cut() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let known_powers: BTreeSet<String> = ["grant_food".to_owned()].into_iter().collect();
    let a = read_answer(
        r#"{"proposals":[{"power":"grant_food","why":"cuu doi","cites":[99]}]}"#,
        &known,
        &known_powers,
    );
    assert_eq!(a.stripped[0].reason, StripReason::UnknownEvent(99));
}

#[test]
fn a_proposal_with_an_unknown_power_is_cut() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let a = read_answer(
        r#"{"proposals":[{"power":"bay_len_troi","why":"vi sao khong","cites":[1]}]}"#,
        &known,
        &BTreeSet::new(),
    );
    assert!(a.proposals.is_empty());
    assert_eq!(
        a.stripped[0].reason,
        StripReason::UnknownPower("bay_len_troi".to_owned())
    );
}

#[test]
fn a_well_formed_proposal_with_a_known_power_is_kept() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let known_powers: BTreeSet<String> = ["grant_food".to_owned()].into_iter().collect();
    let a = read_answer(
        r#"{"proposals":[{"power":"grant_food","why":"cuu doi truoc khi qua muon","cites":[1]}]}"#,
        &known,
        &known_powers,
    );
    assert_eq!(a.proposals.len(), 1);
    assert_eq!(a.proposals[0].power, "grant_food");
    assert!(a.stripped.is_empty());
    let expected = Proposal {
        power: "grant_food".to_owned(),
        why: "cuu doi truoc khi qua muon".to_owned(),
        cites: vec![1],
    };
    assert_eq!(a.proposals[0], expected);
}

#[test]
fn stripped_is_never_hidden_multiple_bad_items_all_appear() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let text = r#"{
        "lines": [
            {"text": "khong trich dan", "cites": []},
            {"text": "trich sai", "cites": [99]}
        ],
        "proposals": [
            {"power": "khong_co_that", "why": "bia", "cites": [1]}
        ]
    }"#;
    let a = read_answer(text, &known, &BTreeSet::new());
    assert!(a.lines.is_empty());
    assert!(a.proposals.is_empty());
    assert_eq!(a.stripped.len(), 3, "{:?}", a.stripped);
}

#[test]
fn garbage_text_yields_an_empty_answer() {
    let a = read_answer(
        "day la mot cau van xuoi khong co JSON gi ca.",
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert_eq!(a, Answer::default());
}

#[test]
fn broken_json_yields_an_empty_answer() {
    let a = read_answer(
        r#"{"lines": [{"text": "x", "cites": [1]}"#,
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert_eq!(a, Answer::default());
}

/// Ranh giới của phép sửa chữa duy nhất: một đối tượng bọc trong hàng rào mã
/// và một câu dẫn vẫn đọc được.
#[test]
fn a_code_fence_wrapped_answer_still_reads() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let text =
        "Đây là kết quả:\n```json\n{\"lines\":[{\"text\":\"on dinh\",\"cites\":[1]}]}\n```\n";
    let a = read_answer(text, &known, &BTreeSet::new());
    assert_eq!(a.lines.len(), 1);
}

#[test]
fn a_key_regurgitated_by_the_model_never_reaches_a_kept_line() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let text = r#"{"lines":[{"text":"khoa la sk-or-v1-KHOATHAT-abc123 day nhe","cites":[1]}]}"#;
    let a = read_answer(text, &known, &BTreeSet::new());
    assert_eq!(a.lines.len(), 1);
    assert!(!a.lines[0].text.contains("KHOATHAT"), "{}", a.lines[0].text);
    assert!(a.lines[0].text.contains("sk-***"), "{}", a.lines[0].text);
}

#[test]
fn a_key_in_a_stripped_line_never_reaches_the_stripped_report() {
    let text = r#"{"lines":[{"text":"khoa la sk-or-v1-KHOATHAT-abc123","cites":[]}]}"#;
    let a = read_answer(text, &BTreeSet::new(), &BTreeSet::new());
    assert_eq!(a.stripped.len(), 1);
    assert!(
        !a.stripped[0].text.contains("KHOATHAT"),
        "{}",
        a.stripped[0].text
    );
}

#[test]
fn a_key_in_an_unknown_power_never_reaches_the_stripped_reason() {
    let known: BTreeSet<u64> = [1].into_iter().collect();
    let text = r#"{"proposals":[{"power":"sk-or-v1-KHOATHAT-xyz","why":"x","cites":[1]}]}"#;
    let a = read_answer(text, &known, &BTreeSet::new());
    let dump = format!("{:?}", a.stripped);
    assert!(!dump.contains("KHOATHAT"), "{dump}");
}

#[test]
fn missing_text_field_still_produces_a_debuggable_stripped_entry() {
    let a = read_answer(
        r#"{"lines":[{"cites":[]}]}"#,
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert_eq!(a.stripped.len(), 1);
    assert!(!a.stripped[0].text.is_empty());
}

#[test]
fn a_non_array_lines_field_is_treated_as_empty_not_a_crash() {
    let a = read_answer(
        r#"{"lines": "khong phai mang"}"#,
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert!(a.lines.is_empty());
    assert!(a.stripped.is_empty());
}

#[test]
fn missing_lines_and_proposals_fields_are_not_a_crash() {
    let a = read_answer(
        r#"{"khong_lien_quan": true}"#,
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert_eq!(a, Answer::default());
}

#[test]
fn read_answer_is_a_pure_function_of_its_three_arguments() {
    let known: BTreeSet<u64> = [1, 2].into_iter().collect();
    let known_powers: BTreeSet<String> = ["grant_food".to_owned()].into_iter().collect();
    let text = r#"{"lines":[{"text":"on dinh","cites":[1,2]}],"proposals":[{"power":"grant_food","why":"vi","cites":[1]}]}"#;
    let first = read_answer(text, &known, &known_powers);
    for _ in 0..20 {
        assert_eq!(read_answer(text, &known, &known_powers), first);
    }
}

// ═══════════════════ without_model ═══════════════════

#[test]
fn suggested_questions_returns_exactly_the_three_design_questions() {
    assert_eq!(
        suggested_questions(),
        &[
            "Vì sao kho lương đang cạn?",
            "Dân làng đang gặp chuyện gì?",
            "Nếu ta không làm gì, chuyện gì sẽ tới?",
        ]
    );
}

/// Bài kiểm thật của thiết kế: `without_model` phải trả lời được cả ba câu
/// hỏi gợi ý, và mọi câu nó nói đều truy được về một sự kiện có thật.
#[test]
fn without_model_answers_all_three_suggested_questions_usefully() {
    let d = village();
    for q in suggested_questions() {
        let a = without_model(&d, q);
        assert!(
            !a.lines.is_empty(),
            "cau hoi `{q}` khong tra loi duoc dong nao co can cu"
        );
        assert!(a.proposals.is_empty(), "khong duoc tu bia de xuat: `{q}`");
        for line in &a.lines {
            assert!(
                !line.cites.is_empty(),
                "dong khong trich dan cho `{q}`: {line:?}"
            );
            for seq in &line.cites {
                assert!(
                    d.known_events().contains(seq),
                    "trich dan {seq} khong co that cho `{q}`"
                );
            }
        }
    }
}

#[test]
fn without_model_stock_answer_cites_the_causal_chain() {
    let d = village();
    let a = without_model(&d, suggested_questions()[0]);
    let grain_line = a
        .lines
        .iter()
        .find(|l| l.text.contains("grain"))
        .expect("phai co dong ve grain");
    assert_eq!(grain_line.cites, vec![3, 2, 1]);
    assert!(
        a.stripped.iter().any(|s| s.text.contains("wood")),
        "wood khong co su kien lien quan, phai duoc bao la khong tra loi duoc chu khong im lang"
    );
}

#[test]
fn without_model_folk_answer_reports_uncited_folk_as_stripped_not_silent() {
    let d = village();
    let a = without_model(&d, suggested_questions()[1]);
    assert!(a.lines.iter().any(|l| l.text.contains("Mara")));
    assert!(a.lines.iter().any(|l| l.text.contains("Doran")));
    assert!(
        a.stripped.iter().any(|s| s.text.contains("Ila")),
        "Ila khong co su kien nao nhung khong duoc im lang bo qua"
    );
}

#[test]
fn without_model_trend_answer_picks_the_deepest_real_chain_not_just_the_latest_seq() {
    let d = village();
    let a = without_model(&d, suggested_questions()[2]);
    assert_eq!(a.lines.len(), 1);
    assert_eq!(
        a.lines[0].cites,
        vec![3, 2, 1],
        "phai chon chuoi nhan qua sau nhat, khong phai seq moi nhat (4)"
    );
}

#[test]
fn without_model_falls_back_to_general_reading_for_an_unmatched_question() {
    let d = village();
    let a = without_model(&d, "Mau yeu thich cua Doran la gi?");
    assert!(!a.lines.is_empty());
    for line in &a.lines {
        assert_eq!(
            line.cites.len(),
            1,
            "doc chung thi moi dong tu trich dan chinh no"
        );
    }
}

#[test]
fn without_model_does_not_panic_on_an_empty_dossier() {
    let d = Dossier::default();
    for q in suggested_questions() {
        let a = without_model(&d, q);
        assert!(a.lines.is_empty());
        assert!(a.proposals.is_empty());
    }
}

#[test]
fn without_model_never_proposes_an_intervention() {
    let d = village();
    for q in suggested_questions() {
        assert!(without_model(&d, q).proposals.is_empty());
    }
}

// ═══════════════════ Yuu — mặt tiền ═══════════════════

#[test]
fn a_valid_model_answer_passes_through() {
    let log = seen();
    let d = village();
    let text = r#"{"lines":[{"text":"kho grain da can vi mua that bat","cites":[1,2]}]}"#;
    let mut yuu = Yuu::new(Fake::saying(text, &log));
    let a = yuu.ask(&d, "Vì sao kho lương đang cạn?");
    assert_eq!(a.lines.len(), 1);
    assert_eq!(a.lines[0].cites, vec![1, 2]);
    assert_eq!(yuu.calls_made(), 1);
}

#[test]
fn prose_instead_of_json_falls_back_to_without_model() {
    let log = seen();
    let d = village();
    let mut yuu = Yuu::new(Fake::saying(
        "Toi nghi kho luong dang can vi han han.",
        &log,
    ));
    let q = suggested_questions()[0];
    let a = yuu.ask(&d, q);
    assert_eq!(a, without_model(&d, q));
}

#[test]
fn broken_json_falls_back_to_without_model() {
    let log = seen();
    let d = village();
    let mut yuu = Yuu::new(Fake::saying(r#"{"lines":[{"text": "x""#, &log));
    let q = suggested_questions()[1];
    let a = yuu.ask(&d, q);
    assert_eq!(a, without_model(&d, q));
}

#[test]
fn a_fully_stripped_model_answer_falls_back_and_keeps_the_stripped_trail() {
    let log = seen();
    let d = village();
    let text = r#"{"lines":[{"text":"kho sap het","cites":[]}]}"#;
    let mut yuu = Yuu::new(Fake::saying(text, &log));
    let q = suggested_questions()[0];
    let a = yuu.ask(&d, q);
    let floor = without_model(&d, q);
    assert_eq!(a.lines, floor.lines);
    assert_eq!(a.proposals, floor.proposals);
    assert!(
        a.stripped
            .iter()
            .any(|s| s.text == "kho sap het" && s.reason == StripReason::NoCitation),
        "{:?}",
        a.stripped
    );
    assert!(a.stripped.len() > floor.stripped.len() || !floor.stripped.is_empty());
}

#[test]
fn no_provider_falls_back_to_without_model() {
    let d = village();
    let mut yuu = Yuu::new(Box::new(Gateway::stub()));
    assert_eq!(yuu.mode(), Mode::Stub);
    let q = suggested_questions()[0];
    let a = yuu.ask(&d, q);
    assert_eq!(a, without_model(&d, q));
}

#[test]
fn a_transport_error_falls_back_to_without_model() {
    let log = seen();
    let d = village();
    let mut yuu = Yuu::new(Fake::failing(
        LlmError::Transport("dns: no such host".to_owned()),
        &log,
    ));
    let q = suggested_questions()[0];
    let a = yuu.ask(&d, q);
    assert_eq!(a, without_model(&d, q));
    assert_eq!(
        yuu.calls_made(),
        1,
        "mot lan goi that bai van phai duoc tinh"
    );
}

#[test]
fn calls_made_counts_every_attempt_including_failures() {
    let d = village();
    let mut yuu = Yuu::new(Box::new(Gateway::stub()));
    assert_eq!(yuu.calls_made(), 0);
    yuu.ask(&d, "cau hoi 1");
    yuu.ask(&d, "cau hoi 2");
    assert_eq!(yuu.calls_made(), 2);
}

#[test]
fn debug_never_prints_the_client() {
    let yuu = Yuu::new(Box::new(Gateway::stub())).with_model("bi mat/model");
    let s = format!("{yuu:?}");
    assert!(!s.contains("Gateway"), "{s}");
    assert!(s.contains("known_powers: 0"), "{s}");
}

#[test]
fn route_role_is_stable() {
    assert_eq!(ROUTE_ROLE, "yuu");
}

#[test]
fn unknown_powers_by_default_strip_every_proposal() {
    let log = seen();
    let d = village();
    let text = r#"{"proposals":[{"power":"grant_food","why":"cuu doi","cites":[1]}]}"#;
    let mut yuu = Yuu::new(Fake::saying(text, &log));
    let q = "cau hoi khong khop heuristic nao ca";
    let a = yuu.ask(&d, q);
    assert!(a.proposals.is_empty());
    let floor = without_model(&d, q);
    assert_eq!(a.lines, floor.lines);
    assert!(a
        .stripped
        .iter()
        .any(|s| matches!(&s.reason, StripReason::UnknownPower(p) if p == "grant_food")));
}

#[test]
fn with_known_powers_lets_a_matching_proposal_through() {
    let log = seen();
    let d = village();
    let text = r#"{"lines":[{"text":"kho grain can","cites":[1]}],"proposals":[{"power":"grant_food","why":"cuu doi ngay","cites":[1]}]}"#;
    let mut yuu = Yuu::new(Fake::saying(text, &log))
        .with_known_powers(["grant_food".to_owned()].into_iter().collect());
    let a = yuu.ask(&d, "cau hoi");
    assert_eq!(a.proposals.len(), 1);
    assert_eq!(a.proposals[0].power, "grant_food");
}

#[test]
fn set_known_powers_updates_validation_between_calls() {
    let log1 = seen();
    let log2 = seen();
    let d = village();
    let text = r#"{"proposals":[{"power":"grant_food","why":"x","cites":[1]}]}"#;

    let mut yuu = Yuu::new(Fake::saying(text, &log1));
    assert!(yuu.ask(&d, "cau hoi").proposals.is_empty());

    yuu = Yuu::new(Fake::saying(text, &log2));
    yuu.set_known_powers(["grant_food".to_owned()].into_iter().collect());
    assert_eq!(yuu.ask(&d, "cau hoi").proposals.len(), 1);
}

#[test]
fn the_prompt_sent_to_the_model_is_exactly_prompt_of() {
    let log = seen();
    let d = village();
    let mut yuu = Yuu::new(Fake::saying(r#"{"lines":[]}"#, &log)).with_model("m/1");
    yuu.ask(&d, "cau hoi kiem tra");
    let s = log.lock().expect("khoa hong");
    assert_eq!(s.prompts.len(), 1);
    assert_eq!(s.prompts[0], prompt_of(&d, "cau hoi kiem tra"));
    assert_eq!(s.models[0], "m/1");
}
