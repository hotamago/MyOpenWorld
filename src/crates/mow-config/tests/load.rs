use mow_config::schema::RouteConfig;
use mow_config::{load, AppConfig, EmbeddingMode, LlmMode};
use std::fs;

fn viet(dir: &std::path::Path, ten: &str, noi_dung: &str) {
    fs::write(dir.join(ten), noi_dung).unwrap();
}

#[test]
fn nap_config_that_cua_du_an() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config");
    let c = load(&root, "dev").expect("config that cua du an phai hop le");
    assert_eq!(c.env, "dev");
    assert_eq!(c.llm.mode, LlmMode::Stub);
    // dev.yaml ghi de log_level cua base.yaml
    assert_eq!(c.observability.log_level, "debug");
    // nhung khong dung toi sim.tick_rate, nen gia tri cua base.yaml con nguyen
    assert_eq!(c.sim.tick_rate, 20);
}

#[test]
fn moi_truong_test_cung_hop_le() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config");
    let c = load(&root, "test").expect("test.yaml phai hop le");
    assert_eq!(c.env, "test");
    assert_eq!(c.sim.snapshot_interval, 0);
}

#[test]
fn thieu_base_thi_bao_ro() {
    let d = tempfile::tempdir().unwrap();
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("base.yaml"), "{e}");
}

#[test]
fn env_khong_ton_tai_thi_van_chay_bang_base() {
    let d = tempfile::tempdir().unwrap();
    viet(d.path(), "base.yaml", "env: dev\n");
    let c = load(d.path(), "khong-co-file-nay").expect("file theo env la tuy chon");
    assert_eq!(c.sim.tick_rate, 20);
}

#[test]
fn field_la_thi_bao_loi_chu_khong_bo_qua() {
    // `deny_unknown_fields`: mot loi chinh ta trong ten field phai la loi, khong
    // duoc tro thanh mot field moi im lang.
    let d = tempfile::tempdir().unwrap();
    viet(d.path(), "base.yaml", "env: dev\nsim:\n  tick_rat: 20\n");
    assert!(load(d.path(), "dev").is_err(), "loi chinh ta phai bi bat");
}

#[test]
fn chunk_size_khong_phai_luy_thua_2_thi_tu_choi() {
    let d = tempfile::tempdir().unwrap();
    viet(d.path(), "base.yaml", "env: dev\nsim:\n  chunk_size: 30\n");
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("sim.chunk_size"), "{e}");
}

#[test]
fn kho_ky_uc_trung_file_save_thi_tu_choi() {
    // P0-09: hai thu nay phai o hai file khac nhau.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\npersistence:\n  url: \"a.sqlite\"\nvector:\n  url: \"a.sqlite\"\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("vector.url"), "{e}");
}

#[test]
fn do_tre_nhan_thuc_bang_0_o_che_do_that_thi_tu_choi() {
    // §20.2.2: bang 0 nghia la the gioi phu thuoc toc do duong truyen.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\nllm:\n  mode: LIVE\n  provider: anthropic\n  cognitive_latency_ticks: 0\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("cognitive_latency_ticks"), "{e}");
}

#[test]
fn bi_mat_lot_vao_yaml_thi_tu_choi_khoi_dong() {
    // §P10.6: file config duoc commit, nen DSN co mat khau khong duoc o day.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\npersistence:\n  url: \"postgres://mow:matkhauthat@db:5432/mow\"\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("persistence.url"), "{e}");
    assert!(
        e.to_string().contains(".env"),
        "loi phai chi ra cho dat dung: {e}"
    );
}

#[test]
fn endpoint_khong_co_thong_tin_dang_nhap_thi_khong_bi_bao_nham() {
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\nobservability:\n  otel_endpoint: \"http://jaeger:4317\"\n",
    );
    load(d.path(), "dev").expect("endpoint binh thuong khong phai bi mat");
}

#[test]
fn bao_het_moi_loi_mot_lan() {
    // Sua tung loi mot roi khoi dong lai la vong lap cham.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\nsim:\n  tick_rate: 0\n  chunk_size: 30\nvector:\n  dimension: 0\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    let s = e.to_string();
    assert!(s.contains("sim.tick_rate"), "{s}");
    assert!(s.contains("sim.chunk_size"), "{s}");
    assert!(s.contains("vector.dimension"), "{s}");
}

