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
use mow_core::EntityId;
use mow_core::{Command, EventSeq, Value};
use serde_json::{json, Map, Value as J};
use std::collections::HashMap;

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
        J::Number(n) => Value::Int(n.as_i64().ok_or_else(|| format!("số không vừa i64: {n}"))?),
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
        ("GET", "/api/blocks") => blocks(g),
        ("GET", "/api/events") => events(g, query),
        ("GET", "/api/cause") => cause(g, query),
        ("POST", "/api/command") => command(g, body),
        ("POST", "/api/view") => view(g, body),
        ("POST", "/api/speed") => speed(g, body),
        ("POST", "/api/goto") => goto(g, body),
        ("POST", "/api/look") => look(g, body),
        ("POST", "/api/genesis") => genesis(g, body),
        ("POST", "/api/preview") => preview(g, body),
        ("POST", "/api/commit") => commit(g, body),
        ("POST", "/api/build") => build(g, body),
        // Trình duyệt hỏi trước bằng `OPTIONS` với mọi `POST` mang JSON. Trả
        // lời rỗng là đủ — các tiêu đề CORS được gắn ở tầng ngoài cho **mọi**
        // phản hồi, nên chỗ này chỉ cần đừng trả 405.
        ("OPTIONS", _) => Reply::ok(&json!({})),
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
        // Cái nhìn của vị thần, không phải một thân xác. Gửi ra vì client cần
        // biết mở khung nhìn ở đâu; **không** phải một thực thể, nên không có
        // định danh nào ở đây.
        "eye": [g.eye().0, g.eye().1],
        "z": g.z(),
        "view_radius": TAM_NHIN,
        "event_cursor": g.last_seq().0,
        "speed_milli": g.speed_milli(),
        // Tổng số bước còn lại của mọi kế hoạch đi. Client dùng nó để biết khi
        // nào xóa đường vẽ: `0` nghĩa là không ai còn đang đi tới đâu cả.
        "steps_remaining": g.pending_steps(),
        // Độ dài lịch sử: xem trước phát lại toàn bộ nhật ký, nên con số này
        // là thứ báo trước khi console True God bắt đầu chậm.
        "built_cells": g.built_cells(),
        "history_len": g.journal_len(),
        "history_limit": crate::game::PREVIEW_JOURNAL_LIMIT,
        "max_speed_milli": crate::game::MAX_SPEED_MILLI,
    }))
}

/// Bảng tra tên dùng chung cho `material`, `surface` và `biome` của một lô ô
/// trả về từ [`tiles`].
///
/// Ba trường đó dùng chung **một** không gian tên — vật liệu, bề mặt và quần
/// xã đều là chuỗi định danh của content pack/worldgen, không phải ba tập
/// riêng biệt — nên một bảng chung vừa đơn giản hơn ba bảng, vừa nhỏ hơn: một
/// khung nhìn ~4000 ô đo được chỉ có vài chục tên khác nhau, và `material`
/// trùng `surface` ở phần lớn bề mặt bằng phẳng (đứng ngay trên mặt đất thì
/// "vật liệu tại lát z" và "vật liệu của ô rắn trên cùng" là cùng một thứ).
///
/// Chỉ số được gán theo thứ tự **gặp lần đầu** khi [`tiles`] duyệt ô theo thứ
/// tự hàng-rồi-cột (`dy` ngoài, `dx` trong) — **không** phải thứ tự nội bộ mà
/// `HashMap` tình cờ đi qua, thứ phụ thuộc hash của tiến trình và có thể ra
/// một bảng khác nhau giữa hai lần chạy dù cùng một khung nhìn. `§P10.2` cấm
/// đúng điều đó: cùng đầu vào phải cho cùng đầu ra. Vì vậy `HashMap` ở đây chỉ
/// dùng để tra "tên này đã có chỉ số chưa" — thứ tự thật nằm ở `Vec names`, nó
/// chỉ được đẩy thêm vào cuối, không bao giờ bị duyệt để tạo ra thứ tự.
struct NameTable {
    /// `names[i]` là tên tại chỉ mục `i` — cũng chính là mảng `names` gửi ra
    /// trên dây.
    names: Vec<String>,
    /// Tra ngược tên -> chỉ mục, để không thêm trùng một tên hai lần.
    index: HashMap<String, u32>,
}

