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
//! ## Đẩy, không phải hỏi
//!
//! Bản đầu phục vụ REST qua `tiny_http` và để client tự `setInterval` rồi hỏi
//! lại. Một người chơi từng hỏi thẳng: thế giới chạy theo tick thì phải đẩy
//! trạng thái, không để ai đoán tần suất hỏi. Tầng vận chuyển ở đây dựng trên
//! `axum` (đã có `extract::ws`, không phải tự viết lại bắt tay WebSocket của
//! RFC 6455) với hai mặt:
//!
//! 1. **REST giữ nguyên** — `mod api` không đổi, mọi đường dẫn cũ vẫn trả
//!    đúng hình dạng JSON cũ. `handler::api_any_handler` chỉ là một lớp vỏ gọi
//!    lại [`api::route`], không phải một bảng định tuyến thứ hai.
//! 2. **`GET /ws` đẩy trạng thái** — xem tài liệu [`ws`] cho giao thức và các
//!    đánh đổi (kênh `watch` thay vì hàng đợi, con trỏ sự kiện theo từng kết
//!    nối, vì sao `tiles` không được đẩy).
//!
//! ## Một luồng sở hữu thế giới
//!
//! `Sim` không phải `Sync`, và điều đó là **đúng**: `§22.1` nói có đúng một
//! đường ghi. Ở đây nó thành một `Mutex` mà luồng tick và tầng mạng cùng
//! giành, với hai quy tắc:
//!
//! 1. Khóa được giữ trong **một** thao tác rồi thả ngay. Không có I/O nào xảy
//!    ra khi đang giữ khóa, và quan trọng hơn với `axum`: **không** có
//!    `.await` nào xảy ra khi đang giữ khóa.
//! 2. Luồng tick **không bao giờ** chờ client (`§P6.8` quy tắc 2). Client chậm
//!    thì nó tụt lại, không phải thế giới dừng.
//!
//! `std::sync::Mutex` là đủ, không cần `tokio::sync::Mutex` hay `parking_lot`:
//! cả hai lý do người ta đổi sang chúng — giữ khóa qua `.await`, hoặc tranh
//! chấp khóa dày đặc trên nhiều luồng tokio — đều không xảy ra ở đây. Mọi nơi
//! khóa `Game` đều khóa trong một khối không `.await`, rồi thả trước khi hàm
//! `async` đi tiếp. Ngoại lệ đáng nói duy nhất: `/api/genesis` gọi
//! `Game::load_content` để nạp lại content pack, và việc đó đọc đĩa **trong
//! lúc giữ khóa** — cùng hành vi với bản `tiny_http` cũ (vốn đơn luồng hoàn
//! toàn), chỉ khác là giờ nó có thể chiếm một luồng công nhân của `tokio` một
//! chốc. Đây là một đánh đổi có ý thức: khởi nguyên lại thế giới là một thao
//! tác hiếm và content pack chỉ vài file nhỏ, nên chi phí đó nhỏ hơn nhiều so
//! với việc dựng `spawn_blocking` cho một đường hiếm khi chạy.
//!
//! ## Không có `Access-Control-Allow-Origin: *`
//!
//! Server này cầm cả thế giới của người chơi. Cho phép mọi origin nghĩa là một
//! tab bất kỳ đang mở cũng ra lệnh được cho thế giới đó. Chỉ `localhost` ở cổng
//! dev của Vite được phép, và chỉ khi `--dev` bật. `GET /ws` cũng tự kiểm
//! Origin khi `--dev` — trình duyệt không áp CORS cho bắt tay WebSocket, nên
//! không tự kiểm ở đó thì hàng rào CORS phía REST coi như không tồn tại.

mod api;
mod game;
mod preview;
mod ws;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use game::Game;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

/// Origin duy nhất được gọi chéo (REST) hoặc nối WebSocket, và chỉ ở chế độ
/// `--dev`.
pub(crate) const ORIGIN_DEV: &str = "http://localhost:5173";

/// Trạng thái dùng chung cho mọi request/kết nối.
///
/// Không giữ thư mục `--web` ở đây: phục vụ file tĩnh là một `tower::Service`
/// (`ServeDir`) được dựng **một lần** lúc khởi động và gắn làm `fallback` của
/// router, không phải thứ mỗi request phải tra lại — xem [`build_router`] và
/// `main`.
pub(crate) struct AppState {
    pub(crate) game: Arc<Mutex<Game>>,
    /// Chỉ khi bật mới kiểm Origin của WebSocket và gắn `CorsLayer` cho REST.
    pub(crate) dev: bool,
    /// Nhịp đẩy trạng thái qua WebSocket, mili giây. Xem [`ws`].
    pub(crate) push_ms: u64,
}

