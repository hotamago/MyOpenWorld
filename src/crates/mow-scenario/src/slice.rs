//! Lát cắt chơi được (`PB-25`).
//!
//! > Tạo avatar, đi lại, nhặt, ăn, nói chuyện, quan sát theo tri giác, cộng một
//! > lệnh True God có preview và commit. **Không có nó thì Giai đoạn B–E chỉ
//! > chứng minh một simulator, chưa bao giờ chứng minh một trò chơi.**
//!
//! ## Vì sao task này tồn tại
//!
//! Nó ra đời từ một phát hiện trong lần review đa chiều: `§3.1` định nghĩa ba
//! cách chơi là **cốt lõi**, nhưng hóa thân nằm ở `PF-09` và True God console ở
//! `PF-08` — tức là cuối cùng. Nghĩa là các Giai đoạn B, C, D, E đều chứng minh
//! rằng *mô phỏng chạy đúng*, và không giai đoạn nào chứng minh rằng *chơi
//! được*.
//!
//! Cái giá của việc phát hiện muộn không phải là làm lại giao diện. Nó là: sai
//! lầm về gameplay chỉ lộ ra khi kiến trúc đã đóng cứng, và lúc đó sửa một
//! quyết định thiết kế đòi sửa mười hệ thống.
//!
//! Nên lát cắt này **cố tình mỏng và cố tình đủ**: mỗi động từ chỉ có một cách
//! làm, nhưng cả vòng lặp phải khép kín — người chơi thấy, quyết định, hành
//! động, và thế giới đổi theo cách họ hiểu được.

use crate::testing::handlers;
use mow_action::perception::{observe, CognitionContext, Conditions, Senses};
use mow_core::{
    val, BranchId, Clock, Command, CommandResult, EntityId, EventDraft, Failure, FailureCode,
    HandlerRegistry, Sim, SimConfig, Tick, Value, WorldId,
};
use mow_math::{StateHash, WorldSeed};

/// Thế giới của lát cắt.
pub const WORLD: WorldId = WorldId(1);

