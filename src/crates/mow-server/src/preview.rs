//! Xem trước một can thiệp trước khi khắc nó vào thế giới (`§16`, `§18.12`).
//!
//! ## Vì sao không dùng `mow_scenario::slice::preview`
//!
//! Nó dựng lại thế giới bằng `build_slice_world(seed)`. Thế giới thật của server
//! khác: avatar đã được dời tới chỗ ở được, content pack đã nạp, và người chơi
//! đã đi lại. Preview chạy trên một thế giới khác thế giới đang xem là một
//! preview **nói dối một cách thuyết phục** — nó đưa ra những con số đúng về
//! một vũ trụ không tồn tại.
//!
//! ## Cách làm: phát lại nhật ký lệnh
//!
//! Server giữ **mọi lệnh đã áp**, theo thứ tự. Preview dựng một thế giới mới từ
//! cùng seed, phát lại nhật ký đó, rồi thử lệnh ứng viên trên bản sao.
//!
//! Đắt hơn "tính nhẩm hậu quả", và đó là điểm: một preview đoán mò thì tệ hơn
//! không có preview, vì người chơi sẽ tin nó. Cách này còn chứng minh một tính
//! chất mạnh hơn tốc độ — **lịch sử đủ để dựng lại thế giới**, đúng thứ `§8.4`
//! đòi hỏi ở replay.
//!
//! ## Preview bị ràng buộc với `state_hash`
//!
//! Đây là ràng buộc quan trọng nhất ở đây, và nó là chuyện đúng-sai chứ không
//! phải chuyện tiện nghi.
//!
//! Thế giới chạy theo tick. Giữa lúc người chơi xem preview và lúc họ bấm khắc,
//! NPC đã đi, cây đã đổ, có người đã chết. Nếu commit vẫn chạy thì thứ được
//! khắc vào thế giới **không phải** thứ họ đã xem — và một vị thần bị lừa bởi
//! chính công cụ của mình thì không còn là thần.
//!
//! Nên mọi preview mang theo `base_hash`, và commit từ chối khi hash đã trôi.
//! Người chơi xem lại rồi quyết định lần nữa. Đó là một bước thừa, và nó là
//! bước duy nhất giữ cho preview có nghĩa.

use mow_core::{Command, EntityId, Value};
use mow_math::StateHash;
use serde_json::{json, Value as J};
use std::collections::BTreeMap;

/// Một thực thể sẽ đổi, và đổi thế nào.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityChange {
    /// Ai.
    pub id: EntityId,
    /// Tên hiển thị, để không phải tra lại ở client.
    pub name: String,
    /// Vị trí trước, nếu có.
    pub from: Option<(i64, i64)>,
    /// Vị trí sau, nếu có.
    pub to: Option<(i64, i64)>,
    /// Thuộc tính đã đổi, tên khóa.
    pub attrs: Vec<String>,
}

impl EntityChange {
    /// Có dịch chuyển không — client vẽ mũi tên cho trường hợp này.
    #[must_use]
    pub fn moved(&self) -> bool {
        self.from != self.to && self.from.is_some() && self.to.is_some()
    }
}

/// Kết quả xem trước.
#[derive(Debug, Clone)]
pub struct Diff {
    /// Lệnh sẽ chạy.
    pub command: String,
    /// Hash thế giới **lúc xem**. Commit phải mang lại đúng giá trị này.
    pub base_hash: StateHash,
    /// Hash sau khi khắc.
    pub after_hash: StateHash,
    /// Lệnh sẽ thất bại, kèm lý do. `Some` nghĩa là không có gì để khắc.
    pub error: Option<String>,
    /// Sự kiện sẽ được ghi, dạng `(loại, tóm tắt)`.
    pub events: Vec<(String, String)>,
    /// Thực thể bị đụng tới.
    pub changes: Vec<EntityChange>,
}