#[test]
fn sinh_duoc_json_schema() {
    let s = AppConfig::json_schema_string();
    assert!(s.contains("cognitive_latency_ticks"), "{s}");
    assert!(s.contains("AppConfig"));
}

// ── Cấu hình mô hình thật (`§P10.6`) ─────────────────────────────────────────
//
// Mọi bài dưới đây dùng `validate_with_env` chứ không đụng vào biến môi trường
// thật: `set_var` là toàn cục cho cả tiến trình, và một bài test đặt nó sẽ làm
// bài chạy song song bên cạnh hỏng theo kiểu không lặp lại được.

fn khong_co_bien(_: &str) -> Option<String> {
    None
}

fn co_khoa(k: &str) -> Option<String> {
    if k == "OPENROUTER_API_KEY" {
        Some("sk-or-v1-gia".to_owned())
    } else if k == "EMBEDDINGS_API_KEY" {
        Some("khong-can".to_owned())
    } else {
        None
    }
}

fn cfg_live() -> AppConfig {
    let mut c = AppConfig {
        env: "live".to_owned(),
        ..AppConfig::default()
    };
    c.llm.mode = LlmMode::Live;
    c.llm.provider = "openrouter".to_owned();
    c.llm.base_url = "https://openrouter.ai/api/v1".to_owned();
    c.llm.model = "deepseek/deepseek-v4-flash-0731".to_owned();
    c
}

#[test]
fn live_du_moi_manh_va_co_khoa_thi_hop_le() {
    cfg_live()
        .validate_with_env(co_khoa)
        .expect("đủ mảnh thì phải qua");
}

#[test]
fn live_thieu_khoa_thi_tu_choi_ngay_luc_khoi_dong() {
    // Đây là lý do tồn tại của cả bước kiểm này: thiếu khóa mà vẫn chạy tiếp
    // nghĩa là lỗi nổ ở lần suy nghĩ đầu tiên của một NPC, giữa một thế giới
    // đang chạy — và triệu chứng là "NPC bỗng ngờ nghệch", không phải một
    // thông báo cấu hình.
    let e = cfg_live()
        .validate_with_env(khong_co_bien)
        .expect_err("thiếu khóa phải là lỗi");
    let s = e.to_string();
    assert!(s.contains("OPENROUTER_API_KEY"), "{s}");
    assert!(s.contains(".env"), "{s}");
}

#[test]
fn khoa_rong_tinh_la_thieu() {
    let e = cfg_live()
        .validate_with_env(|_| Some("   ".to_owned()))
        .expect_err("khóa toàn khoảng trắng vẫn là thiếu");
    assert!(e.to_string().contains("rỗng"), "{e}");
}

#[test]
fn dan_khoa_vao_api_key_env_thi_bi_bat() {
    // Lỗi dễ mắc nhất, và hậu quả là một bí mật nằm trong một file được commit
    // — thứ không rút lại được bằng một commit sau.
    let mut c = cfg_live();
    c.llm.api_key_env = "sk-or-v1-882569e1380b97ae".to_owned();
    let e = c
        .validate_with_env(co_khoa)
        .expect_err("giá trị khóa không phải tên biến");
    let s = e.to_string();
    assert!(s.contains("llm.api_key_env"), "{s}");
    assert!(s.contains("TÊN"), "{s}");
}

#[test]
fn ten_bien_bat_dau_bang_mow_bi_tu_choi() {
    // Lỗi này đã xảy ra thật trong lúc dựng cấu hình: đặt khóa embedding vào
    // `MOW_EMBEDDING_API_KEY` cho "nhất quán", và cả tiến trình chết với
    // `unknown field: embedding_api_key` — một thông báo không nhắc gì tới khóa.
    let mut c = cfg_live();
    c.embedding.api_key_env = "MOW_EMBEDDING_API_KEY".to_owned();
    let e = c
        .validate_with_env(co_khoa)
        .expect_err("MOW_ là tiền tố của config");
    let s = e.to_string();
    assert!(s.contains("MOW_"), "{s}");
    assert!(
        s.contains("EMBEDDINGS_API_KEY"),
        "thông báo phải gợi ý tên thay thế: {s}"
    );
}

#[test]
fn ten_bien_chu_thuong_cung_bi_tu_choi() {
    let mut c = cfg_live();
    c.llm.api_key_env = "openrouter_api_key".to_owned();
    assert!(c.validate_with_env(co_khoa).is_err());
}

