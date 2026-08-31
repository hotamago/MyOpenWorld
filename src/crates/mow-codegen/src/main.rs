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
    println!("  {} file .proto", files.len());

    // `protoc` lấy từ gói `grpcio-tools` trong uv workspace, không phải từ một
    // bản cài sẵn trên máy.
    //
    // Lý do: một `protoc` cài tay có version tùy máy, và hai version protoc khác
    // nhau sinh ra mã hơi khác nhau — đủ để `--check` đỏ trên máy này và xanh
    // trên máy kia, mà không ai đổi gì. Khóa nó vào lockfile Python là cách rẻ
    // nhất để mọi người và CI dùng đúng một bản.
    let protoc = tim_protoc(root)?;

    // **Đường dẫn tương đối, luôn luôn.** Trên Windows, `protoc` tách tham số
    // plugin ở dấu hai chấm, nên một đường dẫn tuyệt đối `D:\...` bị hiểu thành
    // tên plugin `D` cộng tham số — và thông báo lỗi nó in ra không hề nhắc tới
    // ổ đĩa, nên chỗ hỏng rất khó nhìn ra. Chạy với `current_dir(root)` rồi
    // truyền đường tương đối là cách tránh gọn nhất.
    const OUT_PY: &str = "services/agent-service/src/agent_service/generated";
    const FDS: &str = "proto/descriptor_set.bin";
    const TAM_PY: &str = "target/codegen-check/py";
    let out_py = root.join(OUT_PY);

    // Một lần gọi, hai sản phẩm: mã Python, và **descriptor set** cho phía Rust.
    //
    // Descriptor set là chỗ khéo của thiết kế này: `prost-build` biên dịch được
    // trực tiếp từ nó, nên build Rust **không cần protoc**. Người clone repo và
    // gõ `cargo build` không phải cài gì thêm, và mã Rust sinh ra không phụ
    // thuộc version protoc trên máy họ.
    let tam = root.join(TAM_PY);
    let dich_py = if check { TAM_PY } else { OUT_PY };
    std::fs::create_dir_all(root.join(dich_py))
        .map_err(|e| format!("  không tạo được {dich_py}: {e}"))?;

    let mut cmd = std::process::Command::new(&protoc.0);
    cmd.current_dir(root)
        .args(&protoc.1)
        .args(["-I", "proto"])
        .arg(format!("--descriptor_set_out={FDS}"))
        .arg("--include_imports")
        // Mang chú thích trong `.proto` sang mã sinh ở cả ba ngôn ngữ. Không có
        // cờ này, phần giải thích *vì sao* một trường tồn tại chỉ nằm trong file
        // proto — và người đọc mã sinh, tức là hầu hết mọi người, không thấy nó.
        .arg("--include_source_info")
        .arg(format!("--python_betterproto_out={dich_py}"));
    for f in &files {
        // Cũng phải tương đối, vì cùng lý do.
        cmd.arg(f.strip_prefix(root).unwrap_or(f));
    }

    let ra = cmd
        .output()
        .map_err(|e| format!("  không chạy được protoc: {e}"))?;
    if !ra.status.success() {
        return Err(format!(
            "  protoc lỗi:\n{}",
            String::from_utf8_lossy(&ra.stderr)
        ));
    }

    if check {
        // So mã sinh với mã đã commit. Lệch nghĩa là ai đó sửa proto mà quên
        // chạy `make codegen`, và hai phía hợp đồng đã bắt đầu trôi khỏi nhau.
        let lech = so_thu_muc(&tam, &out_py);
        let _ = std::fs::remove_dir_all(&tam);
        if !lech.is_empty() {
            return Err(format!(
                "  mã sinh khác mã đã commit ({} file):\n    {}\n  chạy `make codegen` rồi commit lại.",
                lech.len(),
                lech.join("\n    ")
            ));
        }
        println!("  ✓ mã Python khớp");
    } else {
        println!("  → {}", out_py.display());
    }

    println!("  → {FDS}");
    Ok(())
}

/// Tìm protoc: ưu tiên `grpcio-tools` trong uv workspace, sau đó mới tới máy.
///
/// Trả về `(chương trình, tham số dẫn đầu)` vì bản trong uv là một module Python
/// chứ không phải một file thực thi.
fn tim_protoc(root: &Path) -> Result<(PathBuf, Vec<String>), String> {
    for uv in ["uv", "uv.exe"] {
        let thu = std::process::Command::new(uv)
            .current_dir(root)
            .args(["run", "python", "-c", "import grpc_tools.protoc"])
            .output();
        if matches!(thu, Ok(ref r) if r.status.success()) {
            return Ok((
                PathBuf::from(uv),
                ["run", "python", "-m", "grpc_tools.protoc"]
                    .iter()
                    .map(|s| (*s).to_owned())
                    .collect(),
            ));
        }
    }

    if let Some(p) = which_protoc() {
        println!("  (dùng protoc của hệ thống — version có thể khác CI)");
        return Ok((p, Vec::new()));
    }

    Err("  không tìm thấy protoc.\n           Chạy `uv sync` để lấy `grpcio-tools`, hoặc dùng toolbox: `./mow exec make codegen`"
        .to_owned())
}

/// Những file khác nhau giữa hai cây thư mục, tính cả file chỉ có ở một bên.
fn so_thu_muc(a: &Path, b: &Path) -> Vec<String> {
    let mut lech = Vec::new();
    let mut ta = Vec::new();
    let mut tb = Vec::new();
    thu_thap_moi(a, a, &mut ta);
    thu_thap_moi(b, b, &mut tb);
    ta.sort();
    tb.sort();

    for r in &ta {
        if !tb.contains(r) {
            lech.push(format!("{} (thừa)", r.display()));
        } else if std::fs::read(a.join(r)).ok() != std::fs::read(b.join(r)).ok() {
            lech.push(format!("{} (khác)", r.display()));
        }
    }
    for r in &tb {
        if !ta.contains(r) {
            lech.push(format!("{} (thiếu)", r.display()));
        }
    }
    lech
}

fn thu_thap_moi(goc: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            thu_thap_moi(goc, &p, out);
        } else if let Ok(r) = p.strip_prefix(goc) {
            out.push(r.to_path_buf());
        }
    }
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