impl NameTable {
    fn new() -> Self {
        Self {
            names: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Trả về chỉ mục của `s` trong bảng, gán một chỉ mục mới — bằng đúng độ
    /// dài hiện tại của `names` — nếu đây là lần đầu gặp `s`.
    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.index.get(s) {
            return i;
        }
        let i = self.names.len() as u32;
        self.names.push(s.to_owned());
        self.index.insert(s.to_owned(), i);
        i
    }
}

/// Trả về một lô ô địa hình quanh `(x, y)`.
///
/// ## Tại sao chuỗi biến thành chỉ mục
///
/// Một khung nhìn ~4000 ô đo được cho ra khoảng 350 KB JSON khi `material`,
/// `surface`, `biome` là ba mảng **chuỗi** lặp lại cùng vài chục giá trị hàng
/// nghìn lần (`"topsoil"` viết ra 4000 lần nặng hơn hẳn số `1` viết ra 4000
/// lần). Đưa chúng qua một bảng tra `names` dùng chung ([`NameTable`]) và ba
/// mảng số nguyên nhỏ đưa con số đó xuống khoảng 30 KB — không đổi **ý
/// nghĩa** của dữ liệu, chỉ đổi cách nó được viết ra dây.
///
/// Cái lợi nằm ở băng thông và thời gian `JSON.parse` phía client — cả hai đã
/// thu được ngay khi phản hồi này được gửi/đọc. Client (`api/game.ts`) giải
/// mã chỉ mục ngược lại thành chuỗi ngay khi nhận, để phần còn lại của giao
/// diện (`render/*`) không phải biết gì về đổi thay này. Bước kế tiếp — khi
/// `render/*` rảnh tay để đổi theo — là bỏ luôn bước giải mã đó và để chỉ mục
/// chạy thẳng tới tầng vẽ.
fn tiles(g: &mut Game, query: &str) -> Reply {
    let x0 = q_int(query, "x").unwrap_or(0);
    let y0 = q_int(query, "y").unwrap_or(0);
    let w = q_int(query, "w").unwrap_or(33).clamp(1, 129);
    let h = q_int(query, "h").unwrap_or(33).clamp(1, 129);
    if let Some(z) = q_int(query, "z") {
        g.set_z(z);
    }

    let n = (w * h) as usize;
    let mut names = NameTable::new();
    let mut material = Vec::with_capacity(n);
    let mut surface = Vec::with_capacity(n);
    let mut drop = Vec::with_capacity(n);
    let mut built = Vec::with_capacity(n);
    let mut biome = Vec::with_capacity(n);
    let mut height = Vec::with_capacity(n);
    let mut river = Vec::with_capacity(n);

    for dy in 0..h {
        for dx in 0..w {
            let t = g.tile(x0 + dx, y0 + dy);
            material.push(names.intern(&t.material));
            surface.push(names.intern(&t.surface));
            drop.push(J::from(t.drop));
            built.push(J::from(u8::from(t.built)));
            biome.push(names.intern(t.biome));
            height.push(J::from(t.height));
            river.push(J::from(u8::from(t.river)));
        }
    }

    Reply::ok(&json!({
        "x": x0, "y": y0, "w": w, "h": h, "z": g.z(),
        // Bảng tra: chỉ mục -> chuỗi, theo thứ tự gặp lần đầu (xem tài liệu
        // `NameTable`). Vài chục phần tử, không phải vài nghìn.
        "names": names.names,
        "material": material,
        "surface": surface,
        "drop": drop,
        "built": built,
        "biome": biome,
        "height": height,
        "river": river,
    }))
}

/// Bảng vật liệu của content pack đang nạp.
///
/// Client dựng bảng tra từ đây thay vì giữ một bản chép tay. Hai bản chép tay
/// là hai chỗ để chúng lệch nhau, và khi lệch thì người chơi thấy một màu khác
/// màu mà pack khai — một sai lệch không có test nào bắt được.
fn blocks(g: &Game) -> Reply {
    let Some(reg) = g.blocks() else {
        // Không nạp được pack: nói thẳng bằng một danh sách rỗng, để client
        // dùng bảng dự phòng thay vì chờ mãi.
        return Reply::ok(&json!({ "blocks": [], "loaded": false }));
    };
    let ds: Vec<J> = reg
        .iter()
        .map(|b| {
            json!({
                "id": b.id,
                "name": { "en": b.name.en, "vi": b.name.vi },
                // Màu đi ra dạng chuỗi hex: client đọc được bằng mắt khi gỡ lỗi,
                // và không có chuyện `0x0d1014` bị đọc thành số thập phân.
                "color": format!("#{:06x}", b.color & 0x00ff_ffff),
                "liquid": b.liquid,
                "walkable": b.walkable,
                "hardness": b.hardness,
                "tags": b.tags,
            })
        })
        .collect();
    Reply::ok(&json!({ "blocks": ds, "loaded": true }))
}

fn entities(g: &Game) -> Reply {
    let s = g.sim().store();
    let ds: Vec<J> = g
        .placed()
        .into_iter()
        .map(|id| {
            let la_vat_pham =
                s.attr_int(id, "item.nutrition").is_some() || s.attr_text(id, "item.def").is_some();
            json!({
                "id": id.get().to_string(),
                "name": s.attr_text(id, "core.name").unwrap_or("?"),
                "x": s.attr_int(id, "core.pos.x").unwrap_or(0),
                "y": s.attr_int(id, "core.pos.y").unwrap_or(0),
                "kind": if la_vat_pham { "item" } else { "being" },
                "hunger": s.attr_int(id, "need.hunger"),
                // Vai và ý định: `§18.3` đòi giao diện trả lời được "vì sao nó
                // làm thế". Không gửi ra thì câu đó chỉ trả lời được bằng cách
                // đoán từ chuyển động — và đoán thì sai.
                "role": s.attr_text(id, "npc.role"),
                "intent": s.attr_text(id, "npc.intent"),
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
                "cause": e.cause.map(|c| c.0),
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

/// Chuỗi nhân quả của một sự kiện, từ nó ngược về gốc (`§18.10`).
///
/// Cạnh nhân quả được ghi **lúc tạo** sự kiện chứ không suy ngược về sau. Một
/// chuỗi đoán ra thì tệ hơn không có chuỗi nào, vì người xem sẽ tin nó — và đó
/// là lý do endpoint này chỉ đọc, không dựng lại gì.
fn cause(g: &Game, query: &str) -> Reply {
    let Some(seq) = q_int(query, "seq").filter(|v| *v >= 0) else {
        return Reply::loi(400, "thiếu `seq`");
    };
    let chain: Vec<J> = g
        .sim()
        .log()
        .cause_chain(EventSeq(seq as u64), 32)
        .into_iter()
        .map(|e| {
            json!({
                "seq": e.seq.0,
                "tick": e.tick.0,
                "kind": e.kind.0,
                "actor": e.actor.map(|a| a.get().to_string()),
                "summary": crate::preview::summarize(&e.kind.0, &e.payload),
            })
        })
        .collect();
    Reply::ok(&json!({ "chain": chain }))
}

fn command(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let Some(kind) = j.get("kind").and_then(J::as_str) else {
        return Reply::loi(400, "thiếu `kind`");
    };
    let fields = j
        .get("fields")
        .cloned()
        .unwrap_or_else(|| J::Object(Map::new()));
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

/// Đổi tốc độ thời gian.
///
/// Là **query**, không phải command: tốc độ không đổi kết quả mô phỏng, chỉ đổi
/// tốc độ thời gian thật trôi giữa hai tick. Cho nó ghi event sẽ làm lịch sử
/// thế giới phụ thuộc vào việc người chơi có tua nhanh hay không — đúng thứ
/// `§22.46` cấm.
fn speed(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let Some(v) = j.get("speed_milli").and_then(J::as_u64) else {
        return Reply::loi(400, "thiếu `speed_milli`");
    };
    g.set_speed_milli(u32::try_from(v).unwrap_or(u32::MAX));
    Reply::ok(&json!({
        "speed_milli": g.speed_milli(),
        "state_hash": g.state_hash().to_hex(),
    }))
}

/// Đọc `{kind, fields}` thành một `Command`.
fn read_command(g: &Game, j: &J) -> Result<Command, Reply> {
    let Some(kind) = j.get("kind").and_then(J::as_str) else {
        return Err(Reply::loi(400, "thiếu `kind`"));
    };
    let fields = j
        .get("fields")
        .cloned()
        .unwrap_or_else(|| J::Object(Map::new()));
    let payload = sang_value(&fields).map_err(|e| Reply::loi(400, &e))?;
    Ok(Command::new(kind, g.world(), payload))
}

/// Xem trước một can thiệp. **Không** đổi thế giới.
fn preview(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let cmd = match read_command(g, &j) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match g.preview(&cmd) {
        Ok(d) => Reply::ok(&d.to_json()),
        // Không dựng lại được thế giới là lỗi **của server**, không phải của
        // lệnh: nói bằng 500 để client phân biệt "lệnh này sẽ hỏng" với "công
        // cụ xem trước đang hỏng". Hai thứ đó đòi hai phản ứng khác nhau.
        Err(e) => Reply::loi(500, &e),
    }
}

/// Khắc một can thiệp vào thế giới, chỉ khi thế giới chưa đổi.
fn commit(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let Some(base) = j.get("base_hash").and_then(J::as_str).map(str::to_owned) else {
        return Reply::loi(400, "thiếu `base_hash` — mọi lần khắc phải kèm hash đã xem");
    };
    let cmd = match read_command(g, &j) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match g.commit_checked(&cmd, &base) {
        Ok(d) => Reply::ok(&json!({
            "ok": true,
            "after_hash": d.after_hash.to_hex(),
            "changes": d.changes.len(),
            "events": d.events.len(),
        })),
        // Từ chối **không** phải lỗi server: đó là thế giới nói "không, không
        // phải như thế nữa". 200 kèm `ok: false` để client hiện lại preview.
        Err(r) => Reply::ok(&json!({
            "ok": false,
            "reason": match r {
                crate::preview::Refusal::WorldMoved { .. } => "world_moved",
                crate::preview::Refusal::CommandFails(_) => "command_fails",
            },
            "message": r.to_string(),
            "state_hash": g.state_hash().to_hex(),
        })),
    }
}

/// Khắc địa hình: đổi vật liệu một ô.
///
/// Đây là quyền năng True God tác động lên **vật chất** chứ không lên thực thể
/// (`§16`). Nó không đi qua `Sim` vì địa hình là `seed + delta` (`§7.2`), nhưng
/// nó **có** vào `state_hash` — nếu không, hai thế giới có hai ngôi làng khác
/// nhau sẽ cho cùng một hash.
fn build(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let (Some(x), Some(y), Some(m)) = (
        j.get("x").and_then(J::as_i64),
        j.get("y").and_then(J::as_i64),
        j.get("material").and_then(J::as_str),
    ) else {
        return Reply::loi(400, "cần `x`, `y` và `material`");
    };
    // Vật liệu phải có định nghĩa trong pack. Không kiểm thì một lỗi gõ cho ra
    // một ô màu tím trên bản đồ và không có gì báo.
    if let Some(reg) = g.blocks() {
        if !reg.contains(m) {
            return Reply::ok(&json!({
                "ok": false,
                "error": format!("không có vật liệu `{m}` trong content pack"),
            }));
        }
    }
    g.set_cell(x, y, m);
    Reply::ok(
        &json!({ "ok": true, "state_hash": g.state_hash().to_hex(), "built_cells": g.built_cells() }),
    )
}

/// Đặt đích tới cho avatar. Đây là hành động chính của chuột.
fn goto(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let (Some(x), Some(y)) = (
        j.get("x").and_then(J::as_i64),
        j.get("y").and_then(J::as_i64),
    ) else {
        return Reply::loi(400, "thiếu `x` hoặc `y`");
    };
    // `who` là **bắt buộc**. Người chơi không có thân xác để ra lệnh cho, nên
    // "đi tới đó" luôn là một mệnh lệnh gửi cho **một ai đó** — một quyền năng
    // của thần, không phải một bước chân của thần.
    let Some(who) = j.get("who").and_then(J::as_str).and_then(parse_entity) else {
        return Reply::loi(400, "thiếu `who`: phải nói rõ ra lệnh cho ai");
    };
    if !g.sim().store().contains(who) {
        return Reply::loi(404, "không có thực thể đó");
    }
    // `{"cancel": true}` là chuột phải: dừng tại chỗ. Không có nó thì cách duy
    // nhất để dừng là bấm vào chính ô đang đứng, và đó là một thao tác người
    // chơi phải tự nghĩ ra.
    if j.get("cancel").and_then(J::as_bool) == Some(true) {
        g.cancel_destination(who);
        return Reply::ok(&json!({ "steps": 0, "outcome": "cancelled", "walkable": true }));
    }
    let (steps, why) = g.set_destination(who, (x, y));
    // Trần 400 điểm: một đường dài hơn thế thì vẽ ra cũng không đọc được, và
    // gửi cả nghìn cặp tọa độ mỗi cú bấm là trả giá cho thứ không ai nhìn.
    let path: Vec<J> = g
        .planned_path(who)
        .into_iter()
        .take(400)
        .map(|(px, py)| json!([px, py]))
        .collect();
    Reply::ok(&json!({
        "steps": steps,
        "outcome": why,
        "walkable": g.walkable(x, y),
        "path": path,
    }))
}

/// Đọc một định danh thực thể từ chuỗi.
///
/// **Chuỗi**, không phải số: `§22.10` cấm cho id 64-bit đi qua `Number` của
/// JavaScript, vốn chỉ giữ trọn 53 bit. Một id lớn hơn thế sẽ về tới đây đã bị
/// làm tròn, và nó vẫn là một id **hợp lệ** — chỉ là của người khác.
fn parse_entity(s: &str) -> Option<EntityId> {
    s.parse::<u64>().ok().filter(|v| *v != 0).map(EntityId::new)
}

/// Khởi nguyên một thế giới mới từ seed người chơi chọn.
///
/// Thay **toàn bộ** `Game` chứ không cố gột rửa cái cũ: thế giới là hàm thuần
/// của seed (`§P2`), nên dựng lại từ đầu là cách duy nhất chắc chắn không còn
/// sót một mảnh trạng thái nào của ván trước. Gột rửa từng trường là cách để
/// quên đúng một trường, và cái quên đó sẽ hiện ra dưới dạng một `state_hash`
/// không khớp với chính seed của nó.
///
/// Content pack được nạp lại sau, vì nó **không** thuộc về seed — nó là dữ liệu
/// bên ngoài, và một thế giới mới vẫn phải biết những vật liệu mà tiến trình
/// này đã nạp lúc khởi động.
fn genesis(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    // Seed đi ra dạng **chuỗi** (`§22.10`); nhận cả số cho tiện gõ tay, nhưng
    // chuỗi mới là đường chính thức.
    let seed = match j.get("seed") {
        Some(J::String(t)) => t.parse::<u64>().ok(),
        Some(J::Number(n)) => n.as_u64(),
        _ => None,
    };
    let Some(seed) = seed else {
        return Reply::loi(400, "thiếu `seed` (số nguyên không dấu, dạng chuỗi)");
    };

    let dir = g.content_dir().map(str::to_owned);
    *g = Game::new(seed);
    if let Some(d) = dir {
        // Nạp lỗi thì nói ra, đừng im lặng chạy tiếp với bảng màu dự phòng: một
        // thế giới vẽ bằng màu tím là thứ người chơi sẽ báo lỗi cho renderer.
        if let Err(e) = g.load_content(&d) {
            return Reply::ok(&json!({
                "seed": seed.to_string(),
                "state_hash": g.state_hash().to_hex(),
                "content_error": e,
            }));
        }
    }
    Reply::ok(&json!({
        "seed": seed.to_string(),
        "state_hash": g.state_hash().to_hex(),
    }))
}

/// Dời cái nhìn của vị thần. Không sinh sự kiện.
///
/// `§P6.8`: camera là một **truy vấn khung nhìn**, không phải một lệnh. Ghi một
/// sự kiện mỗi lần người chơi kéo bản đồ sẽ nhấn chìm nhật ký bằng thứ không
/// phải lịch sử của thế giới. Bài test dưới giữ đúng lời hứa đó bằng cách so
/// `state_hash` trước và sau.
fn look(g: &mut Game, body: &str) -> Reply {
    let j: J = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Reply::loi(400, &format!("thân không phải JSON: {e}")),
    };
    let (Some(x), Some(y)) = (
        j.get("x").and_then(J::as_i64),
        j.get("y").and_then(J::as_i64),
    ) else {
        return Reply::loi(400, "thiếu `x` hoặc `y`");
    };
    g.look_at(x, y);
    Reply::ok(&json!({ "eye": [x, y] }))
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

    fn get(g: &mut Game, path: &str, query: &str) -> J {
        let r = route(g, "GET", path, query, "");
        assert_eq!(r.status, 200, "{}", r.body);
        serde_json::from_str(&r.body).expect("JSON hợp lệ")
    }

    /// Một cư dân bất kỳ: vị thần không có thân xác, nên mọi mệnh lệnh đi lại
    /// đều gửi cho một ai đó.
    fn a_villager(g: &Game) -> EntityId {
        g.sim()
            .store()
            .with_attr("npc.role")
            .next()
            .expect("làng phải có cư dân")
    }

    /// Tra một chỉ mục trong lô ô về chuỗi qua bảng `names`.
    ///
    /// `material`/`surface`/`biome` không còn là chuỗi trực tiếp; bài test so
    /// bằng chuỗi thì phải đi qua hàm này thay vì so `v[field][i]` với một
    /// chuỗi trần.
    fn ten(v: &J, field: &str, i: usize) -> String {
        let idx = v[field][i].as_u64().expect("chỉ mục phải là số nguyên") as usize;
        v["names"][idx]
            .as_str()
            .expect("`names` phải toàn chuỗi")
            .to_owned()
    }

    #[test]
    fn meta_co_du_thu_client_can() {
        let v = get(&mut g(), "/api/meta", "");
        for k in ["world", "seed", "tick", "state_hash", "eye", "z"] {
            assert!(v.get(k).is_some(), "thiếu `{k}` trong /api/meta");
        }
    }

    #[test]
    fn seed_va_dinh_danh_di_ra_duoi_dang_chuoi() {
        // `§22.10`: `u64` vượt 2^53 mất chính xác khi qua `Number` của JS.
        let mut g = g();
        let v = get(&mut g, "/api/meta", "");
        assert!(v["seed"].is_string(), "seed phải là chuỗi: {}", v["seed"]);
        // Cái nhìn là **tọa độ ô**, không phải định danh — nó là số nguyên nhỏ
        // và đi ra dạng số là đúng.
        assert!(v["eye"].is_array(), "eye phải là cặp toạ độ: {}", v["eye"]);

        let e = get(&mut g, "/api/entities", "");
        for it in e["entities"].as_array().unwrap() {
            assert!(it["id"].is_string(), "định danh phải là chuỗi: {it}");
        }
    }

    #[test]
    fn genesis_dung_lai_the_gioi_tu_seed_moi() {
        let mut g = g();
        let truoc = get(&mut g, "/api/meta", "");
        let r = route(
            &mut g,
            "POST",
            "/api/genesis",
            "",
            &json!({ "seed": "777" }).to_string(),
        );
        assert_eq!(r.status, 200, "{}", r.body);
        let sau = get(&mut g, "/api/meta", "");
        assert_eq!(sau["seed"], "777");
        assert_ne!(truoc["state_hash"], sau["state_hash"], "thế giới không đổi");
        // Và nó phải là **đúng** thế giới của seed đó, không phải một thế giới
        // mang seed mới nhưng còn sót trạng thái của ván trước.
        let sach = Game::new(777);
        assert_eq!(sau["state_hash"], sach.state_hash().to_hex());
    }

    #[test]
    fn preflight_duoc_tra_loi_chu_khong_phai_405() {
        // Không có nhánh này thì **mọi** `POST` từ trình duyệt đều chết với một
        // `TypeError: Failed to fetch` không nói gì về nguyên nhân — và không
        // một bài test nào của server bắt được, vì `curl` không hỏi trước.
        let mut g = g();
        for path in [
            "/api/genesis",
            "/api/look",
            "/api/goto",
            "/api/command",
            "/api/build",
            "/api/speed",
            "/api/view",
            "/api/preview",
            "/api/commit",
        ] {
            let r = route(&mut g, "OPTIONS", path, "", "");
            assert_eq!(r.status, 200, "preflight cho `{path}` bị từ chối");
        }
    }

    #[test]
    fn genesis_thieu_seed_thi_bao_loi_chu_khong_doan() {
        let mut g = g();
        let r = route(&mut g, "POST", "/api/genesis", "", &json!({}).to_string());
        assert_eq!(r.status, 400, "{}", r.body);
    }

    #[test]
    fn look_khong_ghi_gi_vao_the_gioi() {
        // `§P6.8`: camera là truy vấn khung nhìn. Nếu nó ghi sự kiện thì mỗi cú
        // kéo bản đồ là một dòng lịch sử, và nhật ký hết đọc được.
        let mut g = g();
        let truoc = g.state_hash();
        let seq = g.last_seq();
        let r = route(
            &mut g,
            "POST",
            "/api/look",
            "",
            &json!({ "x": 100, "y": 200 }).to_string(),
        );
        assert_eq!(r.status, 200, "{}", r.body);
        assert_eq!(g.eye(), (100, 200));
        assert_eq!(g.state_hash(), truoc, "dời cái nhìn đã đổi thế giới");
        assert_eq!(g.last_seq(), seq, "dời cái nhìn đã ghi một sự kiện");
    }

    #[test]
    fn tiles_tra_ve_dung_so_o() {
        let v = get(&mut g(), "/api/tiles", "x=0&y=0&w=9&h=7");
        assert_eq!(v["material"].as_array().unwrap().len(), 63);
        assert_eq!(v["biome"].as_array().unwrap().len(), 63);
        assert_eq!(v["height"].as_array().unwrap().len(), 63);
    }

    #[test]
    fn building_an_unknown_material_is_refused() {
        // Một lỗi gõ không được phép cho ra một ô màu tím im lặng.
        let mut g = g();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core");
        g.load_content(root.to_str().unwrap()).unwrap();
        let before = g.built_cells();
        let r = route(
            &mut g,
            "POST",
            "/api/build",
            "",
            r#"{"x":1,"y":1,"material":"khong_co_that"}"#,
        );
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(g.built_cells(), before, "ô hỏng vẫn bị ghi vào thế giới");
    }

    #[test]
    fn building_changes_the_world_hash() {
        let mut g = g();
        let before = g.state_hash();
        let r = route(
            &mut g,
            "POST",
            "/api/build",
            "",
            r#"{"x":4,"y":4,"material":"path_gravel"}"#,
        );
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["ok"], true, "{}", r.body);
        assert_ne!(g.state_hash(), before);
    }

    #[test]
    fn built_cells_show_up_in_the_tile_batch() {
        // Client cần phân biệt công trình với thiên nhiên: một ô đá do người
        // xây và một ô đá do worldgen sinh ra trông giống nhau về vật liệu
        // nhưng khác hẳn về ý nghĩa.
        let mut g = g();
        let (ax, ay) = g.eye();
        g.set_cell(ax, ay, "path_gravel");

        let v = get(&mut g, "/api/tiles", &format!("x={ax}&y={ay}&w=1&h=1"));
        assert_eq!(v["built"][0], 1);
        assert_eq!(ten(&v, "surface", 0), "path_gravel");
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
        let tren = get(
            &mut g,
            "/api/tiles",
            &format!("x=0&y=0&w=1&h=1&z={}", mat_dat + 40),
        );
        assert_eq!(ten(&tren, "material", 0), "air");
    }

    #[test]
    fn tiles_names_khong_trung_va_chi_so_hop_le() {
        // `NameTable::intern` phải cấp đúng một chỉ số cho mỗi chuỗi khác
        // nhau, và mọi chỉ số ở ba mảng `material`/`surface`/`biome` phải trỏ
        // vào trong phạm vi `names` — một chỉ số ngoài phạm vi ở phía server
        // là lỗi, không phải thứ client phải tự vệ trước.
        let v = get(&mut g(), "/api/tiles", "x=-30&y=-20&w=60&h=40");
        let names = v["names"].as_array().unwrap();

        let mut seen = std::collections::HashSet::new();
        for name in names {
            let s = name.as_str().expect("`names` phải toàn chuỗi");
            assert!(seen.insert(s), "tên `{s}` xuất hiện hai lần trong `names`");
        }

        let n_names = names.len() as u64;
        for field in ["material", "surface", "biome"] {
            for idx in v[field].as_array().unwrap() {
                let i = idx.as_u64().expect("chỉ mục phải là số nguyên");
                assert!(
                    i < n_names,
                    "chỉ mục {i} ở `{field}` vượt quá bảng `names` ({n_names} phần tử)"
                );
            }
        }
    }

    #[test]
    fn tiles_giai_ma_lai_dung_nhu_truoc() {
        // Giải mã chỉ mục qua `names` phải cho lại đúng những chuỗi mà
        // `Game::tile` trả về trực tiếp — bảng chỉ mục không được đổi ý nghĩa
        // của dữ liệu, chỉ đổi cách nó được viết ra dây.
        let mut g = g();
        let (w, h) = (9, 7);
        let v = get(&mut g, "/api/tiles", &format!("x=0&y=0&w={w}&h={h}"));
        let names = v["names"].as_array().unwrap();
        let material = v["material"].as_array().unwrap();
        let surface = v["surface"].as_array().unwrap();
        let biome = v["biome"].as_array().unwrap();

        let mut i = 0usize;
        for dy in 0..h {
            for dx in 0..w {
                let t = g.tile(dx, dy);
                let lay = |arr: &[J]| -> &str {
                    names[arr[i].as_u64().unwrap() as usize].as_str().unwrap()
                };
                assert_eq!(lay(material), t.material, "material ở ô ({dx},{dy})");
                assert_eq!(lay(surface), t.surface, "surface ở ô ({dx},{dy})");
                assert_eq!(lay(biome), t.biome, "biome ở ô ({dx},{dy})");
                i += 1;
            }
        }
    }

    #[test]
    fn tiles_json_nho_hon_mot_nua_so_voi_kieu_cu() {
        // Đo trực tiếp cái lợi mà bảng chỉ mục hứa: với một khung nhìn 60×40,
        // JSON kiểu mới phải nhỏ hơn **một nửa** JSON kiểu cũ (ba mảng chuỗi
        // lặp lại). Dựng lại kiểu cũ ngay trong test bằng cách giải chỉ mục
        // ra chuỗi rồi gói lại — đây là đường cơ sở "trước khi đổi", không
        // phải một định dạng server còn phải sinh ra thật.
        let mut g = g();
        let r = route(&mut g, "GET", "/api/tiles", "x=0&y=0&w=60&h=40", "");
        let new_len = r.body.len();

        let v: J = serde_json::from_str(&r.body).unwrap();
        let names = v["names"].as_array().unwrap();
        let expand = |field: &str| -> Vec<J> {
            v[field]
                .as_array()
                .unwrap()
                .iter()
                .map(|idx| names[idx.as_u64().unwrap() as usize].clone())
                .collect()
        };
        let old = json!({
            "x": v["x"], "y": v["y"], "w": v["w"], "h": v["h"], "z": v["z"],
            "material": expand("material"),
            "surface": expand("surface"),
            "drop": v["drop"],
            "built": v["built"],
            "biome": expand("biome"),
            "height": v["height"],
            "river": v["river"],
        });
        let old_len = old.to_string().len();

        assert!(
            new_len * 2 < old_len,
            "kiểu mới ({new_len} bytes) không nhỏ hơn một nửa kiểu cũ ({old_len} bytes)"
        );
    }

    #[test]
    fn blocks_endpoint_serves_the_pack() {
        let mut g = g();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core");
        g.load_content(root.to_str().unwrap()).unwrap();

        let v = get(&mut g, "/api/blocks", "");
        assert_eq!(v["loaded"], true);
        let ds = v["blocks"].as_array().unwrap();
        assert!(ds.len() >= 11, "chỉ có {} vật liệu", ds.len());
        let air = ds.iter().find(|b| b["id"] == "air").expect("phải có `air`");
        assert!(
            air["color"].as_str().unwrap().starts_with('#'),
            "màu phải là chuỗi hex: {}",
            air["color"]
        );
        assert!(air["name"]["vi"].is_string());
    }

    #[test]
    fn blocks_endpoint_says_so_when_nothing_is_loaded() {
        // Danh sách rỗng kèm `loaded: false` là một câu trả lời; một lỗi 500 thì
        // không, vì client không biết nên dùng bảng dự phòng hay thử lại.
        let mut g = g();
        let v = get(&mut g, "/api/blocks", "");
        assert_eq!(v["loaded"], false);
        assert!(v["blocks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn entities_la_cu_dan_that_khong_phai_do_dung_tam() {
        let v = get(&mut g(), "/api/entities", "");
        let ds = v["entities"].as_array().unwrap();
        assert!(!ds.is_empty(), "thế giới không có ai");
        // Không còn `is_avatar`: vị thần không có thân xác nằm trong danh sách
        // thực thể, và một trường luôn `false` là một trường sẽ mục ruỗng.
        assert!(ds.iter().all(|e| e.get("is_avatar").is_none()));
        assert!(ds
            .iter()
            .any(|e| e["kind"] == "being" && e["role"].is_string()));
        // Kho lương của làng nằm trên bản đồ, không trong một con số ẩn.
        assert!(ds.iter().any(|e| e["kind"] == "item"));
    }

    #[test]
    fn command_di_duoc_va_doi_vi_tri() {
        let mut g = g();
        let w = a_villager(&g);
        let pos = |g: &Game| {
            (
                g.sim().store().attr_int(w, "core.pos.x").unwrap_or(0),
                g.sim().store().attr_int(w, "core.pos.y").unwrap_or(0),
            )
        };
        let truoc = pos(&g);
        let body = json!({
            "kind": "core.walk",
            "fields": { "who": {"entity": w.get()}, "dx": 1, "dy": 0 }
        })
        .to_string();
        let r = route(&mut g, "POST", "/api/command", "", &body);
        assert_eq!(r.status, 200, "{}", r.body);
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["ok"], true, "{}", r.body);

        let sau = pos(&g);
        assert_eq!(sau.0, truoc.0 + 1, "đi một bước phải dịch đúng một ô");
        assert_eq!(sau.1, truoc.1);
    }

    #[test]
    fn entity_gui_dang_so_tran_thi_bao_loi_ro_rang() {
        // Đây chính là cái bẫy mà quy tắc `{"entity": N}` tồn tại để chặn:
        // engine đòi `Uint`, JSON chỉ có "number".
        let mut g = g();
        let av = a_villager(&g).get();
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
        let av = a_villager(&g).get();
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

    fn walk_body(g: &Game, dx: i64, dy: i64) -> J {
        json!({
            "kind": "core.walk",
            "fields": { "who": {"entity": a_villager(g).get()}, "dx": dx, "dy": dy }
        })
    }

    #[test]
    fn preview_returns_a_drawable_diff() {
        let mut g = g();
        let body = walk_body(&g, 1, 0).to_string();
        let r = route(&mut g, "POST", "/api/preview", "", &body);
        assert_eq!(r.status, 200, "{}", r.body);
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["changes_anything"], true);
        // Diff phải vẽ được **trên bản đồ**, nên mỗi thay đổi mang tọa độ.
        let c = &v["changes"][0];
        assert!(c["from"].is_array(), "{}", r.body);
        assert!(c["to"].is_array());
        assert_eq!(c["moved"], true);
        assert!(v["base_hash"].as_str().unwrap().len() == 64);
    }

    #[test]
    fn preview_leaves_the_world_alone() {
        let mut g = g();
        let before = g.state_hash();
        let body = walk_body(&g, 1, 0).to_string();
        route(&mut g, "POST", "/api/preview", "", &body);
        assert_eq!(g.state_hash(), before);
    }

    #[test]
    fn commit_without_a_hash_is_refused() {
        // Mọi lần khắc phải kèm hash đã xem. Không có ràng buộc này thì preview
        // chỉ là trang trí.
        let mut g = g();
        let body = walk_body(&g, 1, 0).to_string();
        let r = route(&mut g, "POST", "/api/commit", "", &body);
        assert_eq!(r.status, 400);
        assert!(r.body.contains("base_hash"));
    }

    #[test]
    fn commit_with_a_stale_hash_is_refused_but_not_an_error() {
        let mut g = g();
        let mut body = walk_body(&g, 1, 0);
        body["base_hash"] = json!("0".repeat(64));
        let r = route(&mut g, "POST", "/api/commit", "", &body.to_string());
        assert_eq!(r.status, 200, "từ chối không phải lỗi server");
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["reason"], "world_moved");
        // Client cần hash hiện tại để xem lại ngay, không phải hỏi thêm một vòng.
        assert!(v["state_hash"].as_str().unwrap().len() == 64);
    }

    #[test]
    fn preview_then_commit_lands_exactly_where_promised() {
        let mut g = g();
        let pbody = walk_body(&g, 1, 0).to_string();
        let pr = route(&mut g, "POST", "/api/preview", "", &pbody);
        let pv: J = serde_json::from_str(&pr.body).unwrap();

        let mut body = walk_body(&g, 1, 0);
        body["base_hash"] = pv["base_hash"].clone();
        let cr = route(&mut g, "POST", "/api/commit", "", &body.to_string());
        let cv: J = serde_json::from_str(&cr.body).unwrap();
        assert_eq!(cv["ok"], true, "{}", cr.body);
        assert_eq!(
            cv["after_hash"], pv["after_hash"],
            "xem trước hứa sai kết quả"
        );
    }

    #[test]
    fn goto_plans_a_path() {
        let mut g = g();
        let w = a_villager(&g);
        let ax = g.sim().store().attr_int(w, "core.pos.x").unwrap_or(0);
        let ay = g.sim().store().attr_int(w, "core.pos.y").unwrap_or(0);
        let body = json!({ "who": w.get().to_string(), "x": ax + 3, "y": ay }).to_string();
        let r = route(&mut g, "POST", "/api/goto", "", &body);
        assert_eq!(r.status, 200, "{}", r.body);
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert!(v["steps"].as_u64().unwrap() > 0, "{}", r.body);
        // Đường đi phải đi kèm: client cần vẽ nó ra để người chơi biết lệnh đã
        // được hiểu.
        let path = v["path"].as_array().expect("thiếu `path`");
        assert_eq!(path.len(), v["steps"].as_u64().unwrap() as usize);
        assert_eq!(path[0].as_array().unwrap().len(), 2);
    }

    #[test]
    fn cancel_stops_the_walk() {
        let mut g = g();
        let w = a_villager(&g);
        let ax = g.sim().store().attr_int(w, "core.pos.x").unwrap_or(0);
        let ay = g.sim().store().attr_int(w, "core.pos.y").unwrap_or(0);
        route(
            &mut g,
            "POST",
            "/api/goto",
            "",
            &json!({"who": w.get().to_string(), "x": ax + 8, "y": ay}).to_string(),
        );
        assert!(
            get(&mut g, "/api/meta", "")["steps_remaining"]
                .as_u64()
                .unwrap()
                > 0
        );

        let r = route(
            &mut g,
            "POST",
            "/api/goto",
            "",
            &json!({"who": w.get().to_string(), "x": ax, "y": ay, "cancel": true}).to_string(),
        );
        assert_eq!(r.status, 200);
        assert_eq!(get(&mut g, "/api/meta", "")["steps_remaining"], 0);
    }

    #[test]
    fn goto_without_coordinates_is_a_client_error() {
        let mut g = g();
        let r = route(&mut g, "POST", "/api/goto", "", r#"{"x": 3}"#);
        assert_eq!(r.status, 400);
    }

    #[test]
    fn changing_speed_does_not_write_to_the_world() {
        // Cùng lý do với đổi lát `z`: nếu tua nhanh ghi event thì lịch sử thế
        // giới phụ thuộc vào thói quen xem của người chơi.
        let mut g = g();
        let before = g.state_hash();
        let r = route(
            &mut g,
            "POST",
            "/api/speed",
            "",
            &json!({"speed_milli": 50_000}).to_string(),
        );
        assert_eq!(r.status, 200);
        assert_eq!(g.speed_milli(), 50_000);
        assert_eq!(g.state_hash(), before);
    }

    #[test]
    fn speed_above_the_cap_is_clamped_not_rejected() {
        // Kẹp chứ không từ chối: một thanh trượt kéo hết cỡ không nên cho ra
        // một thông báo lỗi.
        let mut g = g();
        let r = route(
            &mut g,
            "POST",
            "/api/speed",
            "",
            &json!({"speed_milli": 9_999_999}).to_string(),
        );
        let v: J = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["speed_milli"], crate::game::MAX_SPEED_MILLI);
    }

    #[test]
    fn meta_reports_speed() {
        let v = get(&mut g(), "/api/meta", "");
        assert_eq!(v["speed_milli"], 1_000, "mặc định phải là ×1");
        assert!(v["max_speed_milli"].is_number());
        assert!(v["history_len"].is_number());
        assert!(v["history_limit"].as_u64().unwrap() > 0);
    }

    #[test]
    fn doi_lat_z_khong_doi_state_hash() {
        // `§P6.8`: kéo camera và đổi lát là **query**. Nếu chúng ghi event thì
        // lịch sử thế giới phụ thuộc vào việc người chơi đã nhìn đi đâu.
        let mut g = g();
        let truoc = g.state_hash();
        let r = route(
            &mut g,
            "POST",
            "/api/view",
            "",
            &json!({"z": -50}).to_string(),
        );
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
    fn cause_chain_walks_back_to_the_root() {
        let mut g = g();
        for _ in 0..12 {
            g.tick_once();
        }
        let evs = get(&mut g, "/api/events", "after=0");
        let last = evs["events"].as_array().unwrap().last().unwrap()["seq"]
            .as_u64()
            .unwrap();

        let v = get(&mut g, "/api/cause", &format!("seq={last}"));
        let chain = v["chain"].as_array().unwrap();
        assert!(
            !chain.is_empty(),
            "chuỗi phải chứa ít nhất chính sự kiện đó"
        );
        assert_eq!(chain[0]["seq"], last, "mắt đầu phải là sự kiện đã hỏi");
        // Chuỗi đi **ngược** thời gian: mỗi mắt sau không mới hơn mắt trước.
        let ticks: Vec<u64> = chain.iter().map(|c| c["tick"].as_u64().unwrap()).collect();
        for w in ticks.windows(2) {
            assert!(w[1] <= w[0], "chuỗi nhân quả đi tới tương lai: {ticks:?}");
        }
    }

    #[test]
    fn cause_of_an_unknown_event_is_empty_not_an_error() {
        // Hỏi một sự kiện không tồn tại là chuyện bình thường sau khi rewind;
        // trả 500 sẽ làm panel nhân quả chết thay vì hiện "không có gì".
        let mut g = g();
        let v = get(&mut g, "/api/cause", "seq=999999");
        assert!(v["chain"].as_array().unwrap().is_empty());
    }

    #[test]
    fn cause_without_seq_is_a_client_error() {
        let mut g = g();
        assert_eq!(route(&mut g, "GET", "/api/cause", "", "").status, 400);
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