#[test]
fn live_thieu_base_url_thi_bao_ro() {
    let mut c = cfg_live();
    c.llm.base_url = String::new();
    let e = c.validate_with_env(co_khoa).expect_err("thiếu base_url");
    assert!(e.to_string().contains("llm.base_url"), "{e}");
}

#[test]
fn live_thieu_model_thi_bao_ro() {
    let mut c = cfg_live();
    c.llm.model = String::new();
    let e = c.validate_with_env(co_khoa).expect_err("thiếu model");
    assert!(e.to_string().contains("llm.model"), "{e}");
}

#[test]
fn max_output_tokens_bang_0_bi_tu_choi() {
    let mut c = cfg_live();
    c.llm.max_output_tokens = 0;
    let e = c.validate_with_env(co_khoa).expect_err("0 token đầu ra");
    assert!(e.to_string().contains("max_output_tokens"), "{e}");
}

#[test]
fn record_cung_can_khoa_nhu_live() {
    // `RECORD` gọi thật rồi ghi lại. Bỏ sót nó ở bước kiểm nghĩa là bộ ghi
    // được dựng bằng một loạt lỗi mạng, và không ai biết cho tới lúc `REPLAY`.
    let mut c = cfg_live();
    c.llm.mode = LlmMode::Record;
    assert!(c.validate_with_env(khong_co_bien).is_err());
}

#[test]
fn replay_khong_can_khoa() {
    // Cả điểm của REPLAY là chạy được khi không có mạng và không có khóa.
    let mut c = cfg_live();
    c.llm.mode = LlmMode::Replay;
    c.validate_with_env(khong_co_bien)
        .expect("REPLAY không được đòi khóa");
}

#[test]
fn stub_khong_can_gi_ca() {
    AppConfig::default()
        .validate_with_env(khong_co_bien)
        .expect("STUB phải chạy được trên một máy trắng");
}

// ── Embedding ────────────────────────────────────────────────────────────────

#[test]
fn embedding_live_thieu_base_url_thi_bao_ro() {
    let mut c = AppConfig::default();
    c.embedding.mode = EmbeddingMode::Live;
    c.embedding.model = "jina-embeddings-v5-text-small".to_owned();
    let e = c.validate_with_env(co_khoa).expect_err("thiếu base_url");
    assert!(e.to_string().contains("embedding.base_url"), "{e}");
    // Thông báo phải nói làm gì tiếp, không chỉ nói cái gì sai.
    assert!(e.to_string().contains("./mow ai up"), "{e}");
}

#[test]
fn embedding_batch_bang_0_thi_tu_choi() {
    // Một `batch_size` bằng 0 không panic và không báo lỗi — nó chỉ làm mọi lô
    // rỗng, và chỉ mục lặng lẽ không có gì trong đó.
    let mut c = AppConfig::default();
    c.embedding.batch_size = 0;
    let e = c.validate_with_env(co_khoa).expect_err("batch 0");
    assert!(e.to_string().contains("embedding.batch_size"), "{e}");
}

#[test]
fn so_chieu_chi_khai_o_mot_cho() {
    // `embedding` **không** có trường `dimension`: hai chỗ khai cùng một con số
    // là hai chỗ để chúng lệch nhau. Bài này giữ lời hứa đó bằng cách bắt
    // `deny_unknown_fields` từ chối nó.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev
embedding:
  dimension: 512
",
    );
    let e = load(d.path(), "dev").expect_err("embedding.dimension không tồn tại");
    assert!(e.to_string().contains("dimension"), "{e}");
}

// ── Định tuyến model theo vai (`§20.7`) ──────────────────────────────────────

#[test]
fn vai_de_trong_thi_ke_thua_mac_dinh() {
    // Cả điểm của `routes`: khai đúng thứ đi chệch, im lặng về phần còn lại.
    // Một route phải khai lại mọi trường là một route sẽ trôi khỏi `llm.*` mà
    // không ai để ý, vì không có gì nói rằng hai chỗ đó lẽ ra phải giống nhau.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev
llm:
  provider: openrouter
  base_url: \"https://openrouter.ai/api/v1\"
  api_key_env: OPENROUTER_API_KEY
  model: \"model-mac-dinh\"
  max_output_tokens: 2048
  temperature_milli: 300
  timeout_ms: 30000
  routes:
    npc:
      model: \"model-cua-npc\"
",
    );
    let c = load(d.path(), "dev").expect("hợp lệ");
    let r = c.llm.route("npc");
    assert_eq!(r.role, "npc");
    // Khai thì đổi.
    assert_eq!(r.model, "model-cua-npc");
    // Không khai thì kế thừa, từng trường một.
    assert_eq!(r.provider, "openrouter");
    assert_eq!(r.base_url, "https://openrouter.ai/api/v1");
    assert_eq!(r.api_key_env, "OPENROUTER_API_KEY");
    assert_eq!(r.max_output_tokens, 2048);
    assert_eq!(r.temperature_milli, 300);
    assert_eq!(r.timeout_ms, 30_000);
}

