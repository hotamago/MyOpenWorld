//! Cấu trúc config. **Đây là nguồn**; JSON Schema sinh ra từ đây.

use crate::error::{ConfigError, ConfigResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Toàn bộ cấu hình ứng dụng.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// Tên môi trường đã thật sự được nạp.
    ///
    /// **Dẫn xuất, không phải cấu hình.** [`crate::load`] ghi đè trường này
    /// bằng tên môi trường nó vừa nạp, nên giá trị trong YAML chỉ có tác dụng
    /// làm tài liệu và `MOW_ENV` không sửa được nó. Xem `load` để biết vì sao.
    #[serde(default = "mac_dinh_env")]
    pub env: String,
    /// Mô phỏng.
    #[serde(default)]
    pub sim: SimConfig,
    /// Lưu trữ.
    #[serde(default)]
    pub persistence: PersistenceConfig,
    /// Chỉ mục ký ức.
    #[serde(default)]
    pub vector: VectorConfig,
    /// Mô hình ngôn ngữ.
    #[serde(default)]
    pub llm: LlmConfig,
    /// Sinh vector cho chỉ mục ký ức.
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    /// Ngân sách nhận thức.
    #[serde(default)]
    pub budget: BudgetConfig,
    /// Content pack.
    #[serde(default)]
    pub content: ContentConfig,
    /// Quan sát và log.
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

fn mac_dinh_env() -> String {
    "dev".to_owned()
}

impl Default for AppConfig {
    /// Cấu hình chạy được trên một máy trắng: STUB, `SQLite`, không mạng.
    ///
    /// Đây không phải tiện nghi cho test — nó là phát biểu rằng dự án **khởi
    /// động được khi không có gì cả**. Một mặc định đòi khóa API là một mặc
    /// định biến "thử xem nó là gì" thành một buổi chiều.
    fn default() -> Self {
        AppConfig {
            env: mac_dinh_env(),
            sim: SimConfig::default(),
            persistence: PersistenceConfig::default(),
            vector: VectorConfig::default(),
            llm: LlmConfig::default(),
            embedding: EmbeddingConfig::default(),
            budget: BudgetConfig::default(),
            content: ContentConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

/// Cấu hình mô phỏng.
///
/// **Mọi trường ở đây ảnh hưởng kết quả mô phỏng**, nên đổi chúng giữa chừng
/// phải ghi vào event log (`§P6.1`, `§8.4`) — nếu không replay sẽ lệch mà không
/// có gì trong lịch sử giải thích tại sao.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SimConfig {
    /// Số tick thần mỗi giây thời gian thật khi chạy ở tốc độ ×1.
    #[serde(default = "mac_dinh_tick_rate")]
    pub tick_rate: u32,
    /// Cạnh của một chunk, tính bằng ô.
    #[serde(default = "mac_dinh_chunk_size")]
    pub chunk_size: u32,
    /// Bán kính chunk giữ ở mức `active`, tính từ tiêu điểm.
    #[serde(default = "mac_dinh_active_radius")]
    pub active_radius: u32,
    /// Số tick giữa hai lần chụp ảnh tự động. `0` là tắt.
    #[serde(default = "mac_dinh_snapshot_interval")]
    pub snapshot_interval: u64,
}

fn mac_dinh_tick_rate() -> u32 {
    20
}
fn mac_dinh_chunk_size() -> u32 {
    32
}
fn mac_dinh_active_radius() -> u32 {
    3
}
fn mac_dinh_snapshot_interval() -> u64 {
    10_000
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            tick_rate: mac_dinh_tick_rate(),
            chunk_size: mac_dinh_chunk_size(),
            active_radius: mac_dinh_active_radius(),
            snapshot_interval: mac_dinh_snapshot_interval(),
        }
    }
}

/// Cấu hình lưu trữ.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PersistenceConfig {
    /// Đường dẫn file save (`SQLite`) hoặc DSN (Postgres, từ Giai đoạn C).
    #[serde(default = "mac_dinh_save")]
    pub url: String,
}

fn mac_dinh_save() -> String {
    "saves/world.sqlite".to_owned()
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        PersistenceConfig {
            url: mac_dinh_save(),
        }
    }
}

