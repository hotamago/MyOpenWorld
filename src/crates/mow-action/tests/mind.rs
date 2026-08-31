//! Test tri giác và utility AI.
//!
//! Bài quan trọng nhất: [`bong_nguoi_trong_suong_khong_co_danh_tinh`] — nếu nó
//! fail thì mọi cơ chế dựa trên "không biết" đều sụp: tin đồn, trộm cắp, điều
//! tra, chẩn đoán sai.

use mow_action::perception::{observe, CognitionContext, Conditions, Observation, Sense, Senses};
use mow_action::utility::{villager_brain, Consideration, Layer, RoutineSlot, Scorer};
use mow_action::Brain;
use mow_core::{
    val, BranchId, Clock, Command, EntityId, HandlerRegistry, Sim, SimConfig, Tick, WorldId,
};
use mow_math::{Unit, WorldPos, WorldSeed};

const W: WorldId = WorldId(1);

fn sim_voi_nguoi() -> Sim {
    let mut r = HandlerRegistry::new();
    r.on("t.spawn", |ctx| {
        let id = ctx.spawn();
        let x = ctx.require_int("x")?;
        let y = ctx.require_int("y")?;
        ctx.set(id, "core.pos.x", x);
        ctx.set(id, "core.pos.y", y);
        ctx.set(
            id,
            "core.pos.z",
            ctx.command.payload.get_int("z").unwrap_or(0),
        );
        if let Some(s) = ctx.command.payload.get_text("sign") {
            ctx.set(id, &format!("sign.sight.{s}"), true);
        }
        Ok(())
    });
    Sim::new(
        SimConfig {
            world: W,
            branch: BranchId(1),
            seed: WorldSeed(1),
            clock: Clock::synchronous(),
        },
        r,
    )
}

fn dat(sim: &mut Sim, x: i64, y: i64) -> EntityId {
    sim.apply(&Command::new("t.spawn", W, val! { "x" => x, "y" => y }))
        .unwrap();
    sim.store().ids().next_back().unwrap()
}

// ─────────────────────────────────────────────────────────────────────────────
// §10.2, §22.4 — tri giác là nguồn duy nhất
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn xa_qua_tam_thi_khong_thay() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    dat(&mut sim, 1_000, 0);

    let o = observe(
        sim.store(),
        toi,
        &Senses::default(),
        Conditions::default(),
        Tick(0),
    );
    assert!(o.is_empty(), "thấy được thứ cách 1000 ô");
}

#[test]
fn gan_thi_thay_ro() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    let ai_do = dat(&mut sim, 3, 0);

    let o = observe(
        sim.store(),
        toi,
        &Senses::default(),
        Conditions::default(),
        Tick(0),
    );
    let nhin = o
        .iter()
        .find(|x| x.sense == Sense::Sight)
        .expect("phải nhìn thấy");
    assert_eq!(nhin.identity, Some(ai_do));
    assert!(nhin.fidelity > Unit::from_frac(6, 10).unwrap());
}

/// **Bài quan trọng nhất.**
#[test]
fn bong_nguoi_trong_suong_khong_co_danh_tinh() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    dat(&mut sim, 3, 0);

    let suong_mu = Conditions {
        obscurity: Unit::from_frac(8, 10).unwrap(),
        ..Conditions::default()
    };
    let o = observe(sim.store(), toi, &Senses::default(), suong_mu, Tick(0));
    let nhin = o
        .iter()
        .find(|x| x.sense == Sense::Sight)
        .expect("vẫn thấy bóng");

    assert_eq!(
        nhin.identity, None,
        "nhận ra danh tính qua sương mù dày — cải trang và trốn trở nên vô nghĩa"
    );
    // Nhưng vẫn biết "có ai đó ở đó".
    assert!(nhin.fidelity > Unit::ZERO);
}

#[test]
fn toi_den_thi_khong_nhin_duoc_nhung_van_nghe_duoc() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    dat(&mut sim, 5, 0);

    let dem = Conditions {
        light: Unit::ZERO,
        ..Conditions::default()
    };
    let o = observe(sim.store(), toi, &Senses::default(), dem, Tick(0));
    assert!(
        !o.iter().any(|x| x.sense == Sense::Sight),
        "nhìn được trong bóng tối tuyệt đối"
    );
    assert!(
        o.iter().any(|x| x.sense == Sense::Hearing),
        "phải vẫn nghe được"
    );
}

