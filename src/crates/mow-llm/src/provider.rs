//! Provider thật cho `LIVE`/`RECORD` — `OpenRouter` và mọi endpoint cùng lược đồ
//! `OpenAI` (`§P6.7`, `§20.10`).
//!
//! Cho tới trước file này, [`Mode::Live`] và [`Mode::Record`] đều trả
//! [`LlmError::NoProvider`]: `Gateway` có chỗ cắm `upstream` nhưng **không có gì
//! cắm vào**. Nghĩa là toàn bộ dự án chạy được đúng một chế độ — `STUB`.
//!
//! ## Vì sao là "`OpenAI-compatible`" chứ không phải "`OpenRouter`"
//!
//! `OpenRouter` dùng nguyên lược đồ của `OpenAI`: `POST {base}/chat/completions`,
//! `Authorization: Bearer`, cùng hình dạng `choices[0].message.content`. Viết
//! client theo lược đồ thay vì theo nhà cung cấp nghĩa là cùng một đoạn mã chạy
//! với `OpenRouter`, `OpenAI`, Together, hay một `llama.cpp` chạy trên máy — chỉ
//! đổi `base_url`. Khóa vào một nhà cung cấp ở đây không mua được gì.
//!
//! ## `Transport` tồn tại để test không cần mạng
//!
//! Phần khó của client này không phải là HTTP — nó là **dựng yêu cầu và đọc trả
//! lời cho đúng**, gồm cả những trả lời sai hình dạng. Nếu lời gọi mạng nằm
//! thẳng trong `call()` thì mọi bài test cho phần đó đều cần một khóa API thật
//! và một kết nối, tức là **sẽ không có bài test nào**.
//!
//! [`Transport`] tách đúng một dòng — "gửi chuỗi này tới URL kia, đưa tôi mã
//! trạng thái và thân trả lời" — ra khỏi mọi thứ còn lại. Phần còn lại kiểm
//! được ngoại tuyến, xác định, và ở đây nó có kiểm.
//!
//! ## Ba cái bẫy đã gặp và đã đóng
//!
//! 1. **Lỗi mang mã 200.** `OpenRouter` trả `{"error": {...}}` kèm `HTTP 200` khi
//!    provider phía sau từ chối. Chỉ nhìn mã trạng thái sẽ đọc nó thành một câu
//!    trả lời rỗng, và NPC im lặng mà không ai biết vì sao.
//! 2. **`content` rỗng nhưng hợp lệ.** `finish_reason: "length"` cho một chuỗi
//!    rỗng. Đó là một lỗi cấu hình (`max_output_tokens` quá nhỏ), không phải một
//!    câu trả lời — nên nó phải kêu.
//! 3. **Khóa API lọt vào thông báo lỗi.** Thông báo lỗi đi vào log, log đi vào
//!    báo cáo sự cố. [`che_bi_mat`] chạy trên **mọi** chuỗi trước khi nó vào một
//!    biến thể lỗi.

use crate::client::{LlmError, LlmResult, Mode, ModelClient, Request, Response};
use serde_json::{json, Value};

/// Một lời gọi HTTP đã hoàn tất: mã trạng thái và thân trả lời thô.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReply {
    /// Mã trạng thái.
    pub status: u16,
    /// Thân trả lời, chưa phân tích.
    pub body: String,
}

/// Đường ra mạng, tách khỏi phần dựng yêu cầu và đọc trả lời.
///
/// Xem tài liệu module: cái tách ra ở đây là thứ *không* kiểm được ngoại tuyến,
/// và nó được giữ nhỏ nhất có thể để mọi thứ còn lại kiểm được.
pub trait Transport: Send {
    /// Gửi `body` dạng JSON tới `url` kèm `headers`, trả về mã và thân.
    ///
    /// Lỗi ở đây là lỗi **tầng vận chuyển** (không nối được, quá hạn) — một mã
    /// `4xx` là `Ok`, vì nó có thân trả lời và thân đó thường nói rõ chuyện gì.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: &str,
    ) -> Result<HttpReply, String>;
}

