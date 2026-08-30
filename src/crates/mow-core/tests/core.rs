//! Test hạt nhân.
//!
//! Bài đáng giá nhất ở đây là [`giao_dich_that_bai_khong_de_lai_dau_vet`]. Nó
//! kiểm chứng thứ mà `§22.1` hứa: một handler chết giữa chừng **không** để lại
//! thế giới sửa dở. Nếu bài này fail thì mọi bài khác trong dự án đều vô nghĩa,
//! vì state đã không còn tin được.

use mow_core::clock::{rebase, ClockDomain, Deadline};
use mow_core::invariant::Cost;
use mow_core::transaction::Mutation;
use mow_core::{
    val, BranchId, Clock, Command, EntityId, EventDraft, Failure, FailureCode, HandlerRegistry,
    InvariantRunner, Sim, SimConfig, Tick, Value, WorldId,
};
use mow_math::{CanonicalHash, Rate, WorldSeed};

const W: WorldId = WorldId(1);

/// Sổ handler dùng chung cho phần lớn bài test.
fn so_handler() -> HandlerRegistry {
    let mut r = HandlerRegistry::new();

    // Tạo một sinh vật đơn giản.
    r.on("core.spawn_creature", |ctx| {
        let ten = ctx.require_text("name")?.to_owned();
        let id = ctx.spawn();
        ctx.set(id, "core.name", ten.clone());
        ctx.set(id, "core.pos.x", 0i64);
        ctx.set(id, "core.pos.y", 0i64);
        ctx.emit(EventDraft::new("core.entity.spawned", val! { "name" => ten }).on(id));
        Ok(())
    });

    // Di chuyển.
    r.on("core.move", |ctx| {
        let who = ctx.require_entity_field("who")?;
        ctx.require_entity(who)?;
        let dx = ctx.require_int("dx")?;
        let cu = ctx.store.attr_int(who, "core.pos.x").unwrap_or(0);
        let moi = cu
            .checked_add(dx)
            .ok_or_else(|| Failure::new(FailureCode::Arithmetic, "tọa độ tràn"))?;
        ctx.set(who, "core.pos.x", moi);
        ctx.emit(EventDraft::new("core.entity.moved", val! { "to" => moi }).on(who));
        Ok(())
    });

    // Handler hỏng có chủ đích: đẩy vài mutation hợp lệ rồi mới thất bại.
    // Đây là hình dạng chính xác của lỗi mà kiến trúc phải chịu được.
    r.on("test.half_way_failure", |ctx| {
        let who = ctx.require_entity_field("who")?;
        ctx.set(who, "core.name", "đã đổi");
        let moi = ctx.spawn();
        ctx.set(moi, "core.name", "kẻ không nên tồn tại");
        Err(Failure::new(FailureCode::PreconditionFailed, "đổi ý"))
    });

    // Handler đẩy mutation không áp được, để kiểm bước kiểm-trước-áp-sau.
    r.on("test.bad_mutation", |ctx| {
        ctx.mutate(Mutation::SetAttr {
            id: EntityId(9_999),
            key: "core.name".into(),
            value: Value::from("ma"),
        });
        Ok(())
    });

    r
}

fn sim_moi() -> Sim {
    Sim::new(
        SimConfig {
            world: W,
            branch: BranchId(1),
            seed: WorldSeed(1234),
            clock: Clock::synchronous(),
        },
        so_handler(),
    )
}