/// Cấu hình chỉ mục ký ức.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorConfig {
    /// Đường dẫn file chỉ mục.
    ///
    /// **Phải khác** [`PersistenceConfig::url`] — `P0-09`. Kho ký ức nằm ở file
    /// riêng để không tranh khóa với tiến trình mô phỏng, và để `PC-06` xóa
    /// sạch chỉ mục mà không đụng vào save của người chơi.
    #[serde(default = "mac_dinh_vector_path")]
    pub url: String,
    /// Số chiều của embedding.
    #[serde(default = "mac_dinh_dim")]
    pub dimension: usize,
}

fn mac_dinh_vector_path() -> String {
    "memory-index/index.sqlite".to_owned()
}
fn mac_dinh_dim() -> usize {
    768
}

impl Default for VectorConfig {
    fn default() -> Self {
        VectorConfig {
            url: mac_dinh_vector_path(),
            dimension: mac_dinh_dim(),
        }
    }
}

/// Chế độ gọi mô hình (`§P6.7`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum LlmMode {
    /// Gọi thật.
    Live,
    /// Gọi thật và ghi lại để phát lại sau.
    Record,
    /// Phát lại từ bản ghi; không có mạng.
    Replay,
    /// Trả lời cố định. Toàn bộ Giai đoạn B chạy ở chế độ này.
    Stub,
}

/// Cấu hình mô hình ngôn ngữ.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LlmConfig {
    /// Chế độ.
    #[serde(default = "mac_dinh_llm_mode")]
    pub mode: LlmMode,
    /// Nhà cung cấp, ví dụ `openrouter`, `openai`.
    ///
    /// Chỉ là nhãn cho thông báo lỗi và cho log. Thứ thật sự quyết định gọi đi
    /// đâu là [`LlmConfig::base_url`] — mọi endpoint theo lược đồ `OpenAI` đều
    /// dùng chung một client, nên khóa vào tên nhà cung cấp không mua được gì.
    #[serde(default)]
    pub provider: String,
    /// Gốc API, **không** kèm `/chat/completions`.
    ///
    /// `OpenRouter`: `https://openrouter.ai/api/v1`.
    #[serde(default)]
    pub base_url: String,
    /// **Tên biến môi trường** chứa API key — không phải key.
    ///
    /// Đây là chỗ dễ hiểu sai nhất trong cả file, nên nói thẳng: giá trị đúng
    /// là `OPENROUTER_API_KEY`, không phải `sk-or-v1-...`. File này được commit
    /// (`§P10.6`), và [`AppConfig::validate`] từ chối khởi động khi giá trị ở
    /// đây trông giống một khóa thật.
    #[serde(default = "mac_dinh_api_key_env")]
    pub api_key_env: String,
    /// Mô hình mặc định, ví dụ `deepseek/deepseek-v4-flash-0731`.
    #[serde(default)]
    pub model: String,
    /// Trần token đầu ra mỗi lời gọi.
    ///
    /// Quá nhỏ thì mô hình bị cắt giữa chừng và trả về một chuỗi rỗng kèm
    /// `finish_reason: "length"` — thứ trông y hệt "mô hình không có gì để
    /// nói", và dẫn tới một lần fallback không ai giải thích được.
    #[serde(default = "mac_dinh_max_output")]
    pub max_output_tokens: u32,
    /// Nhiệt độ lấy mẫu, **tính theo phần nghìn** (`0` = tham lam).
    ///
    /// Số nguyên chứ không phải số thực, và đó không phải sự cầu kỳ: cấu hình
    /// ảnh hưởng mô phỏng phải ghi được vào event log (`§8.4`), và một `f32`
    /// trong event log là một giá trị có thể tuần tự hóa khác nhau giữa hai bản
    /// build (`§P10.2.1`).
    ///
    /// Mặc định 0 cũng là một phát biểu: sự đa dạng của thế giới phải đến từ
    /// seed, không từ bộ lấy mẫu — vì đa dạng của bộ lấy mẫu không phát lại được.
    #[serde(default)]
    pub temperature_milli: u32,
    /// URL ứng dụng gửi kèm để ghi công (`OpenRouter`: `HTTP-Referer`).
    #[serde(default)]
    pub app_url: String,
    /// Tên ứng dụng gửi kèm để ghi công (`OpenRouter`: `X-Title`).
    #[serde(default)]
    pub app_title: String,
    /// **Độ trễ nhận thức cố định `D`, tính bằng tick** (`§20.2.2`).
    ///
    /// Đây là trường quan trọng nhất trong cả file config, và lý do nó tồn tại
    /// đáng đọc kỹ: một thực thể suy nghĩ ở tick `T` sẽ hành động ở tick `T+D`,
    /// **bất kể mô hình trả lời nhanh hay chậm**. Không có nó, một mô hình trả
    /// lời trong 500ms và một mô hình trả lời trong 3 giây sẽ tạo ra hai thế
    /// giới khác nhau từ cùng một seed, và replay trở thành vô nghĩa.
    ///
    /// Muốn nhân vật phản ứng nhanh hơn thì tăng `cognition_rate` của nó — đó
    /// là thuộc tính của thế giới. Đường truyền nhanh không phải là một thuộc
    /// tính của thế giới.
    #[serde(default = "mac_dinh_do_tre")]
    pub cognitive_latency_ticks: u64,
    /// Thời gian chờ mỗi lời gọi, tính bằng mili giây.
    #[serde(default = "mac_dinh_timeout")]
    pub timeout_ms: u64,
    /// Thư mục chứa bản ghi cho chế độ `RECORD`/`REPLAY`.
    #[serde(default = "mac_dinh_cassette")]
    pub cassette_dir: String,
}