/// Che những đoạn trông như khóa API trong một chuỗi sắp đi vào log.
///
/// Không cố phân tích: chỉ cần một tiền tố `sk-` là đủ để cắt phần còn lại của
/// token. Thà che nhầm một chuỗi vô hại còn hơn để lọt một khóa thật — và khóa
/// `OpenRouter` có dạng `sk-or-v1-...`, đúng dạng bị bắt ở đây.
#[must_use]
pub fn che_bi_mat(s: &str) -> String {
    let mut ra = String::with_capacity(s.len());
    let bytes: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(&['s', 'k', '-']) {
            ra.push_str("sk-***");
            // Nuốt hết phần thân token: chữ, số, `-`, `_`.
            i += 3;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], '-' | '_'))
            {
                i += 1;
            }
        } else {
            ra.push(bytes[i]);
            i += 1;
        }
    }
    ra
}

/// Cắt một thân trả lời dài về độ dài đọc được trong log.
fn cat_ngan(s: &str) -> String {
    const TRAN: usize = 400;
    let s = che_bi_mat(s);
    if s.chars().count() <= TRAN {
        return s;
    }
    let dau: String = s.chars().take(TRAN).collect();
    format!("{dau}… (còn {} ký tự)", s.chars().count() - TRAN)
}

/// Thông tin ghi công ứng dụng.
///
/// `OpenRouter` dùng `HTTP-Referer` và `X-Title` để xếp hạng ứng dụng trên bảng
/// công khai của họ. Cả hai đều tùy chọn và **không** ảnh hưởng kết quả; để
/// trống thì không gửi header.
#[derive(Debug, Clone, Default)]
pub struct Attribution {
    /// URL của ứng dụng (`HTTP-Referer`).
    pub url: String,
    /// Tên hiển thị (`X-Title`).
    pub title: String,
}

/// Client cho mọi endpoint theo lược đồ `OpenAI`.
pub struct OpenAiCompatClient<T: Transport> {
    base: String,
    api_key: String,
    model_mac_dinh: String,
    max_output_tokens: u32,
    temperature_milli: u32,
    ghi_cong: Attribution,
    transport: T,
}

impl<T: Transport> core::fmt::Debug for OpenAiCompatClient<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Không in `api_key`. Một `{:?}` vô tình trong log là cách khóa rò ra
        // dễ nhất, và nó không để lại dấu vết nào cho tới khi quá muộn.
        f.debug_struct("OpenAiCompatClient")
            .field("base", &self.base)
            .field("model_mac_dinh", &self.model_mac_dinh)
            .field("co_khoa", &!self.api_key.is_empty())
            .finish_non_exhaustive()
    }
}

impl<T: Transport> OpenAiCompatClient<T> {
    /// Dựng client.
    ///
    /// `base` là gốc API **không** kèm `/chat/completions`, ví dụ
    /// `https://openrouter.ai/api/v1`.
    pub fn new(base: &str, api_key: &str, model_mac_dinh: &str, transport: T) -> Self {
        OpenAiCompatClient {
            base: base.trim_end_matches('/').to_owned(),
            api_key: api_key.to_owned(),
            model_mac_dinh: model_mac_dinh.to_owned(),
            max_output_tokens: 1024,
            temperature_milli: 0,
            ghi_cong: Attribution::default(),
            transport,
        }
    }

    /// Trần token đầu ra khi [`Request::max_output_tokens`] bằng 0.
    #[must_use]
    pub fn with_max_output_tokens(mut self, n: u32) -> Self {
        self.max_output_tokens = n;
        self
    }

    /// Nhiệt độ lấy mẫu, tính theo phần nghìn (`0` = tham lam).
    ///
    /// Số nguyên chứ không phải số thực, và đó không phải là sự cầu kỳ: cấu hình
    /// đi vào event log khi nó đổi (`§8.4`), và một `f32` trong event log là một
    /// giá trị có thể tuần tự hóa khác nhau giữa hai bản build (`§P10.2.1`).
    #[must_use]
    pub fn with_temperature_milli(mut self, n: u32) -> Self {
        self.temperature_milli = n;
        self
    }

