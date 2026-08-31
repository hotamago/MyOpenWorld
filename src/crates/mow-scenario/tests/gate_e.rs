//! Cổng Giai đoạn E (`plan.md §9`, `PE-GATE`).
//!
//! Bốn điều kiện, nguyên văn:
//!
//! > 1. NPC ghép được module **chỉ từ node nó biết**.
//! > 2. Transfer lỗi **không nhân đôi / mất entity**.
//! > 3. Người ủ bệnh qua portal **không khỏi tức thì**.
//! > 4. Quần thể tách qua portal nhiều thế kỷ cho con lai **giảm sinh sản đo được**.
//!
//! Điểm chung: cả bốn đều là những chỗ mà một cài đặt sai vẫn **chạy trơn**.
//! Một NPC ghép bừa vẫn ra một spell hoạt động; một transfer nhân đôi entity
//! vẫn cho người chơi bước qua cổng; một deadline nhân đồng loạt vẫn ra một
//! con số; hai quần thể gặp lại vẫn lai được. Không cái nào panic, và không
//! cái nào tự lộ ra. Đó là lý do chúng là điều kiện cổng.

use mow_core::clock::{Clock, ClockDomain, Deadline, Tick};
use mow_core::{EntityId, WorldId};
use mow_life::speciation::{secondary_contact, IsolatedPopulation, SpeciationRoute};
use mow_magic::artifact::{check_synthesis, Synthesis, SynthesisError};
use mow_magic::sandbox::{Capability, ContextKind, LoadError, ModuleManifest, ModuleRegistry};
use mow_math::Rate;
use mow_portal::clock::Process;
use mow_portal::contact::{
    ContactRegime, DisputeForum, LegalPersonhood, Quarantine, Residency, TransportLaw,
};
use mow_portal::portal::{
    count_copies, recover, AccessPolicy, EscrowLedger, NeedsProfile, Portal, PortalState, Recovery,
    Traveller, WorldConditions,
};
use std::collections::BTreeMap;

// ─────────────────────────── Điều kiện 1 ───────────────────────────

/// **NPC ghép được module chỉ từ node nó biết.**
///
/// Hai nửa, và nửa thứ hai mới là nửa khó: một NPC **ghép được** (nếu không thì
/// cả cơ chế vô dụng) và **không ghép được từ thứ nó không biết** (nếu không
/// thì nền văn minh trong game phát minh ra thứ chưa ai nghĩ tới, và cây tri
/// thức ở `§13` mất nghĩa).
#[test]
fn gate_e1_npc_ghep_module_chi_tu_node_no_biet() {
    // Một pháp sư học trò: biết hai node, skill trung bình.
    let hoc_tro_biet = BTreeMap::from([
        ("spell.frost".to_owned(), 3_i64),
        ("spell.shape".to_owned(), 2),
    ]);

    // ── Nửa 1: ghép được thật ──
    let ghep_hop_le = Synthesis {
        author: EntityId(1),
        from_nodes: vec!["spell.frost".into(), "spell.shape".into()],
        complexity: 15,
    };
    assert!(
        check_synthesis(&ghep_hop_le, &hoc_tro_biet, 800, 100).is_empty(),
        "một NPC phải ghép được, không thì cơ chế này vô dụng"
    );

    // ── Nửa 2: không ghép được từ node nó không biết ──
    let ghep_bua = Synthesis {
        author: EntityId(1),
        from_nodes: vec!["spell.frost".into(), "spell.forbidden_star".into()],
        complexity: 15,
    };
    let loi = check_synthesis(&ghep_bua, &hoc_tro_biet, 800, 100);
    assert!(
        loi.contains(&SynthesisError::UnknownNode("spell.forbidden_star".into())),
        "ghép từ node chưa học phải bị chặn: {loi:?}"
    );

    // ── Và học trò không tạo ra thứ đại sư không tạo nổi ──
    let qua_suc = Synthesis {
        author: EntityId(1),
        from_nodes: vec!["spell.frost".into()],
        complexity: 90,
    };
    assert!(
        check_synthesis(&qua_suc, &hoc_tro_biet, 200, 100)
            .iter()
            .any(|e| matches!(e, SynthesisError::TooComplex { .. })),
        "trần độ phức tạp phải theo skill"
    );
    assert!(
        check_synthesis(&qua_suc, &hoc_tro_biet, 1_000, 100).is_empty(),
        "đại sư thì làm được"
    );

    // ── Và không có đường vòng: một module Agent không xin được toàn tri ──
    let mut reg = ModuleRegistry::new();
    let chan = reg
        .load(ModuleManifest {
            id: "npc.synthesised.frostshape".into(),
            version: 1,
            context: ContextKind::Agent,
            capabilities: vec![Capability::ReadAuthoritative("terrain".into())],
            fuel_limit: 10_000,
            memory_limit: 1 << 16,
            imports: vec!["mow.emit_proposal".into()],
        })
        .unwrap_err();
    assert!(matches!(chan, LoadError::AgentWantsAuthoritative { .. }));
}

// ─────────────────────────── Điều kiện 2 ───────────────────────────

fn gaia() -> Clock {
    Clock::synchronous()
}