impl Diff {
    /// Lệnh này có đổi gì không.
    #[must_use]
    pub fn changes_anything(&self) -> bool {
        self.base_hash != self.after_hash
    }

    /// Dịch sang JSON cho client.
    #[must_use]
    pub fn to_json(&self) -> J {
        json!({
            "command": self.command,
            "base_hash": self.base_hash.to_hex(),
            "after_hash": self.after_hash.to_hex(),
            "changes_anything": self.changes_anything(),
            "error": self.error,
            "events": self.events.iter().map(|(k, s)| json!({ "kind": k, "summary": s })).collect::<Vec<_>>(),
            "changes": self.changes.iter().map(|c| json!({
                "id": c.id.get().to_string(),
                "name": c.name,
                "from": c.from.map(|(x, y)| json!([x, y])),
                "to": c.to.map(|(x, y)| json!([x, y])),
                "moved": c.moved(),
                "attrs": c.attrs,
            })).collect::<Vec<_>>(),
        })
    }
}

/// Vì sao commit bị từ chối.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Thế giới đã đổi từ lúc xem trước.
    WorldMoved {
        /// Hash mà người chơi đã xem.
        expected: String,
        /// Hash hiện tại.
        actual: String,
    },
    /// Chính lệnh đó sẽ thất bại.
    CommandFails(String),
}

impl core::fmt::Display for Refusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Refusal::WorldMoved { .. } => write!(
                f,
                "thế giới đã đổi từ lúc Người xem — hãy xem lại trước khi khắc"
            ),
            Refusal::CommandFails(e) => write!(f, "{e}"),
        }
    }
}

/// Ảnh chụp thuộc tính của mọi thực thể, để so trước–sau.
pub(crate) type Snapshot = BTreeMap<EntityId, BTreeMap<String, Value>>;

/// Chụp toàn bộ thuộc tính của mọi thực thể.
///
/// Chụp **tất cả** chứ không chỉ vị trí: một can thiệp đổi `need.hunger` hay
/// `item.def` cũng phải hiện ra trong diff. Chỉ theo dõi vị trí sẽ cho một
/// preview trông sạch sẽ trong khi nó vừa bỏ đói cả một ngôi làng.
pub(crate) fn snapshot(store: &mow_core::Store) -> Snapshot {
    store
        .ids()
        .map(|id| {
            let attrs = store
                .attrs(id)
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            (id, attrs)
        })
        .collect()
}

fn pos_of(attrs: &BTreeMap<String, Value>) -> Option<(i64, i64)> {
    let x = match attrs.get("core.pos.x") {
        Some(Value::Int(v)) => *v,
        _ => return None,
    };
    let y = match attrs.get("core.pos.y") {
        Some(Value::Int(v)) => *v,
        _ => return None,
    };
    Some((x, y))
}

fn name_of(attrs: &BTreeMap<String, Value>) -> String {
    match attrs.get("core.name") {
        Some(Value::Text(t)) => t.clone(),
        _ => "?".to_owned(),
    }
}

/// So hai ảnh chụp thành danh sách thay đổi.
///
/// Thứ tự theo `EntityId` vì `Snapshot` là `BTreeMap` — cùng một can thiệp phải
/// cho cùng một diff, kể cả thứ tự dòng. Một diff xáo thứ tự giữa hai lần xem
/// làm người đọc mất niềm tin nhanh hơn một diff sai.
pub(crate) fn compare(before: &Snapshot, after: &Snapshot) -> Vec<EntityChange> {
    let mut out = Vec::new();
    for (id, after_attrs) in after {
        let before_attrs = before.get(id);
        let changed: Vec<String> = match before_attrs {
            Some(b) => after_attrs
                .iter()
                .filter(|(k, v)| b.get(*k) != Some(*v))
                .map(|(k, _)| k.clone())
                .chain(b.keys().filter(|k| !after_attrs.contains_key(*k)).cloned())
                .collect(),
            // Thực thể mới: mọi thuộc tính đều là thay đổi.
            None => after_attrs.keys().cloned().collect(),
        };
        if changed.is_empty() {
            continue;
        }
        out.push(EntityChange {
            id: *id,
            name: name_of(after_attrs),
            from: before_attrs.and_then(pos_of),
            to: pos_of(after_attrs),
            attrs: changed,
        });
    }
    // Thực thể biến mất.
    for (id, before_attrs) in before {
        if after.contains_key(id) {
            continue;
        }
        out.push(EntityChange {
            id: *id,
            name: name_of(before_attrs),
            from: pos_of(before_attrs),
            to: None,
            attrs: vec!["<biến mất>".to_owned()],
        });
    }
    out.sort_by_key(|c| c.id.get());
    out
}

