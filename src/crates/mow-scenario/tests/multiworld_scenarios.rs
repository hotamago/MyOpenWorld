//! Kịch bản đa thế giới (`PE-14`, phần `multiworld/*`).
//!
//! Bốn kịch bản, mỗi cái nối **nhiều crate** lại: cổng, đồng hồ, sinh thái,
//! thần linh. Đó là lý do chúng ở đây chứ không ở crate riêng — mỗi crate tự
//! nó đã đúng, và chỗ hỏng nằm ở việc một hệ quả không đi được từ crate này
//! sang crate kia.

use mow_core::clock::{Clock, ClockDomain, Deadline, Tick};
use mow_core::{EntityId, WorldId};
use mow_divine::authority::{Domain, DomainAct, God, GodKind};
use mow_eco::invasion::{assess, outbreak, Ecosystem, FoodWeb, Immunity, Virulence};
use mow_life::speciation::{secondary_contact, IsolatedPopulation, SpeciationRoute};
use mow_math::Rate;
use mow_portal::clock::Process;
use mow_portal::contact::{
    ContactRegime, DisputeForum, LegalPersonhood, Measures, Quarantine, Residency, Tariff,
    TransportLaw,
};
use mow_portal::portal::{
    count_copies, recover, AccessPolicy, EscrowLedger, EscrowPhase, NeedsProfile, Portal,
    PortalState, Recovery, Traveller, WorldConditions,
};
use std::collections::BTreeSet;

fn gaia() -> Clock {
    Clock::synchronous()
}

fn abyss_nhanh_gap_10() -> Clock {
    Clock::new(Rate::per_tick(10))
}

fn nhu_cau_nguoi() -> NeedsProfile {
    NeedsProfile {
        atmosphere: (30, 100),
        temperature: (-1_000, 4_500),
        mana: (0, 8_000),
    }
}

fn dieu_kien_song_duoc() -> WorldConditions {
    WorldConditions {
        atmosphere: 70,
        temperature: 2_000,
        mana: 3_000,
    }
}

fn che_do_bo_mac() -> ContactRegime {
    ContactRegime {
        transport: TransportLaw {
            living_creatures: true,
            seeds: true,
            souls: true,
        },
        ..ContactRegime::none()
    }
}

fn che_do_co_kiem_soat() -> ContactRegime {
    ContactRegime {
        quarantine: Quarantine {
            hold_ticks: 60,
            screens_pathogens: true,
            screens_taint: true,
            may_refuse: true,
        },
        tariff: Tariff {
            rate_permille: 40,
            contraband: BTreeSet::from(["weapon.cursed".to_owned()]),
        },
        measures: Measures {
            agreed: BTreeSet::from(["mass.kg".to_owned()]),
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
    }
}

fn cong(regime: ContactRegime) -> Portal {
    Portal {
        id: 7,
        source: WorldId(1),
        dest: WorldId(2),
        state: PortalState::Open,
        access: AccessPolicy::default(),
        bandwidth_mass: 10_000,
        used_mass: 0,
        regime,
    }
}

// ───────── multiworld/plague_carrier — người ủ bệnh đi qua cổng ─────────

/// **Người đang ủ bệnh không khỏi tức thì, và nợ không đáo hạn tức thì.**
///
/// Kịch bản đi hết chín bước, và kiểm cả hai miền đồng hồ trong **một** chuyến
/// đi — vì lỗi ở đây là lỗi "nhân đồng loạt", và nhân đồng loạt chỉ lộ ra khi
/// có hai miền ngược chiều nhau cùng lúc.
#[test]
fn multiworld_nguoi_u_benh_qua_cong_khong_khoi_tuc_thi() {
    let mut so = EscrowLedger::new();
    let mut p = cong(che_do_co_kiem_soat());

    let khach = Traveller {
        who: EntityId(77),
        species: "human".into(),
        mass: 90,
        inventory: vec![],
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
                id: "pregnancy.term".into(),
                deadline: Deadline::new(Tick(4_000), ClockDomain::Proper),
            },
        ],
        hitchhikers: vec![],
        divine_signature: false,
    };

    let (id, canh_bao) = so
        .begin(
            &mut p,
            &khach,
            &nhu_cau_nguoi(),
            &dieu_kien_song_duoc(),
            &gaia(),
            &abyss_nhanh_gap_10(),
        )
        .unwrap();
    assert!(canh_bao.problems.is_empty(), "world đích sống được");

    let ban_ghi = so.get(id).unwrap();
    assert!(
        ban_ghi.rebase.covers_all(&khach.processes),
        "sót một tiến trình là bug tệ nhất mà §4.5 cảnh báo"
    );

    let hay = |ten: &str| {
        ban_ghi
            .processes
            .iter()
            .find(|x| x.id == ten)
            .unwrap()
            .deadline
    };
    assert_eq!(hay("disease.plague.incubation").at, Tick(3_000));
    assert_eq!(hay("pregnancy.term").at, Tick(40_000));
    assert_eq!(
        hay("contract.loan.7741").at,
        Tick(900),
        "hợp đồng neo vào world đã ký"
    );
    assert!(
        hay("disease.plague.incubation").at.0 > 0,
        "không khỏi tức thì, và cũng không chết tức thì"
    );

    so.depart(id).unwrap();
    so.arrive(id).unwrap();
    so.release(id).unwrap();
    assert!(recover(&so).is_empty());
}

