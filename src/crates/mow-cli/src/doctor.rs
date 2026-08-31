//! `config check`, `llm ping`, `embed probe` — ba lệnh trả lời câu hỏi
//! *"cấu hình của tôi đã dùng được chưa"* ở ba mức khác nhau.
//!
//! | Lệnh | Trả lời | Có chạm mạng |
//! |---|---|---|
//! | `config check` | file có hợp lệ, khóa có mặt chưa | không |
//! | `llm ping` | khóa có **dùng được**, model có tồn tại | có |
//! | `embed probe` | máy chủ embedding có đúng số chiều | có |
//!
//! Ba mức là cố ý. `config check` chạy được ở mọi nơi kể cả CI không mạng, và
//! nó bắt phần lớn lỗi — thiếu biến, dán khóa vào YAML, sai tên field. Hai lệnh
//! sau trả lời câu mà chỉ mạng mới trả lời được, và chúng **phải được gọi
//! tường minh**: một lệnh kiểm cấu hình mà lặng lẽ tiêu token là một lệnh
//! người ta chỉ dám chạy một lần.
//!
//! ## Không bao giờ in khóa
//!
//! `config check` in **tên biến** và một chữ `có`/`thiếu`. Không in giá trị,
//! không in bốn ký tự cuối, không in độ dài. Đầu ra của lệnh này là thứ người
//! ta dán vào issue khi hỏi, và mọi thứ in ra đây sẽ có ngày nằm trong một
//! issue công khai.

use mow_config::{AppConfig, EmbeddingMode, LlmMode};
use mow_llm::{
    EmbedRole, Embedder, HttpEmbedder, ModelClient, OpenAiCompatClient, Request, UreqTransport,
};
use std::path::PathBuf;
use std::process::ExitCode;

/// Thư mục `config/` mặc định, tìm ngược lên từ thư mục hiện tại.
///
/// Chạy `mow-cli` từ `src/` hay từ gốc repo đều phải ra cùng một kết quả —
/// bắt người dùng nhớ mình đang đứng ở đâu là một loại thuế không cần thiết.
fn tim_config_root(chi_dinh: Option<&str>) -> PathBuf {
    if let Some(p) = chi_dinh {
        return PathBuf::from(p);
    }
    let mut d = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..5 {
        for ung_vien in [d.join("config"), d.join("src/config")] {
            if ung_vien.join("base.yaml").exists() {
                return ung_vien;
            }
        }
        if !d.pop() {
            break;
        }
    }
    PathBuf::from("config")
}

fn doc_co(args: &[&str], ten: &str) -> Option<String> {
    args.iter()
        .position(|a| *a == ten)
        .and_then(|i| args.get(i + 1))
        .map(|s| (*s).to_owned())
}

struct DaNap {
    cfg: AppConfig,
    root: PathBuf,
    env: String,
    dotenv: mow_config::dotenv::KetQua,
}

/// Nạp `.env` rồi nạp config. Trả về lỗi đã in sẵn.
fn nap(args: &[&str]) -> Result<DaNap, ExitCode> {
    let root = tim_config_root(doc_co(args, "--root").as_deref());
    let env = doc_co(args, "--env")
        .unwrap_or_else(|| std::env::var("MOW_ENV").unwrap_or_else(|_| "dev".to_owned()));

    // `.env` trước, vì lớp biến môi trường nằm sau YAML và bước kiểm khóa đọc
    // môi trường.
    let dotenv = match mow_config::dotenv::nap_canh_config(&root) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("không đọc được `.env`: {e}");
            return Err(ExitCode::from(78));
        }
    };

    match mow_config::load(&root, &env) {
        Ok(cfg) => Ok(DaNap {
            cfg,
            root,
            env,
            dotenv,
        }),
        Err(e) => {
            eprintln!(
                "cấu hình không hợp lệ ({}, env `{env}`):\n{e}",
                root.display()
            );
            Err(ExitCode::from(78))
        }
    }
}

