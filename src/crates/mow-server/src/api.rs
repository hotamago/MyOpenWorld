//! Định tuyến HTTP, tách hẳn khỏi socket.
//!
//! [`route`] là một hàm nhận `(phương thức, đường dẫn, query, thân)` và trả về
//! `(mã, JSON)`. Không có `TcpStream` nào trong chữ ký, nên **toàn bộ API kiểm
//! được bằng test thường** — không cần mở cổng, không cần chờ, không có bài
//! test nào phải `sleep`.
//!
//! Cùng một thủ thuật với `Transport` ở `mow-llm`: phần khó không phải là ổ
//! cắm mạng, mà là hình dạng dữ liệu đi qua nó.
//!
//! ## Kiểu trên dây là **tường minh**, không đoán
//!
//! Engine phân biệt `Uint` (định danh thực thể) với `Int` (số có thang tự do),
//! và một `core.walk` gửi `who` dưới dạng `Int` bị từ chối với
//! `wrong_type`. JSON không có sự phân biệt đó, nên nếu server đoán thì nó sẽ
//! đoán sai ở đúng chỗ khó tìm nhất.
//!
//! Nên client nói rõ:
//!
//! ```json
//! { "kind": "core.walk", "fields": { "who": {"entity": 3}, "dx": -1, "dy": 0 } }
//! ```
//!
//! `{"entity": N}` → `Uint`. Số trần → `Int`. Chuỗi → `Text`.
//!
//! ## Địa hình trả về dạng cột song song, không phải mảng object
//!
//! `{"material": [...], "biome": [...]}` thay vì `[{material, biome}, ...]`.
//! Một vùng 49×49 là 2401 ô; dạng object lặp tên khóa 2401 lần và phồng payload
//! lên gấp ba. Dạng cột cũng giải mã thẳng vào `TypedArray` ở phía client.

use crate::game::{Game, TAM_NHIN};
use mow_core::{Command, EventSeq, Value};
use serde_json::{json, Map, Value as J};

/// Một câu trả lời HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// Mã trạng thái.
    pub status: u16,
    /// Thân, luôn là JSON.
    pub body: String,
}

impl Reply {
    fn ok(v: &J) -> Reply {
        Reply {
            status: 200,
            body: v.to_string(),
        }
    }
    fn loi(status: u16, msg: &str) -> Reply {
        Reply {
            status,
            body: json!({ "error": msg }).to_string(),
        }
    }
}

/// Đọc một tham số số nguyên từ query string.
fn q_int(query: &str, key: &str) -> Option<i64> {
    query.split('&').find_map(|p| {
        let (k, v) = p.split_once('=')?;
        if k == key {
            v.parse().ok()
        } else {
            None
        }
    })
}

/// Chuyển một giá trị JSON sang [`Value`] của engine, theo quy tắc tường minh
/// ở tài liệu module.
fn sang_value(j: &J) -> Result<Value, String> {
    Ok(match j {
        J::Null => Value::Null,
        J::Bool(b) => Value::Bool(*b),
        J::String(s) => Value::Text(s.clone()),
        J::Number(n) => Value::Int(
            n.as_i64()
                .ok_or_else(|| format!("số không vừa i64: {n}"))?,
        ),
        J::Array(a) => Value::List(a.iter().map(sang_value).collect::<Result<_, _>>()?),
        J::Object(o) => {
            // `{"entity": N}` là cách duy nhất để nói "đây là một định danh".
            if let (1, Some(e)) = (o.len(), o.get("entity")) {
                let n = e.as_u64().ok_or("`entity` phải là số nguyên không âm")?;
                return Ok(Value::Uint(n));
            }
            if let (1, Some(e)) = (o.len(), o.get("uint")) {
                let n = e.as_u64().ok_or("`uint` phải là số nguyên không âm")?;
                return Ok(Value::Uint(n));
            }
            Value::Map(
                o.iter()
                    .map(|(k, v)| sang_value(v).map(|v| (k.clone(), v)))
                    .collect::<Result<_, _>>()?,
            )
        }
    })
}