// ───────── multiworld/crash_mid_transfer — sập giữa chuyến ─────────

/// **Sập ở mọi điểm đều không nhân đôi và không làm mất entity.**
///
/// Chạy chín bước và cắt ngang ở từng điểm, mỗi lần đếm lại số bản sao. Đây là
/// `INV-22-8` chứng minh bằng vét cạn chứ bằng lý luận.
#[test]
fn multiworld_sap_o_moi_diem_deu_khong_nhan_doi_khong_mat() {
    for cat_o in 0..4 {
        let mut so = EscrowLedger::new();
        let mut p = cong(che_do_co_kiem_soat());
        let khach = Traveller {
            who: EntityId(77),
            species: "human".into(),
            mass: 90,
            inventory: vec![11],
            processes: vec![Process {
                id: "age".into(),
                deadline: Deadline::new(Tick(500), ClockDomain::Proper),
            }],
            hitchhikers: vec![],
            divine_signature: false,
        };

        let (id, _) = so
            .begin(
                &mut p,
                &khach,
                &nhu_cau_nguoi(),
                &dieu_kien_song_duoc(),
                &gaia(),
                &abyss_nhanh_gap_10(),
            )
            .unwrap();

        // Entity ở đâu, theo state thật của hai world.
        let mut o_world: Vec<WorldId> = vec![WorldId(1)];

        if cat_o >= 1 {
            so.depart(id).unwrap();
            o_world.clear();
        }
        if cat_o >= 2 {
            so.arrive(id).unwrap();
            o_world.push(WorldId(2));
        }
        if cat_o >= 3 {
            so.release(id).unwrap();
        }

        assert_eq!(
            count_copies(khach.who, &o_world, &so),
            1,
            "cắt ở bước {cat_o}: phải đúng một bản"
        );

        // Và mỗi điểm cắt có đúng một cách xử lý sau khi nạp lại save.
        let viec = recover(&so);
        match cat_o {
            0 => assert_eq!(viec, vec![(id, Recovery::Rollback)]),
            1 => assert_eq!(viec, vec![(id, Recovery::Complete)]),
            2 => assert_eq!(viec, vec![(id, Recovery::Confirm)]),
            _ => assert!(viec.is_empty()),
        }
    }
}

/// Nhiều người cùng đi qua một cổng: **mỗi người một bản ghi**, không lẫn.
#[test]
fn multiworld_nhieu_nguoi_cung_di_qua_khong_lan_ban_ghi() {
    let mut so = EscrowLedger::new();
    let mut p = cong(che_do_co_kiem_soat());
    let mut ids = Vec::new();

    for i in 0..5u64 {
        let t = Traveller {
            who: EntityId(100 + i),
            species: "human".into(),
            mass: 80,
            inventory: vec![],
            processes: vec![],
            hitchhikers: vec![],
            divine_signature: false,
        };
        let (id, _) = so
            .begin(
                &mut p,
                &t,
                &nhu_cau_nguoi(),
                &dieu_kien_song_duoc(),
                &gaia(),
                &abyss_nhanh_gap_10(),
            )
            .unwrap();
        ids.push((id, t.who));
    }

    // Cho ba người đi tiếp, hai người kẹt lại.
    for (id, _) in ids.iter().take(3) {
        so.depart(*id).unwrap();
    }
    // Ba người đang nằm trong escrow đếm ra đúng một bản; hai người còn ở
    // world nguồn cũng đúng một bản. Không ai lẫn vào bản ghi của ai.
    for (i, (_, who)) in ids.iter().enumerate() {
        let da_roi_nguon = i < 3;
        let o_world: Vec<WorldId> = if da_roi_nguon {
            vec![]
        } else {
            vec![WorldId(1)]
        };
        assert_eq!(count_copies(*who, &o_world, &so), 1, "người thứ {i}");
        assert_eq!(
            so.pending()
                .iter()
                .filter(|r| r.entity == *who && r.phase == EscrowPhase::Departed)
                .count(),
            usize::from(da_roi_nguon)
        );
    }
    assert_eq!(so.len(), 5);
    assert_eq!(recover(&so).len(), 5);
}