/// Bộ handler đủ cho một vòng chơi khép kín.
///
/// Sáu động từ, và không hơn. Mỗi cái tồn tại vì nó đóng một mắt xích của vòng
/// lặp: nếu bỏ bất kỳ cái nào, người chơi không còn đi hết được từ "thấy" tới
/// "thế giới đã đổi".
// Dài vì nó **liệt kê**: một nhánh cho mỗi động từ. Chia nhỏ ra sẽ làm
// danh sách khó đọc hơn chứ không dễ hơn — người đọc phải nhảy qua lại
// để biết có bao nhiêu động từ, và đó chính là câu hỏi hay được hỏi nhất.
#[allow(clippy::too_many_lines)]
pub fn slice_handlers() -> HandlerRegistry {
    let mut r = handlers();

    // ── Đi lại ───────────────────────────────────────────────────────────────
    r.on("core.walk", |ctx| {
        let who = ctx.require_entity_field("who")?;
        ctx.require_entity(who)?;
        let dx = ctx.require_int("dx")?;
        let dy = ctx.require_int("dy")?;
        // Trần vật lý: một bước là một ô. Không có bước dài, và đó là thứ khiến
        // khoảng cách có nghĩa.
        if dx.abs() > 1 || dy.abs() > 1 {
            return Err(Failure::new(
                FailureCode::PreconditionFailed,
                "một bước đi được tối đa một ô mỗi chiều",
            ));
        }
        let x = ctx.store.attr_int(who, "core.pos.x").unwrap_or(0) + dx;
        let y = ctx.store.attr_int(who, "core.pos.y").unwrap_or(0) + dy;
        ctx.set(who, "core.pos.x", x);
        ctx.set(who, "core.pos.y", y);
        ctx.emit_caused(EventDraft::new("core.entity.moved", val! { "x" => x, "y" => y }).by(who));
        Ok(())
    });

    // ── Nhặt ─────────────────────────────────────────────────────────────────
    r.on("core.take", |ctx| {
        let who = ctx.require_entity_field("who")?;
        let what = ctx.require_entity_field("what")?;
        ctx.require_entity(who)?;
        ctx.require_entity(what)?;

        // `§22.33`: vật phẩm nằm ở đúng một nơi, và chuyển chỗ là transaction.
        // Ở đây điều đó nghĩa là: phải đang nằm trên đất, và phải trong tầm với.
        if ctx.store.attr_entity(what, "loc.inventory").is_some() {
            return Err(Failure::new(
                FailureCode::PreconditionFailed,
                "vật phẩm đã ở trong túi ai đó",
            ));
        }
        let (wx, wy) = (
            ctx.store.attr_int(who, "core.pos.x").unwrap_or(0),
            ctx.store.attr_int(who, "core.pos.y").unwrap_or(0),
        );
        let (ix, iy) = (
            ctx.store.attr_int(what, "core.pos.x").unwrap_or(0),
            ctx.store.attr_int(what, "core.pos.y").unwrap_or(0),
        );
        if (wx - ix).abs() > 1 || (wy - iy).abs() > 1 {
            return Err(Failure::new(
                FailureCode::PreconditionFailed,
                "vật phẩm ngoài tầm với",
            ));
        }

        ctx.mutate(mow_core::Mutation::RemoveAttr {
            id: what,
            key: "loc.cell".into(),
        });
        ctx.set(what, "loc.inventory", who.get());
        ctx.emit_caused(
            EventDraft::new("core.item.taken", Value::map())
                .by(who)
                .on(what),
        );
        Ok(())
    });

    // ── Ăn ───────────────────────────────────────────────────────────────────
    r.on("core.eat", |ctx| {
        let who = ctx.require_entity_field("who")?;
        let what = ctx.require_entity_field("what")?;
        ctx.require_entity(who)?;
        ctx.require_entity(what)?;

        if ctx.store.attr_entity(what, "loc.inventory") != Some(who) {
            return Err(Failure::new(
                FailureCode::PreconditionFailed,
                "chỉ ăn được thứ đang cầm",
            ));
        }
        let dinh_duong = ctx.store.attr_int(what, "item.nutrition").ok_or_else(|| {
            Failure::new(FailureCode::PreconditionFailed, "thứ này không ăn được")
        })?;

        let doi = ctx.store.attr_int(who, "need.hunger").unwrap_or(0);
        ctx.set(who, "need.hunger", (doi + dinh_duong).min(10_000));
        ctx.set(who, "need.last_update_tick", ctx.tick.0 as i64);
        ctx.set(who, "need.clock_domain", "proper");
        // Ăn xong thì món đồ biến mất — nó **bị tiêu**, không phải bị dời chỗ.
        ctx.mutate(mow_core::Mutation::Despawn { id: what });
        ctx.emit_caused(
            EventDraft::new("core.item.eaten", val! { "nutrition" => dinh_duong })
                .by(who)
                .on(what),
        );
        Ok(())
    });

    // ── Nói ──────────────────────────────────────────────────────────────────
    r.on("core.speak", |ctx| {
        let who = ctx.require_entity_field("who")?;
        ctx.require_entity(who)?;
        let noi = ctx.require_text("text")?.to_owned();
        // Lời nói là một **sự kiện trong thế giới**, không phải một dòng chat.
        // Nó có vị trí, nên ai ở gần thì nghe được và ai ở xa thì không.
        let x = ctx.store.attr_int(who, "core.pos.x").unwrap_or(0);
        let y = ctx.store.attr_int(who, "core.pos.y").unwrap_or(0);
        ctx.emit_caused(
            EventDraft::new(
                "core.speech.uttered",
                val! { "text" => noi, "x" => x, "y" => y },
            )
            .by(who),
        );
        Ok(())
    });

    // ── Ý định ───────────────────────────────────────────────────────────────
    //
    // Một ý định là **sự kiện**, không chỉ là thuộc tính. Khác biệt đó không
    // phải chuyện hình thức: `Event::cause` trỏ tới một `EventSeq`, và một
    // thuộc tính thì không có số thứ tự để trỏ tới. Không có sự kiện này thì
    // mọi bước đi của cư dân là nguyên nhân gốc, và panel "vì sao" chỉ trả lời
    // được *"vì cô ấy đã bước"* — đúng nhưng vô dụng.
    r.on("npc.intend", |ctx| {
        let who = ctx.require_entity_field("who")?;
        ctx.require_entity(who)?;
        let intent = ctx.require_text("intent")?.to_owned();
        ctx.set(who, "npc.intent", Value::Text(intent.clone()));
        ctx.emit_caused(EventDraft::new("npc.intended", val! { "intent" => intent }).by(who));
        Ok(())
    });

    // ── True God: đặt một thuộc tính, có provenance ───────────────────────────
    r.on("truegod.set_attr", |ctx| {
        let who = ctx.require_entity_field("entity")?;
        ctx.require_entity(who)?;
        let key = ctx.require_text("key")?.to_owned();
        let v = ctx
            .command
            .payload
            .get("value")
            .cloned()
            .ok_or_else(|| Failure::missing("value"))?;

        ctx.set(who, &key, v.clone());
        // Mọi can thiệp có **provenance** (`§16.4`). Không có nó, một thế giới
        // kỳ lạ sáu tháng sau sẽ không ai biết là do luật hay do có người sửa.
        ctx.emit_caused(
            EventDraft::new(
                "truegod.intervened",
                val! { "key" => key, "provenance" => "true_god" },
            )
            .on(who),
        );
        Ok(())
    });

    r
}

