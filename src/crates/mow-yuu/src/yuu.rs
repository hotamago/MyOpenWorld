//! [`Yuu`] — mặt tiền cho lời tư vấn của True God (`idea.md §3.1` bước 2).
//!
//! Cùng khuôn với `mow_mind::Mind`: một cổng vào model, một hàm hỏi, và một
//! đường lui xác định khi model không trả lời được hoặc trả lời không dùng
//! được. Khác với `Mind`, không có action registry để kiểm — thứ Yuu kiểm là
//! **trích dẫn**, và việc kiểm đó nằm trong [`crate::read_answer`], không nằm
//! trong struct này. `Yuu::ask` chỉ nối ba việc: dựng prompt, gọi model, đọc
//! trả lời — và quyết định khi nào phải rơi về [`crate::without_model`].
//!
//! ## Ba đường rơi về `without_model`, gộp thành một điều kiện
//!
//! 1. **Không gọi được model** ([`mow_llm::LlmError`] bất kỳ loại nào) — cùng
//!    lý do `Mind` rơi về fallback: ba tầng dưới phải khiến thế giới chạy đúng
//!    khi LLM chết hẳn, và True God không hỏi được Yuu vẫn không được phép là
//!    một màn hình trống.
//! 2. **Model trả rác hoàn toàn** — [`crate::read_answer`] không tìm được một
//!    đối tượng JSON, trả về `Answer` ba trường đều rỗng.
//! 3. **Mọi câu model nói đều bị cắt** — `Answer` có nội dung (`stripped`
//!    không rỗng) nhưng không câu nào qua được kiểm chứng, nên `lines` và
//!    `proposals` vẫn đều rỗng.
//!
//! Cả ba đều dẫn tới cùng một triệu chứng nhìn từ người chơi: một câu trả lời
//! không có gì để đọc. `Yuu::ask` gộp chúng thành đúng một điều kiện
//! (`lines.is_empty() && proposals.is_empty()`) và xử lý như nhau — không có
//! `FallbackReason` riêng cho Yuu vì `§1.2.4` không cần một bảng phân loại lý
//! do rơi tinh vi ở đây, nó chỉ cần lời hứa "không bao giờ trả về màn hình
//! trống" được giữ. Trường hợp 3 vẫn giữ lại dấu vết: `stripped` của model
//! được nối vào trước `stripped` của `without_model`, nên "vì sao model bị bỏ
//! qua" không bao giờ biến mất — chỉ là không được coi là câu trả lời cuối.
//!
//! ## Vì sao `known_powers` là trạng thái của `Yuu`, không phải tham số của `ask`
//!
//! Tập quyền năng có thật đổi chậm hơn nhiều so với câu hỏi (nó đổi khi True
//! God mở khóa một quyền năng mới, không đổi mỗi lượt hỏi), và giữ nó ở đây —
//! qua [`Yuu::with_known_powers`]/[`Yuu::set_known_powers`] — nghĩa là chỗ gọi
//! không phải truyền lại nguyên tập đó ở mỗi câu hỏi. **Mặc định là tập rỗng**:
//! chưa cấu hình thì mọi `proposal` đều bị cắt vì `UnknownPower`. Đây là lựa
//! chọn có chủ ý theo đúng trụ cột `§1.2.4` — "engine quyết định hành động nào
//! hợp lệ" — nên thà Yuu im lặng về can thiệp còn hơn ngầm định một quyền năng
//! chưa ai xác nhận là có thật.

use crate::answer::Answer;
use crate::dossier::Dossier;
use crate::parse::read_answer;
use crate::prompt::{prompt_of, PROMPT_ID, PROMPT_VERSION};
use crate::without_model::without_model;
use mow_llm::{Mode, ModelClient, Request};
use std::collections::BTreeSet;

/// Vai dùng để tra định tuyến model trong cấu hình (`§20.7`).
///
/// Cùng lập luận với `mow_mind::ROUTE_ROLE`: giữ `mow-config` ngoài crate này
/// là có chủ ý. Chỗ gọi làm đúng một dòng:
///
/// ```text
/// let route = cfg.llm.route(mow_yuu::ROUTE_ROLE);
/// let yuu = Yuu::new(client)
///     .with_model(&route.model)
///     .with_max_output_tokens(route.max_output_tokens);
/// ```
pub const ROUTE_ROLE: &str = "yuu";