#[test]
fn on_ao_at_tieng_dong() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    dat(&mut sim, 30, 0);

    let cho = Conditions {
        noise: Unit::ONE,
        ..Conditions::default()
    };
    let o = observe(sim.store(), toi, &Senses::default(), cho, Tick(0));
    assert!(!o.iter().any(|x| x.sense == Sense::Hearing));
}

#[test]
fn nghe_thi_biet_huong_khong_biet_dung_o() {
    // Đây là thứ khiến rình rập có ý nghĩa.
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    dat(&mut sim, 30, 7);

    let dem = Conditions {
        light: Unit::ZERO,
        ..Conditions::default()
    };
    let o = observe(sim.store(), toi, &Senses::default(), dem, Tick(0));
    let nghe = o.iter().find(|x| x.sense == Sense::Hearing).unwrap();
    assert_ne!(
        nghe.at,
        WorldPos::new(30, 7, 0),
        "vị trí nghe được phải là ước lượng"
    );
    assert_eq!(nghe.at, WorldPos::new(28, 4, 0));
}

#[test]
fn ngui_khong_phu_thuoc_sang_toi() {
    // Đây là lý do chó dẫn đường có giá trị.
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    dat(&mut sim, 5, 0);

    let cho_ngui = Senses {
        ranges: vec![(Sense::Smell, 20)],
        acuity: Unit::ONE,
    };
    let dem = Conditions {
        light: Unit::ZERO,
        obscurity: Unit::ONE,
        ..Conditions::default()
    };
    let o = observe(sim.store(), toi, &cho_ngui, dem, Tick(0));
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].sense, Sense::Smell);
}

#[test]
fn khac_tang_thi_khong_thay() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    sim.apply(&Command::new(
        "t.spawn",
        W,
        val! { "x" => 1i64, "y" => 0i64, "z" => 5i64 },
    ))
    .unwrap();

    let o = observe(
        sim.store(),
        toi,
        &Senses::default(),
        Conditions::default(),
        Tick(0),
    );
    assert!(o.is_empty());
}

#[test]
fn khong_tu_quan_sat_chinh_minh() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    let o = observe(
        sim.store(),
        toi,
        &Senses::default(),
        Conditions::default(),
        Tick(0),
    );
    assert!(!o.iter().any(|x| x.identity == Some(toi)));
}

#[test]
fn ket_qua_quan_sat_xac_dinh() {
    // Nó chảy thẳng vào prompt, nên thứ tự là một phần của thế giới.
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    for i in 1..6i64 {
        dat(&mut sim, i, i);
    }
    let a = observe(
        sim.store(),
        toi,
        &Senses::default(),
        Conditions::default(),
        Tick(0),
    );
    let b = observe(
        sim.store(),
        toi,
        &Senses::default(),
        Conditions::default(),
        Tick(0),
    );
    assert_eq!(a, b);
}

#[test]
fn nguoi_khong_co_kenh_thi_khong_dung_kenh_do() {
    let mut sim = sim_voi_nguoi();
    let toi = dat(&mut sim, 0, 0);
    dat(&mut sim, 3, 0);

    let mu = Senses {
        ranges: vec![(Sense::Hearing, 40)],
        acuity: Unit::ONE,
    };
    let o = observe(sim.store(), toi, &mu, Conditions::default(), Tick(0));
    assert!(!o.iter().any(|x| x.sense == Sense::Sight));
    assert!(!mu.has(Sense::Sight));
}

