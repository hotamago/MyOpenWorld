//! Test vòng đời cổng, transfer nguyên tử, rebase và kiểm dịch (`PE-09`–`PE-11`).

use mow_core::clock::{Clock, ClockDomain, Deadline, Tick};
use mow_core::{EntityId, WorldId};
use mow_math::Rate;
use mow_portal::clock::{rebase_processes, Process, RebaseReason};
use mow_portal::contact::{
    Cargo, ContactRegime, Decision, DisputeForum, Failure, LegalPersonhood, Measures, Quarantine,
    Residency, Tariff, TransportLaw,
};
use mow_portal::portal::{
    count_copies, recover, AccessPolicy, EscrowLedger, EscrowPhase, NeedsProfile, Portal,
    PortalState, Recovery, TransferError, Traveller, WorldConditions,
};
use std::collections::BTreeSet;

// ───────────────────────────── nền chung ─────────────────────────────

/// Gaia: đồng hồ địa phương chạy bằng đồng hồ thần.
fn gaia() -> Clock {
    Clock::synchronous()
}

/// Một world chảy nhanh gấp 10.
fn nhanh_gap_10() -> Clock {
    Clock::new(Rate::per_tick(10))
}

fn cong_mo() -> Portal {
    Portal {
        id: 1,
        source: WorldId(1),
        dest: WorldId(2),
        state: PortalState::Open,
        access: AccessPolicy::default(),
        bandwidth_mass: 1_000,
        used_mass: 0,
        regime: che_do_day_du(),
    }
}

/// Một chế độ tiếp xúc đã thỏa thuận đủ bảy điều khoản.
fn che_do_day_du() -> ContactRegime {
    ContactRegime {
        quarantine: Quarantine {
            hold_ticks: 40,
            screens_pathogens: true,
            screens_taint: true,
            may_refuse: true,
        },
        tariff: Tariff {
            rate_permille: 50,
            contraband: BTreeSet::from(["weapon.cursed".to_owned()]),
        },
        measures: Measures {
            agreed: BTreeSet::from(["mass.kg".to_owned(), "mana.mmu".to_owned()]),
        },
        personhood: LegalPersonhood {
            recognized: true,
            contracts_enforceable: true,
        },
        residency: Residency {
            visa_free_ticks: 900,
            may_work: true,
        },
        transport: TransportLaw {
            living_creatures: true,
            seeds: false,
            souls: false,
        },
        dispute: DisputeForum {
            forum: Some("hội đồng cổng Gaia–Abyss".to_owned()),
            interpreters: true,
        },
    }
}

fn nhu_cau() -> NeedsProfile {
    NeedsProfile {
        atmosphere: (30, 100),
        temperature: (-1_000, 4_500),
        mana: (0, 8_000),
    }
}

fn dieu_kien_tot() -> WorldConditions {
    WorldConditions {
        atmosphere: 70,
        temperature: 2_000,
        mana: 3_000,
    }
}

fn lu_khach() -> Traveller {
    Traveller {
        who: EntityId(77),
        species: "human".into(),
        mass: 90,
        inventory: vec![11, 12],
        processes: vec![
            Process {
                id: "disease.plague.incubation".into(),
                deadline: Deadline::new(Tick(300), ClockDomain::Proper),
            },
            Process {
                id: "contract.loan.7741".into(),
                deadline: Deadline::new(Tick(900), ClockDomain::WorldLocal),
            },
        ],
        hitchhikers: vec![],
        divine_signature: false,
    }
}

fn di_het_chin_buoc(l: &mut EscrowLedger, p: &mut Portal, t: &Traveller) -> u64 {
    let (id, _) = l
        .begin(p, t, &nhu_cau(), &dieu_kien_tot(), &gaia(), &nhanh_gap_10())
        .unwrap();
    l.depart(id).unwrap();
    l.arrive(id).unwrap();
    l.release(id).unwrap();
    id
}

// ───────────────────── PE-09 · vòng đời và transfer ─────────────────────