/// `config check` — nạp, kiểm, in tóm tắt đã che bí mật.
pub fn config_check(args: &[&str]) -> ExitCode {
    let d = match nap(args) {
        Ok(d) => d,
        Err(c) => return c,
    };
    let c = &d.cfg;

    println!("config   {} (env `{}`)", d.root.display(), d.env);
    if d.dotenv.co_thay_doi() {
        println!("  .env   đã nạp {} biến", d.dotenv.da_dat.len());
    } else {
        println!("  .env   không có biến nào được nạp thêm");
    }
    for dong in &d.dotenv.dong_hong {
        println!("  .env   ! dòng {dong} không phân tích được, đã bỏ qua");
    }

    println!("\nllm");
    println!("  mode           {:?}", c.llm.mode);
    println!("  provider       {}", hoac_trong(&c.llm.provider));
    println!("  base_url       {}", hoac_trong(&c.llm.base_url));
    println!("  model          {}", hoac_trong(&c.llm.model));
    println!("  max_output     {}", c.llm.max_output_tokens);
    println!("  temperature    {}/1000", c.llm.temperature_milli);
    println!(
        "  độ trễ nhận thức {} tick (§20.2.2)",
        c.llm.cognitive_latency_ticks
    );
    in_khoa("  khóa", &c.llm.api_key_env);

    println!("\nembedding");
    println!("  mode           {:?}", c.embedding.mode);
    println!("  base_url       {}", hoac_trong(&c.embedding.base_url));
    println!("  model          {}", hoac_trong(&c.embedding.model));
    println!(
        "  số chiều       {} (từ `vector.dimension`)",
        c.vector.dimension
    );
    println!("  batch          {}", c.embedding.batch_size);
    in_khoa("  khóa", &c.embedding.api_key_env);

    println!("\nlưu trữ");
    println!("  save           {}", c.persistence.url);
    println!("  chỉ mục        {}", c.vector.url);

    // Nói thẳng cái gì đang thật sự chạy. Một người đọc tới đây đang muốn biết
    // "tôi bấm chạy thì cái gì xảy ra", không muốn suy ra từ ba dòng ở trên.
    println!("\ntóm lại");
    match c.llm.mode {
        LlmMode::Stub => {
            println!("  NPC suy nghĩ bằng câu trả lời cố định — không mạng, không token.")
        }
        LlmMode::Replay => println!(
            "  NPC suy nghĩ bằng bản ghi trong `{}`.",
            c.llm.cassette_dir
        ),
        LlmMode::Record => println!("  NPC gọi mô hình THẬT và ghi lại — có tính tiền."),
        LlmMode::Live => println!("  NPC gọi mô hình THẬT — có tính tiền."),
    }
    match c.embedding.mode {
        EmbeddingMode::Stub => println!(
            "  Ký ức đánh chỉ mục bằng băm từ vựng — xác định, không mạng, KHÔNG có ngữ nghĩa."
        ),
        EmbeddingMode::Live => println!("  Ký ức đánh chỉ mục bằng máy chủ embedding thật."),
    }

    if c.llm.mode == LlmMode::Live {
        println!("\nthử thật:  mow-cli llm ping --env {}", d.env);
    }
    if c.embedding.mode == EmbeddingMode::Live {
        println!("           mow-cli embed probe --env {}", d.env);
    }
    ExitCode::SUCCESS
}

fn hoac_trong(s: &str) -> &str {
    if s.is_empty() {
        "(trống)"
    } else {
        s
    }
}

/// In trạng thái một biến khóa. **Chỉ** tên biến và có/thiếu.
fn in_khoa(nhan: &str, ten_bien: &str) {
    let co = std::env::var(ten_bien).is_ok_and(|v| !v.trim().is_empty());
    println!(
        "{nhan}           ${ten_bien} — {}",
        if co { "có" } else { "THIẾU" }
    );
}

fn lay_khoa(ten: &str) -> String {
    std::env::var(ten).unwrap_or_default()
}

/// `llm ping` — một lời gọi thật, nhỏ nhất có thể.
pub fn llm_ping(args: &[&str]) -> ExitCode {
    let d = match nap(args) {
        Ok(d) => d,
        Err(c) => return c,
    };
    let c = &d.cfg;

    if c.llm.base_url.is_empty() || c.llm.model.is_empty() {
        eprintln!(
            "`llm.base_url` hoặc `llm.model` trống. Lệnh này gọi thật, nên nó cần cả hai.\n\
             Thử `mow-cli config check --env live`."
        );
        return ExitCode::from(78);
    }

    let mut client = OpenAiCompatClient::new(
        &c.llm.base_url,
        &lay_khoa(&c.llm.api_key_env),
        &c.llm.model,
        UreqTransport::new(c.llm.timeout_ms),
    )
    .with_max_output_tokens(c.llm.max_output_tokens)
    .with_temperature_milli(c.llm.temperature_milli)
    .with_attribution(mow_llm::Attribution {
        url: c.llm.app_url.clone(),
        title: c.llm.app_title.clone(),
    });

    let req = Request {
        prompt_id: "cli.ping".to_owned(),
        prompt_version: 1,
        model: c.llm.model.clone(),
        rendered: "Trả lời đúng một từ: ok".to_owned(),
        // Dùng đúng trần của cấu hình, **không** dùng một con số nhỏ cho rẻ.
        //
        // Lần đầu viết lệnh này nó đặt 32, và lời gọi thật trả về một chuỗi
        // rỗng kèm `finish_reason: length`. Lý do: `deepseek-v4-flash` là model
        // suy luận — nó tiêu token cho phần suy luận **trước** khi sinh ra chữ
        // nào của câu trả lời. 32 token biến mất trong phần đó.
        //
        // Nên ping phải chạy với đúng trần mà thế giới sẽ chạy. Một lệnh kiểm
        // chạy với tham số khác tham số thật là một lệnh kiểm nói dối.
        max_output_tokens: c.llm.max_output_tokens,
    };

    println!("gọi {} tại {} ...", c.llm.model, c.llm.base_url);
    let bat_dau = std::time::Instant::now();
    match client.call(&req) {
        Ok(r) => {
            let ms = bat_dau.elapsed().as_millis();
            println!("  ✓ {ms} ms");
            // Model **thật sự** trả lời, có thể khác model đã xin (`§20.10`).
            println!("  model trả lời: {}", r.model);
            if r.model != c.llm.model {
                println!(
                    "  ! khác model đã yêu cầu ({}) — gateway đã định tuyến lại",
                    c.llm.model
                );
            }
            println!("  token: {} vào / {} ra", r.input_tokens, r.output_tokens);
            println!("  nội dung: {}", r.text.trim());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("  ✗ {e}");
            eprintln!(
                "\nKiểm theo thứ tự: biến ${} có giá trị chưa, khóa còn hạn mức không,\n\
                 tên model `{}` có đúng như trên trang nhà cung cấp không.",
                c.llm.api_key_env, c.llm.model
            );
            ExitCode::FAILURE
        }
    }
}

