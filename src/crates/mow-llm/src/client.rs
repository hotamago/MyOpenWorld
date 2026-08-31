//! `ModelClient` — bốn chế độ, một interface (`plan.md §P6.7`).
//!
//! Bốn chế độ và mỗi cái giải một bài toán khác nhau:
//!
//! | Chế độ | Dùng khi | Tính chất |
//! |---|---|---|
//! | [`Mode::Stub`] | toàn bộ Giai đoạn B | không mạng, không token, xác định |
//! | [`Mode::Record`] | dựng bộ ghi cho CI | gọi thật, ghi lại |
//! | [`Mode::Replay`] | CI, gỡ lỗi | không mạng, **xác định bit-perfect** |
//! | [`Mode::Live`] | chạy thật | gọi thật |
//!
//! `SetLlmMode` là "chi tiết nhỏ nhưng quyết định" (`§P7.1`): ở `REPLAY` mọi
//! output lấy từ bản ghi nên test hoàn toàn xác định; ở `STUB` không tốn token.
//!
//! `§P7.1` cũng chỉ ra một cái bẫy đáng nhớ: **ai thật sự sở hữu chế độ này**.
//! Gateway sở hữu, không phải server. Một `SetLlmMode` không có ack từ gateway
//! mà vẫn coi là thành công sẽ dẫn tới một bài test tưởng mình đang `REPLAY`
//! trong khi thực ra vẫn `LIVE` — và nó cho kết quả xanh sai.

use mow_math::{StateHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Chế độ gọi mô hình.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Mode {
    /// Trả lời cố định theo prompt id. Mặc định.
    #[default]
    Stub,
    /// Gọi thật và ghi lại.
    Record,
    /// Phát lại từ bản ghi.
    Replay,
    /// Gọi thật.
    Live,
}

/// Yêu cầu gửi tới mô hình.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// Định danh prompt.
    pub prompt_id: String,
    /// Phiên bản prompt.
    pub prompt_version: u32,
    /// Mô hình được yêu cầu.
    pub model: String,
    /// Nội dung đã render.
    pub rendered: String,
    /// Số token tối đa cho đầu ra.
    pub max_output_tokens: u32,
}

impl Request {
    /// Hash canonical của yêu cầu.
    ///
    /// Đây là **khóa của bản ghi phát lại**, nên nó phải là hàm thuần của mọi
    /// thứ ảnh hưởng câu trả lời. Thiếu `model` trong hash thì đổi mô hình sẽ
    /// vẫn trúng bản ghi cũ, và bài test sẽ xanh trong khi thực ra chưa bao giờ
    /// chạy trên mô hình mới.
    pub fn hash(&self) -> StateHash {
        let mut h = StateHasher::with_domain("mow.llm.request.v1");
        h.write_str(&self.prompt_id);
        h.write_u64(u64::from(self.prompt_version));
        h.write_str(&self.model);
        h.write_str(&self.rendered);
        h.write_u64(u64::from(self.max_output_tokens));
        h.finish()
    }
}

/// Câu trả lời.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    /// Nội dung.
    pub text: String,
    /// Mô hình **thật sự** đã sinh ra nó, có thể khác `Request::model` nếu
    /// gateway đã hạ cấp (`§20.10`).
    pub model: String,
    /// Token đầu vào.
    pub input_tokens: u32,
    /// Token đầu ra.
    pub output_tokens: u32,
}

/// Lỗi của gateway.
#[derive(Debug, Error)]
pub enum LlmError {
    /// Ở chế độ `REPLAY` mà không có bản ghi khớp.
    #[error(
        "không có bản ghi cho prompt `{prompt_id}` (hash {hash}). \
         Ở chế độ REPLAY, thiếu bản ghi là LỖI — trả lời tạm sẽ làm bài test \
         xanh mà chưa bao giờ chạy trên dữ liệu thật"
    )]
    NoCassette {
        /// Prompt.
        prompt_id: String,
        /// Hash yêu cầu.
        hash: String,
    },
    /// Không có câu trả lời stub cho prompt này.
    #[error("không có stub cho prompt `{0}`")]
    NoStub(String),
    /// Chế độ cần mạng nhưng chưa có provider.
    #[error("chế độ {0:?} cần một provider thật, chưa được cấu hình")]
    NoProvider(Mode),
    /// Lỗi tệp.
    #[error("lỗi tệp `{path}`: {source}")]
    Io {
        /// Đường dẫn.
        path: String,
        /// Nguyên nhân.
        #[source]
        source: std::io::Error,
    },
    /// Lỗi mã hóa bản ghi.
    #[error("bản ghi hỏng: {0}")]
    BadCassette(String),
    /// Không nối được tới provider: DNS, TLS, quá hạn.
    ///
    /// Tách khỏi [`LlmError::Upstream`] vì hai bên đòi hai phản ứng khác nhau:
    /// lỗi vận chuyển đáng thử lại, còn `400 Bad Request` thì thử lại bao nhiêu
    /// lần cũng vẫn `400`.
    #[error("không gọi được provider: {0}")]
    Transport(String),
    /// Provider trả về mã lỗi, hoặc trả về `{{"error": ...}}` kèm mã 200.
    #[error("provider trả lỗi {status}: {message}")]
    Upstream {
        /// Mã trạng thái HTTP.
        status: u16,
        /// Thông báo, **đã che bí mật**.
        message: String,
    },
    /// Trả lời có mã 2xx nhưng không đúng hình dạng đã hứa.
    #[error("trả lời không đúng hình dạng: {0}")]
    BadResponse(String),
}

/// Kết quả.
pub type LlmResult<T> = Result<T, LlmError>;

