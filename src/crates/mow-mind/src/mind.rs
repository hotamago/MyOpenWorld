//! [`Mind`] — một lượt suy nghĩ, và sáu cách nó có thể không thành.
//!
//! ## Hợp đồng
//!
//! [`Mind::think`] **không bao giờ** trả lỗi ra ngoài. Một NPC không quyết định
//! được thì thế giới đứng, và một thế giới đứng vì provider nghẽn mạng là thứ
//! `§10.3` nói thẳng là không được phép: ba tầng dưới phải khiến thế giới hoạt
//! động đúng khi LLM chậm hoặc mất kết nối.
//!
//! Cái nó không làm là im lặng. Mỗi lần rơi về fallback đều:
//!
//! - hiện ra trong [`Decision::Fell`] với một [`FallbackReason`] có tên;
//! - được ghi một dòng [`FallbackNote`] vào sổ, kèm chi tiết thô đã che bí mật;
//! - được cộng vào [`Mind::fallbacks_total`], con số này **không bao giờ mất**
//!   kể cả khi sổ đã bị cắt bớt.
//!
//! ## Vì sao sổ có trần mà số đếm thì không
//!
//! Một provider hỏng vài giờ sẽ sinh hàng chục nghìn lần rơi. Giữ hết là một
//! chỗ rò bộ nhớ trong đúng cái lúc hệ thống đang yếu nhất. Giữ hờ hững lại làm
//! mất câu trả lời cho câu hỏi "chuyện này xảy ra bao nhiêu lần".
//!
//! Nên tách đôi: sổ giữ [`JOURNAL_CAP`] dòng gần nhất để còn đọc được chi tiết,
//! còn [`Mind::fallbacks_total`] và [`Mind::calls_made`] là số đếm chính xác.
//! Chỗ gọi nên [`Mind::take_journal`] mỗi tick để đẩy sang event log thật.
//!
//! ## Ngân sách trừ **trước** khi gọi
//!
//! Một lời gọi thất bại vẫn đã gửi token đi và vẫn đã bị tính tiền. Trừ ngân
//! sách sau khi thành công sẽ cho một vòng lặp gọi mãi không hết ngân sách
//! trong khi provider trả `500` — đúng kịch bản `§20.10` muốn chặn.

use crate::choice::{Choice, Decision, FallbackNote, FallbackReason};
use crate::observation::Observation;
use crate::parse::{read_choice, trim_for_log};
use crate::prompt::{canonicalize, prompt_of, PROMPT_ID, PROMPT_VERSION};
use mow_llm::{LlmError, Mode, ModelClient, Request};

/// Vai dùng để tra định tuyến model trong cấu hình (`§20.7`).
///
/// Không giữ một `mow-config` ở đây là có chủ ý: một tầng nhận thức không cần
/// biết YAML được phân tích thế nào, và kéo cả bộ phân tích cấu hình vào chỉ để
/// đọc hai trường sẽ làm mọi bài test của crate này phụ thuộc vào định dạng tệp.
/// Chỗ gọi làm đúng một dòng:
///
/// ```text
/// let route = cfg.llm.route(mow_mind::ROUTE_ROLE);
/// let mind = Mind::new(client, registry, fallback)
///     .with_model(&route.model)
///     .with_max_output_tokens(route.max_output_tokens);
/// ```
pub const ROUTE_ROLE: &str = "npc";

/// Số dòng tối đa giữ trong sổ fallback.
pub const JOURNAL_CAP: usize = 256;

/// Một tâm trí: một cổng vào model, một action registry, một fallback có tên.
pub struct Mind {
    client: Box<dyn ModelClient>,
    registry: Vec<String>,
    fallback: Choice,
    model: String,
    max_output_tokens: u32,
    budget: u32,
    calls: u64,
    fallbacks: u64,
    journal: Vec<FallbackNote>,
}

impl core::fmt::Debug for Mind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Không in `client`: nó có thể mang khóa API, và một `{:?}` vô tình
        // trong log là cách khóa rò ra dễ nhất.
        f.debug_struct("Mind")
            .field("mode", &self.client.mode())
            .field("registry", &self.registry.len())
            .field("model", &self.model)
            .field("budget", &self.budget)
            .field("calls", &self.calls)
            .field("fallbacks", &self.fallbacks)
            .finish_non_exhaustive()
    }
}