/// Tóm tắt một sự kiện thành một dòng đọc được.
pub(crate) fn summarize(kind: &str, payload: &Value) -> String {
    match payload {
        Value::Map(m) if !m.is_empty() => {
            let parts: Vec<String> = m
                .iter()
                .take(4)
                .map(|(k, v)| format!("{k}={}", short_value(v)))
                .collect();
            parts.join(" ")
        }
        _ => kind.to_owned(),
    }
}

fn short_value(v: &Value) -> String {
    match v {
        Value::Text(t) if t.chars().count() > 24 => {
            format!("{}…", t.chars().take(24).collect::<String>())
        }
        Value::Text(t) => t.clone(),
        Value::Int(i) => i.to_string(),
        Value::Uint(u) => u.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_owned(),
        Value::List(l) => format!("[{}]", l.len()),
        Value::Map(m) => format!("{{{}}}", m.len()),
        Value::Bytes(b) => format!("{} byte", b.len()),
        Value::Fixed(f) => f.raw().to_string(),
    }
}

/// Lệnh đã áp, đủ để phát lại.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    /// Tick địa phương lúc lệnh được áp.
    ///
    /// Bắt buộc, và lý do đáng ghi lại: bản đầu chỉ lưu `(kind, payload)` rồi
    /// phát lại tất cả ở tick 0 và sau đó mới `advance` một lần. Kết quả là
    /// `state_hash` lệch — vì sự kiện mang tick, và một NPC bước ở tick 12 khác
    /// hẳn một NPC bước ở tick 0 rồi mười hai tick trôi qua.
    pub tick: u64,
    /// Loại lệnh.
    pub kind: String,
    /// Payload.
    pub payload: Value,
}

impl JournalEntry {
    /// Dựng lại `Command` để phát lại.
    #[must_use]
    pub fn to_command(&self, world: mow_core::WorldId) -> Command {
        Command::new(&self.kind, world, self.payload.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    #[test]
    fn no_change_means_empty_diff() {
        let a: Snapshot = [(EntityId(1), attrs(&[("core.pos.x", Value::Int(3))]))]
            .into_iter()
            .collect();
        assert!(compare(&a, &a).is_empty());
    }

    #[test]
    fn a_move_is_reported_with_both_ends() {
        let before: Snapshot = [(
            EntityId(1),
            attrs(&[
                ("core.name", Value::Text("Aren".into())),
                ("core.pos.x", Value::Int(3)),
                ("core.pos.y", Value::Int(4)),
            ]),
        )]
        .into_iter()
        .collect();
        let after: Snapshot = [(
            EntityId(1),
            attrs(&[
                ("core.name", Value::Text("Aren".into())),
                ("core.pos.x", Value::Int(4)),
                ("core.pos.y", Value::Int(4)),
            ]),
        )]
        .into_iter()
        .collect();

        let d = compare(&before, &after);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].from, Some((3, 4)));
        assert_eq!(d[0].to, Some((4, 4)));
        assert!(d[0].moved());
        assert_eq!(d[0].name, "Aren");
    }