fn abyss() -> Clock {
    Clock::new(Rate::per_tick(10))
}

fn che_do_du() -> ContactRegime {
    ContactRegime {
        quarantine: Quarantine {
            hold_ticks: 60,
            screens_pathogens: true,
            screens_taint: true,
            may_refuse: true,
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
            forum: Some("hội đồng cổng".to_owned()),
            interpreters: true,
        },
        ..ContactRegime::none()
    }
}

fn cong_mo() -> Portal {
    Portal {
        id: 7,
        source: WorldId(1),
        dest: WorldId(2),
        state: PortalState::Open,
        access: AccessPolicy::default(),
        bandwidth_mass: 10_000,
        used_mass: 0,
        regime: che_do_du(),
    }
}

fn khach(who: u64) -> Traveller {
    Traveller {
        who: EntityId(who),
        species: "human".into(),
        mass: 90,
        inventory: vec![11, 12],
        processes: vec![Process {
            id: "age".into(),
            deadline: Deadline::new(Tick(500), ClockDomain::Proper),
        }],
        hitchhikers: vec![],
        divine_signature: false,
    }
}

fn nhu_cau() -> NeedsProfile {
    NeedsProfile {
        atmosphere: (30, 100),
        temperature: (-1_000, 4_500),
        mana: (0, 8_000),
    }
}

fn song_duoc() -> WorldConditions {
    WorldConditions {
        atmosphere: 70,
        temperature: 2_000,
        mana: 3_000,
    }
}

/// **Transfer lỗi không nhân đôi và không làm mất entity.**
///
/// Vét cạn: cắt chuyến đi ở từng điểm trong bốn điểm, và ở mỗi điểm đếm số bản
/// sao trên toàn multiverse. Con số phải là **1**, luôn luôn — kể cả khi entity
/// không nằm ở world nào.
#[test]
fn gate_e2_transfer_loi_khong_nhan_doi_khong_mat_entity() {
    for cat_o in 0..4 {
        let mut so = EscrowLedger::new();
        let mut p = cong_mo();
        let t = khach(77);

        let (id, _) = so
            .begin(&mut p, &t, &nhu_cau(), &song_duoc(), &gaia(), &abyss())
            .unwrap();
        let mut o_world = vec![WorldId(1)];

        if cat_o >= 1 {
            so.depart(id).unwrap();
            o_world.clear(); // không ở world nào — nằm trong escrow
        }
        if cat_o >= 2 {
            so.arrive(id).unwrap();
            o_world.push(WorldId(2));
        }
        if cat_o >= 3 {
            so.release(id).unwrap();
        }

        assert_eq!(
            count_copies(t.who, &o_world, &so),
            1,
            "cắt ở bước {cat_o}: không được nhân đôi, không được bốc hơi"
        );

        // Và bản ghi dang dở luôn dò lại được, với đúng một việc phải làm.
        match cat_o {
            0 => assert_eq!(recover(&so), vec![(id, Recovery::Rollback)]),
            1 => assert_eq!(recover(&so), vec![(id, Recovery::Complete)]),
            2 => assert_eq!(recover(&so), vec![(id, Recovery::Confirm)]),
            _ => assert!(recover(&so).is_empty()),
        }
    }
}

/// Transfer bị **từ chối** thì không để lại bản ghi và không trừ băng thông.
///
/// Nửa còn lại của điều kiện 2: một chuyến đi hỏng ở bước kiểm không được để
/// lại rác trong sổ trung chuyển — rác ở đó sẽ được `recover` đem đi hoàn tất.
#[test]
fn gate_e2_transfer_bi_tu_choi_khong_de_lai_rac() {
    let mut so = EscrowLedger::new();
    let mut p = Portal {
        state: PortalState::Collapsing,
        ..cong_mo()
    };
    assert!(so
        .begin(
            &mut p,
            &khach(77),
            &nhu_cau(),
            &song_duoc(),
            &gaia(),
            &abyss()
        )
        .is_err());
    assert!(so.is_empty());
    assert_eq!(p.used_mass, 0);
    assert!(recover(&so).is_empty());
}

// ─────────────────────────── Điều kiện 3 ───────────────────────────