fn mac_dinh_llm_mode() -> LlmMode {
    LlmMode::Stub
}
fn mac_dinh_api_key_env() -> String {
    "OPENROUTER_API_KEY".to_owned()
}
fn mac_dinh_max_output() -> u32 {
    1024
}
fn mac_dinh_do_tre() -> u64 {
    10
}
fn mac_dinh_timeout() -> u64 {
    30_000
}
fn mac_dinh_cassette() -> String {
    "llm-cassettes".to_owned()
}

impl Default for LlmConfig {
    fn default() -> Self {
        LlmConfig {
            mode: mac_dinh_llm_mode(),
            provider: String::new(),
            base_url: String::new(),
            api_key_env: mac_dinh_api_key_env(),
            model: String::new(),
            max_output_tokens: mac_dinh_max_output(),
            temperature_milli: 0,
            app_url: String::new(),
            app_title: String::new(),
            cognitive_latency_ticks: mac_dinh_do_tre(),
            timeout_ms: mac_dinh_timeout(),
            cassette_dir: mac_dinh_cassette(),
        }
    }
}

/// Chế độ sinh vector.
///
/// Chỉ hai giá trị, và việc **không** có `RECORD`/`REPLAY` ở đây là có chủ ý.
/// `§P6.3` nói chỉ mục ký ức là thứ **dựng lại được**, không phải nguồn sự
/// thật — nên một bộ ghi embedding không mua thêm gì so với [`EmbeddingMode::Stub`],
/// vốn đã xác định tuyệt đối và không cần mạng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum EmbeddingMode {
    /// Băm đặc trưng chạy tại chỗ: xác định, không mạng, không khóa.
    ///
    /// Cho tương đồng **từ vựng**, không phải ngữ nghĩa. Đủ để toàn bộ đường
    /// ống ký ức chạy và replay được trước khi có bất kỳ khóa API nào.
    Stub,
    /// Gọi một máy chủ embedding thật.
    Live,
}