/// `embed probe` — mã hóa hai câu, in số chiều và độ tương đồng.
///
/// In độ tương đồng chứ không chỉ "kết nối được", vì hai câu chia nhiều từ mà
/// điểm gần 0 nghĩa là máy chủ đang trả về thứ gì đó không phải embedding của
/// văn bản mình gửi — chuyện xảy ra khi tiền tố tác vụ sai, hoặc khi máy chủ
/// đang phục vụ một model khác model mình nghĩ.
pub fn embed_probe(args: &[&str]) -> ExitCode {
    let d = match nap(args) {
        Ok(d) => d,
        Err(c) => return c,
    };
    let c = &d.cfg;
    let dim = c.vector.dimension;

    let cau = [
        "người thợ rèn đúc một thanh kiếm trong lò lửa",
        "người thợ rèn đúc một cái cuốc trong lò lửa",
        "trận mưa sao băng tháng tám trên biển",
    ];

    let ket_qua = if c.embedding.mode == EmbeddingMode::Stub {
        println!("embedding.mode = STUB — dùng băm từ vựng tại chỗ, không gọi mạng.");
        mow_llm::HashingEmbedder::new(dim).embed(EmbedRole::Document, &cau)
    } else {
        println!("gọi {} tại {} ...", c.embedding.model, c.embedding.base_url);
        let e = HttpEmbedder::new(
            &c.embedding.base_url,
            &lay_khoa(&c.embedding.api_key_env),
            &c.embedding.model,
            dim,
            UreqTransport::new(c.embedding.timeout_ms),
        )
        .with_batch(c.embedding.batch_size)
        .with_send_dimensions(c.embedding.send_dimensions)
        .with_prefixes(&c.embedding.query_prefix, &c.embedding.document_prefix);
        e.embed(EmbedRole::Document, &cau)
    };

    match ket_qua {
        Ok(v) => {
            println!("  ✓ {} vector, {} chiều", v.len(), v[0].len());
            let cham = |a: &[f32], b: &[f32]| -> f64 {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| f64::from(*x) * f64::from(*y))
                    .sum()
            };
            let gan = cham(&v[0], &v[1]);
            let xa = cham(&v[0], &v[2]);
            println!("  gần  (kiếm ↔ cuốc):        {gan:.4}");
            println!("  xa   (kiếm ↔ mưa sao băng): {xa:.4}");
            if gan > xa {
                println!("  ✓ xếp hạng đúng chiều");
                ExitCode::SUCCESS
            } else {
                // Không đỏ, vì đây là một phép đo chứ không phải một hợp đồng.
                // Nhưng phải nói, vì kết quả này im lặng làm hỏng truy xuất.
                println!(
                    "  ! xếp hạng NGƯỢC chiều mong đợi. Thường là do tiền tố tác vụ:\n\
                     \x20   xem `embedding.query_prefix` / `document_prefix`, hoặc máy chủ\n\
                     \x20   đang phục vụ một model khác model bạn nghĩ."
                );
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("  ✗ {e}");
            eprintln!("\nMáy chủ đã bật chưa: `./mow ai up`, rồi `./mow ai logs`.");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tim_config_root_ton_trong_chi_dinh() {
        let p = tim_config_root(Some("/duong/dan/khac"));
        assert_eq!(p, std::path::Path::new("/duong/dan/khac"));
    }

    #[test]
    fn doc_co_lay_dung_gia_tri() {
        let a = ["--env", "live", "--root", "x"];
        assert_eq!(doc_co(&a, "--env").as_deref(), Some("live"));
        assert_eq!(doc_co(&a, "--root").as_deref(), Some("x"));
        assert_eq!(doc_co(&a, "--khong-co"), None);
    }

    #[test]
    fn co_thieu_gia_tri_thi_khong_panic() {
        // `mow-cli config check --env` (thiếu giá trị) phải rơi về mặc định,
        // không được panic ở `args[i + 1]`.
        let a = ["--env"];
        assert_eq!(doc_co(&a, "--env"), None);
    }

    #[test]
    fn hoac_trong_noi_ro_la_trong() {
        assert_eq!(hoac_trong(""), "(trống)");
        assert_eq!(hoac_trong("x"), "x");
    }
}