    /// Ghi công ứng dụng.
    #[must_use]
    pub fn with_attribution(mut self, a: Attribution) -> Self {
        self.ghi_cong = a;
        self
    }

    fn headers(&self) -> Vec<(&'static str, String)> {
        let mut h = vec![
            ("Authorization", format!("Bearer {}", self.api_key)),
            ("Content-Type", "application/json".to_owned()),
        ];
        if !self.ghi_cong.url.is_empty() {
            h.push(("HTTP-Referer", self.ghi_cong.url.clone()));
        }
        if !self.ghi_cong.title.is_empty() {
            h.push(("X-Title", self.ghi_cong.title.clone()));
        }
        h
    }

    /// Thân yêu cầu cho `POST /chat/completions`.
    fn than_yeu_cau(&self, req: &Request) -> Value {
        let model = if req.model.is_empty() {
            &self.model_mac_dinh
        } else {
            &req.model
        };
        let max = if req.max_output_tokens == 0 {
            self.max_output_tokens
        } else {
            req.max_output_tokens
        };
        json!({
            "model": model,
            "messages": [{ "role": "user", "content": req.rendered }],
            "max_tokens": max,
            // `temperature` phải là số trong JSON; phép chia này nằm ở **biên
            // gửi đi**, không nằm trên đường commit.
            "temperature": f64::from(self.temperature_milli) / 1000.0,
        })
    }

    /// Đọc một trả lời `2xx` thành [`Response`].
    fn doc_tra_loi(v: &Value, mac_dinh_model: &str) -> LlmResult<Response> {
        // Bẫy 1: lỗi mang mã 200.
        if let Some(e) = v.get("error") {
            let msg = e
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("không có message");
            return Err(LlmError::Upstream {
                status: 200,
                message: che_bi_mat(msg),
            });
        }

        let choice = v
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .ok_or_else(|| {
                LlmError::BadResponse(format!("thiếu `choices`: {}", cat_ngan(&v.to_string())))
            })?;

        let text = choice
            .pointer("/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LlmError::BadResponse(format!(
                    "thiếu `choices[0].message.content`: {}",
                    cat_ngan(&choice.to_string())
                ))
            })?;

        // Bẫy 2: rỗng vì hết token là lỗi cấu hình, không phải câu trả lời.
        if text.trim().is_empty() {
            let ly_do = choice
                .get("finish_reason")
                .and_then(Value::as_str)
                .unwrap_or("không rõ");
            return Err(LlmError::BadResponse(format!(
                "câu trả lời rỗng (finish_reason: {ly_do}). Nếu là `length` thì \
                 `llm.max_output_tokens` quá nhỏ — một chuỗi rỗng đi tiếp vào \
                 validator sẽ thành một lần fallback không ai giải thích được"
            )));
        }

        Ok(Response {
            text: text.to_owned(),
            // `§20.10`: mô hình **thật sự** trả lời có thể khác mô hình đã xin.
            // OpenRouter báo lại ở trường `model` cấp cao nhất, và giữ đúng giá
            // trị đó là điều kiện để `§22.17` truy được nguồn về sau.
            model: v
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or(mac_dinh_model)
                .to_owned(),
            input_tokens: v
                .pointer("/usage/prompt_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            output_tokens: v
                .pointer("/usage/completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
        })
    }
}

impl<T: Transport> ModelClient for OpenAiCompatClient<T> {
    fn mode(&self) -> Mode {
        // Client này **luôn** gọi thật. Chế độ là việc của `Gateway`: nó quyết
        // định có gọi tới đây hay không, và giữ quyết định đó ở một chỗ duy nhất.
        Mode::Live
    }

