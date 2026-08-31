//! WebSocket: đẩy trạng thái theo nhịp, nhận lệnh theo yêu cầu.
//!
//! ## Vì sao đẩy, không phải hỏi
//!
//! Client cũ chạy `setInterval(400ms)` rồi hỏi lại bốn endpoint — một thế giới
//! chạy theo tick phải **đẩy** trạng thái, không để client đoán tần suất. Ở
//! đây server tự quyết khi nào có gì mới để gửi, và gửi đúng một lần.
//!
//! ## Giao thức: một object JSON mỗi khung, trường `t` là loại
//!
//! Server → Client:
//! ```jsonc
//! {"t":"hello","meta":{...}}
//! {"t":"state","meta":{...},"entities":[...],"events":[...]}
//! {"t":"ack","id":123,"ok":true,"body":{...}}
//! {"t":"ack","id":123,"ok":false,"error":"..."}
//! ```
//! Client → Server:
//! ```jsonc
//! {"t":"cmd","id":123,"path":"/api/look","body":{"x":1,"y":2}}
//! ```
//!
//! ## Không lặp lại bảng điều hướng
//!
//! [`crate::api::route`] đã có test riêng và là **nguồn sự thật duy nhất** cho
//! hình dạng JSON của mỗi endpoint. Module này không tự dựng `meta`/`entities`
//! /`events` bằng tay — nó gọi thẳng `route(g, "GET", "/api/meta", ...)` như
//! một client HTTP nội bộ. Hai bản chép tay (một ở `api.rs`, một ở đây) là hai
//! chỗ để chúng lệch nhau; gọi lại `route` thì `meta` gửi qua WebSocket **luôn
//! là đúng** JSON mà `GET /api/meta` trả về, không cần một bài test nào giữ
//! chúng đồng bộ bằng tay.
//!
//! Cùng lý do, một lệnh `cmd` chỉ đơn giản là gọi
//! `route(g, "POST", path, "", &body)` rồi bọc kết quả thành `ack` — không có
//! đường xử lý lệnh thứ hai nào tồn tại song song với REST.
//!
//! ## Vì sao `tiles` không được đẩy
//!
//! Một lô ô nhìn thấy được là hàng nghìn số; nó chỉ đổi khi khung nhìn dời,
//! không đổi mỗi tick như `entities`. Đẩy nó theo nhịp `push-ms` là trả giá
//! băng thông cho thứ đứng yên gần như mọi lúc. Client tiếp tục hỏi
//! `GET /api/tiles` khi (và chỉ khi) khung nhìn thật sự dời.
//!
//! ## Con trỏ sự kiện là của **từng kết nối**, không phải toàn cục
//!
//! Hai người xem cùng một thế giới có thể nối vào ở hai thời điểm khác nhau;
//! mỗi người cần "sự kiện mới kể từ lần đẩy trước **của chính mình**", không
//! phải của người kia. Nên con trỏ `after` sống trong biến cục bộ của
//! [`push_loop`] — một future riêng cho mỗi kết nối — chứ không phải một
//! trường dùng chung.
//!
//! ## Khoá, làm, thả, rồi mới `await`
//!
//! `Game` không có gì đặc biệt khiến `std::sync::MutexGuard` của nó an toàn
//! giữ qua một điểm `await` — thật ra **không guard nào** của
//! `std::sync::Mutex` an toàn giữ qua `await`, vì hệ điều hành có thể đòi mở
//! khoá đúng luồng đã khoá nó, còn một tác vụ `tokio` thì có thể bị dời sang
//! luồng khác ngay tại điểm `await`. Kỷ luật ở đây là **cơ học**, không phải
//! kiểu dữ liệu: mọi lần khoá đều nằm trong một khối `{ ... }` không chứa
//! `.await` nào, và giá trị cần dùng tiếp (một `String` JSON, một `EventSeq`)
//! được sao ra khỏi khối đó trước khi hàm `async` đi tiếp. Không cần
//! `tokio::sync::Mutex` hay `parking_lot`: chưa có chỗ nào trong module này
//! thật sự cần giữ khoá qua một lần chờ mạng.
//!
//! ## Kênh trạng thái: `watch`, không phải hàng đợi
//!
//! Yêu cầu bền vững là "đầy thì bỏ khung tin cũ, không chặn" — một client vẽ
//! chậm không được làm nghẽn phần còn lại. `tokio::sync::watch` là đúng hình
//! dạng đó mà không cần tự dựng: nó giữ **đúng một** giá trị mới nhất,
//! `Sender::send` không phải `async fn` (không bao giờ chặn người gửi), và một
//! bên nhận chậm chỉ đơn giản bỏ lỡ những khung ở giữa — nó luôn thấy giá trị
//! **mới nhất**, chưa bao giờ một hàng đợi bị tràn phải xử lý.
//!
//! Khung trả lời lệnh (`ack`) thì ngược lại: **không được** đánh rơi, vì mỗi
//! khung mang một `id` mà client đang chờ đúng nó. Nó đi qua một kênh
//! `mpsc` riêng, sức chứa nhỏ vì tần suất lệnh chơi game (đi, xây, xem trước)
//! thấp hơn nhiều so với nhịp đẩy trạng thái.