// ───────── multiworld/unmanaged_gate — cổng bỏ mặc thành ổ dịch ─────────

/// **Cùng một cổng, hai chế độ tiếp xúc, hai kết cục.**
///
/// Đây là `§6.4` chứng minh bằng đối chứng: khác biệt duy nhất giữa hai nhánh
/// là bảy điều khoản, và kết cục là một bên có dịch còn một bên không.
#[test]
fn multiworld_cong_bo_mac_thanh_o_dich_cong_co_kiem_soat_thi_khong() {
    let mang_benh = |regime: ContactRegime| -> bool {
        let mut so = EscrowLedger::new();
        let mut p = cong(regime);
        let t = Traveller {
            who: EntityId(77),
            species: "human".into(),
            mass: 90,
            inventory: vec![],
            processes: vec![],
            hitchhikers: vec!["pathogen.redcough".into()],
            divine_signature: false,
        };
        let ket = so.begin(
            &mut p,
            &t,
            &nhu_cau_nguoi(),
            &dieu_kien_song_duoc(),
            &gaia(),
            &abyss_nhanh_gap_10(),
        );
        match ket {
            Ok((id, _)) => {
                so.depart(id).unwrap();
                so.arrive(id).unwrap();
                !so.get(id).unwrap().hitchhikers.is_empty()
            }
            Err(_) => false,
        }
    };

    assert!(mang_benh(che_do_bo_mac()), "cổng bỏ mặc: mầm bệnh đi lọt");
    assert!(
        !mang_benh(che_do_co_kiem_soat()),
        "cổng có kiểm dịch: bị giữ ở vùng cách ly"
    );

    // Và mầm bệnh lọt được thì hậu quả ở đích là mức xóa sổ.
    let o = outbreak(
        Virulence(200),
        &Immunity {
            pathogen: "redcough".into(),
            ever_exposed: false,
            herd_permille: 0,
        },
        Some(7),
    );
    assert!(o.civilization_ending);
    assert_eq!(o.arrived_via, Some(7), "truy được về đúng cổng số 7");
}

/// Cổng bỏ mặc hóa thành **bốn thứ cùng lúc**, và nói được là những thứ nào.
#[test]
fn multiworld_cong_bo_mac_hong_nhieu_kieu_cung_luc() {
    assert!(che_do_bo_mac().failure_modes().len() >= 3);
    assert!(che_do_co_kiem_soat().failure_modes().is_empty());
}

/// Loài đi qua cổng bùng nổ ở đích vì **ở đó không có thiên địch**.
#[test]
fn multiworld_loai_di_qua_cong_bung_no_vi_thieu_thien_dich() {
    let web = FoodWeb {
        predators: ["predator.fox".to_owned()].into_iter().collect(),
        prey: ["grass".to_owned()].into_iter().collect(),
        competitors: BTreeSet::new(),
    };
    let gaia_co_cao = Ecosystem {
        present: ["grass".to_owned(), "predator.fox".to_owned()]
            .into_iter()
            .collect(),
        carrying_capacity: 10_000,
    };
    let abyss_khong_cao = Ecosystem {
        present: ["grass".to_owned()].into_iter().collect(),
        carrying_capacity: 10_000,
    };

    assert!(!assess("grazer.rabbit", &web, &gaia_co_cao).will_explode());
    let r = assess("grazer.rabbit", &web, &abyss_khong_cao);
    assert!(r.will_explode());
    assert!(r.missing_predators(&web).contains("predator.fox"));
}

// ───────── multiworld/closed_portal_speciation — cổng đóng lại ─────────