    fn call(&mut self, req: &Request) -> LlmResult<Response> {
        let url = format!("{}/chat/completions", self.base);
        let than = self.than_yeu_cau(req).to_string();
        let hdr = self.headers();

        let reply = self
            .transport
            .post_json(&url, &hdr, &than)
            .map_err(|e| LlmError::Transport(che_bi_mat(&e)))?;

        if !(200..300).contains(&reply.status) {
            let msg = serde_json::from_str::<Value>(&reply.body)
                .ok()
                .and_then(|v| {
                    v.pointer("/error/message")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| cat_ngan(&reply.body));
            return Err(LlmError::Upstream {
                status: reply.status,
                message: che_bi_mat(&msg),
            });
        }

        let v: Value = serde_json::from_str(&reply.body)
            .map_err(|e| LlmError::BadResponse(format!("{e}: {}", cat_ngan(&reply.body))))?;
        Self::doc_tra_loi(&v, &self.model_mac_dinh)
    }
}

// ── Đường ra mạng thật ───────────────────────────────────────────────────────

/// [`Transport`] chạy trên `ureq`.
///
/// `ureq` chứ không phải `reqwest`: [`ModelClient`] là một trait **đồng bộ**, và
/// kéo `tokio` vào để phục vụ một trait đồng bộ là kéo cả một runtime cho một
/// lời gọi request–response. Cùng lập luận với `mow-vector::qdrant`.
#[derive(Debug)]
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// Dựng transport với thời gian chờ toàn cục.
    #[must_use]
    pub fn new(timeout_ms: u64) -> UreqTransport {
        let cau_hinh = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_millis(timeout_ms)))
            // Để `4xx` về dạng `Ok` kèm thân trả lời. Mặc định của `ureq` biến
            // chúng thành `Err` và **vứt thân đi** — mà thân chính là chỗ nhà
            // cung cấp nói "khóa sai" hay "hết hạn mức".
            .http_status_as_error(false)
            .build();
        UreqTransport {
            agent: ureq::Agent::new_with_config(cau_hinh),
        }
    }
}