struct Args {
    port: u16,
    seed: u64,
    web: Option<String>,
    dev: bool,
    tick_ms: u64,
    push_ms: u64,
    content: String,
}

fn doc_args() -> Result<Args, String> {
    let mut a = Args {
        port: 17777,
        seed: 42,
        web: None,
        dev: false,
        tick_ms: 250,
        push_ms: 100,
        content: "content/core".to_owned(),
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
            "--push-ms" => {
                a.push_ms = lay(i)?.parse().map_err(|e| format!("--push-ms: {e}"))?;
                i += 1;
            }
            "--content" => {
                a.content = lay(i)?;
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
         --tick-ms N     nhịp tick, mili giây (mặc định 250)\n\
         --push-ms N     nhịp đẩy trạng thái qua WebSocket, mili giây (mặc định 100)\n\
         --content <dir> content pack (mặc định `content/core`)\n"
    );
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::process::ExitCode {
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
    let listener = match tokio::net::TcpListener::bind(&dia_chi).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("không nghe được ở {dia_chi}: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut world = Game::new(args.seed);
    // Pack hỏng không làm server chết: client có bảng dự phòng, và một thế giới
    // vẽ bằng màu dự phòng vẫn tốt hơn một tiến trình không khởi động được.
    match world.load_content(&args.content) {
        Ok(n) => println!("  nạp {n} vật liệu từ {}", args.content),
        Err(e) => eprintln!("  ! không nạp được content `{}`: {e}", args.content),
    }
    let game = Arc::new(Mutex::new(world));
    let chay = Arc::new(AtomicBool::new(true));

    // ── Luồng tick ──────────────────────────────────────────────────────────
    // Vẫn là một `std::thread` thường, tách hẳn khỏi runtime `tokio`: nó
    // không cần `await` bao giờ, và giữ nó là một luồng OS đơn giản là cách
    // chắc chắn nhất để không client nào — dù chậm tới đâu — làm nó tụt nhịp.
    {
        let game = Arc::clone(&game);
        let chay = Arc::clone(&chay);
        let base_tick_ms = args.tick_ms.max(1);
        // Nhịp thức dậy **cố định**, tốc độ nằm ở số tick mỗi nhịp.
        //
        // Cách hiển nhiên là đổi thời gian ngủ theo tốc độ. Nó hỏng ở cả hai
        // đầu: ở ×100 thời gian ngủ thành 3 ms và luồng dành phần lớn thời gian
        // để giành khóa; ở ×0.001 nó ngủ 300 giây và người chơi kéo thanh trượt
        // xong phải chờ năm phút mới thấy phản ứng.
        const WAKE_MS: u64 = 50;
        std::thread::spawn(move || {
            let mut carry = 0u64;
            while chay.load(Ordering::Relaxed) {
                // Khóa mở trong một biểu thức rồi thả ngay: không `sleep` khi
                // đang giữ khóa, nếu không mọi yêu cầu HTTP đứng theo nhịp tick.
                if let Ok(mut g) = game.lock() {
                    let n = g.ticks_due(WAKE_MS, base_tick_ms, &mut carry);
                    for _ in 0..n {
                        g.tick_once();
                    }
                }
                std::thread::sleep(Duration::from_millis(WAKE_MS));
            }
        });
    }

    println!(
        "mow-server: http://{dia_chi}  (seed {}, tick {}ms, push {}ms)",
        args.seed, args.tick_ms, args.push_ms
    );
    if let Some(w) = &args.web {
        println!("  phục vụ giao diện từ {w}");
    }
    if args.dev {
        println!("  chế độ dev: cho phép {ORIGIN_DEV}");
    }

    let state = Arc::new(AppState {
        game,
        dev: args.dev,
        push_ms: args.push_ms.max(1),
    });
    let mut app = build_router(state);
    if let Some(dir) = &args.web {
        let index = std::path::Path::new(dir).join("index.html");
        // `ServeDir` chặn `..` và các đường dẫn thoát khỏi thư mục gốc tự nó
        // — không cần tự viết lại phần đó như bản `tiny_http` cũ.
        // `ServeFile` làm `fallback` cho SPA: mọi đường dẫn không khớp file
        // tĩnh nào (route của client, deep link, nút Back) đều trả về
        // `index.html` thay vì `404`.
        app = app.fallback_service(ServeDir::new(dir).not_found_service(ServeFile::new(index)));
    }

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(cho_ctrl_c())
        .await
    {
        eprintln!("lỗi khi phục vụ: {e}");
    }

    chay.store(false, Ordering::Relaxed);
    std::process::ExitCode::SUCCESS
}

/// Chờ `Ctrl-C` rồi trả quyền điều khiển lại cho `with_graceful_shutdown`.
///
/// `axum::serve` sẽ ngừng nhận kết nối mới và cho những yêu cầu đang dở hoàn
/// tất trước khi trả về — "tắt gọn" nghĩa là không cắt ngang một `POST` đang
/// giữa chừng, không phải chỉ là thoát tiến trình nhanh nhất có thể.
async fn cho_ctrl_c() {
    let _ = tokio::signal::ctrl_c().await;
    println!("nhận Ctrl-C, đang tắt...");
}

/// Dựng router dùng chung cho `main` lẫn test.
///
/// **Không** gắn phục vụ file tĩnh ở đây — đó là việc của lời gọi bên ngoài
/// (xem `main`), vì thư mục `--web` không phải một phần trạng thái xin qua
/// từng request mà là một service cấu hình một lần lúc khởi động. `fallback`
/// mặc định ở đây chỉ nói rõ "chưa gắn giao diện" thay vì `404` trống rỗng —
/// đúng hành vi cũ khi không truyền `--web`.
fn build_router(state: Arc<AppState>) -> Router {
    let mut app = Router::new()
        .route("/ws", get(ws::ws_handler))
        // Axum 0.8 dùng cú pháp `{*rest}` cho phần đuôi bắt tất cả. Giá trị
        // `rest` không được đọc — bên trong `api_any_handler` lấy lại đường
        // dẫn **đầy đủ** từ chính `Uri` của request, tránh phải ráp lại
        // `"/api/" + rest` (một chỗ nữa để lệch khỏi những gì `route` mong).
        .route("/api/{*rest}", any(api_any_handler))
        .fallback(chua_gan_giao_dien);
    if state.dev {
        app = app.layer(cors_layer());
    }
    app.with_state(state)
}

async fn chua_gan_giao_dien() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "mow-server đang chạy; giao diện chưa được gắn (--web)",
    )
}

/// `CorsLayer` cho REST: chỉ origin dev của Vite, không bao giờ `Any`.
///
/// `tower_http` tự trả lời `OPTIONS` preflight trước khi request chạm tới
/// handler — thiếu bước đó thì **mọi** `POST` mang `Content-Type:
/// application/json` từ trình duyệt chết với `TypeError: Failed to fetch`
/// không nói gì về nguyên nhân (lỗi này từng xảy ra thật với bản `tiny_http`
/// cũ, và `api::route` vẫn giữ một nhánh `OPTIONS` riêng làm lớp phòng thủ thứ
/// hai cho những preflight không mang đủ tiêu đề để `tower_http` tự bắt).
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(
            ORIGIN_DEV
                .parse::<HeaderValue>()
                // Hằng số biên dịch sẵn, không phải đầu vào của request: một
                // chuỗi ASCII cố định không bao giờ là một `HeaderValue`
                // không hợp lệ, nên `unreachable!` đúng nghĩa của nó hơn là
                // một `unwrap` phải xử lý một lỗi có thể xảy ra thật.
                .unwrap_or_else(|e| unreachable!("origin hằng số luôn hợp lệ: {e}")),
        )
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .max_age(Duration::from_secs(600))
}

