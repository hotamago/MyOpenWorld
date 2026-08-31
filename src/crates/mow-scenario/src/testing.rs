//! Thế giới nhỏ viết tay, dùng cho kịch bản khói.
//!
//! Đây là **chỗ tạm** cho tới `PA-04`, khi worldseed thật và genesis command
//! thay thế nó. Nó tồn tại vì một lý do cụ thể: harness phải tự kiểm được chính
//! nó trước khi có worldgen. Nếu đợi worldgen xong rồi mới viết kịch bản, thì
//! kịch bản đầu tiên đỏ sẽ có hai nghi phạm — worldgen hoặc runner — và không
//! có cách nào tách chúng ra.
//!
//! `§P7.3` quy tắc 4 nói đúng cách làm điều này: *"Worldseed dùng cho test có
//! thể khai báo sẵn `named_entities`; khi đó kịch bản tham chiếu thẳng tên."*
//! Các thế giới ở đây đi theo tinh thần đó — nhỏ, viết tay, đọc hết được.

use crate::runner::WorldFactory;
use mow_core::{
    val, BranchId, Clock, EventDraft, Failure, FailureCode, HandlerRegistry, Sim, SimConfig, Value,
    WorldId,
};
use mow_math::WorldSeed;
use std::collections::BTreeMap;

/// Bộ handler tối thiểu để kịch bản khói chạy được.
///
/// Đây cũng là ví dụ mẫu về việc bước của kịch bản chính là loại command: thêm
/// một handler ở đây là thêm một bước dùng được trong mọi kịch bản, không phải
/// sửa gì trong runner.
pub fn handlers() -> HandlerRegistry {
    let mut r = HandlerRegistry::new();

    r.on("core.spawn", |ctx| {
        let kind = ctx.require_text("kind")?.to_owned();
        let id = ctx.spawn();
        ctx.set(id, "core.kind", kind.clone());
        if let Some(Value::Text(name)) = ctx.command.payload.get("name") {
            ctx.set(id, "core.name", name.clone());
        }
        if let Some(Value::Int(age)) = ctx.command.payload.get("age") {
            ctx.set(id, "core.age", *age);
        }
        if let Some(Value::Uint(w)) = ctx.command.payload.get("within") {
            ctx.set(id, "core.within", *w as i64);
        }
        let mut la_sapient = false;
        if let Some(Value::List(tags)) = ctx.command.payload.get("tags") {
            for t in tags.clone() {
                if let Value::Text(t) = t {
                    la_sapient |= t == "sapient";
                    ctx.set(id, &format!("tag.{t}"), true);
                }
            }
        }

        // `INV-22-3`: gắn tag `Sapient` mà không kèm hợp đồng nhận thức là vi
        // phạm. Đường sinh ra thực thể phải cấp đủ hợp đồng **cùng lúc** với
        // tag, chứ không phải để một bước sau đó bổ sung — nếu không, sẽ có một
        // khoảng thời gian thực thể tồn tại ở trạng thái không hợp lệ, và bất
        // biến chạy trong khoảng đó sẽ đỏ một cách chính đáng.
        if la_sapient {
            ctx.set(id, "cognition.persona_version", 1i64);
            ctx.set(id, "cognition.memory_namespace", format!("e{}", id.get()));
            ctx.set(id, "cognition.branch_scope", ctx.command.world.get());
            ctx.set(id, "cognition.fallback", "routine");
        }
        ctx.emit_caused(EventDraft::new("core.entity.spawned", val! { "kind" => kind }).on(id));
        Ok(())
    });

    r.on("core.set_attr", |ctx| {
        let who = ctx.require_entity_field("entity")?;
        ctx.require_entity(who)?;
        let key = ctx.require_text("key")?.to_owned();
        let v = ctx
            .command
            .payload
            .get("value")
            .cloned()
            .ok_or_else(|| Failure::missing("value"))?;
        ctx.set(who, &key, v);
        Ok(())
    });

    // Đặt nhu cầu, kèm mốc và miền đồng hồ để không vi phạm `INV-22-24`.
    r.on("core.set_need", |ctx| {
        let who = ctx.require_entity_field("entity")?;
        ctx.require_entity(who)?;
        let need = ctx.require_text("need")?.to_owned();
        let value = ctx.require_int("value")?;
        let tick = ctx.tick.0 as i64;
        ctx.set(who, &format!("need.{need}"), value);
        ctx.set(who, "need.last_update_tick", tick);
        ctx.set(who, "need.clock_domain", "proper");
        ctx.emit_caused(
            EventDraft::new("core.need.set", val! { "need" => need, "value" => value }).on(who),
        );
        Ok(())
    });

    // Bước "làm cho chuyện gì đó xảy ra", để kịch bản khói có thứ để chờ.
    r.on("core.commit_act", |ctx| {
        let who = ctx.require_entity_field("actor")?;
        ctx.require_entity(who)?;
        let act = ctx.require_text("act")?.to_owned();
        let doi = ctx.store.attr_int(who, "need.hunger").unwrap_or(i64::MAX);
        if doi > 20 {
            return Err(Failure::new(
                FailureCode::PreconditionFailed,
                format!("chưa đủ đói để {act}: hunger = {doi}"),
            ));
        }
        ctx.emit_caused(EventDraft::new("core.act.committed", val! { "act" => act }).by(who));
        Ok(())
    });

    r
}