/// Cấu hình sinh vector cho chỉ mục ký ức.
///
/// **Số chiều không nằm ở đây** — nó là [`VectorConfig::dimension`]. Hai chỗ
/// khai cùng một con số là hai chỗ để chúng lệch nhau, và khi chúng lệch thì
/// triệu chứng là một chỉ mục im lặng trả về kết quả sai chứ không phải một lỗi.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingConfig {
    /// Chế độ.
    #[serde(default = "mac_dinh_embed_mode")]
    pub mode: EmbeddingMode,
    /// Nhãn nhà cung cấp, cho log và thông báo lỗi.
    #[serde(default)]
    pub provider: String,
    /// Gốc API, **không** kèm `/embeddings`.
    ///
    /// Máy chủ cục bộ dựng bằng `./mow ai up`: `http://localhost:18080/v1`.
    #[serde(default)]
    pub base_url: String,
    /// **Tên biến môi trường** chứa API key. Xem [`LlmConfig::api_key_env`].
    #[serde(default = "mac_dinh_embed_key_env")]
    pub api_key_env: String,
    /// Tên model như máy chủ công bố.
    #[serde(default)]
    pub model: String,
    /// Số văn bản mỗi lời gọi.
    #[serde(default = "mac_dinh_batch")]
    pub batch_size: usize,
    /// Thời gian chờ mỗi lời gọi, mili giây.
    #[serde(default = "mac_dinh_timeout")]
    pub timeout_ms: u64,
    /// Có gửi trường `dimensions` hay không.
    ///
    /// Bật là đúng với model Matryoshka — `jina-embeddings-v5` cắt được về
    /// 32/64/128/256/512/768/1024 chiều. Tắt khi máy chủ từ chối trường này.
    #[serde(default = "mac_dinh_gui_dim")]
    pub send_dimensions: bool,
    /// Tiền tố cho văn bản đóng vai **truy vấn**.
    ///
    /// Model truy xuất hiện đại được huấn luyện bất đối xứng: câu hỏi mã hóa
    /// một kiểu, đoạn văn mã hóa kiểu khác. Để trống nếu máy chủ tự áp tiền tố
    /// (`jina-embeddings-v5-*-retrieval` đã gộp adapter cho việc đó).
    ///
    /// Dùng nhầm không làm hỏng gì ra mặt — nó chỉ làm chất lượng truy xuất tụt
    /// vài phần trăm, mãi mãi, và không bài test nào đỏ.
    #[serde(default)]
    pub query_prefix: String,
    /// Tiền tố cho văn bản đóng vai **tài liệu**. Xem [`EmbeddingConfig::query_prefix`].
    #[serde(default)]
    pub document_prefix: String,
}

fn mac_dinh_embed_mode() -> EmbeddingMode {
    EmbeddingMode::Stub
}
fn mac_dinh_embed_key_env() -> String {
    "EMBEDDINGS_API_KEY".to_owned()
}
fn mac_dinh_batch() -> usize {
    32
}
fn mac_dinh_gui_dim() -> bool {
    true
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        EmbeddingConfig {
            mode: mac_dinh_embed_mode(),
            provider: String::new(),
            base_url: String::new(),
            api_key_env: mac_dinh_embed_key_env(),
            model: String::new(),
            batch_size: mac_dinh_batch(),
            timeout_ms: mac_dinh_timeout(),
            send_dimensions: mac_dinh_gui_dim(),
            query_prefix: String::new(),
            document_prefix: String::new(),
        }
    }
}

/// Ngân sách nhận thức (`§20.2`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BudgetConfig {
    /// Số lời gọi tối đa mỗi tick trên toàn thế giới.
    #[serde(default = "mac_dinh_calls")]
    pub max_calls_per_tick: u32,
    /// Số lời gọi song song tối đa.
    #[serde(default = "mac_dinh_song_song")]
    pub max_concurrent: u32,
}

fn mac_dinh_calls() -> u32 {
    8
}
fn mac_dinh_song_song() -> u32 {
    4
}

impl Default for BudgetConfig {
    fn default() -> Self {
        BudgetConfig {
            max_calls_per_tick: mac_dinh_calls(),
            max_concurrent: mac_dinh_song_song(),
        }
    }
}

/// Content pack.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentConfig {
    /// Thư mục gốc chứa pack.
    #[serde(default = "mac_dinh_content")]
    pub root: String,
    /// Danh sách pack nạp, **theo thứ tự**. Thứ tự quyết định ghi đè.
    #[serde(default = "mac_dinh_packs")]
    pub packs: Vec<String>,
}

fn mac_dinh_content() -> String {
    "content".to_owned()
}
fn mac_dinh_packs() -> Vec<String> {
    vec!["core".to_owned()]
}