impl Mind {
    /// Dựng một tâm trí.
    ///
    /// `registry` được chuẩn hóa ngay ([`crate::canonical_registry`]), nên thứ
    /// tự khai báo của chỗ gọi không lọt được vào prompt.
    ///
    /// `fallback` là hành động dùng khi model không cho được câu trả lời hợp
    /// lệ. Nó nên là quyết định của tầng Routine (`§10.3`) — xem
    /// [`crate::bridge::routine_fallback`] — chứ không nên là một hằng số vô
    /// hồn: một cư dân đứng ngây ra mỗi lần mạng chập là một cư dân trông như
    /// hỏng, còn một cư dân quay về nhịp sinh hoạt của vai thì không ai nhận ra.
    #[must_use]
    pub fn new(client: Box<dyn ModelClient>, registry: Vec<String>, fallback: Choice) -> Mind {
        Mind {
            client,
            registry: canonicalize(registry),
            fallback,
            model: String::new(),
            max_output_tokens: 0,
            // Mặc định là không giới hạn. Trần ngân sách là một quyết định vận
            // hành, và đặt sẵn một con số ở đây sẽ làm NPC câm lặng ở một chỗ
            // không ai ngờ tới.
            budget: u32::MAX,
            calls: 0,
            fallbacks: 0,
            journal: Vec::new(),
        }
    }

    /// Model để xin, thường lấy từ `cfg.llm.route(`[`ROUTE_ROLE`]`).model`.
    ///
    /// Chuỗi rỗng nghĩa là "dùng mặc định của client" — `mow_llm` xử lý đúng như
    /// vậy, nên không cần một `Option` ở đây.
    #[must_use]
    pub fn with_model(mut self, model: &str) -> Mind {
        model.clone_into(&mut self.model);
        self
    }

    /// Trần token đầu ra. `0` nghĩa là "dùng mặc định của client".
    #[must_use]
    pub fn with_max_output_tokens(mut self, n: u32) -> Mind {
        self.max_output_tokens = n;
        self
    }

    /// Số lời gọi model còn được phép thực hiện (`§20.10`).
    #[must_use]
    pub fn with_budget(mut self, calls: u32) -> Mind {
        self.budget = calls;
        self
    }

    /// Đổi fallback mặc định.
    ///
    /// Có mặt vì fallback đúng **đổi theo từng tick**: nhịp sinh hoạt lúc nửa
    /// đêm không giống nhịp lúc giữa trưa. Chỗ gọi nào tính lại được fallback
    /// mỗi tick thì nên dùng [`Mind::think_with`] và bỏ qua hàm này.
    pub fn set_fallback(&mut self, fallback: Choice) {
        self.fallback = fallback;
    }

    /// Action registry đã chuẩn hóa.
    #[must_use]
    pub fn registry(&self) -> &[String] {
        &self.registry
    }