#[test]
fn vai_khai_day_du_thi_ghi_de_het() {
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev
llm:
  provider: openrouter
  base_url: \"https://openrouter.ai/api/v1\"
  api_key_env: OPENROUTER_API_KEY
  model: \"model-mac-dinh\"
  max_output_tokens: 2048
  temperature_milli: 300
  timeout_ms: 30000
  routes:
    action:
      provider: local
      base_url: \"http://localhost:18081/v1\"
      api_key_env: LOCAL_LLM_API_KEY
      model: \"model-cuc-bo\"
      max_output_tokens: 256
      temperature_milli: 700
      timeout_ms: 120000
",
    );
    let c = load(d.path(), "dev").expect("hợp lệ");
    let r = c.llm.route("action");
    assert_eq!(r.provider, "local");
    assert_eq!(r.base_url, "http://localhost:18081/v1");
    assert_eq!(r.api_key_env, "LOCAL_LLM_API_KEY");
    assert_eq!(r.model, "model-cuc-bo");
    assert_eq!(r.max_output_tokens, 256);
    assert_eq!(r.temperature_milli, 700);
    assert_eq!(r.timeout_ms, 120_000);
    // Và mặc định chung không bị route nào sửa: `route()` hợp nhất chứ không
    // ghi vào `llm.*`.
    assert_eq!(c.llm.model, "model-mac-dinh");
}

#[test]
fn vai_khong_ton_tai_thi_chay_bang_mac_dinh_chu_khong_no() {
    // Định tuyến là chuyện tối ưu chi phí, không phải chuyện đúng sai của mô
    // phỏng. Một vai gõ sai phải cho ra một lời gọi bình thường — không phải
    // một panic giữa lượt của một thế giới đang chạy.
    let c = cfg_live();
    let r = c.llm.route("vai-khong-ai-khai");
    assert_eq!(r.model, c.llm.model);
    assert_eq!(r.base_url, c.llm.base_url);
    assert_eq!(r.api_key_env, c.llm.api_key_env);
    assert_eq!(r.max_output_tokens, c.llm.max_output_tokens);
    assert_eq!(r.timeout_ms, c.llm.timeout_ms);
    // Không khai route nào cả cũng vậy.
    assert!(c.llm.routes.is_empty());
}

#[test]
fn api_key_env_cua_vai_bat_dau_bang_mow_bi_tu_choi() {
    // Cùng bộ luật với `llm.api_key_env`. Đây là chỗ dễ lọt nhất vì các vai
    // được thêm về sau, khi luật kia đã thành thứ hiển nhiên không ai đọc lại.
    let mut c = cfg_live();
    c.llm.routes.insert(
        "action".to_owned(),
        RouteConfig {
            api_key_env: "MOW_LOCAL_LLM_API_KEY".to_owned(),
            ..RouteConfig::default()
        },
    );
    let e = c
        .validate_with_env(co_khoa)
        .expect_err("`MOW_` là tiền tố của lớp cấu hình");
    let s = e.to_string();
    assert!(s.contains("llm.routes"), "{s}");
    assert!(s.contains("MOW_"), "{s}");
    // Đường dẫn field của một route không phải hằng, nên thông báo phải tự nói
    // vai nào sai — nếu không thì với năm vai, người đọc phải đoán.
    assert!(s.contains("action"), "phải chỉ ra vai nào sai: {s}");
}

#[test]
fn api_key_env_cua_vai_khong_phai_ten_bien_thi_bi_tu_choi() {
    let mut c = cfg_live();
    c.llm.routes.insert(
        "action".to_owned(),
        RouteConfig {
            api_key_env: "sk-or-v1-882569e1380b97ae".to_owned(),
            ..RouteConfig::default()
        },
    );
    let e = c.validate_with_env(co_khoa).expect_err("khóa không phải tên");
    assert!(e.to_string().contains("TÊN"), "{e}");
}