use crate::api;
use crate::game::Game;
use crate::AppState;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value as J};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

/// Nâng cấp `GET /ws` lên WebSocket rồi giao cho [`handle_socket`].
///
/// Origin chỉ được kiểm khi `--dev`, cùng chính sách với CORS ở `main.rs`:
/// trình duyệt **không** áp CORS cho bắt tay WebSocket (không có preflight,
/// không có `Access-Control-Allow-Origin` nào ngăn được nó), nên nếu không tự
/// kiểm ở đây thì bất kỳ trang nào người chơi mở cũng nối được vào và ra lệnh
/// cho thế giới của họ trong lúc dev server đang chạy.
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    if state.dev {
        let origin = headers
            .get(axum::http::header::ORIGIN)
            .and_then(|v| v.to_str().ok());
        if let Some(o) = origin {
            if o != crate::ORIGIN_DEV {
                return (
                    axum::http::StatusCode::FORBIDDEN,
                    "origin không được phép nối WebSocket ở chế độ dev",
                )
                    .into_response();
            }
        }
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Vòng đời một kết nối: chào, rồi chạy ba việc song song cho tới khi client
/// đóng kết nối — đẩy trạng thái, ghi socket, và đọc lệnh.
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut tx, mut rx) = socket.split();

    // `hello`: gọi lại `/api/meta` như mọi client HTTP khác, để không có một
    // hình dạng `meta` thứ hai chỉ WebSocket mới thấy.
    let (hello_meta, cursor0) = {
        let mut g = match state.game.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let meta = route_json(&mut g, "/api/meta", "");
        (meta, g.last_seq())
    };
    let hello = json!({ "t": "hello", "meta": hello_meta }).to_string();
    if tx.send(Message::Text(hello.into())).await.is_err() {
        // Client biến mất trước khi kịp chào — không có gì để dọn thêm, vì
        // chưa task nào được sinh ra.
        return;
    }

    // Xem tài liệu module: `watch` cho trạng thái (đè, không chặn), `mpsc`
    // cho `ack` (không đánh rơi).
    let (state_tx, state_rx) = watch::channel::<Option<String>>(None);
    let (ack_tx, ack_rx) = mpsc::channel::<String>(32);

    let pusher = tokio::spawn(push_loop(
        Arc::clone(&state.game),
        state.push_ms,
        cursor0,
        state_tx,
    ));
    let writer = tokio::spawn(write_loop(tx, state_rx, ack_rx));

    // Vòng đọc chạy ngay tại đây, không `spawn`: khi nó kết thúc (client đóng,
    // hoặc lỗi socket) thì kết nối coi như xong, và ta biết chắc chắn phải
    // dọn hai task còn lại — không chờ chúng tự phát hiện qua một lỗi gửi có
    // thể không bao giờ tới (`watch` không báo lỗi nếu writer đang bận, chỉ
    // báo khi *receiver* đã rơi hẳn).
    read_loop(&mut rx, Arc::clone(&state.game), ack_tx).await;

    writer.abort();
    pusher.abort();
}