/// Vòng đời đi đúng một chiều `§6.2`.
#[test]
fn vong_doi_di_dung_mot_chieu() {
    use PortalState::*;
    let duong_di = [
        (Dormant, Charging),
        (Charging, Open),
        (Open, Unstable),
        (Unstable, Collapsing),
        (Collapsing, Closed),
    ];
    for (a, b) in duong_di {
        assert!(a.may_become(b), "{a:?} → {b:?} phải hợp lệ");
    }
    // Không nhảy cóc.
    assert!(!Dormant.may_become(Open));
    assert!(!Open.may_become(Closed));
}

/// **`CLOSED` là hấp thụ** — sập rồi thì phải mở cổng mới.
#[test]
fn cong_da_sap_khong_mo_lai_duoc() {
    use PortalState::*;
    for s in [Dormant, Charging, Open, Unstable, Collapsing, Closed] {
        assert!(
            !Closed.may_become(s),
            "cổng đã đóng mà quay lại {s:?} thì phá cổng chẳng còn nghĩa gì"
        );
    }
}

/// `Unstable` **vẫn đi được** — đó là một canh bạc, không phải một lỗi.
#[test]
fn cong_bat_on_van_di_duoc() {
    assert!(PortalState::Unstable.passable());
    assert!(PortalState::Open.passable());
    for s in [
        PortalState::Dormant,
        PortalState::Charging,
        PortalState::Collapsing,
        PortalState::Closed,
    ] {
        assert!(!s.passable());
    }
}