    /// Chế độ của cổng model bên dưới.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.client.mode()
    }

    /// Số lời gọi còn lại.
    #[must_use]
    pub fn budget_left(&self) -> u32 {
        self.budget
    }

    /// Tổng số lời gọi model đã thực hiện.
    #[must_use]
    pub fn calls_made(&self) -> u64 {
        self.calls
    }

    /// Tổng số lần rơi về fallback. Chính xác kể cả khi sổ đã bị cắt.
    #[must_use]
    pub fn fallbacks_total(&self) -> u64 {
        self.fallbacks
    }

    /// [`JOURNAL_CAP`] lần rơi gần nhất.
    #[must_use]
    pub fn journal(&self) -> &[FallbackNote] {
        &self.journal
    }

    /// Lấy sổ ra và để lại một sổ rỗng, để đẩy sang event log.
    pub fn take_journal(&mut self) -> Vec<FallbackNote> {
        core::mem::take(&mut self.journal)
    }

    /// Suy nghĩ một lượt với fallback mặc định.
    pub fn think(&mut self, obs: &Observation) -> Decision {
        let fallback = self.fallback.clone();
        self.think_with(obs, &fallback)
    }

    /// Suy nghĩ một lượt với một fallback tính riêng cho tick này.
    ///
    /// Đây là hàm chỗ gọi nên dùng khi nó có sẵn một [`mow_society::routine`]:
    /// fallback đúng là quyết định của lịch sinh hoạt **tại tick đó**.
    pub fn think_with(&mut self, obs: &Observation, fallback: &Choice) -> Decision {
        // Registry rỗng: hỏi model cũng vô nghĩa vì mọi câu trả lời đều sẽ bị
        // từ chối. Chặn ở đây tiết kiệm đúng một lời gọi mỗi tick mỗi NPC — và
        // quan trọng hơn, nó ghi đúng lý do: lỗi thuộc về chỗ gọi, không thuộc
        // về model.
        if self.registry.is_empty() {
            return self.fall(obs, fallback, FallbackReason::EmptyRegistry, String::new());
        }
        if self.budget == 0 {
            return self.fall(obs, fallback, FallbackReason::BudgetSpent, String::new());
        }

        let req = Request {
            prompt_id: PROMPT_ID.to_owned(),
            prompt_version: PROMPT_VERSION,
            model: self.model.clone(),
            rendered: prompt_of(obs, &self.registry),
            max_output_tokens: self.max_output_tokens,
        };

        // Trừ trước khi gọi: token đã gửi đi là token đã tiêu, dù có trả lời hay
        // không.
        self.budget -= 1;
        self.calls += 1;

        let text = match self.client.call(&req) {
            Ok(answer) => answer.text,
            Err(e) => {
                let (reason, detail) = classify(&e);
                return self.fall(obs, fallback, reason, detail);
            }
        };

        match read_choice(&text, &self.registry) {
            Ok(choice) => Decision::Chose(choice),
            Err(e) => self.fall(obs, fallback, e.reason, e.detail),
        }
    }

    /// Ghi một lần rơi và trả về quyết định fallback.
    fn fall(
        &mut self,
        obs: &Observation,
        fallback: &Choice,
        reason: FallbackReason,
        detail: String,
    ) -> Decision {
        self.fallbacks += 1;
        if self.journal.len() >= JOURNAL_CAP {
            self.journal.remove(0);
        }
        self.journal.push(FallbackNote {
            self_name: obs.self_name.clone(),
            reason: reason.clone(),
            detail,
            used: fallback.clone(),
        });
        Decision::Fell {
            to: fallback.clone(),
            reason,
        }
    }
}

/// Xếp một [`LlmError`] vào đúng nhánh fallback.
///
/// Ánh xạ này là chỗ dễ lười nhất trong cả crate — một `_ =>` ở đây sẽ gom mọi
/// sự cố tương lai vào một nhãn duy nhất, và nhãn đó sẽ là nhãn sai. Nên nó
/// liệt kê **đủ mọi biến thể**: thêm một biến thể mới vào `mow_llm::LlmError`
/// sẽ làm chỗ này không biên dịch được, tức là buộc phải quyết định.
fn classify(e: &LlmError) -> (FallbackReason, String) {
    let detail = trim_for_log(&e.to_string());
    let reason = match e {
        // Năm tình huống, một nghĩa với thế giới: phía sau cổng không có ai trả
        // lời. Chúng khác nhau ở cách sửa — thiếu stub, chưa cắm provider,
        // thiếu bản ghi, tệp hỏng — và cách sửa nằm nguyên vẹn trong `detail`.
        LlmError::NoStub(_)
        | LlmError::NoProvider(_)
        | LlmError::NoCassette { .. }
        | LlmError::Io { .. }
        | LlmError::BadCassette(_) => FallbackReason::NoProvider,
        // `402` là hết tiền, `429` là hết hạn mức. Cả hai là "không được phép
        // hỏi nữa", và gọi chúng là timeout sẽ khiến người trực hệ thống đi tìm
        // sự cố mạng trong khi vấn đề nằm ở hóa đơn.
        LlmError::Upstream {
            status: 402 | 429, ..
        } => FallbackReason::BudgetSpent,
        // Không nối được, quá hạn, hoặc provider trả một mã lỗi khác: đã hỏi mà
        // không có câu trả lời nào về.
        LlmError::Transport(_) | LlmError::Upstream { .. } => FallbackReason::Timeout,
        // `2xx` nhưng không đúng hình dạng đã hứa — kể cả `content` rỗng.
        LlmError::BadResponse(msg) => FallbackReason::BadShape(trim_for_log(msg)),
    };
    (reason, detail)
}

#[cfg(test)]
mod tests {
    use super::{Mind, JOURNAL_CAP};
    use crate::bridge;
    use crate::choice::{Choice, Decision, FallbackReason};
    use crate::observation::Observation;
    use crate::prompt::{prompt_of, PROMPT_ID, PROMPT_VERSION};
    use mow_llm::{Gateway, LlmError, LlmResult, Mode, ModelClient, Request, Response};
    use mow_society::routine::{Intent, Place, Role, Situation};
    use std::collections::{BTreeSet, VecDeque};
    use std::sync::{Arc, Mutex};

