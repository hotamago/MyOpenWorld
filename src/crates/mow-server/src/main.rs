//! `mow-server` — tiến trình giữ thế giới và phục vụ màn hình.
//!
//! ```bash
//! mow-server --port 17777 --seed 42 --web src/web/dist
//! ```
//!
//! ## Vì sao binary này ra đời muộn như vậy
//!
//! `plan.md §P3.1` mô tả `mow-server` là tiến trình trung tâm từ đầu, nhưng
//! `progress.md` chưa bao giờ có task dựng nó: 147/147 task xây engine dưới
//! dạng **thư viện**. Hệ quả là mọi thứ đều đúng và không có gì hiện ra.
//!
//! ## Một luồng sở hữu thế giới
//!
//! `Sim` không phải `Sync`, và điều đó là **đúng**: `§22.1` nói có đúng một
//! đường ghi. Ở đây nó thành một `Mutex` mà luồng tick và luồng HTTP cùng giành,
//! với hai quy tắc:
//!
//! 1. Khóa được giữ trong **một** thao tác rồi thả ngay. Không có I/O nào xảy
//!    ra khi đang giữ khóa.
//! 2. Luồng tick **không bao giờ** chờ client (`§P6.8` quy tắc 2). Client chậm
//!    thì nó tụt lại, không phải thế giới dừng.
//!
//! ## Không có `Access-Control-Allow-Origin: *`
//!
//! Server này cầm cả thế giới của người chơi. Cho phép mọi origin nghĩa là một
//! tab bất kỳ đang mở cũng ra lệnh được cho thế giới đó. Chỉ `localhost` ở cổng
//! dev của Vite được phép, và chỉ khi `--dev` bật.

mod api;
mod game;

use game::Game;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Origin duy nhất được gọi chéo, và chỉ ở chế độ `--dev`.
const ORIGIN_DEV: &str = "http://localhost:5173";

struct Args {
    port: u16,
    seed: u64,
    web: Option<String>,
    dev: bool,
    tick_ms: u64,
}

fn doc_args() -> Result<Args, String> {
    let mut a = Args {
        port: 17777,
        seed: 42,
        web: None,
        dev: false,
        tick_ms: 250,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        let lay = |i: usize| -> Result<String, String> {
            argv.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("`{}` cần một giá trị", argv[i]))
        };
        match argv[i].as_str() {
            "--port" => {
                a.port = lay(i)?.parse().map_err(|e| format!("--port: {e}"))?;
                i += 1;
            }
            "--seed" => {
                a.seed = lay(i)?.parse().map_err(|e| format!("--seed: {e}"))?;
                i += 1;
            }
            "--web" => {
                a.web = Some(lay(i)?);
                i += 1;
            }
            "--tick-ms" => {
                a.tick_ms = lay(i)?.parse().map_err(|e| format!("--tick-ms: {e}"))?;
                i += 1;
            }
            "--dev" => a.dev = true,
            "--help" | "-h" => return Err(String::new()),
            khac => return Err(format!("không hiểu tham số `{khac}`")),
        }
        i += 1;
    }
    Ok(a)
}

fn tro_giup() {
    println!(
        "mow-server — giữ thế giới và phục vụ giao diện\n\
         \n\
         --port N        cổng nghe (mặc định 17777)\n\
         --seed N        seed thế giới (mặc định 42)\n\
         --web <thư mục> phục vụ file tĩnh, thường là `src/web/dist`\n\
         --dev           cho phép origin http://localhost:5173 gọi chéo\n\
         --tick-ms N     nhịp tick, mili giây (mặc định 250)\n"
    );
}