    #[test]
    fn a_changed_attribute_shows_even_without_movement() {
        // Chỉ theo dõi vị trí sẽ cho một preview trông sạch sẽ trong khi nó vừa
        // bỏ đói cả một ngôi làng.
        let before: Snapshot = [(EntityId(1), attrs(&[("need.hunger", Value::Int(100))]))]
            .into_iter()
            .collect();
        let after: Snapshot = [(EntityId(1), attrs(&[("need.hunger", Value::Int(9_000))]))]
            .into_iter()
            .collect();
        let d = compare(&before, &after);
        assert_eq!(d.len(), 1);
        assert!(!d[0].moved());
        assert_eq!(d[0].attrs, vec!["need.hunger"]);
    }

    #[test]
    fn a_new_entity_is_a_change() {
        let before: Snapshot = BTreeMap::new();
        let after: Snapshot = [(
            EntityId(7),
            attrs(&[("core.name", Value::Text("Rồng".into()))]),
        )]
        .into_iter()
        .collect();
        let d = compare(&before, &after);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].from, None);
        assert_eq!(d[0].name, "Rồng");
    }

    #[test]
    fn a_vanished_entity_is_a_change() {
        // Trường hợp này dễ quên nhất, và nó là trường hợp nặng nhất: một can
        // thiệp xóa mất người mà diff không nhắc tới thì người chơi khắc nó vào
        // thế giới mà không biết mình vừa giết ai.
        let before: Snapshot = [(
            EntityId(2),
            attrs(&[
                ("core.name", Value::Text("Linh".into())),
                ("core.pos.x", Value::Int(1)),
                ("core.pos.y", Value::Int(1)),
            ]),
        )]
        .into_iter()
        .collect();
        let after: Snapshot = BTreeMap::new();
        let d = compare(&before, &after);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].to, None);
        assert_eq!(d[0].from, Some((1, 1)));
        assert_eq!(d[0].name, "Linh");
    }

    #[test]
    fn a_removed_attribute_counts_as_a_change() {
        let before: Snapshot = [(EntityId(1), attrs(&[("loc.inventory", Value::Uint(9))]))]
            .into_iter()
            .collect();
        let after: Snapshot = [(EntityId(1), attrs(&[]))].into_iter().collect();
        let d = compare(&before, &after);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].attrs, vec!["loc.inventory"]);
    }

    #[test]
    fn diff_order_is_stable() {
        // Một diff xáo thứ tự giữa hai lần xem làm người đọc mất niềm tin nhanh
        // hơn một diff sai.
        let before: Snapshot = BTreeMap::new();
        let after: Snapshot = (1..=20u64)
            .map(|i| (EntityId(i), attrs(&[("core.pos.x", Value::Int(i as i64))])))
            .collect();
        let a = compare(&before, &after);
        let b = compare(&before, &after);
        let ids_a: Vec<u64> = a.iter().map(|c| c.id.get()).collect();
        let ids_b: Vec<u64> = b.iter().map(|c| c.id.get()).collect();
        assert_eq!(ids_a, ids_b);
        let mut sorted = ids_a.clone();
        sorted.sort_unstable();
        assert_eq!(ids_a, sorted);
    }

    #[test]
    fn refusal_speaks_to_a_god_not_to_a_programmer() {
        let r = Refusal::WorldMoved {
            expected: "aaa".into(),
            actual: "bbb".into(),
        };
        let s = r.to_string();
        assert!(s.contains("Người"), "{s}");
        assert!(!s.contains("hash"), "thông báo không nên nói về hash: {s}");
    }

    #[test]
    fn long_text_in_a_summary_is_cut() {
        let v = Value::Map(
            [("text".to_owned(), Value::Text("m".repeat(200)))]
                .into_iter()
                .collect(),
        );
        let s = summarize("core.speak", &v);
        assert!(s.chars().count() < 60, "tóm tắt dài {}", s.chars().count());
        assert!(s.contains('…'));
    }
}
