//! Phiên gỡ lỗi: giao thức NDJSON trên stdin/stdout.
//!
//! Đây là đường mà `mow-mcp` dùng để "vào thế giới" (`plan.md §P7.2`). Mỗi
//! dòng stdin là một yêu cầu JSON, mỗi dòng stdout là một trả lời JSON.
//!
//! Vì sao stdin/stdout thay vì một cổng gRPC:
//!
//! - **Bảo mật theo cấu trúc.** `§P7.2` đòi hỏi `mow-mcp` chỉ nối được tới
//!   build có feature `devtool`, qua loopback, có token. Một tiến trình con thì
//!   không có cổng nào để nối tới cả — không cần token vì không có bề mặt tấn
//!   công. Bản phát hành không đóng gói `mow-cli`, nên đường này không tồn tại
//!   ở đó theo đúng nghĩa đen.
//! - **Vòng đời rõ ràng.** Agent đóng tiến trình là thế giới biến mất. Không có
//!   world mồ côi chiếm bộ nhớ sau khi phiên gỡ lỗi kết thúc.
//!
//! Khi `mow-server` có thật ở Giai đoạn C, giao thức này giữ nguyên hình dạng
//! và chỉ đổi tầng vận chuyển sang gRPC `Debug` — các tool của MCP không phải
//! viết lại.

use mow_core::invariant::Cost;
use mow_core::{Command, EntityId, InvariantRunner, Sim, Value};
use mow_scenario::testing::TestWorldFactory;
use mow_scenario::WorldFactory;
use serde_json::{json, Value as J};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};

/// Trạng thái một phiên.
pub struct Session {
    worlds: BTreeMap<String, Sim>,
    factory: TestWorldFactory,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Phiên rỗng.
    pub fn new() -> Session {
        Session {
            worlds: BTreeMap::new(),
            factory: TestWorldFactory,
        }
    }

    /// Chạy vòng lặp đọc–xử lý–ghi cho tới khi hết đầu vào.
    pub fn serve(
        &mut self,
        input: &mut dyn BufRead,
        output: &mut dyn Write,
    ) -> std::io::Result<()> {
        let mut dong = String::new();
        loop {
            dong.clear();
            if input.read_line(&mut dong)? == 0 {
                return Ok(());
            }
            if dong.trim().is_empty() {
                continue;
            }
            let tra_loi = match serde_json::from_str::<J>(&dong) {
                Ok(req) => self.handle(&req),
                Err(e) => json!({ "ok": false, "error": format!("JSON hỏng: {e}") }),
            };
            writeln!(output, "{tra_loi}")?;
            output.flush()?;
        }
    }

    /// Xử lý một yêu cầu.
    pub fn handle(&mut self, req: &J) -> J {
        let tool = req.get("tool").and_then(J::as_str).unwrap_or("");
        let a = req.get("args").cloned().unwrap_or(json!({}));
        let id = req.get("id").cloned().unwrap_or(J::Null);

        let ket_qua = self.dispatch(tool, &a);
        match ket_qua {
            Ok(v) => json!({ "id": id, "ok": true, "result": v }),
            Err(e) => json!({ "id": id, "ok": false, "error": e }),
        }
    }