impl Default for ContentConfig {
    fn default() -> Self {
        ContentConfig {
            root: mac_dinh_content(),
            packs: mac_dinh_packs(),
        }
    }
}

/// Định dạng log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Đọc được bằng mắt, cho lúc phát triển.
    Pretty,
    /// JSON, cho lúc thu thập.
    Json,
}

/// Quan sát.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservabilityConfig {
    /// Mức log.
    #[serde(default = "mac_dinh_log_level")]
    pub log_level: String,
    /// Định dạng.
    #[serde(default = "mac_dinh_log_format")]
    pub log_format: LogFormat,
    /// Điểm nhận OTLP. Rỗng là tắt trace.
    #[serde(default)]
    pub otel_endpoint: String,
}

fn mac_dinh_log_level() -> String {
    "info".to_owned()
}
fn mac_dinh_log_format() -> LogFormat {
    LogFormat::Pretty
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        ObservabilityConfig {
            log_level: mac_dinh_log_level(),
            log_format: mac_dinh_log_format(),
            otel_endpoint: String::new(),
        }
    }
}

/// Biến môi trường chứa khóa có tồn tại và khác rỗng không.
///
/// Trả `None` khi ổn, `Some(lỗi)` khi thiếu.
fn thieu_khoa(
    duong_dan: &'static str,
    ten_bien: &str,
    tra_cuu: &impl Fn(&str) -> Option<String>,
) -> Option<(&'static str, String)> {
    if ten_bien.is_empty() {
        return Some((duong_dan, "bắt buộc khi chế độ cần gọi mạng".to_owned()));
    }
    match tra_cuu(ten_bien) {
        Some(v) if !v.trim().is_empty() => None,
        _ => Some((
            duong_dan,
            format!(
                "biến môi trường `{ten_bien}` chưa đặt hoặc rỗng. Chép `.env.example` \
                 thành `.env` rồi điền — bí mật không bao giờ nằm trong `config/*.yaml`"
            ),
        )),
    }
}

/// Những mẫu cho thấy một bí mật đã lọt vào file YAML được commit.
const MAU_BI_MAT: &[&str] = &["sk-", "api_key=", "://", "-----BEGIN"];

impl AppConfig {
    /// Kiểm tra ràng buộc chéo giữa các mục.
    ///
    /// Những ràng buộc ở đây **không** diễn đạt được bằng kiểu, vì chúng liên
    /// quan tới nhiều field cùng lúc. Mỗi cái là một lỗi đã được nghĩ tới.
    pub fn validate(&self) -> ConfigResult<()> {
        self.validate_with_env(|k| std::env::var(k).ok())
    }