#[test]
fn vai_khai_khoa_rieng_ma_thieu_bien_thi_tu_choi_luc_khoi_dong() {
    // Một vai trỏ sang endpoint khác cần khóa của endpoint đó. Thiếu nó mà vẫn
    // chạy nghĩa là mọi hành động của mọi thực thể hỏng ở lần gọi đầu tiên.
    let mut c = cfg_live();
    c.llm.routes.insert(
        "action".to_owned(),
        RouteConfig {
            base_url: "http://localhost:18081/v1".to_owned(),
            api_key_env: "LOCAL_LLM_API_KEY".to_owned(),
            model: "model-cuc-bo".to_owned(),
            ..RouteConfig::default()
        },
    );
    // `co_khoa` chỉ biết OPENROUTER_API_KEY và EMBEDDINGS_API_KEY.
    let e = c
        .validate_with_env(co_khoa)
        .expect_err("thiếu khóa của vai phải là lỗi");
    let s = e.to_string();
    assert!(s.contains("LOCAL_LLM_API_KEY"), "{s}");
    assert!(s.contains("action"), "{s}");

    // Và có khóa thì qua.
    c.validate_with_env(|k| {
        if k == "LOCAL_LLM_API_KEY" {
            Some("khong-can".to_owned())
        } else {
            co_khoa(k)
        }
    })
    .expect("đủ khóa thì phải qua");
}

#[test]
fn vai_ke_thua_khoa_thi_khong_bi_hoi_hai_lan() {
    // Vai để trống `api_key_env` dùng lại `llm.api_key_env` — đã kiểm ở nhánh
    // chung. Kiểm lại chỉ nhân đôi cùng một dòng lỗi, và một thông báo lặp làm
    // người đọc tưởng có hai chỗ hỏng.
    let mut c = cfg_live();
    c.llm.routes.insert(
        "npc".to_owned(),
        RouteConfig {
            model: "model-cua-npc".to_owned(),
            ..RouteConfig::default()
        },
    );
    let e = c
        .validate_with_env(khong_co_bien)
        .expect_err("thiếu khóa mặc định");
    let s = e.to_string();
    assert_eq!(
        s.matches("OPENROUTER_API_KEY").count(),
        1,
        "một khóa thiếu chỉ được báo một lần: {s}"
    );
}

#[test]
fn field_la_trong_route_thi_bao_loi() {
    // `deny_unknown_fields` phải áp cả trong `routes`: một `max_tokens` gõ
    // thiếu chữ sẽ im lặng trở thành "không khai" và vai đó lặng lẽ chạy bằng
    // trần token của `llm.*`.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev
llm:
  routes:
    action:
      max_tokens: 256
",
    );
    let e = load(d.path(), "dev").expect_err("`max_tokens` không phải field của route");
    assert!(e.to_string().contains("max_tokens"), "{e}");
}

// ── File cấu hình thật của dự án ─────────────────────────────────────────────

#[test]
fn base_yaml_that_khai_dung_model_da_chon() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config");
    let c = load(&root, "dev").expect("config thật phải hợp lệ");
    assert_eq!(c.llm.provider, "openrouter");
    assert_eq!(c.llm.model, "deepseek/deepseek-v4-flash-0731");
    assert_eq!(c.llm.api_key_env, "OPENROUTER_API_KEY");
    // Số chiều phải khớp model embedding đã chọn (Qwen3-0.6B → 1024).
    assert_eq!(c.vector.dimension, 1024);
    // Và `dev` vẫn phải là chế độ không mạng.
    assert_eq!(c.llm.mode, LlmMode::Stub);
    assert_eq!(c.embedding.mode, EmbeddingMode::Stub);
}