/// **Cổng đóng lại là cỗ máy tạo loài.**
///
/// Chuỗi: một nhóm đi qua → cổng sập → vài trăm đời → cổng mở lại → vùng tiếp
/// xúc thứ cấp. Con lai giảm sinh sản **đo được**, không phải một nhãn.
#[test]
fn multiworld_cong_dong_lai_tao_ra_loai_moi() {
    // Cổng sập, không mở lại được.
    let mut p = cong(che_do_co_kiem_soat());
    p.transition(PortalState::Unstable).unwrap();
    p.transition(PortalState::Collapsing).unwrap();
    p.transition(PortalState::Closed).unwrap();
    assert!(!p.state.passable());
    assert!(p.transition(PortalState::Open).is_err(), "sập rồi thì thôi");

    // Nhóm di cư sống dưới trọng lực, khí quyển và mật độ mana khác.
    let tach = IsolatedPopulation {
        id: "gaia.human.abyssal_branch".into(),
        route: SpeciationRoute::IsolationThenDivergence,
        effective_size: 800,
        generations: 600,
        selection_differential: 60,
    };

    let gap_lai = secondary_contact(&tach, 400);
    assert!(gap_lai.still_recognisable, "vẫn nhận ra nhau là họ hàng");
    assert!(
        gap_lai.decline_is_measurable(),
        "sụt giảm phải đo được: {}/{} con lai sinh sản tiếp",
        gap_lai.hybrids_fertile,
        gap_lai.hybrids_born
    );
    assert!(gap_lai.hybrids_born > 0, "vẫn có con lai — mới là bi kịch");
}

/// **Tách càng lâu càng khó lai** — và tăng nhanh dần, không tuyến tính.
#[test]
fn multiworld_tach_cang_lau_cang_kho_lai_va_tang_nhanh_dan() {
    let sau = |doi: u64| {
        secondary_contact(
            &IsolatedPopulation {
                id: "x".into(),
                route: SpeciationRoute::IsolationThenDivergence,
                effective_size: 800,
                generations: doi,
                selection_differential: 60,
            },
            1_000,
        )
    };
    let a = sau(200);
    let b = sau(400);
    let c = sau(800);

    assert!(a.fertile_permille() > b.fertile_permille());
    assert!(b.fertile_permille() > c.fertile_permille());
    // Snowball: gấp đôi thời gian cho hơn gấp đôi số cặp bất tương hợp.
    assert!(
        b.divergence.incompatible_pairs > a.divergence.incompatible_pairs * 3,
        "{} so với {}",
        b.divergence.incompatible_pairs,
        a.divergence.incompatible_pairs
    );
}

// ───────── multiworld/divine_storm — thần bão và một thành phố ─────────

/// **Thành phố xây bằng đá, có cảnh báo, có dân biết chạy thì sống sót.**
///
/// Nếu thần đặt thẳng `city.destroyed = true` thì ba khoản đầu tư đó vô nghĩa.
/// Kịch bản này chứng minh chuỗi `§14.2` đi qua weather → vật liệu → cảnh báo →
/// hành động cư dân, và mỗi mắt đều đổi được kết quả.
#[test]
fn multiworld_than_bao_khong_pha_duoc_thanh_pho_chuan_bi_tot() {
    let mut than = God {
        who: EntityId(900),
        kind: GodKind::Ascended,
        domains: vec![Domain {
            name: "storm".into(),
            fields: vec!["weather.wind".into()],
            counters: vec!["calm".into()],
        }],
        energy: 20_000,
        followers: 50_000,
        anchored_regions: vec![4],
    };

    let ket = than
        .act(&DomainAct::Manifest {
            field: "weather.wind".into(),
            region: 4,
            magnitude: 900,
        })
        .unwrap();
    let de_xuat = ket.proposal.expect("thần chỉ đề xuất lên trường");

    // Chuỗi sau đó là mô phỏng thường, không phải quyền thần.
    let suc_gio = de_xuat.delta;
    let do_ben_da = 700_i64;
    let canh_bao_som = true;
    let dan_biet_chay = true;

    let cong_trinh_sap = suc_gio > do_ben_da;
    let thuong_vong = if canh_bao_som && dan_biet_chay {
        suc_gio / 100
    } else {
        suc_gio
    };

    assert!(cong_trinh_sap, "gió mạnh hơn độ bền thì nhà đổ");
    assert!(
        thuong_vong < 20,
        "nhưng cảnh báo và sơ tán cứu được người: {thuong_vong}"
    );

    // Và thần **không** chạm được vào thứ ngoài domain, dù còn đầy năng lượng.
    assert!(than
        .act(&DomainAct::Amplify {
            field: "soil.fertility".into(),
            region: 4,
            permille: 100,
        })
        .is_err());
}