    /// Như [`AppConfig::validate`] nhưng tra biến môi trường qua `tra_cuu`.
    ///
    /// Tồn tại vì bước kiểm khóa API **phải** đọc môi trường, còn test thì
    /// không được đụng vào môi trường: `std::env::set_var` là toàn cục cho cả
    /// tiến trình, nên một bài test đặt nó sẽ làm bài test chạy song song bên
    /// cạnh hỏng — và hỏng theo kiểu không lặp lại được, tức là kiểu tệ nhất.
    pub fn validate_with_env(&self, tra_cuu: impl Fn(&str) -> Option<String>) -> ConfigResult<()> {
        let mut loi = Vec::new();

        if self.sim.tick_rate == 0 {
            loi.push(("sim.tick_rate", "phải lớn hơn 0".to_owned()));
        }
        if !self.sim.chunk_size.is_power_of_two() {
            loi.push((
                "sim.chunk_size",
                format!(
                    "phải là lũy thừa của 2, nhận {}. Chỉ số chunk tính bằng `div_euclid`, \
                     và lũy thừa của 2 giữ cho phép đó là một phép dịch bit",
                    self.sim.chunk_size
                ),
            ));
        }
        if self.vector.dimension == 0 {
            loi.push(("vector.dimension", "phải lớn hơn 0".to_owned()));
        }

        // `P0-09`: kho ký ức phải nằm ở file riêng.
        if self.vector.url == self.persistence.url {
            loi.push((
                "vector.url",
                "trùng với `persistence.url`. Kho ký ức phải nằm ở file riêng để không \
                 tranh khóa với tiến trình mô phỏng, và để rebuild chỉ mục không đụng \
                 vào save của người chơi"
                    .to_owned(),
            ));
        }

        // `§20.2.2`: độ trễ nhận thức phải dương ở mọi chế độ gọi thật.
        if self.llm.mode != LlmMode::Stub && self.llm.cognitive_latency_ticks == 0 {
            loi.push((
                "llm.cognitive_latency_ticks",
                "phải lớn hơn 0 khi không ở chế độ STUB. Bằng 0 nghĩa là kết quả LLM \
                 được áp ngay khi về, và thế giới sẽ phụ thuộc vào tốc độ đường truyền \
                 thay vì vào seed"
                    .to_owned(),
            ));
        }

        if self.llm.mode == LlmMode::Live && self.llm.provider.is_empty() {
            loi.push(("llm.provider", "bắt buộc khi `llm.mode` là LIVE".to_owned()));
        }

        // `api_key_env` nhận **tên biến**, không nhận khóa. Dán thẳng khóa vào
        // đây là lỗi dễ mắc nhất, và hậu quả của nó là một bí mật nằm trong
        // một file được commit — thứ không rút lại được bằng một commit sau.
        for (duong_dan, ten) in [
            ("llm.api_key_env", &self.llm.api_key_env),
            ("embedding.api_key_env", &self.embedding.api_key_env),
        ] {
            if ten.is_empty() {
                continue;
            }
            // Tiền tố `MOW_` là **của lớp cấu hình**, không phải chỗ để bí mật.
            // `Env::prefixed("MOW_")` đọc mọi biến `MOW_*` thành một field, nên
            // `MOW_EMBEDDING_API_KEY` không bao giờ tới được chỗ cần tới: nó bị
            // đọc thành field `embedding_api_key`, và `deny_unknown_fields` từ
            // chối khởi động với một thông báo nói về "unknown field" — không
            // nói một chữ nào về khóa API.
            if ten.starts_with("MOW_") {
                loi.push((
                    duong_dan,
                    format!(
                        "`{ten}` bắt đầu bằng `MOW_`, mà tiền tố đó thuộc về lớp cấu hình: \
                         mọi biến `MOW_*` được đọc thành một field của config, nên biến này \
                         sẽ không bao giờ tới được chỗ cần tới. Đặt tên khác, ví dụ \
                         `EMBEDDINGS_API_KEY`"
                    ),
                ));
            }
            if !ten
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                loi.push((
                    duong_dan,
                    format!(
                        "`{ten}` không phải một tên biến môi trường. Trường này nhận TÊN \
                         (ví dụ `OPENROUTER_API_KEY`), không nhận giá trị khóa — \
                         `config/*.yaml` được commit, nên một khóa đặt ở đây là một khóa \
                         đã lộ"
                    ),
                ));
            }
        }

        // Chế độ cần mạng: kiểm **đủ mọi mảnh, lúc khởi động**. Thiếu một mảnh
        // mà vẫn chạy tiếp nghĩa là lỗi nổ ở lần suy nghĩ đầu tiên của một NPC
        // nào đó, giữa chừng một thế giới đang chạy, và biểu hiện của nó là
        // "NPC bỗng dưng ngờ nghệch" chứ không phải một thông báo cấu hình.
        if matches!(self.llm.mode, LlmMode::Live | LlmMode::Record) {
            if self.llm.base_url.is_empty() {
                loi.push((
                    "llm.base_url",
                    "bắt buộc khi `llm.mode` là LIVE hoặc RECORD. OpenRouter: \
                     `https://openrouter.ai/api/v1`"
                        .to_owned(),
                ));
            }
            if self.llm.model.is_empty() {
                loi.push((
                    "llm.model",
                    "bắt buộc khi `llm.mode` là LIVE hoặc RECORD".to_owned(),
                ));
            }
            if self.llm.max_output_tokens == 0 {
                loi.push((
                    "llm.max_output_tokens",
                    "bằng 0 nghĩa là mô hình bị cắt trước khi nói được chữ nào, và nó \
                     trả về chuỗi rỗng chứ không trả về lỗi"
                        .to_owned(),
                ));
            }
            if let Some(e) = thieu_khoa("llm.api_key_env", &self.llm.api_key_env, &tra_cuu) {
                loi.push(e);
            }
        }