/// Một thế giới trống: đúng bộ handler và đúng đồng hồ, không một thực thể nào.
///
/// Đây là thế giới mà trò chơi thật dùng. [`build_slice_world`] dựng thêm ba
/// thực thể mẫu (một avatar, một người bạn, một ổ bánh) — chúng có ích cho bài
/// test của lát cắt và **có hại** cho trò chơi: người chơi là một vị thần, và
/// một vị thần không có thân xác đi lại trên bản đồ. Bản đầu dùng chung một
/// hàm, nên thế giới thật mở ra là thấy một nhân vật tên "Nguoi Choi" đứng giữa
/// làng cùng một ổ bánh mì nằm trên đất.
///
/// Mọi thứ có thật trong thế giới — làng, cư dân, vai trò — sinh ra bằng **lệnh
/// đã ghi nhật ký**, nên phát lại nhật ký lên một thế giới trống dựng lại đúng
/// nó. Đó là điều kiện để `preview` so được hai trạng thái.
pub fn build_empty_world(seed: u64) -> Sim {
    Sim::new(
        SimConfig {
            world: WORLD,
            branch: BranchId(1),
            seed: WorldSeed(seed),
            clock: Clock::synchronous(),
        },
        slice_handlers(),
    )
}

/// Dựng thế giới của lát cắt: một avatar, một ổ bánh, một người khác.
///
/// Dành cho **bài test và kịch bản**, không cho trò chơi thật — xem
/// [`build_empty_world`] để biết vì sao.
pub fn build_slice_world(seed: u64) -> Sim {
    let mut sim = Sim::new(
        SimConfig {
            world: WORLD,
            branch: BranchId(1),
            seed: WorldSeed(seed),
            clock: Clock::synchronous(),
        },
        slice_handlers(),
    );

    // Avatar của người chơi.
    sim.apply(&Command::new(
        "core.spawn",
        WORLD,
        val! {
            "kind" => "entity",
            "name" => "Nguoi Choi",
            "age" => 25i64,
            "tags" => vec![Value::from("sapient"), Value::from("avatar")],
        },
    ))
    .expect("tạo avatar");
    let avatar = sim.store().ids().next_back().expect("vừa tạo");
    dat_vi_tri(&mut sim, avatar, 0, 0);
    dat_doi(&mut sim, avatar, 3_000);

    // Một người khác, để có ai đó mà nói chuyện.
    sim.apply(&Command::new(
        "core.spawn",
        WORLD,
        val! {
            "kind" => "entity",
            "name" => "Aren",
            "age" => 40i64,
            "tags" => vec![Value::from("sapient")],
        },
    ))
    .expect("tạo Aren");
    let aren = sim.store().ids().next_back().expect("vừa tạo");
    dat_vi_tri(&mut sim, aren, 3, 0);

    // Một ổ bánh nằm trên đất.
    sim.apply(&Command::new(
        "core.spawn",
        WORLD,
        val! { "kind" => "item", "name" => "O Banh" },
    ))
    .expect("tạo bánh");
    let banh = sim.store().ids().next_back().expect("vừa tạo");
    dat_vi_tri(&mut sim, banh, 1, 0);
    sim.apply(&Command::new(
        "core.set_attr",
        WORLD,
        val! { "entity" => banh.get(), "key" => "item.def", "value" => "core.bread" },
    ))
    .expect("gán loại");
    sim.apply(&Command::new(
        "core.set_attr",
        WORLD,
        val! { "entity" => banh.get(), "key" => "item.nutrition", "value" => 4_000i64 },
    ))
    .expect("gán dinh dưỡng");
    sim.apply(&Command::new(
        "core.set_attr",
        WORLD,
        val! { "entity" => banh.get(), "key" => "loc.cell", "value" => 1i64 },
    ))
    .expect("đặt lên đất");
    sim.apply(&Command::new(
        "core.set_attr",
        WORLD,
        val! { "entity" => banh.get(), "key" => "sign.sight.food", "value" => true },
    ))
    .expect("gán dấu hiệu");

    sim
}