/// Nhà máy dựng thế giới nhỏ cho test.
pub struct TestWorldFactory;

impl WorldFactory for TestWorldFactory {
    fn build(&self, worldseed: &str, overrides: &BTreeMap<String, String>) -> Option<Sim> {
        // Seed suy ra từ tên worldseed cộng phần ghi đè, nên hai kịch bản dùng
        // chung worldseed vẫn có dòng ngẫu nhiên riêng.
        let mut h = mow_math::StateHasher::with_domain("mow.testworld.v1");
        h.write_str(worldseed);
        h.write_seq(overrides.iter(), |hh, (k, v)| {
            hh.write_str(k);
            hh.write_str(v);
        });
        let seed = u64::from_le_bytes(h.finish().0[..8].try_into().ok()?);

        let mut sim = Sim::new(
            SimConfig {
                world: WorldId(1),
                branch: BranchId(1),
                seed: WorldSeed(seed),
                clock: Clock::synchronous(),
            },
            handlers(),
        );

        match worldseed {
            "test:tiny_village" => {
                dung_lang(&mut sim)?;
            }
            "test:empty" => {}
            _ => return None,
        }
        Some(sim)
    }
}

fn dung_lang(sim: &mut Sim) -> Option<()> {
    use mow_core::Command;
    let w = sim.world_id();

    sim.apply(&Command::new(
        "core.spawn",
        w,
        val! { "kind" => "settlement", "name" => "Lang Nho" },
    ))
    .ok()?;
    let lang = sim.store().ids().last()?;

    // Ba dân làng cùng tuổi 40 để bài test về phá hòa có ý nghĩa: không có vế
    // `id asc` thì thứ tự giữa họ là không xác định.
    for (ten, tuoi) in [("Aren", 40i64), ("Bram", 40), ("Cira", 31)] {
        sim.apply(&Command::new(
            "core.spawn",
            w,
            val! {
                "kind" => "entity",
                "name" => ten,
                "age" => tuoi,
                "within" => lang.get(),
                "tags" => vec![Value::from("sapient")],
            },
        ))
        .ok()?;
    }

    sim.apply(&Command::new(
        "core.spawn",
        w,
        val! {
            "kind" => "building",
            "name" => "Kho Thoc",
            "within" => lang.get(),
            "tags" => vec![Value::from("food_store")],
        },
    ))
    .ok()?;

    Some(())
}