/// Giao diện gọi mô hình, không phụ thuộc nhà cung cấp.
pub trait ModelClient: Send {
    /// Chế độ hiện tại.
    fn mode(&self) -> Mode;

    /// Gọi.
    fn call(&mut self, req: &Request) -> LlmResult<Response>;
}

/// Client dùng cho `STUB` và `REPLAY`, và cho `RECORD` khi có provider bên dưới.
pub struct Gateway {
    mode: Mode,
    /// Bản ghi: hash yêu cầu → câu trả lời.
    cassettes: BTreeMap<String, Response>,
    /// Câu trả lời cố định theo `prompt_id`.
    stubs: BTreeMap<String, String>,
    /// Provider thật, nếu có.
    upstream: Option<Box<dyn ModelClient>>,
    /// Nơi ghi bản ghi mới ở chế độ `RECORD`.
    cassette_dir: PathBuf,
}

impl core::fmt::Debug for Gateway {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Gateway")
            .field("mode", &self.mode)
            .field("cassettes", &self.cassettes.len())
            .field("stubs", &self.stubs.len())
            .field("has_upstream", &self.upstream.is_some())
            .finish_non_exhaustive()
    }
}

impl Gateway {
    /// Gateway ở chế độ `STUB`, không có gì.
    pub fn stub() -> Gateway {
        Gateway {
            mode: Mode::Stub,
            cassettes: BTreeMap::new(),
            stubs: BTreeMap::new(),
            upstream: None,
            cassette_dir: PathBuf::from("llm-cassettes"),
        }
    }

    /// Đặt một câu trả lời cố định cho một prompt.
    #[must_use]
    pub fn with_stub(mut self, prompt_id: &str, answer: &str) -> Gateway {
        self.stubs.insert(prompt_id.to_owned(), answer.to_owned());
        self
    }

    /// Gắn provider thật.
    #[must_use]
    pub fn with_upstream(mut self, up: Box<dyn ModelClient>) -> Gateway {
        self.upstream = Some(up);
        self
    }

    /// Thư mục bản ghi.
    #[must_use]
    pub fn with_cassette_dir(mut self, dir: impl Into<PathBuf>) -> Gateway {
        self.cassette_dir = dir.into();
        self
    }

    /// Đổi chế độ.
    ///
    /// Trả về chế độ cũ — chỗ gọi cần nó để khôi phục sau một bài test, và để
    /// ghi vào event log rằng chế độ đã đổi tại tick nào (`§P7.1`).
    pub fn set_mode(&mut self, m: Mode) -> Mode {
        std::mem::replace(&mut self.mode, m)
    }

    /// Nạp bản ghi từ một file NDJSON.
    pub fn load_cassettes(&mut self, path: impl AsRef<Path>) -> LlmResult<usize> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| LlmError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let mut n = 0;
        for (i, dong) in text.lines().enumerate() {
            if dong.trim().is_empty() {
                continue;
            }
            let rec: CassetteRecord = serde_json::from_str(dong)
                .map_err(|e| LlmError::BadCassette(format!("dòng {}: {e}", i + 1)))?;
            self.cassettes.insert(rec.request_hash, rec.response);
            n += 1;
        }
        Ok(n)
    }

    /// Ghi một bản ghi.
    fn ghi_cassette(&self, req: &Request, res: &Response) -> LlmResult<()> {
        std::fs::create_dir_all(&self.cassette_dir).map_err(|source| LlmError::Io {
            path: self.cassette_dir.display().to_string(),
            source,
        })?;
        let p = self
            .cassette_dir
            .join(format!("{}.cassette.jsonl", req.prompt_id));
        let rec = CassetteRecord {
            request_hash: req.hash().to_hex(),
            prompt_id: req.prompt_id.clone(),
            prompt_version: req.prompt_version,
            response: res.clone(),
        };
        let dong = serde_json::to_string(&rec).map_err(|e| LlmError::BadCassette(e.to_string()))?;

        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
            .map_err(|source| LlmError::Io {
                path: p.display().to_string(),
                source,
            })?;
        writeln!(f, "{dong}").map_err(|source| LlmError::Io {
            path: p.display().to_string(),
            source,
        })
    }
}

/// Một dòng trong file bản ghi.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CassetteRecord {
    request_hash: String,
    prompt_id: String,
    prompt_version: u32,
    response: Response,
}

impl ModelClient for Gateway {
    fn mode(&self) -> Mode {
        self.mode
    }

    fn call(&mut self, req: &Request) -> LlmResult<Response> {
        match self.mode {
            Mode::Stub => {
                let text = self
                    .stubs
                    .get(&req.prompt_id)
                    .cloned()
                    .ok_or_else(|| LlmError::NoStub(req.prompt_id.clone()))?;
                Ok(Response {
                    text,
                    model: "stub".to_owned(),
                    input_tokens: 0,
                    output_tokens: 0,
                })
            }

            Mode::Replay => {
                // Thiếu bản ghi là **lỗi**, không phải lý do để gọi thật hay
                // trả lời tạm. Cả hai lối thoát đó đều biến một bài test xanh
                // thành một lời nói dối.
                self.cassettes
                    .get(&req.hash().to_hex())
                    .cloned()
                    .ok_or_else(|| LlmError::NoCassette {
                        prompt_id: req.prompt_id.clone(),
                        hash: req.hash().short(),
                    })
            }

            Mode::Record => {
                let up = self
                    .upstream
                    .as_mut()
                    .ok_or(LlmError::NoProvider(Mode::Record))?;
                let res = up.call(req)?;
                self.ghi_cassette(req, &res)?;
                Ok(res)
            }

            Mode::Live => {
                let up = self
                    .upstream
                    .as_mut()
                    .ok_or(LlmError::NoProvider(Mode::Live))?;
                up.call(req)
            }
        }
    }
}
