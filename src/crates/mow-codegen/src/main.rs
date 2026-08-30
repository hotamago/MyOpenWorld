//! `mow-codegen` — sinh mã và hợp đồng (`plan.md §P4`).
//!
//! **Ba pipeline, ba nguồn khác nhau, không giao nhau.** Việc tách chúng ra là
//! kết quả của một lỗi cụ thể: bản kế hoạch đầu tiên có `§P4.1` sinh Rust từ
//! schema và `§P4.2` sinh schema từ Rust — một chu trình. Khi có chu trình,
//! không ai còn biết bên nào là nguồn sự thật, và mỗi lần chạy codegen lại cho
//! một kết quả hơi khác.
//!
//! ```text
//! Pipeline 1 — RPC       nguồn: proto/           → Rust, Python, TypeScript
//! Pipeline 2 — CONTENT   nguồn: schemas/content/ → Rust, Python, TypeScript
//! Pipeline 3 — CONFIG    nguồn: struct Rust      → schemas/config/
//! ```
//!
//! Pipeline 3 đi **ngược chiều** hai cái kia, và đó là chủ đích. Sinh struct
//! config từ YAML sẽ làm trình biên dịch không kiểm được gì: một lỗi chính tả
//! trong tên field trở thành một field mới im lặng thay vì một lỗi biên dịch.
//!
//! `--check` không ghi gì, chỉ so mã sinh với mã đã commit rồi thoát khác 0 nếu
//! lệch. CI chạy chế độ đó (`P0-04`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let check = refs.contains(&"--check");
    let root = goc_du_an();

    let ket_qua = match refs.first().copied() {
        Some("config") => pipeline_config(&root, check),
        Some("content") => pipeline_content(&root, check),
        Some("rpc") => pipeline_rpc(&root, check),
        Some("all") | None => {
            let mut loi = Vec::new();
            for f in [pipeline_config, pipeline_content, pipeline_rpc] {
                if let Err(e) = f(&root, check) {
                    loi.push(e);
                }
            }
            if loi.is_empty() {
                Ok(())
            } else {
                Err(loi.join("\n"))
            }
        }
        Some(khac) => Err(format!(
            "không biết pipeline `{khac}`. Có: config, content, rpc, all"
        )),
    };

    match ket_qua {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn goc_du_an() -> PathBuf {
    // `CARGO_MANIFEST_DIR` là `src/crates/mow-codegen`; lùi hai bậc về `src/`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Ghi hoặc so sánh một file sinh ra.
///
/// Gộp hai chế độ vào một hàm để chúng không thể lệch nhau. Nếu `--check` so
/// một cách và `--write` ghi một cách khác, thì CI sẽ xanh trong khi file trên
/// đĩa vẫn sai.
fn ghi_hoac_kiem(path: &Path, noi_dung: &str, check: bool) -> Result<(), String> {
    let hien_tai = std::fs::read_to_string(path).ok();

    if check {
        return match hien_tai {
            Some(cu) if chuan_hoa(&cu) == chuan_hoa(noi_dung) => Ok(()),
            Some(_) => Err(format!(
                "{}: mã sinh khác mã đã commit. Chạy `make codegen` rồi commit lại.",
                path.display()
            )),
            None => Err(format!(
                "{}: thiếu file mã sinh. Chạy `make codegen`.",
                path.display()
            )),
        };
    }

    if hien_tai.as_deref().map(chuan_hoa) == Some(chuan_hoa(noi_dung)) {
        // Không ghi lại file không đổi: giữ mtime để build hệ thống không phải
        // biên dịch lại mọi thứ phụ thuộc.
        println!("  = {}", path.display());
        return Ok(());
    }

    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    std::fs::write(path, noi_dung).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("  → {}", path.display());
    Ok(())
}

/// Bỏ khác biệt về kiểu xuống dòng.
///
/// Repo phát triển trên Windows và chạy CI trên Linux. Không chuẩn hóa thì
/// `--check` sẽ đỏ trên mọi PR vì `\r\n` khác `\n`, và cả đội sẽ học cách bỏ
/// qua bước kiểm này.
fn chuan_hoa(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end().to_owned()
}

// ── Pipeline 3 — CONFIG: struct Rust → JSON Schema ───────────────────────────

fn pipeline_config(root: &Path, check: bool) -> Result<(), String> {
    println!("pipeline 3 — CONFIG (struct Rust → JSON Schema)");
    let schema = mow_config::AppConfig::json_schema_string();
    ghi_hoac_kiem(
        &root.join("schemas/config/app_config.v1.json"),
        &format!("{schema}\n"),
        check,
    )
}

// ── Pipeline 2 — CONTENT: JSON Schema viết tay → mã ba ngôn ngữ ──────────────

fn pipeline_content(root: &Path, check: bool) -> Result<(), String> {
    println!("pipeline 2 — CONTENT (JSON Schema → Rust/Python/TS)");
    let dir = root.join("schemas/content");
    if !dir.exists() {
        println!("  (chưa có schema content nào — bỏ qua)");
        return Ok(());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    // Sắp xếp: thứ tự hệ thống tệp khác nhau giữa các nền tảng, và nó sẽ lọt
    // vào file sinh ra dưới dạng thứ tự khai báo.
    files.sort();

    if files.is_empty() {
        println!("  (chưa có schema content nào — bỏ qua)");
        return Ok(());
    }

    // Chỉ mục các schema, để mã sinh ở ba ngôn ngữ đều tham chiếu cùng một danh
    // sách. Bộ sinh đầy đủ cho từng ngôn ngữ đến ở `PA-04` khi schema content
    // đầu tiên (worldseed) tồn tại; trước đó, sinh mã cho một tập rỗng chỉ tạo
    // ra file rỗng để rồi phải xóa.
    let ten: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_owned))
        .collect();
    let index = format!(
        "// SINH RA TỰ ĐỘNG — đừng sửa tay.\n\
         // Nguồn: schemas/content/*.json (pipeline 2, plan.md §P4.1)\n\
         \n\
         /// Danh sách schema content đã biết, theo thứ tự tên.\n\
         pub const CONTENT_SCHEMAS: &[&str] = &[\n{}];\n",
        ten.iter()
            .map(|t| format!("    \"{t}\",\n"))
            .collect::<String>()
    );
    ghi_hoac_kiem(
        &root.join("crates/mow-schema/src/generated.rs"),
        &index,
        check,
    )
}

// ── Pipeline 1 — RPC: proto → mã ba ngôn ngữ ─────────────────────────────────

fn pipeline_rpc(root: &Path, check: bool) -> Result<(), String> {
    println!("pipeline 1 — RPC (proto → Rust/Python/TS)");
    let dir = root.join("proto");
    let mut files = Vec::new();
    thu_thap_proto(&dir, &mut files);
    files.sort();

    if files.is_empty() {
        println!("  (chưa có file .proto nào — bỏ qua)");
        return Ok(());
    }

    // Bộ sinh thật cần `protoc`. Nó có trong toolbox (`deploy/docker/toolbox.Dockerfile`)
    // nhưng không nhất thiết có trên máy thật, nên báo rõ thay vì fail khó hiểu.
    if which_protoc().is_none() {
        let msg = "  không tìm thấy `protoc`. Chạy trong toolbox: `./mow exec make codegen`";
        if check {
            // Ở chế độ kiểm, thiếu công cụ **không** được coi là đạt: một CI
            // thiếu protoc sẽ xanh mà chưa bao giờ kiểm gì.
            return Err(format!(
                "{msg}\n  (chế độ --check không được bỏ qua bước này)"
            ));
        }
        println!("{msg}");
        return Ok(());
    }

    println!("  {} file .proto", files.len());
    Ok(())
}

fn thu_thap_proto(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            thu_thap_proto(&p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("proto") {
            out.push(p);
        }
    }
}

fn which_protoc() -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|d| {
            for ten in ["protoc", "protoc.exe"] {
                let p = d.join(ten);
                if p.is_file() {
                    return Some(p);
                }
            }
            None
        })
    })
}