/// **Người ủ bệnh qua portal không khỏi tức thì** — và nợ không đáo hạn tức thì.
///
/// Hai vế phải cùng đúng trong **một** chuyến đi. Nếu chỉ kiểm vế đầu thì một
/// cài đặt "nhân mọi deadline với tỉ lệ" vẫn qua bài, và nó sẽ làm mọi hợp đồng
/// đáo hạn tức thì mà không ai phát hiện cho tới khi có người bỏ trốn qua cổng
/// để xù nợ.
#[test]
fn gate_e3_nguoi_u_benh_qua_portal_khong_khoi_tuc_thi() {
    let mut so = EscrowLedger::new();
    let mut p = cong_mo();

    let benh_nhan = Traveller {
        processes: vec![
            Process {
                id: "disease.plague.incubation".into(),
                deadline: Deadline::new(Tick(300), ClockDomain::Proper),
            },
            Process {
                id: "contract.loan.7741".into(),
                deadline: Deadline::new(Tick(900), ClockDomain::WorldLocal),
            },
            Process {
                id: "curse.by_moon".into(),
                deadline: Deadline::new(Tick(700), ClockDomain::LawDefined),
            },
            Process {
                id: "divine.council_summons".into(),
                deadline: Deadline::new(Tick(1_200), ClockDomain::Divine),
            },
        ],
        ..khach(77)
    };

    let (id, _) = so
        .begin(
            &mut p,
            &benh_nhan,
            &nhu_cau(),
            &song_duoc(),
            &gaia(),
            &abyss(),
        )
        .unwrap();
    let r = so.get(id).unwrap();

    assert!(
        r.rebase.covers_all(&benh_nhan.processes),
        "cả bốn miền phải được xem xét, kể cả những miền không đổi"
    );

    let han = |ten: &str| r.processes.iter().find(|x| x.id == ten).unwrap().deadline;

    // Ủ bệnh: quy đổi, **còn hạn** — không khỏi và không chết tức thì.
    assert_eq!(han("disease.plague.incubation").at, Tick(3_000));
    // Hợp đồng: neo vào world đã ký.
    assert_eq!(han("contract.loan.7741").at, Tick(900));
    // Lời nguyền: luật sở hữu đồng hồ đó, engine không đụng.
    assert_eq!(han("curse.by_moon").at, Tick(700));
    // Đồng hồ thần: chung cho cả multiverse.
    assert_eq!(han("divine.council_summons").at, Tick(1_200));

    // **Đúng một miền đổi số** — đây là chỗ "nhân đồng loạt" chết.
    assert_eq!(r.rebase.changed().count(), 1);
}

// ─────────────────────────── Điều kiện 4 ───────────────────────────

/// **Quần thể tách qua portal nhiều thế kỷ cho con lai giảm sinh sản đo được.**
///
/// Chữ *"đo được"* là yêu cầu thật: kết quả phải là những con số mà một nhà tự
/// nhiên học trong game đếm được — bao nhiêu cặp thử, bao nhiêu con lai sinh
/// ra, bao nhiêu con sinh sản tiếp — chứ không phải một nhãn `is_different_species`.
#[test]
fn gate_e4_quan_the_tach_qua_portal_cho_con_lai_giam_sinh_san_do_duoc() {
    // Cổng sập sau khi nhóm di cư đi qua.
    let mut p = cong_mo();
    p.transition(PortalState::Unstable).unwrap();
    p.transition(PortalState::Collapsing).unwrap();
    p.transition(PortalState::Closed).unwrap();
    assert!(!p.state.passable());

    let nhanh_abyss = IsolatedPopulation {
        id: "gaia.human.abyssal_branch".into(),
        route: SpeciationRoute::IsolationThenDivergence,
        effective_size: 800,
        generations: 600,
        selection_differential: 60,
    };

    let gap_lai = secondary_contact(&nhanh_abyss, 400);

    // Đo được: có mẫu, và tỉ lệ sinh sản thấp hơn hẳn mức cùng loài.
    assert!(
        gap_lai.decline_is_measurable(),
        "{}/{} con lai sinh sản tiếp — không đo được thì không qua cổng",
        gap_lai.hybrids_fertile,
        gap_lai.hybrids_born
    );
    assert!(gap_lai.hybrids_born >= 20, "phải có mẫu đủ để kết luận");
    assert!(gap_lai.fertile_permille() < 900);

    // Vẫn nhận ra nhau là họ hàng — đó là chỗ bi kịch nằm.
    assert!(gap_lai.still_recognisable);
    assert!(
        gap_lai.hybrids_born > 0,
        "lai được nhưng con lai vô sinh dần: đó mới là §9.5.5"
    );

    // Và càng tách lâu càng nặng, theo nhịp tăng dần chứ không tuyến tính.
    let ngan = secondary_contact(
        &IsolatedPopulation {
            generations: 200,
            ..nhanh_abyss.clone()
        },
        400,
    );
    assert!(ngan.fertile_permille() > gap_lai.fertile_permille());
    assert!(
        gap_lai.divergence.incompatible_pairs > ngan.divergence.incompatible_pairs * 3,
        "hiệu ứng snowball: gấp ba thời gian cho hơn gấp ba bất tương hợp"
    );
}

/// Không cách ly thì **không** phân kỳ — bài kiểm chứng ngược.
///
/// Nếu bỏ bài này thì một cài đặt luôn trả về "giảm sinh sản" cũng qua được
/// điều kiện 4, và nó sẽ làm mọi cặp trong thế giới không sinh con được.
#[test]
fn gate_e4_khong_cach_ly_thi_khong_phan_ky() {
    let vua_tach = secondary_contact(
        &IsolatedPopulation {
            id: "x".into(),
            route: SpeciationRoute::IsolationThenDivergence,
            effective_size: 800,
            generations: 5,
            selection_differential: 60,
        },
        400,
    );
    assert!(!vua_tach.decline_is_measurable());
    assert_eq!(vua_tach.divergence.incompatible_pairs, 0);
    assert_eq!(vua_tach.fertile_permille(), 1_000);
}