/// Một handler duy nhất bắt mọi `/api/*`, gọi lại [`api::route`] rồi dựng
/// `Response` từ [`api::Reply`].
///
/// Cố tình **không** chép bảng `match (method, path)` của `api::route` sang
/// đây: đó là một hàm thuần đã có test riêng, và một bảng định tuyến thứ hai
/// chỉ là một chỗ nữa để nó lệch khỏi bảng thật.
async fn api_any_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    let path = uri.path();
    let query = uri.query().unwrap_or("");
    // Thân không phải UTF-8 hợp lệ thành thân rỗng: `api::route` tự báo "thân
    // không phải JSON" cho trường hợp đó, không cần một nhánh lỗi riêng ở đây.
    let body_str = std::str::from_utf8(&body).unwrap_or("");

    let reply = {
        let mut g = match state.game.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        api::route(&mut g, method.as_str(), path, query, body_str)
    };
    json_response(&reply)
}

/// Dựng `Response` từ một [`api::Reply`], giữ nguyên hai tiêu đề mà bản
/// `tiny_http` cũ luôn gắn cho JSON.
fn json_response(reply: &api::Reply) -> Response {
    let status = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            // Trạng thái thế giới đổi mỗi tick; cache nó là cách chắc chắn
            // nhất để màn hình hiện một quá khứ.
            (header::CACHE_CONTROL, "no-store"),
        ],
        reply.body.clone(),
    )
        .into_response()
}