/// Chuyển [`Value`] của engine sang JSON để hiển thị.
fn tu_value(v: &Value) -> J {
    match v {
        Value::Null => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Int(i) => J::from(*i),
        // `u64` vượt 2^53 không biểu diễn đúng bằng `Number` của JS (`§22.10`),
        // nên định danh đi ra dưới dạng chuỗi kèm nhãn. Client biết đọc.
        Value::Uint(u) => json!({ "entity": u.to_string() }),
        Value::Fixed(f) => json!({ "fx": f.raw() }),
        Value::Text(t) => J::String(t.clone()),
        Value::Bytes(b) => J::String(hex(b)),
        Value::List(l) => J::Array(l.iter().map(tu_value).collect()),
        Value::Map(m) => J::Object(m.iter().map(|(k, v)| (k.clone(), tu_value(v))).collect()),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Định tuyến một yêu cầu.
pub fn route(g: &mut Game, method: &str, path: &str, query: &str, body: &str) -> Reply {
    match (method, path) {
        ("GET", "/api/meta") => meta(g),
        ("GET", "/api/tiles") => tiles(g, query),
        ("GET", "/api/entities") => entities(g),
        ("GET", "/api/events") => events(g, query),
        ("POST", "/api/command") => command(g, body),
        ("POST", "/api/view") => view(g, body),
        ("GET" | "POST", _) => Reply::loi(404, "không có đường dẫn này"),
        _ => Reply::loi(405, "phương thức không được hỗ trợ"),
    }
}

fn meta(g: &Game) -> Reply {
    Reply::ok(&json!({
        "world": g.world().0,
        "seed": g.seed().to_string(),
        "tick": g.tick().0,
        "state_hash": g.state_hash().to_hex(),
        "avatar": g.avatar().get().to_string(),
        "z": g.z(),
        "view_radius": TAM_NHIN,
        "event_cursor": g.last_seq().0,
    }))
}

fn tiles(g: &mut Game, query: &str) -> Reply {
    let x0 = q_int(query, "x").unwrap_or(0);
    let y0 = q_int(query, "y").unwrap_or(0);
    let w = q_int(query, "w").unwrap_or(33).clamp(1, 129);
    let h = q_int(query, "h").unwrap_or(33).clamp(1, 129);
    if let Some(z) = q_int(query, "z") {
        g.set_z(z);
    }

    let n = (w * h) as usize;
    let mut material = Vec::with_capacity(n);
    let mut surface = Vec::with_capacity(n);
    let mut drop = Vec::with_capacity(n);
    let mut biome = Vec::with_capacity(n);
    let mut height = Vec::with_capacity(n);
    let mut river = Vec::with_capacity(n);

    for dy in 0..h {
        for dx in 0..w {
            let t = g.tile(x0 + dx, y0 + dy);
            material.push(J::from(t.material));
            surface.push(J::from(t.surface));
            drop.push(J::from(t.drop));
            biome.push(J::from(t.biome));
            height.push(J::from(t.height));
            river.push(J::from(u8::from(t.river)));
        }
    }

    Reply::ok(&json!({
        "x": x0, "y": y0, "w": w, "h": h, "z": g.z(),
        "material": material,
        "surface": surface,
        "drop": drop,
        "biome": biome,
        "height": height,
        "river": river,
    }))
}

fn entities(g: &Game) -> Reply {
    let s = g.sim().store();
    let ds: Vec<J> = g
        .placed()
        .into_iter()
        .map(|id| {
            let la_vat_pham = s.attr_int(id, "item.nutrition").is_some()
                || s.attr_text(id, "item.def").is_some();
            json!({
                "id": id.get().to_string(),
                "name": s.attr_text(id, "core.name").unwrap_or("?"),
                "x": s.attr_int(id, "core.pos.x").unwrap_or(0),
                "y": s.attr_int(id, "core.pos.y").unwrap_or(0),
                "kind": if la_vat_pham { "item" } else { "being" },
                "is_avatar": id == g.avatar(),
                "hunger": s.attr_int(id, "need.hunger"),
            })
        })
        .collect();
    Reply::ok(&json!({ "entities": ds }))
}

fn events(g: &Game, query: &str) -> Reply {
    let after = q_int(query, "after").unwrap_or(0).max(0) as u64;
    let ds: Vec<J> = g
        .events_after(EventSeq(after))
        .into_iter()
        .rev()
        .take(60)
        .map(|e| {
            json!({
                "seq": e.seq.0,
                "tick": e.tick.0,
                "kind": e.kind.0,
                "actor": e.actor.map(|a| a.get().to_string()),
                "payload": tu_value(&e.payload),
            })
        })
        .collect();
    // `rev().take()` lấy phần **mới nhất**; đảo lại để client nhận theo thứ tự
    // thời gian. Không đảo thì nhật ký chạy ngược, và đó là loại lỗi người ta
    // nhìn thấy nhưng không tin vào mắt mình.
    let ds: Vec<J> = ds.into_iter().rev().collect();
    Reply::ok(&json!({ "cursor": g.last_seq().0, "events": ds }))
}

fn command(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let Some(kind) = j.get("kind").and_then(J::as_str) else {
        return Reply::loi(400, "thiếu `kind`");
    };
    let fields = j.get("fields").cloned().unwrap_or_else(|| J::Object(Map::new()));
    let payload = match sang_value(&fields) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &e),
    };

    let cmd = Command::new(kind, g.world(), payload);
    match g.apply(&cmd) {
        Ok(()) => Reply::ok(&json!({
            "ok": true,
            "tick": g.tick().0,
            "state_hash": g.state_hash().to_hex(),
            "event_cursor": g.last_seq().0,
        })),
        // Lệnh bị từ chối **không phải** lỗi server: nó là một câu trả lời hợp
        // lệ của thế giới ("bạn không với tới"). Trả 200 kèm `ok: false` để
        // client phân biệt được với một server hỏng.
        Err(f) => Reply::ok(&json!({
            "ok": false,
            "code": format!("{:?}", f.code),
            "error": f.to_string(),
            "tick": g.tick().0,
        })),
    }
}

