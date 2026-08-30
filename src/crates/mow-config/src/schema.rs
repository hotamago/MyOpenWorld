//! Cấu trúc config. **Đây là nguồn**; JSON Schema sinh ra từ đây.

use crate::error::{ConfigError, ConfigResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Toàn bộ cấu hình ứng dụng.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// Tên môi trường, để đối chiếu với file đã nạp.
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
    /// Đường dẫn file save (SQLite) hoặc DSN (Postgres, từ Giai đoạn C).
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
    /// Nhà cung cấp, ví dụ `anthropic`, `openai`.
    #[serde(default)]
    pub provider: String,
    /// Mô hình mặc định.
    #[serde(default)]
    pub model: String,
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
            model: String::new(),
            cognitive_latency_ticks: mac_dinh_do_tre(),
            timeout_ms: mac_dinh_timeout(),
            cassette_dir: mac_dinh_cassette(),
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

/// Những mẫu cho thấy một bí mật đã lọt vào file YAML được commit.
const MAU_BI_MAT: &[&str] = &["sk-", "api_key=", "://", "-----BEGIN"];

impl AppConfig {
    /// Kiểm tra ràng buộc chéo giữa các mục.
    ///
    /// Những ràng buộc ở đây **không** diễn đạt được bằng kiểu, vì chúng liên
    /// quan tới nhiều field cùng lúc. Mỗi cái là một lỗi đã được nghĩ tới.
    pub fn validate(&self) -> ConfigResult<()> {
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
    pub fn json_schema_string() -> String {
        let schema = schemars::schema_for!(AppConfig);
        serde_json::to_string_pretty(&schema).expect("schema tuần tự hóa được")
    }
}