/// Cổng đang sập thì không nhận thêm ai.
#[test]
fn cong_dang_sap_khong_nhan_them_ai() {
    let mut p = cong_mo();
    p.state = PortalState::Collapsing;
    let loi = EscrowLedger::new()
        .begin(
            &mut p,
            &lu_khach(),
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap_err();
    assert!(matches!(loi, TransferError::NotPassable { .. }));
}

/// **`INV-22-8`, hướng nhân đôi**: không lúc nào entity có mặt ở hai chỗ.
#[test]
fn khong_bao_gio_co_hai_ban_sao_o_bat_ky_pha_nao() {
    let mut l = EscrowLedger::new();
    let mut p = cong_mo();
    let t = lu_khach();

    let (id, _) = l
        .begin(
            &mut p,
            &t,
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap();

    // Reserved: vẫn ở nguồn, escrow chưa tính.
    assert_eq!(l.get(id).unwrap().phase, EscrowPhase::Reserved);
    assert_eq!(count_copies(t.who, &[WorldId(1)], &l), 1);

    // Departed: rời nguồn, nằm trong escrow.
    l.depart(id).unwrap();
    assert_eq!(count_copies(t.who, &[], &l), 1);

    // Arrived: đích đã spawn, escrow không tính nữa.
    l.arrive(id).unwrap();
    assert_eq!(count_copies(t.who, &[WorldId(2)], &l), 1);

    // Released: xong.
    l.release(id).unwrap();
    assert_eq!(count_copies(t.who, &[WorldId(2)], &l), 1);
}

/// **`INV-22-8`, hướng bốc hơi**: crash ở giữa để lại một bản ghi dò được.
#[test]
fn crash_giua_chung_khong_lam_boc_hoi_entity() {
    let mut l = EscrowLedger::new();
    let mut p = cong_mo();
    let t = lu_khach();

    let (id, _) = l
        .begin(
            &mut p,
            &t,
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap();
    l.depart(id).unwrap();

    // …crash. Sau khi nạp lại save:
    let con_dang_do = recover(&l);
    assert_eq!(con_dang_do, vec![(id, Recovery::Complete)]);
    assert_eq!(count_copies(t.who, &[], &l), 1, "entity vẫn đếm được");
}

/// Sau crash, mỗi pha có **đúng một** việc phải làm — và không việc nào lùi
/// một event đã ghi.
#[test]
fn moi_pha_dang_do_co_dung_mot_cach_xu_ly() {
    let mut l = EscrowLedger::new();
    let mut p = cong_mo();

    let mut t = lu_khach();
    let (a, _) = l
        .begin(
            &mut p,
            &t,
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap();

    t.who = EntityId(78);
    let (b, _) = l
        .begin(
            &mut p,
            &t,
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap();
    l.depart(b).unwrap();

    t.who = EntityId(79);
    let (c, _) = l
        .begin(
            &mut p,
            &t,
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap();
    l.depart(c).unwrap();
    l.arrive(c).unwrap();

    let mut v = recover(&l);
    v.sort();
    assert_eq!(
        v,
        vec![
            (a, Recovery::Rollback),
            (b, Recovery::Complete),
            (c, Recovery::Confirm)
        ]
    );
}

/// **Đã rời nguồn thì không lùi được** — event rời nguồn đã ghi.
#[test]
fn da_toi_dich_thi_khong_hoan_tac_duoc() {
    let mut l = EscrowLedger::new();
    let mut p = cong_mo();
    let t = lu_khach();
    let (id, _) = l
        .begin(
            &mut p,
            &t,
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap();

    // Chưa rời hoặc vừa rời thì còn hoàn tác được.
    l.depart(id).unwrap();
    l.arrive(id).unwrap();
    assert!(matches!(
        l.rollback(id).unwrap_err(),
        TransferError::WrongPhase { .. }
    ));
}

/// **Không nhảy pha**: `release` khi chưa `arrive` bị từ chối.
#[test]
fn khong_nhay_pha_escrow() {
    let mut l = EscrowLedger::new();
    let mut p = cong_mo();
    let (id, _) = l
        .begin(
            &mut p,
            &lu_khach(),
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10(),
        )
        .unwrap();
    assert!(matches!(
        l.release(id).unwrap_err(),
        TransferError::WrongPhase {
            phase: EscrowPhase::Reserved,
            ..
        }
    ));
    // Và không commit hai lần được.
    l.depart(id).unwrap();
    assert!(l.depart(id).is_err());
}

/// Chuyến đi trọn vẹn thì không còn gì dang dở.
#[test]
fn chuyen_di_tron_ven_khong_de_lai_gi_dang_do() {
    let mut l = EscrowLedger::new();
    let mut p = cong_mo();
    let id = di_het_chin_buoc(&mut l, &mut p, &lu_khach());
    assert!(recover(&l).is_empty());
    assert_eq!(l.get(id).unwrap().phase, EscrowPhase::Released);
    // Bản ghi **vẫn còn** — để dò lịch sử về sau.
    assert_eq!(l.len(), 1);
}

/// Băng thông chặn được, và chặn **trước** khi giữ chỗ.
#[test]
fn bang_thong_chan_duoc() {
    let mut l = EscrowLedger::new();
    let mut p = Portal {
        bandwidth_mass: 100,
        ..cong_mo()
    };
    let t = Traveller {
        mass: 150,
        ..lu_khach()
    };
    assert!(matches!(
        l.begin(
            &mut p,
            &t,
            &nhu_cau(),
            &dieu_kien_tot(),
            &gaia(),
            &nhanh_gap_10()
        )
        .unwrap_err(),
        TransferError::OverBandwidth { .. }
    ));
    assert!(l.is_empty(), "từ chối rồi mà vẫn lập bản ghi trung chuyển");
    assert_eq!(p.used_mass, 0, "từ chối rồi mà vẫn trừ băng thông");
}

/// Danh sách cấm thắng danh sách cho phép.
#[test]
fn danh_sach_cam_thang_danh_sach_cho_phep() {
    let a = AccessPolicy {
        allow_entities: vec![EntityId(77)],
        deny_entities: vec![EntityId(77)],
        ..AccessPolicy::default()
    };
    assert!(!a.permits(EntityId(77), "human", true));
}

/// Cần chữ ký True God thì thiếu chữ ký là không qua (`INV-22-7`).
#[test]
fn thieu_chu_ky_true_god_thi_khong_qua() {
    let a = AccessPolicy {
        requires_divine_signature: true,
        ..AccessPolicy::default()
    };
    assert!(!a.permits(EntityId(1), "human", false));
    assert!(a.permits(EntityId(1), "human", true));
}

/// **Không sống nổi vẫn đi được** — nhưng phải mang theo cảnh báo (`§6.2` bước 4).
#[test]
fn khong_song_noi_van_di_duoc_nhung_co_canh_bao() {
    let mut l = EscrowLedger::new();
    let mut p = cong_mo();
    let chet_nguoi = WorldConditions {
        atmosphere: 0,
        temperature: 9_000,
        mana: 3_000,
    };
    let (id, canh_bao) = l
        .begin(
            &mut p,
            &lu_khach(),
            &nhu_cau(),
            &chet_nguoi,
            &gaia(),
            &nhanh_gap_10(),
        )
        .expect("§6.2 bước 4: đi vào chỗ chết là một quyết định hợp lệ");
    assert_eq!(canh_bao.problems.len(), 2, "{canh_bao:?}");
    assert_eq!(l.get(id).unwrap().phase, EscrowPhase::Reserved);
}

/// **Bước 8**: bản ghi đến mang theo những gì đã đi cùng mà không ai khai.
#[test]
fn ban_ghi_den_mang_theo_thu_di_cung_khong_khai() {
    let mut l = EscrowLedger::new();
    let mut p = Portal {
        regime: ContactRegime {
            transport: TransportLaw {
                living_creatures: true,
                seeds: true,
                souls: false,
            },
            quarantine: Quarantine::default(),
            ..che_do_day_du()
        },
        ..cong_mo()
    };
    let t = Traveller {
        hitchhikers: vec!["seed.thistle".into(), "parasite.gutworm".into()],
        ..lu_khach()
    };
    let id = di_het_chin_buoc(&mut l, &mut p, &t);
    assert_eq!(
        l.get(id).unwrap().hitchhikers,
        vec!["seed.thistle", "parasite.gutworm"],
        "không ghi thì dịch bệnh ở world đích sẽ không truy được nguồn"
    );
}

// ───────────────────────── PE-10 · rebase deadline ─────────────────────────

/// **Người đang ủ bệnh không khỏi cũng không chết vì bước qua cổng.**
#[test]
fn nguoi_dang_u_benh_khong_khoi_cung_khong_chet_khi_qua_cong() {
    let (moi, audit) = rebase_processes(&lu_khach().processes, &gaia(), &nhanh_gap_10()).unwrap();

    let u_benh = moi
        .iter()
        .find(|p| p.id == "disease.plague.incubation")
        .unwrap();
    // World đích chảy nhanh gấp 10 ⇒ 300 tick proper thành 3000 tick địa phương.
    assert_eq!(u_benh.deadline.at, Tick(3_000));
    assert!(
        u_benh.deadline.at.0 > 0,
        "còn hạn — không khỏi tức thì và không chết tức thì"
    );
    assert_eq!(
        audit
            .lines
            .iter()
            .find(|l| l.process == "disease.plague.incubation")
            .unwrap()
            .reason,
        RebaseReason::ProperFollowsEntity
    );
}

/// **Hợp đồng vay không đáo hạn tức thì vì con nợ bỏ trốn qua cổng.**
#[test]
fn hop_dong_vay_khong_dao_han_tuc_thi_khi_con_no_bo_tron() {
    let (moi, _) = rebase_processes(&lu_khach().processes, &gaia(), &nhanh_gap_10()).unwrap();
    let no = moi.iter().find(|p| p.id == "contract.loan.7741").unwrap();
    assert_eq!(
        no.deadline.at,
        Tick(900),
        "world_local neo vào world đã ký, không đổi"
    );
}

/// Hai miền ngược chiều nhau ⇒ **không có một hệ số nào đúng cho cả hai**.
///
/// Đây là bằng chứng phản lại cách chữa hấp dẫn nhất: nhân đồng loạt.
#[test]
fn khong_co_he_so_dong_loat_nao_dung_cho_ca_hai_mien() {
    let (moi, _) = rebase_processes(&lu_khach().processes, &gaia(), &nhanh_gap_10()).unwrap();
    let a = moi.iter().find(|p| p.id.starts_with("disease")).unwrap();
    let b = moi.iter().find(|p| p.id.starts_with("contract")).unwrap();
    assert_ne!(a.deadline.at.0, 300, "proper phải đổi");
    assert_eq!(b.deadline.at.0, 900, "world_local phải giữ");
}

/// Biên bản **phủ hết** — bằng chứng đã không sót tiến trình nào.
#[test]
fn bien_ban_phu_het_moi_tien_trinh() {
    let bon_mien = vec![
        Process {
            id: "age".into(),
            deadline: Deadline::new(Tick(500), ClockDomain::Proper),
        },
        Process {
            id: "harvest".into(),
            deadline: Deadline::new(Tick(500), ClockDomain::WorldLocal),
        },
        Process {
            id: "divine.council".into(),
            deadline: Deadline::new(Tick(500), ClockDomain::Divine),
        },
        Process {
            id: "curse.moon".into(),
            deadline: Deadline::new(Tick(500), ClockDomain::LawDefined),
        },
    ];
    let (_, audit) = rebase_processes(&bon_mien, &gaia(), &nhanh_gap_10()).unwrap();
    assert!(audit.covers_all(&bon_mien));
    assert_eq!(audit.lines.len(), 4);
    // Đúng một miền đổi số.
    assert_eq!(audit.changed().count(), 1);
}

/// Đi rồi quay về thì không tích lũy sai số.
#[test]
fn di_roi_quay_ve_khong_tich_luy_sai_so() {
    let p = vec![Process {
        id: "age".into(),
        deadline: Deadline::new(Tick(300), ClockDomain::Proper),
    }];
    let (sang, _) = rebase_processes(&p, &gaia(), &nhanh_gap_10()).unwrap();
    let (ve, _) = rebase_processes(&sang, &nhanh_gap_10(), &gaia()).unwrap();
    assert_eq!(ve[0].deadline.at, Tick(300));
}

/// Biên bản **serialize được** — nó nằm trong save, không chỉ trong log.
#[test]
fn bien_ban_nam_trong_save_duoc() {
    let (_, audit) = rebase_processes(&lu_khach().processes, &gaia(), &nhanh_gap_10()).unwrap();
    let j = serde_json::to_string(&audit).unwrap();
    let lai: mow_portal::RebaseAudit = serde_json::from_str(&j).unwrap();
    assert_eq!(lai, audit);
    assert!(j.contains("proper_follows_entity"), "{j}");
}

// ───────────────────── PE-11 · chế độ tiếp xúc ─────────────────────

/// **Cổng không thỏa thuận gì vẫn mở** — nó chỉ hóa thành năm thứ cùng lúc.
#[test]
fn cong_khong_thoa_thuan_gi_van_mo_nhung_hong_ca_nam_kieu() {
    let r = ContactRegime::none();
    let mut f = r.failure_modes();
    f.sort();
    assert_eq!(
        f,
        vec![
            Failure::DiseaseVector,
            Failure::BlackMarket,
            Failure::RefugeeCamp,
            Failure::ForceOnly,
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
    );
    // Và nó **vẫn cho người đi qua**.
    assert_eq!(r.screen(&Cargo::default()), Decision::Allow);
}

/// Thỏa thuận đủ thì không còn kiểu hỏng nào.
#[test]
fn thoa_thuan_du_thi_khong_con_kieu_hong_nao() {
    assert!(che_do_day_du().failure_modes().is_empty());
}

/// **Từ chối thắng giữ lại**: hàng cấm không được "cho qua sau khi cách ly".
#[test]
fn tu_choi_thang_giu_lai() {
    let r = che_do_day_du();
    let vua_cam_vua_nghi_benh = Cargo {
        suspected_pathogen: true,
        goods: BTreeSet::from(["weapon.cursed".to_owned()]),
        living: true,
        ..Cargo::default()
    };
    assert!(matches!(
        r.screen(&vua_cam_vua_nghi_benh),
        Decision::Refuse { .. }
    ));
}

/// **Giữ lại không phải một dạng từ chối** — người còn đó và sẽ được xét lại.
#[test]
fn giu_lai_khong_phai_mot_dang_tu_choi() {
    let r = che_do_day_du();
    let nghi_benh = Cargo {
        suspected_pathogen: true,
        living: true,
        ..Cargo::default()
    };
    match r.screen(&nghi_benh) {
        Decision::Hold { ticks, .. } => assert_eq!(ticks, 40),
        khac => panic!("phải giữ lại, nhận được {khac:?}"),
    }
}

/// **Sàng mà không có quyền từ chối chỉ là một cuốn sổ.**
#[test]
fn sang_ma_khong_co_quyen_tu_choi_thi_chi_la_mot_cuon_so() {
    let r = ContactRegime {
        quarantine: Quarantine {
            hold_ticks: 40,
            screens_pathogens: true,
            screens_taint: true,
            may_refuse: false,
        },
        ..che_do_day_du()
    };
    let nghi_benh = Cargo {
        suspected_pathogen: true,
        living: true,
        ..Cargo::default()
    };
    assert_eq!(
        r.screen(&nghi_benh),
        Decision::Allow,
        "phát hiện được mà không giữ được thì mầm bệnh vẫn vào"
    );
    // …và bệnh đó đi qua cổng thật.
    let mut l = EscrowLedger::new();
    let mut p = Portal {
        regime: r,
        ..cong_mo()
    };
    let t = Traveller {
        hitchhikers: vec!["pathogen.redcough".into()],
        ..lu_khach()
    };
    let id = di_het_chin_buoc(&mut l, &mut p, &t);
    assert_eq!(l.get(id).unwrap().hitchhikers, vec!["pathogen.redcough"]);
}

/// Luật vận chuyển chặn được hạt giống và linh hồn riêng biệt.
#[test]
fn luat_van_chuyen_chan_rieng_tung_loai() {
    let r = che_do_day_du(); // seeds: false, souls: false, living: true
    assert!(matches!(
        r.screen(&Cargo {
            seeds: true,
            ..Cargo::default()
        }),
        Decision::Refuse { .. }
    ));
    assert!(matches!(
        r.screen(&Cargo {
            souls: true,
            ..Cargo::default()
        }),
        Decision::Refuse { .. }
    ));
    assert_eq!(
        r.screen(&Cargo {
            living: true,
            ..Cargo::default()
        }),
        Decision::Allow
    );
}

/// **Công nhận pháp nhân mà không có tòa thì không cưỡng chế được gì.**
#[test]
fn cong_nhan_phap_nhan_ma_khong_co_toa_thi_vo_dung() {
    let khong_toa = ContactRegime {
        dispute: DisputeForum {
            forum: None,
            interpreters: true,
        },
        ..che_do_day_du()
    };
    assert!(!khong_toa.contract_enforceable());
    assert!(che_do_day_du().contract_enforceable());
}

/// Hai world không mặc định dùng chung đơn vị.
#[test]
fn hai_world_khong_mac_dinh_chung_don_vi() {
    assert!(che_do_day_du().shares_measure("mass.kg"));
    assert!(!che_do_day_du().shares_measure("length.li"));
    assert!(!ContactRegime::none().shares_measure("mass.kg"));
}

/// **Kiểm dịch chặt nhưng không công nhận pháp nhân** là một loại cổng có thật.
///
/// Test này tồn tại để chặn việc gộp bảy điều khoản thành một chỉ số "quan hệ":
/// một chỉ số duy nhất không diễn tả nổi tổ hợp này.
#[test]
fn kiem_dich_chat_ma_khong_cong_nhan_phap_nhan_la_mot_loai_cong_co_that() {
    let cho_lau_co_kiem_dich = ContactRegime {
        personhood: LegalPersonhood {
            recognized: false,
            contracts_enforceable: false,
        },
        ..che_do_day_du()
    };
    assert!(cho_lau_co_kiem_dich.failure_modes().is_empty());
    assert!(!cho_lau_co_kiem_dich.contract_enforceable());
}

/// Cư trú cho phép nhưng cấm lao động ⇒ kinh tế ngầm, và đó là kết quả hợp lệ.
#[test]
fn cu_tru_ma_cam_lao_dong_khong_bi_coi_la_loi() {
    let r = ContactRegime {
        residency: Residency {
            visa_free_ticks: 900,
            may_work: false,
        },
        ..che_do_day_du()
    };
    assert!(!r.failure_modes().contains(&Failure::RefugeeCamp));
    assert!(!r.residency.may_work);
}