fn view(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    if let Some(z) = j.get("z").and_then(J::as_i64) {
        g.set_z(z);
    }
    // `§P6.8`: đổi lát là **query**, không phải command. Nó không ghi event và
    // không đổi state hash — và bài test dưới giữ đúng lời hứa đó.
    Reply::ok(&json!({ "z": g.z(), "state_hash": g.state_hash().to_hex() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g() -> Game {
        Game::new(42)
    }

    /// Vị trí avatar đọc qua chính API, không đọc tắt vào `Store`.
    fn vi_tri_avatar(g: &mut Game) -> (i64, i64) {
        let e = get(g, "/api/entities", "");
        let me = e["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["is_avatar"] == true)
            .expect("avatar phải có trong danh sách")
            .clone();
        (me["x"].as_i64().unwrap(), me["y"].as_i64().unwrap())
    }

    fn get(g: &mut Game, path: &str, query: &str) -> J {
        let r = route(g, "GET", path, query, "");
        assert_eq!(r.status, 200, "{}", r.body);
        serde_json::from_str(&r.body).expect("JSON hợp lệ")
    }

    #[test]
    fn meta_co_du_thu_client_can() {
        let v = get(&mut g(), "/api/meta", "");
        for k in ["world", "seed", "tick", "state_hash", "avatar", "z"] {
            assert!(v.get(k).is_some(), "thiếu `{k}` trong /api/meta");
        }
    }

    #[test]
    fn seed_va_avatar_di_ra_duoi_dang_chuoi() {
        // `§22.10`: `u64` vượt 2^53 mất chính xác khi qua `Number` của JS.
        let v = get(&mut g(), "/api/meta", "");
        assert!(v["seed"].is_string(), "seed phải là chuỗi: {}", v["seed"]);
        assert!(v["avatar"].is_string());
    }

    #[test]
    fn tiles_tra_ve_dung_so_o() {
        let v = get(&mut g(), "/api/tiles", "x=0&y=0&w=9&h=7");
        assert_eq!(v["material"].as_array().unwrap().len(), 63);
        assert_eq!(v["biome"].as_array().unwrap().len(), 63);
        assert_eq!(v["height"].as_array().unwrap().len(), 63);
    }

    #[test]
    fn tiles_chan_vung_qua_lon() {
        // Không có trần, một client gõ nhầm `w=100000` sẽ bắt server sinh 10 tỉ
        // ô và treo cả trò chơi.
        let v = get(&mut g(), "/api/tiles", "x=0&y=0&w=100000&h=100000");
        assert_eq!(v["w"], 129);
        assert_eq!(v["h"], 129);
    }

    #[test]
    fn tiles_theo_lat_z() {
        let mut g = g();
        let mat_dat = get(&mut g, "/api/meta", "")["z"].as_i64().unwrap();
        let tren = get(&mut g, "/api/tiles", &format!("x=0&y=0&w=1&h=1&z={}", mat_dat + 40));
        assert_eq!(tren["material"][0], "air");
    }

    #[test]
    fn entities_co_avatar_va_vat_pham() {
        let v = get(&mut g(), "/api/entities", "");
        let ds = v["entities"].as_array().unwrap();
        assert!(ds.iter().any(|e| e["is_avatar"] == true));
        assert!(ds.iter().any(|e| e["kind"] == "item"));
        assert!(ds.iter().any(|e| e["kind"] == "being" && e["is_avatar"] == false));
    }

    #[test]
    fn command_di_duoc_va_doi_vi_tri() {
        let mut g = g();
        let truoc = vi_tri_avatar(&mut g);
        let av = g.avatar().get();
        let body = json!({
            "kind": "core.walk",
            "fields": { "who": {"entity": av}, "dx": 1, "dy": 0 }
        })
        .to_string();
        let r = route(&mut g, "POST", "/api/command", "", &body);
        assert_eq!(r.status, 200, "{}", r.body);
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["ok"], true, "{}", r.body);

        let sau = vi_tri_avatar(&mut g);
        assert_eq!(sau.0, truoc.0 + 1, "đi một bước phải dịch đúng một ô");
        assert_eq!(sau.1, truoc.1);
    }

    #[test]
    fn entity_gui_dang_so_tran_thi_bao_loi_ro_rang() {
        // Đây chính là cái bẫy mà quy tắc `{"entity": N}` tồn tại để chặn:
        // engine đòi `Uint`, JSON chỉ có "number".
        let mut g = g();
        let av = g.avatar().get();
        let body = json!({
            "kind": "core.walk",
            "fields": { "who": av, "dx": 1, "dy": 0 }
        })
        .to_string();
        let r = route(&mut g, "POST", "/api/command", "", &body);
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["ok"], false);
        assert!(
            v["error"].as_str().unwrap().contains("who"),
            "thông báo phải chỉ đúng trường: {}",
            v["error"]
        );
    }

    #[test]
    fn lenh_bi_tu_choi_khong_phai_loi_server() {
        // Đi 5 ô một bước là vi phạm luật thế giới, không phải server hỏng.
        let mut g = g();
        let av = g.avatar().get();
        let body = json!({
            "kind": "core.walk",
            "fields": { "who": {"entity": av}, "dx": 5, "dy": 0 }
        })
        .to_string();
        let r = route(&mut g, "POST", "/api/command", "", &body);
        assert_eq!(r.status, 200);
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["code"], "PreconditionFailed");
    }

    #[test]
    fn doi_lat_z_khong_doi_state_hash() {
        // `§P6.8`: kéo camera và đổi lát là **query**. Nếu chúng ghi event thì
        // lịch sử thế giới phụ thuộc vào việc người chơi đã nhìn đi đâu.
        let mut g = g();
        let truoc = g.state_hash();
        let r = route(&mut g, "POST", "/api/view", "", &json!({"z": -50}).to_string());
        assert_eq!(r.status, 200);
        assert_eq!(g.z(), -50);
        assert_eq!(g.state_hash(), truoc, "đổi lát đã ghi vào thế giới");
    }

    #[test]
    fn tiles_doi_z_cung_khong_doi_state_hash() {
        let mut g = g();
        let truoc = g.state_hash();
        let _ = get(&mut g, "/api/tiles", "x=0&y=0&w=3&h=3&z=-99");
        assert_eq!(g.state_hash(), truoc);
    }

    #[test]
    fn events_theo_thu_tu_thoi_gian() {
        let mut g = g();
        for _ in 0..12 {
            g.tick_once();
        }
        let v = get(&mut g, "/api/events", "after=0");
        let ds = v["events"].as_array().unwrap();
        assert!(!ds.is_empty(), "12 tick mà không có sự kiện nào");
        let seqs: Vec<u64> = ds.iter().map(|e| e["seq"].as_u64().unwrap()).collect();
        let mut sap = seqs.clone();
        sap.sort_unstable();
        assert_eq!(seqs, sap, "nhật ký chạy ngược thời gian");
    }

    #[test]
    fn events_cursor_khong_gui_lai_cai_da_gui() {
        let mut g = g();
        for _ in 0..8 {
            g.tick_once();
        }
        let v1 = get(&mut g, "/api/events", "after=0");
        let cur = v1["cursor"].as_u64().unwrap();
        let v2 = get(&mut g, "/api/events", &format!("after={cur}"));
        assert!(
            v2["events"].as_array().unwrap().is_empty(),
            "cursor không chặn được sự kiện đã gửi"
        );
    }

    #[test]
    fn duong_dan_la_tra_404() {
        let mut g = g();
        assert_eq!(route(&mut g, "GET", "/api/khong-co", "", "").status, 404);
    }

    #[test]
    fn than_hong_tra_400_chu_khong_panic() {
        let mut g = g();
        let r = route(&mut g, "POST", "/api/command", "", "{ khong phai json");
        assert_eq!(r.status, 400);
    }

    #[test]
    fn thieu_kind_bao_ro() {
        let mut g = g();
        let r = route(&mut g, "POST", "/api/command", "", r#"{"fields":{}}"#);
        assert_eq!(r.status, 400);
        assert!(r.body.contains("kind"));
    }
}