fn tao(sim: &mut Sim, ten: &str) -> EntityId {
    sim.apply(&Command::new(
        "core.spawn_creature",
        W,
        val! { "name" => ten },
    ))
    .expect("tạo được");
    sim.store().ids().last().expect("vừa tạo xong")
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.1 — giao dịch nguyên tử
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn giao_dich_that_bai_khong_de_lai_dau_vet() {
    let mut sim = sim_moi();
    let ai = tao(&mut sim, "Lan");

    let truoc_hash = sim.state_hash();
    let truoc_so_entity = sim.store().len();
    let truoc_so_event = sim.log().len();

    let kq = sim.apply(&Command::new(
        "test.half_way_failure",
        W,
        val! { "who" => ai.get() },
    ));

    assert!(kq.is_err(), "handler phải thất bại");
    assert_eq!(
        sim.state_hash(),
        truoc_hash,
        "state đã đổi dù giao dịch thất bại — §22.1 bị vi phạm"
    );
    assert_eq!(
        sim.store().len(),
        truoc_so_entity,
        "thực thể lạ được tạo ra"
    );
    assert_eq!(
        sim.log().len(),
        truoc_so_event,
        "sự kiện được ghi khi thất bại"
    );
    assert_eq!(
        sim.store().attr_text(ai, "core.name"),
        Some("Lan"),
        "thuộc tính bị sửa dù giao dịch thất bại"
    );
}

#[test]
fn cap_phat_id_cung_bi_lui_khi_that_bai() {
    // Nếu id không lùi, hai lần chạy khác nhau ở *số lần command từng thất
    // bại* sẽ cho hai thế giới khác nhau — và không log nào giải thích được.
    let mut sim_a = sim_moi();
    let mut sim_b = sim_moi();

    let a = tao(&mut sim_a, "Lan");
    let _ = sim_a.apply(&Command::new(
        "test.half_way_failure",
        W,
        val! { "who" => a.get() },
    ));
    tao(&mut sim_a, "Bình");

    tao(&mut sim_b, "Lan");
    tao(&mut sim_b, "Bình");

    assert_eq!(
        sim_a.state_hash(),
        sim_b.state_hash(),
        "một command thất bại đã làm lệch thế giới"
    );
}

#[test]
fn mutation_khong_ap_duoc_bi_chan_truoc_khi_ap_cai_dau() {
    let mut sim = sim_moi();
    tao(&mut sim, "Lan");
    let truoc = sim.state_hash();

    let e = sim
        .apply(&Command::new("test.bad_mutation", W, Value::map()))
        .expect_err("phải bị từ chối");
    assert_eq!(e.code, FailureCode::NoSuchEntity);
    assert_eq!(sim.state_hash(), truoc);
}

#[test]
fn khong_co_handler_thi_bao_ro_rang() {
    let mut sim = sim_moi();
    let e = sim
        .apply(&Command::new("khong.ton.tai", W, Value::map()))
        .expect_err("phải lỗi");
    assert_eq!(e.code, FailureCode::UnknownCommand);
}

#[test]
fn command_gui_nham_the_gioi_bi_tu_choi() {
    let mut sim = sim_moi();
    let e = sim
        .apply(&Command::new(
            "core.spawn_creature",
            WorldId(999),
            val! { "name" => "x" },
        ))
        .expect_err("phải lỗi");
    assert_eq!(e.code, FailureCode::PreconditionFailed);
}

// ─────────────────────────────────────────────────────────────────────────────
// §20.2.2 — idempotency
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cung_request_id_chi_co_tac_dung_mot_lan() {
    // Kết quả LLM có thể tới muộn rồi tới lại. Nếu nó có tác dụng hai lần thì
    // nhân vật hành động hai lần cho một lần suy nghĩ.
    let mut sim = sim_moi();
    let cmd = Command::new("core.spawn_creature", W, val! { "name" => "Lan" }).with_request_id(77);

    sim.apply(&cmd).expect("lần đầu phải được");
    let sau_lan_dau = sim.state_hash();

    let e = sim.apply(&cmd).expect_err("lần hai phải bị từ chối");
    assert_eq!(e.code, FailureCode::DuplicateRequest);
    assert_eq!(sim.state_hash(), sau_lan_dau);
}

// ─────────────────────────────────────────────────────────────────────────────
// §8.4 — nhật ký chỉ ghi thêm
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nhat_ky_chi_ghi_them_va_hash_tich_luy_doi_theo() {
    let mut sim = sim_moi();
    let h0 = sim.log().running_hash();

    let ai = tao(&mut sim, "Lan");
    let h1 = sim.log().running_hash();
    assert_ne!(h0, h1);

    sim.apply(&Command::new(
        "core.move",
        W,
        val! { "who" => ai.get(), "dx" => 3i64 },
    ))
    .unwrap();
    let h2 = sim.log().running_hash();
    assert_ne!(h1, h2);

    // Số thứ tự liên tục, không có lỗ hổng.
    let seqs: Vec<u64> = sim.log().iter().map(|e| e.seq.0).collect();
    assert_eq!(seqs, vec![0, 1]);
}

#[test]
fn chuoi_nhan_qua_truy_nguoc_duoc() {
    let mut sim = sim_moi();
    let ai = tao(&mut sim, "Lan");
    let goc = sim.last_event().unwrap();

    // Một handler tạm gắn nguyên nhân tường minh.
    let mut r = so_handler();
    r.on("test.caused", move |ctx| {
        let who = ctx.require_entity_field("who")?;
        ctx.emit(
            EventDraft::new("core.entity.noted", Value::map())
                .on(who)
                .caused_by(goc),
        );
        Ok(())
    });
    let mut sim2 = Sim::new(
        SimConfig {
            world: W,
            branch: BranchId(1),
            seed: WorldSeed(1234),
            clock: Clock::synchronous(),
        },
        r,
    );
    let ai2 = tao(&mut sim2, "Lan");
    sim2.apply(&Command::new("test.caused", W, val! { "who" => ai2.get() }))
        .unwrap();

    let cuoi = sim2.last_event().unwrap();
    let chuoi = sim2.log().cause_chain(cuoi, 16);
    assert_eq!(chuoi.len(), 2, "phải truy về được sự kiện sinh ra");
    assert_eq!(chuoi[1].kind.0, "core.entity.spawned");
    let _ = ai;
}

#[test]
fn chuoi_nhan_qua_khong_treo_khi_du_lieu_hong() {
    // Một chuỗi tự trỏ về mình sẽ treo giao diện nếu không có chặn độ sâu, và
    // giao diện treo thì không ai gỡ lỗi bằng nó được nữa.
    let mut r = HandlerRegistry::new();
    r.on("test.self_cause", |ctx| {
        let seq = mow_core::EventSeq(0);
        ctx.emit(EventDraft::new("core.loop", Value::map()).caused_by(seq));
        Ok(())
    });
    let mut sim = Sim::new(SimConfig::default(), r);
    sim.apply(&Command::new("test.self_cause", WorldId(1), Value::map()))
        .unwrap();
    let chuoi = sim.log().cause_chain(mow_core::EventSeq(0), 1_000_000);
    assert_eq!(chuoi.len(), 1, "vòng lặp phải bị cắt");
}

// ─────────────────────────────────────────────────────────────────────────────
// Determinism
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cung_seed_cung_command_cho_cung_hash() {
    let chay = || {
        let mut sim = sim_moi();
        for ten in ["Lan", "Bình", "Cúc", "Dũng"] {
            tao(&mut sim, ten);
        }
        for id in sim.store().ids().collect::<Vec<_>>() {
            sim.apply(&Command::new(
                "core.move",
                W,
                val! { "who" => id.get(), "dx" => 2i64 },
            ))
            .unwrap();
        }
        sim.advance(100).unwrap();
        sim.state_hash()
    };
    assert_eq!(chay(), chay());
}

#[test]
fn seed_khac_thi_hash_khac() {
    let mk = |seed: u64| {
        let mut sim = Sim::new(
            SimConfig {
                world: W,
                branch: BranchId(1),
                seed: WorldSeed(seed),
                clock: Clock::synchronous(),
            },
            so_handler(),
        );
        tao(&mut sim, "Lan");
        sim.state_hash()
    };
    assert_ne!(mk(1), mk(2));
}

#[test]
fn thu_tu_thuoc_tinh_khong_anh_huong_hash() {
    // `BTreeMap` bảo đảm điều này; bài test khóa nó lại để một lần đổi sang
    // `HashMap` "cho nhanh" sẽ bị bắt ngay.
    let mut r = HandlerRegistry::new();
    r.on("t.a", |ctx| {
        let id = ctx.spawn();
        ctx.set(id, "z", 1i64);
        ctx.set(id, "a", 2i64);
        ctx.set(id, "m", 3i64);
        Ok(())
    });
    r.on("t.b", |ctx| {
        let id = ctx.spawn();
        ctx.set(id, "a", 2i64);
        ctx.set(id, "m", 3i64);
        ctx.set(id, "z", 1i64);
        Ok(())
    });
    let mk = |kind: &str| {
        let mut s = Sim::new(SimConfig::default(), {
            let mut rr = HandlerRegistry::new();
            rr.on("t.a", |ctx| {
                let id = ctx.spawn();
                ctx.set(id, "z", 1i64);
                ctx.set(id, "a", 2i64);
                ctx.set(id, "m", 3i64);
                Ok(())
            });
            rr.on("t.b", |ctx| {
                let id = ctx.spawn();
                ctx.set(id, "a", 2i64);
                ctx.set(id, "m", 3i64);
                ctx.set(id, "z", 1i64);
                Ok(())
            });
            rr
        });
        s.apply(&Command::new(kind, WorldId(1), Value::map()))
            .unwrap();
        s.store().attrs(EntityId(1)).unwrap().clone()
    };
    assert_eq!(mk("t.a"), mk("t.b"));
    let _ = r;
}

// ─────────────────────────────────────────────────────────────────────────────
// §4.5 — miền đồng hồ và rebase
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn qua_cong_khong_lam_nguoi_benh_khoi_ngay_lap_tuc() {
    // Thế giới A chạy 1 tick địa phương mỗi tick thần.
    // Thế giới B chạy 10 tick địa phương mỗi tick thần.
    let mut a = Clock::new(Rate::per_tick(1));
    let mut b = Clock::new(Rate::per_tick(10));
    a.advance_divine(100).unwrap();
    b.advance_divine(100).unwrap();
    assert_eq!(a.local(), Tick(100));
    assert_eq!(b.local(), Tick(1000));

    // Còn 50 tick nữa thì hết ủ bệnh, tính theo thời gian riêng.
    let u_benh = Deadline::new(Tick(150), ClockDomain::Proper);
    let sau = rebase(u_benh, &a, &b).unwrap();

    // Ở B, 50 tick riêng phải thành 500 tick địa phương — chứ không phải
    // "đã quá hạn từ lâu" như khi đổi đồng loạt theo đồng hồ thế giới.
    assert_eq!(sau.domain, ClockDomain::Proper);
    assert_eq!(sau.at, Tick(1500));
    assert!(!sau.is_due(&b), "vừa qua cổng đã hết bệnh");
}

#[test]
fn hop_dong_dia_phuong_khong_bi_rebase() {
    // Nếu rebase cả `WorldLocal`, một khoản vay sẽ đáo hạn tức thì khi con nợ
    // bỏ trốn qua cổng — và người chơi sẽ khai thác điều đó trong năm phút.
    let mut a = Clock::new(Rate::per_tick(1));
    let mut b = Clock::new(Rate::per_tick(10));
    a.advance_divine(100).unwrap();
    b.advance_divine(100).unwrap();

    let no = Deadline::new(Tick(150), ClockDomain::WorldLocal);
    assert_eq!(rebase(no, &a, &b).unwrap(), no);

    let than = Deadline::new(Tick(150), ClockDomain::Divine);
    assert_eq!(rebase(than, &a, &b).unwrap(), than);

    let nguyen = Deadline::new(Tick(150), ClockDomain::LawDefined);
    assert_eq!(rebase(nguyen, &a, &b).unwrap(), nguyen);
}

#[test]
fn dong_ho_ti_le_huu_ti_khong_troi_sau_nhieu_tick() {
    // Tỉ lệ 7/3 với số thực sẽ trôi; với hữu tỉ mang số dư thì không.
    let mut c = Clock::new(Rate::new(7, 3).unwrap());
    for _ in 0..300 {
        c.advance_divine(1).unwrap();
    }
    assert_eq!(c.divine(), Tick(300));
    assert_eq!(c.local(), Tick(700), "7/3 × 300 phải đúng bằng 700");
}

// ─────────────────────────────────────────────────────────────────────────────
// Bất biến
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn the_gioi_sach_thi_khong_vi_pham() {
    let mut sim = sim_moi();
    tao(&mut sim, "Lan");
    let rep = sim.check(&InvariantRunner::standard(Cost::Expensive));
    assert!(rep.is_clean(), "{rep}");
    assert!(rep.checked.len() >= 5, "phải chạy ít nhất 5 bất biến");
}

#[test]
fn inv_22_11_bat_toa_do_sai_kieu() {
    let mut r = so_handler();
    r.on("test.toa_do_hong", |ctx| {
        let id = ctx.spawn();
        ctx.set(id, "core.pos.x", "ba mươi");
        ctx.emit(EventDraft::new("core.entity.spawned", Value::map()).on(id));
        Ok(())
    });
    let mut sim = Sim::new(SimConfig::default(), r);
    sim.apply(&Command::new("test.toa_do_hong", WorldId(1), Value::map()))
        .unwrap();

    let rep = sim.check(&InvariantRunner::standard(Cost::Cheap));
    assert!(!rep.is_clean());
    assert!(rep.violations.iter().any(|v| v.id == "INV-22-11"), "{rep}");
}

#[test]
fn inv_22_24_bat_nhu_cau_thieu_moc_thoi_gian() {
    let mut r = so_handler();
    r.on("test.doi_khong_moc", |ctx| {
        let id = ctx.spawn();
        ctx.set(id, "need.hunger", 50i64);
        ctx.emit(EventDraft::new("core.entity.spawned", Value::map()).on(id));
        Ok(())
    });
    let mut sim = Sim::new(SimConfig::default(), r);
    sim.apply(&Command::new(
        "test.doi_khong_moc",
        WorldId(1),
        Value::map(),
    ))
    .unwrap();

    let rep = sim.check(&InvariantRunner::standard(Cost::Cheap));
    let n = rep
        .violations
        .iter()
        .filter(|v| v.id == "INV-22-24")
        .count();
    assert_eq!(n, 2, "phải bắt cả thiếu mốc lẫn thiếu miền đồng hồ: {rep}");
}

#[test]
fn inv_22_33_bat_vat_pham_o_hai_noi() {
    let mut r = so_handler();
    r.on("test.vat_pham_hai_noi", |ctx| {
        let id = ctx.spawn();
        ctx.set(id, "item.def", "core.apple");
        ctx.set(id, "loc.cell", 1i64);
        ctx.set(id, "loc.inventory", 2i64);
        ctx.emit(EventDraft::new("core.entity.spawned", Value::map()).on(id));
        Ok(())
    });
    let mut sim = Sim::new(SimConfig::default(), r);
    sim.apply(&Command::new(
        "test.vat_pham_hai_noi",
        WorldId(1),
        Value::map(),
    ))
    .unwrap();

    let rep = sim.check(&InvariantRunner::standard(Cost::Medium));
    assert!(rep.violations.iter().any(|v| v.id == "INV-22-33"), "{rep}");
}

#[test]
fn inv_22_3_bat_ca_hai_chieu() {
    let mut r = so_handler();
    // Sapient thiếu hợp đồng.
    r.on("test.sapient_thieu", |ctx| {
        let id = ctx.spawn();
        ctx.set(id, "tag.sapient", true);
        ctx.emit(EventDraft::new("core.entity.spawned", Value::map()).on(id));
        Ok(())
    });
    // Con vật lại có memory namespace — nó sẽ ăn ngân sách nhận thức.
    r.on("test.thu_co_ky_uc", |ctx| {
        let id = ctx.spawn();
        ctx.set(id, "cognition.memory_namespace", "sheep-01");
        ctx.emit(EventDraft::new("core.entity.spawned", Value::map()).on(id));
        Ok(())
    });

    let mut sim = Sim::new(SimConfig::default(), r);
    sim.apply(&Command::new(
        "test.sapient_thieu",
        WorldId(1),
        Value::map(),
    ))
    .unwrap();
    sim.apply(&Command::new("test.thu_co_ky_uc", WorldId(1), Value::map()))
        .unwrap();

    let rep = sim.check(&InvariantRunner::standard(Cost::Medium));
    let n = rep.violations.iter().filter(|v| v.id == "INV-22-3").count();
    assert_eq!(n, 5, "4 trường thiếu + 1 con vật có ký ức: {rep}");
}

#[test]
fn inv_22_1_bat_thuc_the_khong_co_su_kien_sinh() {
    let mut r = so_handler();
    r.on("test.sinh_len", |ctx| {
        ctx.spawn(); // không phát sự kiện
        Ok(())
    });
    let mut sim = Sim::new(SimConfig::default(), r);
    sim.apply(&Command::new("test.sinh_len", WorldId(1), Value::map()))
        .unwrap();

    let rep = sim.check(&InvariantRunner::standard(Cost::Medium));
    assert!(rep.violations.iter().any(|v| v.id == "INV-22-1"), "{rep}");
}

#[test]
fn loc_theo_muc_chi_phi() {
    let sim = sim_moi();
    let re = sim.check(&InvariantRunner::standard(Cost::Cheap));
    let day_du = sim.check(&InvariantRunner::standard(Cost::Expensive));
    assert!(
        re.checked.len() < day_du.checked.len(),
        "mức Cheap phải chạy ít bất biến hơn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Value
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn value_khong_co_bien_the_so_thuc() {
    // Không thể viết bài test "không compile được" ở đây, nên bài này khóa
    // hình dạng của kiểu: nếu ai đó thêm `Value::Float`, `type_name` sẽ phải
    // đổi và bài này sẽ nhắc họ đọc lại §P10.2.
    let cac_loai = [
        Value::Null,
        Value::Bool(true),
        Value::Int(1),
        Value::Uint(1),
        Value::Fixed(mow_math::Fx::ONE),
        Value::Text("x".into()),
        Value::Bytes(vec![1]),
        Value::List(vec![]),
        Value::map(),
    ];
    let ten: Vec<&str> = cac_loai.iter().map(Value::type_name).collect();
    assert_eq!(
        ten,
        vec!["null", "bool", "int", "uint", "fixed", "text", "bytes", "list", "map"]
    );
    assert!(!ten.contains(&"float"), "số thực lọt vào Value");
}

#[test]
fn value_phan_biet_int_va_uint_trong_hash() {
    assert_ne!(
        Value::Int(1).state_hash(),
        Value::Uint(1).state_hash(),
        "hai kiểu số khác nhau cho cùng hash"
    );
}