    /// Một câu trả lời đã lên kịch bản.
    enum Scripted {
        Say(String),
        Fail(LlmError),
    }

    /// Những gì client giả đã nhìn thấy, chia sẻ ra ngoài `Box<dyn ModelClient>`.
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

    fn obs() -> Observation {
        Observation {
            self_name: "Mara".to_owned(),
            role: "farmer".to_owned(),
            hunger: 62,
            time_of_day: "morning".to_owned(),
            at: "well".to_owned(),
            nearby: vec!["Doran".to_owned()],
            recent: vec!["gieng gan can".to_owned()],
        }
    }

    /// Fallback dùng chung: về nhà. Khác hẳn mọi hành động mà test cho model
    /// trả về, nên "đã rơi" và "đã chọn" không lẫn vào nhau được.
    fn fallback() -> Choice {
        Choice::new("go_to", Some("home"), "theo nhip sinh hoat cua vai")
    }

    /// Nhan loai y dinh, bo qua tham so — de dem *loai* viec chu khong dem
    /// bien the cua cung mot viec.
    fn tag(i: Intent) -> &'static str {
        match i {
            Intent::GoTo { .. } => "goto",
            Intent::Sleep => "sleep",
            Intent::Eat => "eat",
            Intent::Work => "work",
            Intent::Socialize { .. } => "socialize",
            Intent::Idle => "idle",
        }
    }

    fn mind_with(client: Box<dyn ModelClient>) -> Mind {
        Mind::new(client, bridge::village_registry(), fallback())
    }

    // Đường thành công.

    #[test]
    fn a_valid_answer_inside_the_registry_is_chosen() {
        let log = seen();
        let mut mind = mind_with(Fake::saying(
            r#"{"action": "socialize", "target": "Doran", "reason": "chao mot cau"}"#,
            &log,
        ));
        let d = mind.think(&obs());
        assert_eq!(
            d,
            Decision::Chose(Choice::new("socialize", Some("Doran"), "chao mot cau"))
        );
        assert!(!d.is_fallback());
        assert_eq!(d.reason(), None);
        assert_eq!(mind.fallbacks_total(), 0);
        assert_eq!(mind.calls_made(), 1);
    }

    #[test]
    fn the_prompt_sent_is_exactly_prompt_of() {
        let log = seen();
        let mut mind = mind_with(Fake::saying(r#"{"action": "idle"}"#, &log)).with_model("m/1");
        mind.think(&obs());
        let sent = log.lock().expect("khoa hong");
        assert_eq!(sent.prompts.len(), 1);
        assert_eq!(
            sent.prompts[0],
            prompt_of(&obs(), &bridge::village_registry())
        );
        assert_eq!(sent.models[0], "m/1");
    }

    #[test]
    fn prompt_id_and_version_are_stable() {
        assert_eq!(PROMPT_ID, "npc.mind.decide");
        assert_eq!(PROMPT_VERSION, 1);
    }

    // Bốn cách model làm sai.

    /// `§10.5`: ngoài registry là lỗi validate, **và hành động đó không được
    /// thực hiện**.
    #[test]
    fn an_action_outside_the_registry_is_never_performed() {
        let log = seen();
        let mut mind = mind_with(Fake::saying(
            r#"{"action": "open_portal", "target": "north_gate", "reason": "toi biet lam"}"#,
            &log,
        ));
        let d = mind.think(&obs());
        assert_eq!(
            d.reason(),
            Some(&FallbackReason::NotInRegistry("open_portal".to_owned()))
        );
        // Cái được thực hiện là fallback, không phải đề xuất của model.
        assert_eq!(*d.choice(), fallback());
        assert_ne!(d.choice().action, "open_portal");
        assert_eq!(
            bridge::intent_of(d.choice()),
            Some(Intent::GoTo { place: Place::Home })
        );
        // Và đề xuất bị từ chối vẫn được ghi lại.
        assert_eq!(mind.journal().len(), 1);
        assert_eq!(mind.journal()[0].used, fallback());
        assert_eq!(mind.journal()[0].self_name, "Mara");
    }

    #[test]
    fn prose_instead_of_json_falls_with_bad_shape() {
        let log = seen();
        let mut mind = mind_with(Fake::saying("Toi se ve nha an com.", &log));
        let d = mind.think(&obs());
        assert!(
            matches!(d.reason(), Some(FallbackReason::BadShape(_))),
            "{d:?}"
        );
        assert_eq!(*d.choice(), fallback());
    }

    #[test]
    fn json_without_action_falls_with_bad_shape() {
        let log = seen();
        let mut mind = mind_with(Fake::saying(
            r#"{"target": "home", "reason": "ve nha"}"#,
            &log,
        ));
        let d = mind.think(&obs());
        match d.reason() {
            Some(FallbackReason::BadShape(msg)) => assert!(msg.contains("action"), "{msg}"),
            other => panic!("sai nhanh: {other:?}"),
        }
        assert_eq!(*d.choice(), fallback());
    }

    #[test]
    fn a_transport_error_falls_with_timeout() {
        let log = seen();
        let mut mind = mind_with(Fake::failing(
            LlmError::Transport("dns: no such host".to_owned()),
            &log,
        ));
        let d = mind.think(&obs());
        assert_eq!(d.reason(), Some(&FallbackReason::Timeout));
        assert_eq!(*d.choice(), fallback());
        assert!(
            mind.journal()[0].detail.contains("no such host"),
            "chi tiet bi nuot: {}",
            mind.journal()[0].detail
        );
    }

    #[test]
    fn a_bad_response_from_the_provider_falls_with_bad_shape() {
        let log = seen();
        let mut mind = mind_with(Fake::failing(
            LlmError::BadResponse("cau tra loi rong (finish_reason: length)".to_owned()),
            &log,
        ));
        assert!(
            matches!(
                mind.think(&obs()).reason(),
                Some(FallbackReason::BadShape(_))
            ),
            "2xx sai hinh dang phai la bad_shape"
        );
    }

    // Không có provider.

    /// `Mode::Stub` không có stub: cổng mở nhưng phía sau không có ai.
    #[test]
    fn a_stub_gateway_without_stubs_falls_with_no_provider() {
        let mut mind = mind_with(Box::new(Gateway::stub()));
        assert_eq!(mind.mode(), Mode::Stub);
        let d = mind.think(&obs());
        assert_eq!(d.reason(), Some(&FallbackReason::NoProvider));
        assert_eq!(*d.choice(), fallback());
    }

    #[test]
    fn live_mode_without_an_upstream_falls_with_no_provider() {
        let mut gw = Gateway::stub();
        gw.set_mode(Mode::Live);
        let mut mind = mind_with(Box::new(gw));
        assert_eq!(
            mind.think(&obs()).reason(),
            Some(&FallbackReason::NoProvider)
        );
    }

    /// Không có provider thì thế giới **vẫn phải chạy**: mỗi tick vẫn ra một ý
    /// định thực hiện được, và một ngày vẫn có nhiều loại việc.
    #[test]
    fn the_world_keeps_running_with_no_provider_at_all() {
        const DAY: u64 = 200;
        let mut mind = mind_with(Box::new(Gateway::stub()));
        let mut at = Place::Home;
        let mut kinds = BTreeSet::new();
        for tick in 0..DAY {
            let s = Situation {
                tick,
                ticks_per_day: DAY,
                role: Role::Farmer,
                hunger: 10,
                fatigue: 10,
                at,
                nearby: 1,
                nearest: Some(7),
            };
            let observation = bridge::observation_of(&s, "Mara", &["Doran".to_owned()], &[]);
            let d = mind.think_with(&observation, &bridge::routine_fallback(&s));
            assert!(d.is_fallback(), "khong co provider thi phai roi");
            let intent = bridge::intent_of(d.choice())
                .unwrap_or_else(|| panic!("fallback khong thuc hien duoc: {:?}", d.choice()));
            kinds.insert(tag(intent));
            if let Intent::GoTo { place } = intent {
                at = place;
            }
        }
        assert!(
            kinds.len() >= 3,
            "mot ngay khong co LLM chi co {} loai viec",
            kinds.len()
        );
        assert_eq!(mind.fallbacks_total(), DAY);
    }

    // Registry rỗng.

    #[test]
    fn an_empty_registry_always_falls_and_never_panics() {
        let log = seen();
        let mut mind = Mind::new(
            Fake::saying(r#"{"action": "eat"}"#, &log),
            Vec::new(),
            fallback(),
        );
        for _ in 0..10 {
            let d = mind.think(&obs());
            assert_eq!(d.reason(), Some(&FallbackReason::EmptyRegistry));
            assert_eq!(*d.choice(), fallback());
        }
        // Và không tốn một lời gọi model nào: lỗi thuộc về chỗ gọi.
        assert_eq!(mind.calls_made(), 0);
        assert!(log.lock().expect("khoa hong").prompts.is_empty());
    }

    // Ngân sách.

    #[test]
    fn a_spent_budget_stops_asking_and_says_so() {
        let log = seen();
        let mut mind = mind_with(Fake::scripted(
            vec![
                Scripted::Say(r#"{"action": "eat"}"#.to_owned()),
                Scripted::Say(r#"{"action": "work"}"#.to_owned()),
            ],
            &log,
        ))
        .with_budget(1);
        assert!(!mind.think(&obs()).is_fallback());
        assert_eq!(mind.budget_left(), 0);

        let d = mind.think(&obs());
        assert_eq!(d.reason(), Some(&FallbackReason::BudgetSpent));
        assert_eq!(*d.choice(), fallback());
        // Lời gọi thứ hai không bao giờ được gửi đi.
        assert_eq!(log.lock().expect("khoa hong").prompts.len(), 1);
        assert_eq!(mind.calls_made(), 1);
    }

    /// Một lời gọi thất bại vẫn đã tiêu token, nên nó vẫn phải trừ ngân sách.
    #[test]
    fn a_failed_call_still_costs_budget() {
        let log = seen();
        let mut mind = mind_with(Fake::failing(
            LlmError::Transport("timeout".to_owned()),
            &log,
        ))
        .with_budget(2);
        mind.think(&obs());
        assert_eq!(mind.budget_left(), 1);
    }

    #[test]
    fn rate_limit_and_out_of_credit_are_budget_not_timeout() {
        for status in [402_u16, 429] {
            let log = seen();
            let mut mind = mind_with(Fake::failing(
                LlmError::Upstream {
                    status,
                    message: "het han muc".to_owned(),
                },
                &log,
            ));
            assert_eq!(
                mind.think(&obs()).reason(),
                Some(&FallbackReason::BudgetSpent),
                "ma {status} phai la budget_spent"
            );
        }
    }

    #[test]
    fn a_server_error_is_a_timeout_not_a_budget_problem() {
        let log = seen();
        let mut mind = mind_with(Fake::failing(
            LlmError::Upstream {
                status: 503,
                message: "upstream down".to_owned(),
            },
            &log,
        ));
        assert_eq!(mind.think(&obs()).reason(), Some(&FallbackReason::Timeout));
    }

    // Sổ ghi.

    /// `§20.10`: không có exception nào bị nuốt. Sổ có trần, số đếm thì không.
    #[test]
    fn the_journal_is_capped_but_the_count_is_exact() {
        let mut mind = mind_with(Box::new(Gateway::stub()));
        let rounds = JOURNAL_CAP + 50;
        for _ in 0..rounds {
            mind.think(&obs());
        }
        assert_eq!(mind.journal().len(), JOURNAL_CAP, "so phai co tran");
        assert_eq!(
            mind.fallbacks_total(),
            rounds as u64,
            "so dem khong duoc mat"
        );
        let taken = mind.take_journal();
        assert_eq!(taken.len(), JOURNAL_CAP);
        assert!(mind.journal().is_empty());
        assert_eq!(
            mind.fallbacks_total(),
            rounds as u64,
            "lay so ra khong xoa so dem"
        );
    }

    #[test]
    fn every_fallback_reason_has_a_stable_label() {
        let labels = [
            FallbackReason::NoProvider.label(),
            FallbackReason::Timeout.label(),
            FallbackReason::BadShape(String::new()).label(),
            FallbackReason::NotInRegistry(String::new()).label(),
            FallbackReason::BudgetSpent.label(),
            FallbackReason::EmptyRegistry.label(),
        ];
        let unique: BTreeSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), labels.len(), "nhan trung nhau: {labels:?}");
    }

    #[test]
    fn set_fallback_changes_what_think_falls_back_to() {
        let mut mind = mind_with(Box::new(Gateway::stub()));
        let sleep = Choice::new("sleep", None, "kiet suc");
        mind.set_fallback(sleep.clone());
        assert_eq!(*mind.think(&obs()).choice(), sleep);
    }

    #[test]
    fn debug_never_prints_the_client() {
        let mind = mind_with(Box::new(Gateway::stub())).with_model("secret/model");
        let s = format!("{mind:?}");
        assert!(s.contains("registry: 6"), "{s}");
        assert!(!s.contains("Gateway"), "{s}");
    }
}