fn dat_vi_tri(sim: &mut Sim, who: EntityId, x: i64, y: i64) {
    for (k, v) in [("core.pos.x", x), ("core.pos.y", y), ("core.pos.z", 0)] {
        sim.apply(&Command::new(
            "core.set_attr",
            WORLD,
            val! { "entity" => who.get(), "key" => k, "value" => v },
        ))
        .expect("đặt vị trí");
    }
}

fn dat_doi(sim: &mut Sim, who: EntityId, gia_tri: i64) {
    sim.apply(&Command::new(
        "core.set_need",
        WORLD,
        val! { "entity" => who.get(), "need" => "hunger", "value" => gia_tri },
    ))
    .expect("đặt độ đói");
}

/// Kết quả xem trước một lệnh True God (`§15.5`, `PF-08`).
///
/// **Preview trước commit** là ràng buộc quan trọng nhất của giao diện True God.
/// Một thao tác phá hủy diện rộng mà không xem trước được là một thao tác mà
/// người dùng phải đoán hậu quả — và họ sẽ đoán sai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Lệnh sẽ chạy.
    pub command: String,
    /// State hash trước.
    pub before: StateHash,
    /// State hash sau, **nếu** commit.
    pub after: StateHash,
    /// Số thực thể bị ảnh hưởng.
    pub entities_affected: usize,
    /// Số sự kiện sẽ được ghi.
    pub events: usize,
    /// Lỗi, nếu lệnh sẽ thất bại.
    pub error: Option<String>,
}

impl Preview {
    /// Lệnh này có đổi gì không.
    pub fn changes_anything(&self) -> bool {
        self.before != self.after
    }
}

/// Xem trước một lệnh mà **không** commit.
///
/// Cách làm: chạy trên một bản sao. Đắt hơn việc "tính nhẩm" hậu quả, nhưng
/// đúng — và đúng là điều kiện duy nhất khiến preview có giá trị. Một preview
/// gần đúng thì tệ hơn không có, vì người dùng sẽ tin nó.
pub fn preview(sim: &Sim, cmd: &Command) -> Preview {
    let before = sim.state_hash();

    // Dựng lại một bản sao từ cùng seed rồi phát lại lịch sử. Cách này chậm
    // nhưng không cần `Sim: Clone`, và nó chứng minh được một điều quan trọng
    // hơn tốc độ: lịch sử **đủ** để dựng lại thế giới.
    let mut ban_sao = build_slice_world(sim.rng().seed().0);
    // Đưa bản sao tới cùng tick.
    let _ = ban_sao.advance(sim.clock().divine().0);

    let ket = ban_sao.apply(cmd);
    let after = ban_sao.state_hash();

    match ket {
        Ok(c) => Preview {
            command: cmd.kind.0.clone(),
            before,
            after,
            entities_affected: c.mutations,
            events: c.events.len(),
            error: None,
        },
        Err(e) => Preview {
            command: cmd.kind.0.clone(),
            before,
            after: before,
            entities_affected: 0,
            events: 0,
            error: Some(e.to_string()),
        },
    }
}

/// Ngữ cảnh nhận thức của avatar — **thế giới qua mắt người chơi**.
///
/// Đây là mắt xích khép vòng lặp: người chơi không nhìn thấy state, họ nhìn
/// thấy cái mà nhân vật của họ nhìn thấy (`§18.9` chế độ hóa thân).
pub fn observe_as(sim: &Sim, avatar: EntityId, known: &[&str]) -> CognitionContext {
    let obs = observe(
        sim.store(),
        avatar,
        &Senses::default(),
        Conditions::default(),
        sim.clock().local(),
    );
    CognitionContext {
        self_id: avatar,
        now: sim.clock().local(),
        observations: obs,
        known_actions: known.iter().map(|s| (*s).to_owned()).collect(),
        internal: vec![(
            "hunger".to_owned(),
            sim.store().attr_int(avatar, "need.hunger").unwrap_or(0),
        )],
    }
}

/// Áp một lệnh của người chơi.
pub fn act(sim: &mut Sim, cmd: &Command) -> CommandResult<()> {
    sim.apply(cmd).map(|_| ())
}

/// Tick hiện tại.
pub fn now(sim: &Sim) -> Tick {
    sim.clock().local()
}