fn main() -> std::process::ExitCode {
    let args = match doc_args() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            tro_giup();
            return std::process::ExitCode::from(if e.is_empty() { 0 } else { 2 });
        }
    };

    let dia_chi = format!("127.0.0.1:{}", args.port);
    let server = match tiny_http::Server::http(&dia_chi) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("không nghe được ở {dia_chi}: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let game = Arc::new(Mutex::new(Game::new(args.seed)));
    let chay = Arc::new(AtomicBool::new(true));

    // ── Luồng tick ──────────────────────────────────────────────────────────
    {
        let game = Arc::clone(&game);
        let chay = Arc::clone(&chay);
        let nhip = Duration::from_millis(args.tick_ms.max(10));
        std::thread::spawn(move || {
            while chay.load(Ordering::Relaxed) {
                // Khóa mở trong một biểu thức rồi thả ngay: không `sleep` khi
                // đang giữ khóa, nếu không mọi yêu cầu HTTP đứng theo nhịp tick.
                if let Ok(mut g) = game.lock() {
                    g.tick_once();
                }
                std::thread::sleep(nhip);
            }
        });
    }

    println!("mow-server: http://{dia_chi}  (seed {}, tick {}ms)", args.seed, args.tick_ms);
    if let Some(w) = &args.web {
        println!("  phục vụ giao diện từ {w}");
    }
    if args.dev {
        println!("  chế độ dev: cho phép {ORIGIN_DEV}");
    }

    for mut req in server.incoming_requests() {
        let mut than = String::new();
        if req.as_reader().read_to_string(&mut than).is_err() {
            than.clear();
        }
        let url = req.url().to_owned();
        let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
        let method = req.method().as_str().to_owned();

        let tra = if path.starts_with("/api/") {
            let r = {
                let mut g = match game.lock() {
                    Ok(g) => g,
                    Err(e) => e.into_inner(),
                };
                api::route(&mut g, &method, path, query, &than)
            };
            Some(r)
        } else {
            None
        };

        let ket_qua = match tra {
            Some(r) => gui_json(req, &r, args.dev),
            None => gui_tinh(req, path, args.web.as_deref()),
        };
        if let Err(e) = ket_qua {
            eprintln!("không gửi được trả lời: {e}");
        }
    }

    chay.store(false, Ordering::Relaxed);
    std::process::ExitCode::SUCCESS
}

fn header(k: &str, v: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes())
        .unwrap_or_else(|()| unreachable!("header hằng số luôn hợp lệ"))
}

fn gui_json(req: tiny_http::Request, r: &api::Reply, dev: bool) -> std::io::Result<()> {
    let mut resp = tiny_http::Response::from_string(r.body.clone())
        .with_status_code(r.status)
        .with_header(header("Content-Type", "application/json; charset=utf-8"))
        // Trạng thái thế giới đổi mỗi tick; cache nó là cách chắc chắn nhất để
        // màn hình hiện một quá khứ.
        .with_header(header("Cache-Control", "no-store"));
    if dev {
        resp = resp
            .with_header(header("Access-Control-Allow-Origin", ORIGIN_DEV))
            .with_header(header("Access-Control-Allow-Headers", "Content-Type"));
    }
    req.respond(resp)
}

/// Kiểu MIME theo đuôi file. Danh sách ngắn vì `dist/` chỉ có ngần này.
fn mime(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, e)| e) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn gui_tinh(req: tiny_http::Request, path: &str, goc: Option<&str>) -> std::io::Result<()> {
    let Some(goc) = goc else {
        return req.respond(
            tiny_http::Response::from_string("mow-server đang chạy; giao diện chưa được gắn (--web)")
                .with_status_code(404),
        );
    };

    // Chỉ nhận đường dẫn tuyệt đối một tầng, và **cấm `..`**. Không có dòng
    // này thì `GET /../../.env` đọc được khóa API của người chơi.
    let sach = path.trim_start_matches('/');
    if sach.split('/').any(|p| p == ".." || p == ".") {
        return req.respond(tiny_http::Response::from_string("").with_status_code(400));
    }
    let ten = if sach.is_empty() { "index.html" } else { sach };
    let duong_dan = std::path::Path::new(goc).join(ten);

    match std::fs::read(&duong_dan) {
        Ok(b) => {
            let resp = tiny_http::Response::from_data(b)
                .with_header(header("Content-Type", mime(ten)));
            req.respond(resp)
        }
        // SPA: mọi đường dẫn không phải file đều trả `index.html`, để nút Back
        // và deep link không cho ra 404.
        Err(_) => match std::fs::read(std::path::Path::new(goc).join("index.html")) {
            Ok(b) => req.respond(
                tiny_http::Response::from_data(b)
                    .with_header(header("Content-Type", "text/html; charset=utf-8")),
            ),
            Err(e) => req.respond(
                tiny_http::Response::from_string(format!("không đọc được giao diện: {e}"))
                    .with_status_code(404),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_theo_duoi_file() {
        assert_eq!(mime("index.html"), "text/html; charset=utf-8");
        assert_eq!(mime("assets/app-x1.js"), "text/javascript; charset=utf-8");
        assert_eq!(mime("khong-co-duoi"), "application/octet-stream");
    }
}