    fn dispatch(&mut self, tool: &str, a: &J) -> Result<J, String> {
        match tool {
            // ── World ────────────────────────────────────────────────────────
            "world_create" => {
                let seed = a
                    .get("worldseed")
                    .and_then(J::as_str)
                    .unwrap_or("test:tiny_village");
                let ten = a.get("name").and_then(J::as_str).unwrap_or("w1").to_owned();
                if self.worlds.contains_key(&ten) {
                    return Err(format!("đã có world tên `{ten}`"));
                }
                let sim = self
                    .factory
                    .build(seed, &BTreeMap::new())
                    .ok_or_else(|| format!("không có worldseed `{seed}`"))?;
                let tom_tat = tom_tat(&sim, &ten);
                self.worlds.insert(ten, sim);
                Ok(tom_tat)
            }

            "world_list" => Ok(json!({
                "worlds": self.worlds.iter().map(|(k, s)| tom_tat(s, k)).collect::<Vec<_>>()
            })),

            "world_drop" => {
                let ten = self.ten_world(a)?;
                self.worlds.remove(&ten);
                Ok(json!({ "dropped": ten }))
            }

            // ── Time ─────────────────────────────────────────────────────────
            "sim_step" => {
                let n = a.get("ticks").and_then(J::as_u64).unwrap_or(1);
                let ten = self.ten_world(a)?;
                let sim = self.world_mut(&ten)?;
                sim.advance(n).map_err(|e| e.to_string())?;
                Ok(tom_tat(sim, &ten))
            }

            // ── Query ────────────────────────────────────────────────────────
            "query_entity" => {
                let ten = self.ten_world(a)?;
                let eid = EntityId(
                    a.get("entity")
                        .and_then(J::as_u64)
                        .ok_or("thiếu `entity`")?,
                );
                let sim = self.world(&ten)?;
                let attrs = sim
                    .store()
                    .attrs(eid)
                    .ok_or_else(|| format!("không có thực thể {eid}"))?;
                Ok(json!({
                    "entity": eid.get(),
                    "attrs": attrs.iter().map(|(k, v)| (k.clone(), gia_tri(v))).collect::<serde_json::Map<_, _>>()
                }))
            }

            "query_entities" => {
                let ten = self.ten_world(a)?;
                let kind = a.get("kind").and_then(J::as_str);
                let tag = a.get("tag").and_then(J::as_str);
                let sim = self.world(&ten)?;
                let ds: Vec<J> = sim
                    .store()
                    .ids()
                    .filter(|id| {
                        kind.is_none_or(|k| sim.store().attr_text(*id, "core.kind") == Some(k))
                    })
                    .filter(|id| {
                        tag.is_none_or(|t| {
                            sim.store()
                                .attrs(*id)
                                .is_some_and(|m| m.contains_key(&format!("tag.{t}")))
                        })
                    })
                    .map(|id| {
                        json!({
                            "entity": id.get(),
                            "kind": sim.store().attr_text(id, "core.kind"),
                            "name": sim.store().attr_text(id, "core.name"),
                        })
                    })
                    .collect();
                Ok(json!({ "count": ds.len(), "entities": ds }))
            }

            "query_timeline" => {
                let ten = self.ten_world(a)?;
                let tu = a.get("from").and_then(J::as_u64).unwrap_or(0);
                let den = a.get("to").and_then(J::as_u64).unwrap_or(u64::MAX);
                let sim = self.world(&ten)?;
                let ds: Vec<J> = sim
                    .log()
                    .iter()
                    .filter(|e| e.tick.0 >= tu && e.tick.0 <= den)
                    .map(su_kien)
                    .collect();
                Ok(json!({ "count": ds.len(), "events": ds }))
            }

            "query_cause_chain" => {
                let ten = self.ten_world(a)?;
                let seq =
                    mow_core::EventSeq(a.get("seq").and_then(J::as_u64).ok_or("thiếu `seq`")?);
                let sim = self.world(&ten)?;
                let ds: Vec<J> = sim
                    .log()
                    .cause_chain(seq, 256)
                    .into_iter()
                    .map(su_kien)
                    .collect();
                Ok(json!({ "depth": ds.len(), "chain": ds }))
            }

            // ── Mutate ───────────────────────────────────────────────────────
            "debug_apply_command" => {
                let ten = self.ten_world(a)?;
                let kind = a
                    .get("kind")
                    .and_then(J::as_str)
                    .ok_or("thiếu `kind`")?
                    .to_owned();
                let payload = doi_gia_tri(a.get("payload").cloned().unwrap_or(json!({})));
                let sim = self.world_mut(&ten)?;
                let cmd = Command::new(&kind, sim.world_id(), payload);
                let r = sim.apply(&cmd).map_err(|e| e.to_string())?;
                Ok(json!({
                    "events": r.events.iter().map(|s| s.0).collect::<Vec<_>>(),
                    "mutations": r.mutations,
                    "state_hash": sim.state_hash().to_hex(),
                }))
            }

            "debug_list_commands" => {
                let ten = self.ten_world(a)?;
                let sim = self.world(&ten)?;
                Ok(json!({ "kinds": sim.handlers().kinds().collect::<Vec<_>>() }))
            }

            // ── Verify ───────────────────────────────────────────────────────
            "assert_invariants" => {
                let ten = self.ten_world(a)?;
                let sim = self.world(&ten)?;
                let rep = sim.check(&InvariantRunner::standard(Cost::Expensive));
                Ok(json!({
                    "checked": rep.checked,
                    "clean": rep.is_clean(),
                    "violations": rep.violations.iter().map(|v| json!({
                        "id": v.id, "detail": v.detail
                    })).collect::<Vec<_>>(),
                }))
            }

            "assert_state_hash" => {
                let ten = self.ten_world(a)?;
                let sim = self.world(&ten)?;
                let thuc_te = sim.state_hash().to_hex();
                match a.get("expected").and_then(J::as_str) {
                    None => Ok(json!({ "state_hash": thuc_te })),
                    Some(mong) if mong == thuc_te => {
                        Ok(json!({ "state_hash": thuc_te, "match": true }))
                    }
                    Some(mong) => Err(format!("state hash là {thuc_te}, mong đợi {mong}")),
                }
            }

            "list_invariants" => {
                let r = InvariantRunner::standard(Cost::Expensive);
                Ok(json!({
                    "invariants": r.list().iter().map(|(id, cost, desc)| json!({
                        "id": id, "cost": format!("{cost:?}"), "describes": desc
                    })).collect::<Vec<_>>()
                }))
            }

            khac => Err(format!(
                "không biết tool `{khac}`. Có: world_create, world_list, world_drop, \
                 sim_step, query_entity, query_entities, query_timeline, query_cause_chain, \
                 debug_apply_command, debug_list_commands, assert_invariants, \
                 assert_state_hash, list_invariants"
            )),
        }
    }