#[test]
fn base_yaml_that_khai_du_ba_vai() {
    // `§20.7`: ba vai khác nhau về số lần gọi tới vài bậc độ lớn. Bài này khóa
    // chính con số đó lại — một lần "gộp cho gọn" ba vai về một model là một
    // thay đổi phải cố ý sửa test, không được là một dòng bị xóa nhầm.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config");
    let c = load(&root, "dev").expect("config thật phải hợp lệ");
    assert_eq!(c.llm.routes.len(), 3, "phải khai đúng ba vai");

    // `action` chạy cục bộ: nó bị gọi nhiều nhất và làm việc đơn giản nhất.
    let action = c.llm.route("action");
    assert_eq!(action.model, "Qwen/Qwen3.5-4B");
    assert_eq!(action.base_url, "http://localhost:18081/v1");
    assert_eq!(action.api_key_env, "LOCAL_LLM_API_KEY");
    assert_eq!(action.max_output_tokens, 256);

    // `npc` chỉ đổi model; endpoint và khóa kế thừa `llm.*`.
    let npc = c.llm.route("npc");
    assert_eq!(npc.model, "deepseek/deepseek-v4-flash-0731");
    assert_eq!(npc.base_url, c.llm.base_url);
    assert_eq!(npc.api_key_env, "OPENROUTER_API_KEY");

    // `yuu` hiếm nhưng mỗi lần gọi là một quyết định ảnh hưởng cả thế giới.
    let yuu = c.llm.route("yuu");
    assert_eq!(yuu.model, "deepseek/deepseek-v4-pro-0813");
    assert_eq!(yuu.max_output_tokens, 4096);
    assert_eq!(yuu.base_url, c.llm.base_url);

    // Ba vai, ba model — gộp hai vai vào một model là mất chính thứ mục này
    // tồn tại để mua.
    assert_ne!(action.model, npc.model);
    assert_ne!(npc.model, yuu.model);
}

#[test]
fn live_yaml_bat_ca_hai_che_do() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config");
    // Nạp thô, không validate: bài này hỏi về nội dung file, không hỏi về máy
    // đang chạy có khóa hay không.
    let text = std::fs::read_to_string(root.join("live.yaml")).unwrap();
    assert!(text.contains("mode: LIVE"), "{text}");
}

// ── Tiền tố `MOW_` thuộc về lớp cấu hình ─────────────────────────────────────

#[test]
fn bien_ha_tang_khong_duoc_mang_tien_to_mow() {
    // `Env::prefixed("MOW_")` đọc MỌI biến `MOW_*` thành một field, và
    // `deny_unknown_fields` biến mỗi biến không khớp thành lỗi khởi động. Nên
    // một biến `MOW_POSTGRES_URL` không chỉ "không tới được chỗ cần tới" — nó
    // làm **toàn bộ** việc nạp config chết, ở mọi tiến trình thấy biến đó.
    //
    // Lỗi này đã xảy ra thật: `docker-compose.yml` đặt `MOW_DATABASE__URL`,
    // `MOW_BUS__URL` và `MOW_OTEL__ENDPOINT` cho container `toolbox`, và 11 bài
    // test của crate này đỏ khi chạy `./mow test` trong khi xanh trên máy thật.
    //
    // Bài này giữ lời hứa đó bằng cách kiểm chính chỗ dễ quên: tên biến mà tài
    // liệu bảo người ta đặt.
    for ten in [
        "MOWTEST_POSTGRES_URL",
        "MOWTEST_NATS_URL",
        "MOWTEST_QDRANT_URL",
    ] {
        assert!(
            !ten.starts_with("MOW_"),
            "`{ten}` bắt đầu bằng `MOW_`; figment sẽ đọc nó thành một field và              `deny_unknown_fields` sẽ làm mọi lần nạp config thất bại"
        );
    }
}

#[test]
fn muc_cap_cao_khong_ton_tai_thi_bao_loi() {
    // Vế còn lại của cùng một luật: một khóa không khớp field nào phải đỏ ngay,
    // không được bỏ qua im lặng. Không có vế này thì lỗi ở trên sẽ không bao
    // giờ được phát hiện — nó sẽ chỉ là một giá trị bị lờ đi.
    //
    // `MOW_DATABASE__URL` biến thành đúng cấu trúc dưới đây sau khi figment
    // tách `__`.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev
database:
  url: x
",
    );
    let e = load(d.path(), "dev").expect_err("`database` không phải field của AppConfig");
    assert!(e.to_string().contains("database"), "{e}");
}

#[test]
fn env_la_ten_moi_truong_da_nap_chu_khong_phai_gia_tri_trong_file() {
    // File khai một tên khác hẳn; `load` vẫn phải báo cáo tên đã nạp. Không có
    // luật này thì `MOW_ENV` vừa chọn file vừa ghi đè field, và một config đã
    // nạp `test.yaml` có thể tự khai là `dev`.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: noi-lao
",
    );
    viet(
        d.path(),
        "canhan.yaml",
        "env: cung-noi-lao
",
    );
    let c = load(d.path(), "canhan").expect("hợp lệ");
    assert_eq!(c.env, "canhan");
}