        if self.embedding.mode == EmbeddingMode::Live {
            if self.embedding.base_url.is_empty() {
                loi.push((
                    "embedding.base_url",
                    "bắt buộc khi `embedding.mode` là LIVE. Máy chủ cục bộ dựng bằng \
                     `./mow ai up`: `http://localhost:18080/v1`"
                        .to_owned(),
                ));
            }
            if self.embedding.model.is_empty() {
                loi.push((
                    "embedding.model",
                    "bắt buộc khi `embedding.mode` là LIVE".to_owned(),
                ));
            }
            if let Some(e) = thieu_khoa(
                "embedding.api_key_env",
                &self.embedding.api_key_env,
                &tra_cuu,
            ) {
                loi.push(e);
            }
        }

        if self.embedding.batch_size == 0 {
            loi.push((
                "embedding.batch_size",
                "phải lớn hơn 0; bằng 0 thì không văn bản nào được gửi đi và chỉ mục \
                 lặng lẽ rỗng"
                    .to_owned(),
            ));
        }

        if self.content.packs.is_empty() {
            loi.push((
                "content.packs",
                "phải nạp ít nhất một pack; `core` là pack chính thức".to_owned(),
            ));
        }

        // `§P10.6`: bí mật chỉ ở `.env`, không bao giờ trong YAML được commit.
        for (duong_dan, gia_tri) in [
            ("persistence.url", &self.persistence.url),
            ("vector.url", &self.vector.url),
            (
                "observability.otel_endpoint",
                &self.observability.otel_endpoint,
            ),
            ("llm.base_url", &self.llm.base_url),
            ("embedding.base_url", &self.embedding.base_url),
        ] {
            if let Some(mau) = MAU_BI_MAT.iter().find(|m| gia_tri.contains(**m)) {
                // `://` trong một endpoint là bình thường; chỉ báo khi có dấu
                // hiệu của thông tin đăng nhập nhúng trong URL.
                if *mau == "://" && !gia_tri.contains('@') {
                    continue;
                }
                loi.push((
                    duong_dan,
                    format!(
                        "chứa `{mau}` — trông như bí mật. File `config/*.yaml` được commit; \
                         đưa giá trị này vào `.env` và tham chiếu qua biến môi trường MOW_*"
                    ),
                ));
            }
        }

        if loi.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Invalid(loi))
        }
    }

    /// Sinh JSON Schema cho `schemas/config/app_config.v1.json`.
    ///
    /// Gắn `$id` **có version** (`PF-13`). Modder cần một định danh ổn định để
    /// trỏ tới, và version nằm trong định danh chứ không nằm cạnh nó: một
    /// schema đổi hình dạng mà giữ nguyên `$id` sẽ làm mọi công cụ đã tải bản
    /// cũ diễn giải sai bản mới, và không có gì báo.
    ///
    /// Cùng quy tắc với id nội dung ở `§19.7.2`: muốn đổi thì **thêm cái mới**,
    /// không sửa cái đã phát hành.
    pub fn json_schema_string() -> String {
        let mut schema = serde_json::to_value(schemars::schema_for!(AppConfig))
            .expect("schema tuần tự hóa được");
        if let Some(o) = schema.as_object_mut() {
            // Chèn `$id` ngay sau `$schema` — thứ tự khóa trong JSON không có
            // nghĩa với máy, nhưng file này người đọc, và hai dòng định danh
            // đứng cạnh nhau thì đọc được.
            o.insert(
                "$id".to_owned(),
                serde_json::Value::String(SCHEMA_ID.to_owned()),
            );
        }
        serde_json::to_string_pretty(&schema).expect("schema tuần tự hóa được")
    }
}

/// Định danh có version của schema cấu hình (`PF-13`).
///
/// Đổi con số cuối là **phát hành một schema mới**, không phải sửa cái cũ. Một
/// công cụ của bên thứ ba trỏ vào `v1` phải còn diễn giải được `v1` mãi mãi.
pub const SCHEMA_ID: &str = "https://myopenworld.dev/schemas/config/app_config.v1.json";