/// Gọi `api::route` cho một đường `GET` nội bộ rồi trả về JSON đã phân tích.
///
/// Không dùng `unwrap`: nếu `route` (một hàm đã có test riêng, luôn trả JSON
/// hợp lệ cho các đường `GET` này) có lỡ đổi hành vi, chỗ này thà gửi `null`
/// còn hơn làm sập cả kết nối của người đang xem.
fn route_json(g: &mut Game, path: &str, query: &str) -> J {
    let r = api::route(g, "GET", path, query, "");
    serde_json::from_str(&r.body).unwrap_or(J::Null)
}

/// Nhịp đẩy trạng thái cho **một** kết nối.
///
/// Con trỏ sự kiện và hash lần trước sống trong hai biến cục bộ của hàm này —
/// đúng nghĩa "server tự nhớ con trỏ cho từng kết nối": không có bảng nào ánh
/// xạ kết nối → con trỏ ở nơi khác, vì mỗi kết nối *là* một lần gọi hàm này.
async fn push_loop(
    game: Arc<Mutex<Game>>,
    push_ms: u64,
    start_cursor: mow_core::EventSeq,
    tx: watch::Sender<Option<String>>,
) {
    let mut cursor = start_cursor;
    let mut last_hash: Option<String> = None;
    let mut nhip = tokio::time::interval(Duration::from_millis(push_ms.max(1)));
    // Nhịp tỉnh dậy cố định: nếu một lần thức dậy bị trễ (server bận), đừng
    // dồn các lần thức dậy đã lỡ lại thành một loạt liên tiếp — chỉ cần tiếp
    // tục ở nhịp đều, cùng lý do với `WAKE_MS` của luồng tick.
    nhip.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        nhip.tick().await;

        // Khoá, đọc, thả — không có `.await` nào trong khối này.
        let frame = {
            let mut g = match game.lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            let events = route_json(&mut g, "/api/events", &format!("after={}", cursor.0));
            let co_su_kien_moi = events
                .get("events")
                .and_then(J::as_array)
                .is_some_and(|a| !a.is_empty());
            let meta = route_json(&mut g, "/api/meta", "");
            let hash = meta
                .get("state_hash")
                .and_then(J::as_str)
                .map(str::to_owned);

            // Bỏ khung nếu hash không đổi và không có sự kiện mới: đẩy một
            // trạng thái y hệt lần trước là trả giá băng thông cho thứ không
            // đổi (yêu cầu tường minh của hợp đồng WebSocket).
            if !co_su_kien_moi && hash == last_hash {
                None
            } else {
                let entities = route_json(&mut g, "/api/entities", "");
                cursor = events
                    .get("cursor")
                    .and_then(J::as_u64)
                    .map(mow_core::EventSeq)
                    .unwrap_or(cursor);
                last_hash = hash;
                Some(
                    json!({
                        "t": "state",
                        "meta": meta,
                        "entities": entities.get("entities").cloned().unwrap_or_else(|| J::Array(vec![])),
                        "events": events.get("events").cloned().unwrap_or_else(|| J::Array(vec![])),
                    })
                    .to_string(),
                )
            }
        };

        if let Some(text) = frame {
            // `watch::Sender::send` không phải hàm `async`: nó chỉ ghi đè giá
            // trị và đánh thức người nhận, không có `.await` nào ở đây. Lỗi
            // trả về nghĩa là `writer` đã đóng (không còn ai giữ `state_rx`)
            // — dừng vòng lặp thay vì tiếp tục đẩy vào chỗ không ai nghe.
            if tx.send(Some(text)).is_err() {
                break;
            }
        }
    }
}