impl Transport for UreqTransport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, String)],
        body: &str,
    ) -> Result<HttpReply, String> {
        let mut req = self.agent.post(url);
        for (k, v) in headers {
            req = req.header(*k, v.as_str());
        }
        let mut resp = req.send(body).map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        Ok(HttpReply { status, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Một yêu cầu đã gửi: `(url, headers, body)`.
    type DaGui = (String, Vec<(String, String)>, String);

    /// Transport giả: ghi lại yêu cầu, trả lời theo kịch bản.
    struct Gia {
        tra_loi: HttpReply,
        da_nhan: RefCell<Vec<DaGui>>,
    }

    impl Gia {
        fn moi(status: u16, body: &str) -> Gia {
            Gia {
                tra_loi: HttpReply {
                    status,
                    body: body.to_owned(),
                },
                da_nhan: RefCell::new(Vec::new()),
            }
        }
    }

    // `Send` an toàn ở đây vì transport giả chỉ sống trong một luồng test.
    unsafe impl Send for Gia {}

    impl Transport for Gia {
        fn post_json(
            &self,
            url: &str,
            headers: &[(&str, String)],
            body: &str,
        ) -> Result<HttpReply, String> {
            self.da_nhan.borrow_mut().push((
                url.to_owned(),
                headers
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), v.clone()))
                    .collect(),
                body.to_owned(),
            ));
            Ok(self.tra_loi.clone())
        }
    }

    fn yeu_cau() -> Request {
        Request {
            prompt_id: "npc.decide".to_owned(),
            prompt_version: 1,
            model: String::new(),
            rendered: "bạn thấy một cái giếng".to_owned(),
            max_output_tokens: 0,
        }
    }

    const TRA_LOI_TOT: &str = r#"{
        "model": "anthropic/claude-3.5-haiku",
        "choices": [{ "message": { "content": "múc nước" }, "finish_reason": "stop" }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 3 }
    }"#;

    #[test]
    fn goi_thanh_cong_doc_dung_moi_truong() {
        let mut c = OpenAiCompatClient::new(
            "https://openrouter.ai/api/v1/",
            "sk-or-v1-bimat",
            "anthropic/claude-3.5-haiku",
            Gia::moi(200, TRA_LOI_TOT),
        );
        let r = c.call(&yeu_cau()).expect("phải thành công");
        assert_eq!(r.text, "múc nước");
        assert_eq!(r.model, "anthropic/claude-3.5-haiku");
        assert_eq!(r.input_tokens, 12);
        assert_eq!(r.output_tokens, 3);
    }

    #[test]
    fn url_khong_nhan_doi_dau_gach() {
        let c = OpenAiCompatClient::new(
            "https://openrouter.ai/api/v1/",
            "k",
            "m",
            Gia::moi(200, TRA_LOI_TOT),
        );
        assert_eq!(c.base, "https://openrouter.ai/api/v1");
    }

    #[test]
    fn khoa_di_trong_header_authorization() {
        let mut c = OpenAiCompatClient::new(
            "https://x/v1",
            "sk-or-v1-bimat",
            "m",
            Gia::moi(200, TRA_LOI_TOT),
        );
        c.call(&yeu_cau()).unwrap();
        let nhan = c.transport.da_nhan.borrow();
        let (url, hdr, _) = &nhan[0];
        assert_eq!(url, "https://x/v1/chat/completions");
        assert!(hdr
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer sk-or-v1-bimat"));
    }

    #[test]
    fn ghi_cong_rong_thi_khong_gui_header() {
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, TRA_LOI_TOT));
        c.call(&yeu_cau()).unwrap();
        let nhan = c.transport.da_nhan.borrow();
        let ten: Vec<&str> = nhan[0].1.iter().map(|(k, _)| k.as_str()).collect();
        assert!(!ten.contains(&"HTTP-Referer"), "{ten:?}");
        assert!(!ten.contains(&"X-Title"), "{ten:?}");
    }

    #[test]
    fn ghi_cong_co_thi_gui_ca_hai() {
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, TRA_LOI_TOT))
            .with_attribution(Attribution {
                url: "https://myopenworld.dev".to_owned(),
                title: "My Open World".to_owned(),
            });
        c.call(&yeu_cau()).unwrap();
        let nhan = c.transport.da_nhan.borrow();
        let ten: Vec<&str> = nhan[0].1.iter().map(|(k, _)| k.as_str()).collect();
        assert!(ten.contains(&"HTTP-Referer"), "{ten:?}");
        assert!(ten.contains(&"X-Title"), "{ten:?}");
    }

    #[test]
    fn request_rong_thi_dung_mac_dinh_cua_client() {
        let mut c =
            OpenAiCompatClient::new("https://x/v1", "k", "mac/dinh", Gia::moi(200, TRA_LOI_TOT))
                .with_max_output_tokens(777);
        c.call(&yeu_cau()).unwrap();
        let nhan = c.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&nhan[0].2).unwrap();
        assert_eq!(v["model"], "mac/dinh");
        assert_eq!(v["max_tokens"], 777);
    }

    #[test]
    fn request_co_gia_tri_thi_thang_mac_dinh() {
        let mut c =
            OpenAiCompatClient::new("https://x/v1", "k", "mac/dinh", Gia::moi(200, TRA_LOI_TOT))
                .with_max_output_tokens(777);
        let mut r = yeu_cau();
        r.model = "cu/the".to_owned();
        r.max_output_tokens = 42;
        c.call(&r).unwrap();
        let nhan = c.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&nhan[0].2).unwrap();
        assert_eq!(v["model"], "cu/the");
        assert_eq!(v["max_tokens"], 42);
    }

    #[test]
    fn nhiet_do_mac_dinh_la_tham_lam() {
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, TRA_LOI_TOT));
        c.call(&yeu_cau()).unwrap();
        let nhan = c.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&nhan[0].2).unwrap();
        assert_eq!(v["temperature"], 0.0);
    }

    #[test]
    fn nhiet_do_phan_nghin_doi_thanh_so() {
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, TRA_LOI_TOT))
            .with_temperature_milli(700);
        c.call(&yeu_cau()).unwrap();
        let nhan = c.transport.da_nhan.borrow();
        let v: Value = serde_json::from_str(&nhan[0].2).unwrap();
        assert_eq!(v["temperature"], 0.7);
    }

    // ── Ba cái bẫy ──────────────────────────────────────────────────────────

    #[test]
    fn bay_1_loi_mang_ma_200_van_phai_la_loi() {
        let than = r#"{"error": {"message": "provider hết hạn mức", "code": 429}}"#;
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, than));
        let e = c.call(&yeu_cau()).expect_err("200 kèm `error` vẫn là lỗi");
        assert!(e.to_string().contains("hết hạn mức"), "{e}");
    }

    #[test]
    fn bay_2_content_rong_la_loi_chu_khong_phai_cau_tra_loi() {
        let than = r#"{"choices":[{"message":{"content":"  "},"finish_reason":"length"}]}"#;
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, than));
        let e = c
            .call(&yeu_cau())
            .expect_err("chuỗi rỗng không phải câu trả lời");
        assert!(e.to_string().contains("length"), "{e}");
        assert!(e.to_string().contains("max_output_tokens"), "{e}");
    }

    #[test]
    fn bay_3_khoa_khong_bao_gio_lot_vao_thong_bao_loi() {
        // Một endpoint hỏng dội ngược cả yêu cầu — gồm cả header — vào thân lỗi.
        let than = "upstream said: Authorization: Bearer sk-or-v1-KHOATHAT-abc123 rejected";
        let mut c = OpenAiCompatClient::new(
            "https://x/v1",
            "sk-or-v1-KHOATHAT-abc123",
            "m",
            Gia::moi(500, than),
        );
        let e = c.call(&yeu_cau()).expect_err("500 là lỗi");
        let s = e.to_string();
        assert!(!s.contains("KHOATHAT"), "khóa lọt vào log: {s}");
        assert!(s.contains("sk-***"), "{s}");
    }

    #[test]
    fn debug_khong_in_khoa() {
        let c = OpenAiCompatClient::new(
            "https://x/v1",
            "sk-or-v1-KHOATHAT",
            "m",
            Gia::moi(200, "{}"),
        );
        let s = format!("{c:?}");
        assert!(!s.contains("KHOATHAT"), "{s}");
        assert!(s.contains("co_khoa: true"), "{s}");
    }

    #[test]
    fn che_bi_mat_giu_nguyen_phan_con_lai() {
        assert_eq!(
            che_bi_mat("dùng sk-or-v1-abc_DEF-9 để gọi"),
            "dùng sk-*** để gọi"
        );
        assert_eq!(che_bi_mat("không có gì"), "không có gì");
    }

    #[test]
    fn ma_401_noi_ro_nha_cung_cap_noi_gi() {
        let than = r#"{"error":{"message":"No auth credentials found"}}"#;
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(401, than));
        let e = c.call(&yeu_cau()).expect_err("401");
        assert!(e.to_string().contains("No auth credentials"), "{e}");
        assert!(e.to_string().contains("401"), "{e}");
    }

    #[test]
    fn than_khong_phai_json_thi_bao_ro_chu_khong_hoang_mang() {
        let mut c =
            OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, "<html>502</html>"));
        let e = c.call(&yeu_cau()).expect_err("HTML không phải JSON");
        assert!(e.to_string().contains("<html>"), "{e}");
    }

    #[test]
    fn thieu_choices_khong_panic() {
        let mut c =
            OpenAiCompatClient::new("https://x/v1", "k", "m", Gia::moi(200, r#"{"id":"x"}"#));
        let e = c.call(&yeu_cau()).expect_err("thiếu choices");
        assert!(e.to_string().contains("choices"), "{e}");
    }

    #[test]
    fn loi_van_chuyen_khac_loi_giao_thuc() {
        struct Chet;
        impl Transport for Chet {
            fn post_json(
                &self,
                _: &str,
                _: &[(&str, String)],
                _: &str,
            ) -> Result<HttpReply, String> {
                Err("dns: no such host".to_owned())
            }
        }
        let mut c = OpenAiCompatClient::new("https://x/v1", "k", "m", Chet);
        let e = c.call(&yeu_cau()).expect_err("không nối được");
        assert!(matches!(e, LlmError::Transport(_)), "{e:?}");
    }
}