#[test]
fn nghe_va_ngui_xuyen_vat_can_nhin_thi_khong() {
    assert!(Sense::Hearing.penetrates_cover());
    assert!(Sense::Smell.penetrates_cover());
    assert!(!Sense::Sight.penetrates_cover());
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.4 — ngữ cảnh nhận thức là ranh giới
// ─────────────────────────────────────────────────────────────────────────────

fn ngu_canh(
    obs: Vec<Observation>,
    internal: Vec<(&str, i64)>,
    actions: &[&str],
) -> CognitionContext {
    CognitionContext {
        self_id: EntityId(1),
        now: Tick(0),
        observations: obs,
        known_actions: actions.iter().map(|s| (*s).to_owned()).collect(),
        internal: internal
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    }
}

#[test]
fn tham_chieu_ngoai_ngu_canh_khong_co_hieu_luc() {
    // Một mô hình sẽ nhắc tới những thứ nó chưa thấy, một cách thuyết phục.
    let that = Observation {
        sense: Sense::Sight,
        at: WorldPos::new(1, 1, 0),
        identity: Some(EntityId(2)),
        signs: vec![],
        fidelity: Unit::ONE,
        at_tick: Tick(0),
    };
    let bia = Observation {
        at: WorldPos::new(99, 99, 0),
        ..that.clone()
    };
    let ctx = ngu_canh(vec![that.clone()], vec![], &[]);

    assert!(ctx.contains_observation(&that));
    assert!(
        !ctx.contains_observation(&bia),
        "tham chiếu bịa ra được chấp nhận"
    );
}

#[test]
fn chi_nhung_ai_nhan_ra_duoc_moi_xuat_hien() {
    let ro = Observation {
        sense: Sense::Sight,
        at: WorldPos::new(1, 1, 0),
        identity: Some(EntityId(2)),
        signs: vec![],
        fidelity: Unit::ONE,
        at_tick: Tick(0),
    };
    let bong = Observation {
        identity: None,
        ..ro.clone()
    };
    let ctx = ngu_canh(vec![ro, bong], vec![], &[]);
    assert_eq!(
        ctx.identified(),
        vec![EntityId(2)],
        "bóng người không có danh tính"
    );
}

#[test]
fn khong_biet_hanh_dong_thi_khong_lam_duoc() {
    let ctx = ngu_canh(vec![], vec![], &["core.walk"]);
    assert!(ctx.knows_action("core.walk"));
    assert!(!ctx.knows_action("core.cast_fireball"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §10.3 — utility AI, không cần LLM
// ─────────────────────────────────────────────────────────────────────────────

const MOI_HANH_DONG: &[&str] = &[
    "core.flee",
    "core.collapse",
    "core.eat",
    "core.sleep",
    "core.work",
    "core.socialize",
];

#[test]
fn khu_dinh_cu_song_duoc_khong_can_llm() {
    // Bằng chứng cho lời khẳng định ở đầu module.
    let b = villager_brain();
    // Buổi sáng, no đủ, khỏe mạnh: đi làm theo lịch.
    let ctx = ngu_canh(
        vec![],
        vec![("hunger", 9_000), ("fatigue", 9_000), ("pain", 0)],
        MOI_HANH_DONG,
    );
    let q = b.decide(&ctx, 8).expect("phải quyết định được gì đó");
    assert_eq!(q.action, "core.work");
    assert_eq!(q.layer, Layer::Routine);
}

#[test]
fn phan_xa_thang_moi_thu_khac() {
    // Một người đang bàn triết học mà bị đâm thì né, không cần suy nghĩ.
    let b = villager_brain();
    let ctx = ngu_canh(
        vec![],
        vec![("hunger", 100), ("fatigue", 100), ("pain", 95)],
        MOI_HANH_DONG,
    );
    let q = b.decide(&ctx, 8).unwrap();
    assert_eq!(q.action, "core.flee");
    assert_eq!(q.layer, Layer::Reflex);
}

#[test]
fn chien_thuat_thang_thoi_quen() {
    // Người ta bỏ bữa khi nhà cháy; và bỏ việc khi sắp chết đói.
    let b = villager_brain();
    let ctx = ngu_canh(
        vec![],
        vec![("hunger", 500), ("fatigue", 9_000), ("pain", 0)],
        MOI_HANH_DONG,
    );
    let q = b.decide(&ctx, 8).unwrap();
    assert_eq!(q.action, "core.eat", "giờ làm việc mà đói lả vẫn đi làm");
    assert_eq!(q.layer, Layer::Tactical);
}

#[test]
fn khung_gio_vat_qua_nua_dem_hoat_dong() {
    // Khung quan trọng nhất và cũng dễ hỏng nhất.
    let ngu = RoutineSlot {
        from_hour: 22,
        to_hour: 6,
        action: "core.sleep".into(),
    };
    assert!(ngu.contains(23));
    assert!(ngu.contains(0));
    assert!(ngu.contains(5));
    assert!(!ngu.contains(6));
    assert!(!ngu.contains(12));
}

#[test]
fn khong_biet_hanh_dong_thi_khong_chon() {
    let b = villager_brain();
    // Biết mọi thứ trừ `core.eat`.
    let chi_biet: Vec<&str> = MOI_HANH_DONG
        .iter()
        .copied()
        .filter(|a| *a != "core.eat")
        .collect();
    let ctx = ngu_canh(
        vec![],
        vec![("hunger", 500), ("fatigue", 9_000), ("pain", 0)],
        &chi_biet,
    );
    let q = b.decide(&ctx, 8).unwrap();
    assert_ne!(q.action, "core.eat");
}

#[test]
fn khong_co_gi_dang_lam_thi_tra_none() {
    // Đứng yên cũng là một hành vi. Ép mọi thực thể luôn làm gì đó sẽ tạo ra
    // một thế giới bồn chồn không nghỉ.
    let b = Brain::new();
    let ctx = ngu_canh(vec![], vec![], MOI_HANH_DONG);
    assert!(b.decide(&ctx, 8).is_none());
}

#[test]
fn moi_quyet_dinh_giai_thich_duoc() {
    // `§18.13`: "vì sao nó đi ăn" phải trả lời được bằng các phần đóng góp.
    let b = villager_brain();
    let ctx = ngu_canh(
        vec![],
        vec![("hunger", 500), ("fatigue", 9_000), ("pain", 0)],
        MOI_HANH_DONG,
    );
    let q = b.decide(&ctx, 8).unwrap();
    assert!(!q.considerations.is_empty());
    assert_eq!(
        q.score,
        q.considerations.iter().map(|c| c.score).sum::<i64>()
    );
    assert!(q.explain().contains("đói"));
}

#[test]
fn panel_entity_mind_thay_ca_nhung_lua_chon_bi_loai() {
    // Người xem cần biết nhân vật đã cân nhắc gì rồi bỏ.
    let b = villager_brain();
    let ctx = ngu_canh(
        vec![],
        vec![("hunger", 500), ("fatigue", 9_000), ("pain", 0)],
        MOI_HANH_DONG,
    );
    let moi = b.deliberate(&ctx);
    assert!(moi.len() >= 2);
    assert!(moi.iter().any(|c| c.action == "core.sleep" && c.score <= 0));
}

#[test]
fn hai_hanh_dong_cung_diem_pha_hoa_xac_dinh() {
    let mut b = Brain::new();
    for ten in ["core.zzz", "core.aaa"] {
        b.add_scorer(Scorer {
            action: Box::leak(ten.to_owned().into_boxed_str()),
            score: |_| {
                vec![Consideration {
                    name: "bằng nhau",
                    score: 5,
                }]
            },
        });
    }
    let ctx = ngu_canh(vec![], vec![], &["core.aaa", "core.zzz"]);
    assert_eq!(b.decide(&ctx, 0).unwrap().action, "core.aaa");
}

#[test]
fn thay_thuc_an_thi_muon_an_hon() {
    let b = villager_brain();
    let co_thuc_an = Observation {
        sense: Sense::Sight,
        at: WorldPos::new(1, 0, 0),
        identity: None,
        signs: vec!["food".to_owned()],
        fidelity: Unit::ONE,
        at_tick: Tick(0),
    };
    let noi = vec![("hunger", 3_500), ("fatigue", 9_000), ("pain", 0)];
    let khong = b.deliberate(&ngu_canh(vec![], noi.clone(), MOI_HANH_DONG));
    let co = b.deliberate(&ngu_canh(vec![co_thuc_an], noi, MOI_HANH_DONG));

    let diem =
        |v: &[mow_action::Candidate]| v.iter().find(|c| c.action == "core.eat").unwrap().score;
    assert!(diem(&co) > diem(&khong));
}