/// Gộp hai nguồn khung tin (trạng thái + trả lời lệnh) thành một dòng ghi
/// socket duy nhất.
///
/// Phải gộp vào **một** task: `WebSocket::split` cho một `Sink` không `Clone`,
/// và hai task cùng gọi `.send()` trên nó là một cuộc đua ghi xen kẽ giữa
/// khung, làm hỏng cả hai thông điệp.
async fn write_loop(
    mut tx: SplitSink<WebSocket, Message>,
    mut state_rx: watch::Receiver<Option<String>>,
    mut ack_rx: mpsc::Receiver<String>,
) {
    loop {
        tokio::select! {
            thay_doi = state_rx.changed() => {
                if thay_doi.is_err() {
                    break; // pusher đã dừng — không còn gì để chờ nữa
                }
                let noi_dung = state_rx.borrow_and_update().clone();
                if let Some(text) = noi_dung {
                    if tx.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
            }
            khung = ack_rx.recv() => {
                match khung {
                    Some(text) => {
                        if tx.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break, // đầu đọc đã kết thúc, không còn ai gửi thêm ack
                }
            }
        }
    }
}

/// Đọc lệnh từ client cho tới khi kết nối đóng.
async fn read_loop(
    rx: &mut SplitStream<WebSocket>,
    game: Arc<Mutex<Game>>,
    ack_tx: mpsc::Sender<String>,
) {
    while let Some(msg) = rx.next().await {
        let msg = match msg {
            Ok(m) => m,
            // Lỗi giao thức (khung hỏng, đóng đột ngột...) kết thúc kết nối
            // này, không phải lỗi của server.
            Err(_) => break,
        };
        match msg {
            Message::Text(text) => {
                let tra_loi = handle_cmd(&game, &text);
                if ack_tx.send(tra_loi).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            // `Ping`/`Pong` đã được `axum` tự trả lời ở tầng dưới; `Binary`
            // không nằm trong hợp đồng của module này.
            _ => {}
        }
    }
}

/// Diễn giải một khung `{"t":"cmd", ...}` thành một `ack`.
///
/// Không `unwrap`/`expect` ở đâu trong hàm này: một client gửi rác chỉ đáng
/// một `ack` báo lỗi, không đáng làm sập kết nối của chính nó — càng không
/// đáng làm sập kết nối của người khác.
fn handle_cmd(game: &Arc<Mutex<Game>>, text: &str) -> String {
    let j: J = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => return ack_loi(&J::Null, &format!("khung không phải JSON: {e}")),
    };
    let id = j.get("id").cloned().unwrap_or(J::Null);

    if j.get("t").and_then(J::as_str) != Some("cmd") {
        return ack_loi(&id, "chỉ hiểu khung `{\"t\":\"cmd\", ...}`");
    }
    let Some(path) = j.get("path").and_then(J::as_str) else {
        return ack_loi(&id, "thiếu `path`");
    };
    // Chỉ nhận `/api/*`: một client gửi `path` tuỳ ý là một lỗ hổng — nó có
    // thể chạm tới bất kỳ đường xử lý nội bộ nào lỡ được gắn nhầm vào router
    // của `axum` (ví dụ chính `/ws`), thứ chưa chắc chịu nổi một thân `POST`.
    if !path.starts_with("/api/") {
        return ack_loi(&id, "`path` phải bắt đầu bằng `/api/`");
    }
    let body = j
        .get("body")
        .cloned()
        .unwrap_or_else(|| J::Object(Default::default()));

    let reply = {
        let mut g = match game.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        api::route(&mut g, "POST", path, "", &body.to_string())
    };

    let parsed: J = serde_json::from_str(&reply.body).unwrap_or(J::Null);
    if reply.status < 400 {
        json!({ "t": "ack", "id": id, "ok": true, "body": parsed }).to_string()
    } else {
        let loi = parsed
            .get("error")
            .and_then(J::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("mã lỗi {}", reply.status));
        ack_loi(&id, &loi)
    }
}

fn ack_loi(id: &J, msg: &str) -> String {
    json!({ "t": "ack", "id": id, "ok": false, "error": msg }).to_string()
}

#[cfg(test)]
mod tests {
    use crate::{AppState, Game};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value as J};
    use std::sync::{Arc, Mutex};
    use tokio_tungstenite::tungstenite::Message as WsMsg;

    /// Dựng một server thật trên cổng do hệ điều hành cấp (`:0`), trả về địa
    /// chỉ đã gán để test tự nối vào — không đoán cổng, không tranh chấp với
    /// một tiến trình khác đang chạy trên máy CI.
    async fn dung_server() -> std::net::SocketAddr {
        let game = Arc::new(Mutex::new(Game::new(42)));
        let state = Arc::new(AppState {
            game,
            // `dev: false` — bài test này không kiểm Origin, cùng cách một
            // client không phải trình duyệt (như chính test này) sẽ nối vào
            // một server không chạy `--dev`.
            dev: false,
            push_ms: 20,
        });
        let app = crate::build_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind vào cổng 0 luôn thành công trên loopback");
        let addr = listener
            .local_addr()
            .expect("socket vừa bind phải có địa chỉ");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        addr
    }

    #[tokio::test]
    async fn hello_roi_ack_roi_state() {
        let addr = dung_server().await;
        let url = format!("ws://{addr}/ws");
        let (mut ws, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("nối WebSocket phải thành công");

        // 1) `hello` phải tới ngay, mang đúng hình dạng `/api/meta`.
        let hello: J = match ws.next().await {
            Some(Ok(WsMsg::Text(t))) => serde_json::from_str(&t).expect("hello phải là JSON"),
            other => panic!("mong chờ `hello`, nhận {other:?}"),
        };
        assert_eq!(hello["t"], "hello");
        assert!(hello["meta"]["state_hash"].is_string());

        // 2) Gửi một lệnh, phải nhận đúng `ack` mang lại `id` đã gửi.
        let cmd = json!({ "t": "cmd", "id": 123, "path": "/api/look", "body": { "x": 5, "y": 7 } });
        ws.send(WsMsg::Text(cmd.to_string().into()))
            .await
            .expect("gửi cmd phải thành công");

        let ack: J = loop {
            match ws.next().await {
                Some(Ok(WsMsg::Text(t))) => {
                    let v: J = serde_json::from_str(&t).expect("khung phải là JSON");
                    if v["t"] == "ack" {
                        break v;
                    }
                    // Một khung `state` xen vào trước `ack` là hợp lệ (nhịp
                    // đẩy 20ms của server test này rất nhanh); bỏ qua nó và
                    // chờ tiếp đúng `ack`.
                }
                other => panic!("mất kết nối trước khi thấy ack: {other:?}"),
            }
        };
        assert_eq!(ack["id"], 123);
        assert_eq!(ack["ok"], true, "{ack}");
        assert_eq!(ack["body"]["eye"], json!([5, 7]));

        // 3) Sau đó phải có ít nhất một khung `state` (đẩy theo nhịp).
        let state: J = loop {
            match ws.next().await {
                Some(Ok(WsMsg::Text(t))) => {
                    let v: J = serde_json::from_str(&t).expect("khung phải là JSON");
                    if v["t"] == "state" {
                        break v;
                    }
                }
                other => panic!("mất kết nối trước khi thấy state: {other:?}"),
            }
        };
        assert!(state["meta"]["state_hash"].is_string());
        assert!(state["entities"].is_array());
        assert!(state["events"].is_array());
    }

    #[tokio::test]
    async fn cmd_voi_path_ngoai_api_bi_tu_choi() {
        let addr = dung_server().await;
        let url = format!("ws://{addr}/ws");
        let (mut ws, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("nối WebSocket phải thành công");

        // Bỏ qua `hello`.
        let _ = ws.next().await;

        let cmd = json!({ "t": "cmd", "id": 1, "path": "/etc/passwd", "body": {} });
        ws.send(WsMsg::Text(cmd.to_string().into()))
            .await
            .expect("gửi cmd phải thành công");

        // Cùng lý do với bài test trên: nhịp đẩy trạng thái 20ms của server
        // test này có thể chen một khung `state` vào trước khi `ack` tới —
        // bỏ qua nó thay vì coi khung **tiếp theo** luôn là `ack`.
        let ack: J = loop {
            match ws.next().await {
                Some(Ok(WsMsg::Text(t))) => {
                    let v: J = serde_json::from_str(&t).expect("khung phải là JSON");
                    if v["t"] == "ack" {
                        break v;
                    }
                }
                other => panic!("mất kết nối trước khi thấy ack: {other:?}"),
            }
        };
        assert_eq!(ack["t"], "ack");
        assert_eq!(ack["ok"], false);
    }
}