    fn ten_world(&self, a: &J) -> Result<String, String> {
        let ten = a
            .get("world")
            .and_then(J::as_str)
            .unwrap_or("w1")
            .to_owned();
        if self.worlds.contains_key(&ten) {
            Ok(ten)
        } else {
            Err(format!(
                "không có world `{ten}`; đã tạo: {:?}",
                self.worlds.keys().collect::<Vec<_>>()
            ))
        }
    }

    fn world(&self, ten: &str) -> Result<&Sim, String> {
        self.worlds
            .get(ten)
            .ok_or_else(|| format!("không có world `{ten}`"))
    }

    fn world_mut(&mut self, ten: &str) -> Result<&mut Sim, String> {
        self.worlds
            .get_mut(ten)
            .ok_or_else(|| format!("không có world `{ten}`"))
    }
}

fn tom_tat(sim: &Sim, ten: &str) -> J {
    json!({
        "world": ten,
        "tick": sim.clock().local().0,
        "divine_tick": sim.clock().divine().0,
        "entities": sim.store().len(),
        "events": sim.log().len(),
        "state_hash": sim.state_hash().to_hex(),
    })
}

fn su_kien(e: &mow_core::Event) -> J {
    json!({
        "seq": e.seq.0,
        "tick": e.tick.0,
        "kind": e.kind.0,
        "actor": e.actor.map(mow_core::EntityId::get),
        "subject": e.subject.map(mow_core::EntityId::get),
        "cause": e.cause.map(|c| c.0),
        "law_version": e.law_version,
        "payload": gia_tri(&e.payload),
    })
}

fn gia_tri(v: &Value) -> J {
    match v {
        Value::Null => J::Null,
        Value::Bool(b) => json!(b),
        Value::Int(i) => json!(i),
        Value::Uint(u) => json!(u),
        // Q16.16 đi ra ngoài ở dạng **thô**, kèm nhãn. Nếu chuyển nó thành số
        // thực ở đây, một agent đọc rồi ghi lại sẽ làm mất chính xác, và lỗi đó
        // sẽ trông như một bug của engine.
        Value::Fixed(f) => json!({ "fx_raw": f.raw() }),
        Value::Text(s) => json!(s),
        Value::Bytes(b) => json!({ "bytes_len": b.len() }),
        Value::List(l) => J::Array(l.iter().map(gia_tri).collect()),
        Value::Map(m) => J::Object(m.iter().map(|(k, v)| (k.clone(), gia_tri(v))).collect()),
    }
}

fn doi_gia_tri(v: J) -> Value {
    match v {
        J::Null => Value::Null,
        J::Bool(b) => Value::Bool(b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::Uint(u)
            } else {
                Value::Text(n.to_string())
            }
        }
        J::String(s) => Value::Text(s),
        J::Array(xs) => Value::List(xs.into_iter().map(doi_gia_tri).collect()),
        J::Object(m) => Value::Map(m.into_iter().map(|(k, v)| (k, doi_gia_tri(v))).collect()),
    }
}
