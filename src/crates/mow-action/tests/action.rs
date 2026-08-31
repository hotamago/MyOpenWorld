//! Test hành động.
//!
//! Bài quan trọng nhất: [`dao_entity_id_khong_doi_ket_qua`] — đó là `§22.43`, và
//! là ranh giới giữa "thế giới có luật" và "thế giới có một luật vô hình".

use mow_action::consent::{ConsentCapacity, DenialReason};
use mow_action::{
    assess, clamp_move_speed, cognitive_latency, friendly_fire_risk, hit_chance, resolve_all,
    zone_of_control, ConsentDenial, Contention, Cover, Engagement, Facing, Footing,
    IntimacyRegistry, LossReason, Phase, PhaseDurations, Scheduled, Speeds, Tier, Timeline,
};
use mow_core::{EntityId, StableKey, Tick, WorldId};
use mow_math::WorldPos;
use proptest::prelude::*;

const W: WorldId = WorldId(1);

fn tranh_chap(actor: u64, priority: i64) -> Contention {
    Contention {
        actor: EntityId(actor),
        tier: Tier::Transfer,
        action: "core.take".into(),
        target: Some("apple".into()),
        priority,
        key: StableKey::plain(EntityId(actor)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.43 — EntityId chỉ phá hòa, không quyết định thắng thua
// ─────────────────────────────────────────────────────────────────────────────

/// **Bài quan trọng nhất của `PB-10`.**
#[test]
fn dao_entity_id_khong_doi_ket_qua() {
    // Hai người với tay lấy một quả táo. Người nhanh tay hơn (priority cao hơn)
    // phải thắng, bất kể id của họ là bao nhiêu.
    let a = resolve_all(vec![tranh_chap(1, 50), tranh_chap(2, 90)]);
    // Đảo id: người nhanh tay giờ có id nhỏ hơn.
    let b = resolve_all(vec![tranh_chap(2, 50), tranh_chap(1, 90)]);

    assert_eq!(a[0].winner.as_ref().unwrap().priority, 90);
    assert_eq!(b[0].winner.as_ref().unwrap().priority, 90);
    assert_eq!(
        a[0].losers[0].1,
        LossReason::LowerPriority,
        "phải thua vì chậm tay, không vì id"
    );
}

#[test]
fn hoa_diem_thi_moi_dung_toi_khoa_pha_hoa() {
    let r = resolve_all(vec![tranh_chap(9, 50), tranh_chap(3, 50)]);
    assert_eq!(
        r[0].winner.as_ref().unwrap().actor,
        EntityId(3),
        "phá hòa phải xác định"
    );
    assert_eq!(r[0].losers[0].1, LossReason::TieBreak);
}

#[test]
fn ti_le_pha_hoa_la_cong_cu_chan_doan() {
    // Tỉ lệ cao nghĩa là `priority` chưa phân định đủ, và thế giới đang quyết
    // định bằng một luật vô hình nhiều hơn mức nên có.
    let deu_nhau = resolve_all(vec![
        tranh_chap(1, 50),
        tranh_chap(2, 50),
        tranh_chap(3, 50),
    ]);
    let (hoa, tong) = mow_action::resolve::tiebreak_ratio(&deu_nhau);
    assert_eq!((hoa, tong), (2, 2));

    let phan_dinh = resolve_all(vec![
        tranh_chap(1, 10),
        tranh_chap(2, 50),
        tranh_chap(3, 90),
    ]);
    let (hoa2, _) = mow_action::resolve::tiebreak_ratio(&phan_dinh);
    assert_eq!(hoa2, 0);
}

#[test]
fn tang_chay_theo_thu_tu_co_dinh() {
    let mut ds = vec![
        Contention {
            tier: Tier::Record,
            ..tranh_chap(1, 10)
        },
        Contention {
            tier: Tier::Nullify,
            ..tranh_chap(2, 10)
        },
        Contention {
            tier: Tier::Impact,
            ..tranh_chap(3, 10)
        },
    ];
    // Mỗi tầng một mục tiêu riêng để không cạnh tranh nhau.
    for (i, c) in ds.iter_mut().enumerate() {
        c.target = Some(format!("t{i}"));
    }
    let r = resolve_all(ds);
    assert_eq!(
        r.iter()
            .filter_map(|o| o.winner.as_ref().map(|w| w.tier))
            .collect::<Vec<_>>(),
        vec![Tier::Nullify, Tier::Impact, Tier::Record]
    );
}

#[test]
fn tang_nullify_vo_hieu_hoa_hanh_dong_sau_do() {
    // Một xác chết không hoàn thành đòn đánh của nó.
    let choang = Contention {
        actor: EntityId(1),
        tier: Tier::Nullify,
        action: "core.stun".into(),
        target: Some("2".into()),
        priority: 100,
        key: StableKey::plain(EntityId(1)),
    };
    let danh = Contention {
        actor: EntityId(2),
        tier: Tier::Impact,
        action: "core.strike".into(),
        target: Some("1".into()),
        priority: 100,
        key: StableKey::plain(EntityId(2)),
    };
    let r = resolve_all(vec![danh, choang]);
    let bi_vo_hieu: Vec<_> = r
        .iter()
        .flat_map(|o| &o.losers)
        .filter(|(_, l)| *l == LossReason::Nullified)
        .collect();
    assert_eq!(bi_vo_hieu.len(), 1);
    assert_eq!(bi_vo_hieu[0].0.actor, EntityId(2));

    // Và kết quả đó phải được GHI LẠI, không bị bỏ im lặng: chuỗi nhân quả
    // cần trả lời được "vì sao đòn của người 2 không xảy ra".
    let khong_xay_ra: Vec<_> = r.iter().filter(|o| !o.happened()).collect();
    assert_eq!(
        khong_xay_ra.len(),
        1,
        "nhóm bị vô hiệu hóa hết bị bỏ im lặng"
    );
}

proptest! {
    /// Với mọi hoán vị id, người có `priority` cao nhất luôn thắng.
    #[test]
    fn nguoi_nhanh_tay_luon_thang(
        ids in prop::collection::vec(1u64..1000, 2..8),
        prios in prop::collection::vec(0i64..100, 2..8),
    ) {
        let n = ids.len().min(prios.len());
        // Bỏ trùng id: hai thực thể không thể cùng id.
        let mut da_thay = std::collections::BTreeSet::new();
        let ds: Vec<Contention> = (0..n)
            .filter(|i| da_thay.insert(ids[*i]))
            .map(|i| tranh_chap(ids[i], prios[i]))
            .collect();
        prop_assume!(ds.len() >= 2);

        let cao_nhat = ds.iter().map(|c| c.priority).max().unwrap();
        let r = resolve_all(ds);
        prop_assert_eq!(r[0].winner.as_ref().unwrap().priority, cao_nhat);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §10.7, §10.8 — chrono-turn
// ─────────────────────────────────────────────────────────────────────────────

fn lich(actor: u64, s: Speeds) -> Scheduled {
    Scheduled {
        actor: EntityId(actor),
        action: "core.strike".into(),
        world: W,
        phase: Phase::WindUp,
        ready_at: Tick(0),
        key: StableKey::plain(EntityId(actor)),
        durations: PhaseDurations {
            wind_up: 10,
            impact: 2,
            recovery: 8,
        },
        speeds: s,
    }
}

#[test]
fn ba_pha_chay_dung_thu_tu() {
    let mut tl = Timeline::new();
    tl.begin(lich(1, Speeds::default()), Tick(0)).unwrap();

    let mut thay = Vec::new();
    for _ in 0..10 {
        let Some(now) = tl.next_due() else { break };
        for s in tl.due(now) {
            thay.push(s.phase);
            tl.advance(s, now);
        }
    }
    assert_eq!(thay, vec![Phase::WindUp, Phase::Impact, Phase::Recovery]);
}

#[test]
fn wind_up_quan_sat_duoc_nen_phong_thu_co_y_nghia() {
    assert!(Phase::WindUp.is_observable());
    assert!(Phase::Recovery.is_observable());
    assert!(!Phase::Impact.is_observable(), "chạm là tức thời");
}

#[test]
fn pha_khong_bao_gio_dai_0_tick() {
    // Pha 0 tick nghĩa là không quan sát được, và `wind_up` không quan sát được
    // thì phòng thủ biến mất.
    let d = PhaseDurations {
        wind_up: 1,
        impact: 1,
        recovery: 1,
    };
    let nhanh = Speeds {
        action: 10_000,
        recovery: 10_000,
        ..Speeds::default()
    };
    for p in [Phase::WindUp, Phase::Impact, Phase::Recovery] {
        assert!(d.ticks_for(p, nhanh) >= 1, "{p:?} dài 0 tick");
    }
}

#[test]
fn bon_toc_do_doc_lap_nhau() {
    // Lão già thông thái: nghĩ nhanh, ra đòn chậm.
    let lao = Speeds {
        cognition: 4,
        action: 60,
        movement: 70,
        recovery: 50,
    };
    // Chiến binh trẻ: ngược lại.
    let tre = Speeds {
        cognition: 20,
        action: 160,
        movement: 140,
        recovery: 150,
    };

    let d = PhaseDurations {
        wind_up: 10,
        impact: 2,
        recovery: 8,
    };
    assert!(d.ticks_for(Phase::WindUp, lao) > d.ticks_for(Phase::WindUp, tre));
    assert!(cognitive_latency(lao) < cognitive_latency(tre));
}

#[test]
fn do_tre_nhan_thuc_la_thuoc_tinh_cua_the_gioi() {
    // `§20.2.2`: `D` suy từ `cognition_rate`, không từ tốc độ mạng.
    assert_eq!(
        cognitive_latency(Speeds {
            cognition: 7,
            ..Speeds::default()
        }),
        7
    );
    assert_eq!(
        cognitive_latency(Speeds {
            cognition: 0,
            ..Speeds::default()
        }),
        1,
        "không bao giờ 0"
    );
}

#[test]
fn toc_do_duoc_chot_luc_bat_dau_khong_doi_giua_chung() {
    // Nếu tra lại mỗi pha, một đòn đang vung dở sẽ đột ngột nhanh lên khi hết
    // buff — và cam kết là thứ tạo ra chiến thuật.
    let mut tl = Timeline::new();
    let s = Speeds {
        action: 200,
        ..Speeds::default()
    };
    tl.begin(lich(1, s), Tick(0)).unwrap();
    let due = tl.due(Tick(5));
    assert_eq!(due[0].speeds.action, 200);
}

#[test]
fn huy_hanh_dong_khi_thuc_the_chet() {
    let mut tl = Timeline::new();
    tl.begin(lich(1, Speeds::default()), Tick(0)).unwrap();
    tl.begin(lich(2, Speeds::default()), Tick(0)).unwrap();
    assert_eq!(tl.cancel(EntityId(1)), 1);
    assert_eq!(tl.len(), 1);
}

#[test]
fn nhay_thang_toi_tick_co_viec() {
    // Cùng ý tưởng với đánh thức theo ngưỡng: không tiến từng tick một.
    let mut tl = Timeline::new();
    tl.begin(lich(1, Speeds::default()), Tick(100)).unwrap();
    assert_eq!(tl.next_due(), Some(Tick(110)));
    assert!(tl.due(Tick(109)).is_empty());
}

#[test]
fn thu_tu_hang_doi_xac_dinh_khong_theo_thu_tu_them() {
    let mk = |dao: bool| {
        let mut tl = Timeline::new();
        let ids = if dao { vec![3, 1, 2] } else { vec![1, 2, 3] };
        for i in ids {
            tl.begin(lich(i, Speeds::default()), Tick(0)).unwrap();
        }
        tl.due(Tick(100))
            .iter()
            .map(|s| s.actor)
            .collect::<Vec<_>>()
    };
    assert_eq!(mk(false), mk(true));
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.26 — ưng thuận, không có ngoại lệ
// ─────────────────────────────────────────────────────────────────────────────

fn nguoi_lon() -> ConsentCapacity {
    ConsentCapacity {
        sapient: true,
        age_years: 30,
        maturity_years: 18,
        has_agency: true,
    }
}

#[test]
fn ba_dieu_kien_deu_phai_dung() {
    let ok =
        mow_action::consent::validate(&[(EntityId(1), nguoi_lon()), (EntityId(2), nguoi_lon())]);
    assert!(ok.is_ok());

    for (sua, ly_do) in [
        (
            ConsentCapacity {
                sapient: false,
                ..nguoi_lon()
            },
            DenialReason::NotSapient,
        ),
        (
            ConsentCapacity {
                age_years: 10,
                ..nguoi_lon()
            },
            DenialReason::BelowMaturity,
        ),
        (
            ConsentCapacity {
                has_agency: false,
                ..nguoi_lon()
            },
            DenialReason::NoAgency,
        ),
    ] {
        let e = mow_action::consent::validate(&[(EntityId(1), sua), (EntityId(2), nguoi_lon())])
            .expect_err("phải bị từ chối");
        assert_eq!(e[0].reason, ly_do);
    }
}

#[test]
fn tuoi_truong_thanh_theo_loai_khong_theo_mot_hang_so() {
    // Một loài trưởng thành ở 50 tuổi thì 30 tuổi vẫn là chưa trưởng thành.
    let loai_song_lau = ConsentCapacity {
        maturity_years: 50,
        ..nguoi_lon()
    };
    let e =
        mow_action::consent::validate(&[(EntityId(1), loai_song_lau), (EntityId(2), nguoi_lon())])
            .expect_err("phải bị từ chối");
    assert_eq!(e[0].reason, DenialReason::BelowMaturity);
}

#[test]
fn mat_tu_chu_thi_khong_ung_thuan_duoc() {
    // Điều kiện mà một hệ thống chỉ kiểm tuổi sẽ bỏ sót.
    let bi_me_hoac = ConsentCapacity {
        has_agency: false,
        ..nguoi_lon()
    };
    assert!(mow_action::consent::validate(&[
        (EntityId(1), bi_me_hoac),
        (EntityId(2), nguoi_lon())
    ])
    .is_err());
}

#[test]
fn tra_ve_moi_ly_do_khong_dung_o_cai_dau_tien() {
    // Log kiểm toán cần biết đầy đủ.
    let xau = ConsentCapacity {
        sapient: false,
        age_years: 5,
        has_agency: false,
        maturity_years: 18,
    };
    let e = mow_action::consent::validate(&[(EntityId(1), xau), (EntityId(2), xau)])
        .expect_err("phải lỗi");
    assert_eq!(e.len(), 2, "cả hai bên đều phải được báo");
}

#[test]
fn mot_ben_khong_phai_la_giua_cac_ben() {
    let e = mow_action::consent::validate(&[(EntityId(1), nguoi_lon())]).expect_err("phải lỗi");
    assert_eq!(e.len(), 1);
}

#[test]
fn pack_mo_rong_duoc_pham_vi_nhung_khong_thu_hep() {
    let mut r = IntimacyRegistry::standard();
    assert!(r.requires_consent("core.intimacy"));
    r.require_consent("mypack.ritual");
    assert!(r.requires_consent("mypack.ritual"));

    // Không có `remove`. Bài test này khóa hình dạng API lại: nếu ai đó thêm
    // một hàm gỡ, họ phải xóa bài test và đối diện với lý do nó tồn tại.
    let van_con: Vec<&str> = r.kinds().collect();
    assert!(van_con.contains(&"core.intimacy"));
    assert!(van_con.contains(&"mypack.ritual"));
}

#[test]
fn thong_bao_tu_choi_doc_duoc() {
    let d = ConsentDenial {
        party: EntityId(7),
        reason: DenialReason::BelowMaturity,
    };
    assert!(d.to_string().contains("below_maturity"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §10.10 — chiến trường
// ─────────────────────────────────────────────────────────────────────────────

fn cham_tran() -> Engagement {
    Engagement {
        from: WorldPos::new(0, 0, 0),
        to: WorldPos::new(1, 0, 0),
        target_facing: Facing::E,
        cover: Cover::None,
        reach: 1,
        elevation_delta: 0,
        footing: Footing::Solid,
        flanking_allies: 0,
    }
}

#[test]
fn vong_ra_sau_lung_co_gia_tri() {
    // Mục tiêu nhìn về phía đông; ta đánh từ phía tây tới, tức là từ sau lưng.
    let e = Engagement {
        from: WorldPos::new(5, 0, 0),
        to: WorldPos::new(4, 0, 0),
        target_facing: Facing::E,
        ..cham_tran()
    };
    let a = assess(e);
    assert!(a.from_behind);
    assert!(a.hit_modifier > 0);
}

#[test]
fn danh_truc_dien_thi_khong_duoc_thuong() {
    let a = assess(cham_tran());
    assert!(!a.from_behind);
}

#[test]
fn ngoai_tam_thi_khong_trung_duoc() {
    let e = Engagement {
        to: WorldPos::new(10, 0, 0),
        reach: 1,
        ..cham_tran()
    };
    let a = assess(e);
    assert!(!a.in_reach);
    assert_eq!(hit_chance(90, &a), mow_math::Unit::ZERO);
}

#[test]
fn che_chan_lam_kho_trung() {
    let ho = assess(cham_tran());
    let nap = assess(Engagement {
        cover: Cover::Heavy,
        ..cham_tran()
    });
    assert!(nap.hit_modifier < ho.hit_modifier);
    assert_eq!(Cover::Full.miss_bonus_percent(), 100);
}

#[test]
fn moi_yeu_to_bam_duoc_ve_nguon() {
    // Người chơi hỏi "vì sao tôi trượt" và câu trả lời phải là một danh sách.
    let e = Engagement {
        cover: Cover::Partial,
        elevation_delta: 10,
        footing: Footing::Slippery,
        flanking_allies: 2,
        ..cham_tran()
    };
    let a = assess(e);
    let ten: Vec<&str> = a.factors.iter().map(|(n, _)| *n).collect();
    assert!(ten.contains(&"che chắn"));
    assert!(ten.contains(&"cao hơn"));
    assert!(ten.contains(&"mặt nền"));
    assert!(ten.contains(&"vây"));
    assert_eq!(
        a.hit_modifier,
        a.factors.iter().map(|(_, v)| v).sum::<i64>()
    );
}

#[test]
fn loi_the_do_cao_bao_hoa() {
    // Đứng trên đồi tốt hơn đứng dưới; đứng trên núi không tốt hơn bao nhiêu.
    let doi = assess(Engagement {
        elevation_delta: 20,
        ..cham_tran()
    })
    .hit_modifier;
    let nui = assess(Engagement {
        elevation_delta: 2_000,
        ..cham_tran()
    })
    .hit_modifier;
    assert!(
        nui <= doi + 5,
        "lợi thế độ cao không bão hòa: đồi {doi}, núi {nui}"
    );
}

#[test]
fn vay_kin_co_gioi_han() {
    // Người thứ tư không chen vào được nữa.
    let ba = assess(Engagement {
        flanking_allies: 3,
        ..cham_tran()
    })
    .hit_modifier;
    let muoi = assess(Engagement {
        flanking_allies: 10,
        ..cham_tran()
    })
    .hit_modifier;
    assert_eq!(ba, muoi);
}

#[test]
fn mat_nen_anh_huong_toc_do() {
    assert_eq!(Footing::Solid.speed_percent(), 100);
    assert!(Footing::Waterlogged.speed_percent() < Footing::Loose.speed_percent());
    assert!(Footing::Slippery.risks_falling());
    assert!(!Footing::Solid.risks_falling());
}

#[test]
fn ban_nham_khi_dong_doi_can_duong() {
    let ban_tu = WorldPos::new(0, 0, 0);
    let muc_tieu = WorldPos::new(10, 0, 0);
    let can_duong = WorldPos::new(5, 0, 0);
    let dung_canh = WorldPos::new(0, 5, 0);
    let dung_sau = WorldPos::new(15, 0, 0);

    let nguy = friendly_fire_risk(ban_tu, muc_tieu, &[can_duong, dung_canh, dung_sau]);
    assert_eq!(nguy, vec![can_duong], "chỉ người cản đường mới nguy hiểm");
}

#[test]
fn ban_nham_khong_bao_dong_gia_voi_nguoi_dung_canh() {
    // Hình nón sẽ báo động giả; khoảng cách tới đoạn thẳng thì không.
    let nguy = friendly_fire_risk(
        WorldPos::new(0, 0, 0),
        WorldPos::new(20, 0, 0),
        &[WorldPos::new(1, 4, 0)],
    );
    assert!(nguy.is_empty());
}

#[test]
fn vung_kiem_soat_lam_rut_lui_co_gia() {
    let toi = WorldPos::new(5, 5, 0);
    let dich = [
        WorldPos::new(5, 6, 0), // kề
        WorldPos::new(6, 6, 0), // kề chéo
        WorldPos::new(5, 9, 0), // xa
        WorldPos::new(5, 6, 1), // kề nhưng khác tầng
    ];
    let khoa = zone_of_control(toi, &dich);
    assert_eq!(khoa.len(), 2);
}

#[test]
fn tran_toc_do_la_cung() {
    // Không có trần thì không ai vòng ra sau lưng được một người dịch chuyển
    // tức thời, và mọi yếu tố chiến thuật khác vô nghĩa.
    assert_eq!(
        clamp_move_speed(1_000_000),
        mow_action::tactical::MAX_MOVE_CELLS_PER_100_TICKS
    );
    assert_eq!(clamp_move_speed(-50), 0);
    assert_eq!(clamp_move_speed(100), 100);
}

#[test]
fn khong_don_nao_chac_chan_trung_hay_truot() {
    let a = assess(cham_tran());
    assert!(hit_chance(1_000, &a) < mow_math::Unit::ONE);
    assert!(hit_chance(-1_000, &a) > mow_math::Unit::ZERO);
}