/// Yuu: một cổng vào model, một tập quyền năng có thật, một đường lui xác
/// định.
pub struct Yuu {
    client: Box<dyn ModelClient>,
    model: String,
    max_output_tokens: u32,
    known_powers: BTreeSet<String>,
    calls: u64,
}

impl core::fmt::Debug for Yuu {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Không in `client`: nó có thể mang khóa API, và một `{:?}` vô tình
        // trong log là cách khóa rò ra dễ nhất.
        f.debug_struct("Yuu")
            .field("mode", &self.client.mode())
            .field("model", &self.model)
            .field("known_powers", &self.known_powers.len())
            .field("calls", &self.calls)
            .finish_non_exhaustive()
    }
}

impl Yuu {
    /// Dựng một Yuu. `known_powers` bắt đầu rỗng — xem tài liệu module.
    #[must_use]
    pub fn new(client: Box<dyn ModelClient>) -> Yuu {
        Yuu {
            client,
            model: String::new(),
            max_output_tokens: 0,
            known_powers: BTreeSet::new(),
            calls: 0,
        }
    }

    /// Model để xin, thường lấy từ `cfg.llm.route(`[`ROUTE_ROLE`]`).model`.
    #[must_use]
    pub fn with_model(mut self, model: &str) -> Yuu {
        model.clone_into(&mut self.model);
        self
    }

    /// Trần token đầu ra. `0` nghĩa là "dùng mặc định của client".
    #[must_use]
    pub fn with_max_output_tokens(mut self, n: u32) -> Yuu {
        self.max_output_tokens = n;
        self
    }

    /// Tập quyền năng có thật — dùng lúc dựng. Xem tài liệu module vì sao mặc
    /// định rỗng.
    #[must_use]
    pub fn with_known_powers(mut self, powers: BTreeSet<String>) -> Yuu {
        self.known_powers = powers;
        self
    }

    /// Cập nhật tập quyền năng có thật giữa ván, khi chỗ gọi không muốn dựng
    /// lại toàn bộ `Yuu` chỉ vì True God vừa mở khóa thêm một quyền năng.
    pub fn set_known_powers(&mut self, powers: BTreeSet<String>) {
        self.known_powers = powers;
    }

    /// Chế độ của cổng model bên dưới.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.client.mode()
    }

    /// Tổng số lời gọi model đã thực hiện — tính cả những lần thất bại, vì
    /// token đã gửi đi là token đã tiêu dù có trả lời hay không.
    #[must_use]
    pub fn calls_made(&self) -> u64 {
        self.calls
    }

    /// Hỏi Yuu, và luôn nhận về một [`Answer`] có căn cứ.
    ///
    /// Không bao giờ trả lỗi ra ngoài và không bao giờ trả một `Answer` hoàn
    /// toàn rỗng nếu [`crate::without_model`] còn nói được điều gì — xem "Ba
    /// đường rơi về `without_model`" trong tài liệu module.
    pub fn ask(&mut self, d: &Dossier, question: &str) -> Answer {
        let known_events = d.known_events();
        let req = Request {
            prompt_id: PROMPT_ID.to_owned(),
            prompt_version: PROMPT_VERSION,
            model: self.model.clone(),
            rendered: prompt_of(d, question),
            max_output_tokens: self.max_output_tokens,
        };

        // Trừ trước khi gọi: token đã gửi đi là token đã tiêu, dù có trả lời
        // hay không (cùng lý do `mow_mind::Mind::think_with`).
        self.calls += 1;

        let text = match self.client.call(&req) {
            Ok(res) => res.text,
            Err(_) => return without_model(d, question),
        };

        let answer = read_answer(&text, &known_events, &self.known_powers);
        if answer.lines.is_empty() && answer.proposals.is_empty() {
            let mut floor = without_model(d, question);
            let mut stripped = answer.stripped;
            stripped.append(&mut floor.stripped);
            floor.stripped = stripped;
            return floor;
        }
        answer
    }
}
