//! Vong lap bao loi da dong: chup mot repro bundle roi chay lai cho **dung cung
//! state hash**.
//!
//! Day la dieu kien hoan thanh thu tu cua Giai doan 0 (`plan.md §P9`), va la
//! bai test dat gia nhat cua ca giai doan: no chung minh rang khi mot bug xuat
//! hien, ta chup duoc no va tai hien duoc no — **truoc khi** co gameplay de tao
//! ra bug.

use mow_devtool::repro::{Manifest, ReproBundle};
use mow_scenario::testing::TestWorldFactory;
use mow_scenario::WorldFactory;
use std::collections::BTreeMap;

const WORLDSEED: &str = "test:tiny_village";
const TICK_LOI: u64 = 500;

fn dung_the_gioi() -> mow_core::Sim {
    TestWorldFactory
        .build(WORLDSEED, &BTreeMap::new())
        .expect("worldseed hop le")
}

#[test]
fn chup_roi_chay_lai_cho_dung_cung_state_hash() {
    let d = tempfile::tempdir().unwrap();

    // ── Chup ────────────────────────────────────────────────────────────────
    // Mo phong "mot bug xuat hien o tick 500": chay toi do, ghi lai state hash.
    let mut sim = dung_the_gioi();
    sim.advance(TICK_LOI).unwrap();
    let hash_luc_loi = sim.state_hash();

    // Nhat ky su kien la thu tai hien duoc the gioi; day la bundle tu chua.
    let events: Vec<u8> = sim
        .log()
        .iter()
        .map(|e| format!("{} {} {}\n", e.seq.0, e.tick.0, e.kind.0))
        .collect::<String>()
        .into_bytes();

    let mut packs = BTreeMap::new();
    packs.insert(
        "core".to_owned(),
        ("0.1.0".to_owned(), mow_math::StateHash([1u8; 32])),
    );

    let bundle = ReproBundle::capture(
        d.path(),
        Manifest {
            id: "repro-e2e".to_owned(),
            git_sha: "test".to_owned(),
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            captured_at: "2026-08-31T00:00:00Z".to_owned(),
            worldseed: WORLDSEED.to_owned(),
            config_hash: mow_math::StateHash([9u8; 32]),
            packs: packs.clone(),
            from_tick: 0,
            to_tick: TICK_LOI,
            expected_hash: hash_luc_loi,
            symptom: "vi du: state hash lech giua hai lan chay".to_owned(),
        },
        b"snapshot-tai-tick-0",
        &events,
    )
    .expect("chup duoc");

    // ── Chay lai ────────────────────────────────────────────────────────────
    // Mo lai bundle nhu tren mot may khac, sau nay.
    let mo_lai = ReproBundle::open(&bundle.root).expect("mo lai duoc");

    // Kiem moi truong TRUOC khi chay. Chay roi moi phat hien pack da doi la qua
    // muon: luc do ta da co mot ket qua, va mot ket qua sai luon thuyet phuc
    // hon la khong co ket qua nao.
    mo_lai
        .verify(&packs, mow_math::StateHash([9u8; 32]))
        .expect("moi truong khop");

    let mut lai = TestWorldFactory
        .build(&mo_lai.manifest.worldseed, &BTreeMap::new())
        .expect("dung lai duoc tu worldseed trong bundle");
    lai.advance(mo_lai.manifest.to_tick).unwrap();

    mo_lai
        .check_result(lai.state_hash())
        .expect("chay lai phai cho dung cung state hash");

    assert_eq!(lai.state_hash(), hash_luc_loi);
    assert_eq!(mo_lai.events().unwrap(), events);
}

#[test]
fn bundle_chay_tren_pack_da_doi_thi_tu_choi_thay_vi_bao_bug_da_het() {
    let d = tempfile::tempdir().unwrap();
    let mut sim = dung_the_gioi();
    sim.advance(TICK_LOI).unwrap();

    let mut packs = BTreeMap::new();
    packs.insert(
        "core".to_owned(),
        ("0.1.0".to_owned(), mow_math::StateHash([1u8; 32])),
    );

    let bundle = ReproBundle::capture(
        d.path(),
        Manifest {
            id: "repro-pack-doi".to_owned(),
            git_sha: "test".to_owned(),
            engine_version: "0.1.0".to_owned(),
            captured_at: "2026-08-31T00:00:00Z".to_owned(),
            worldseed: WORLDSEED.to_owned(),
            config_hash: mow_math::StateHash([9u8; 32]),
            packs,
            from_tick: 0,
            to_tick: TICK_LOI,
            expected_hash: sim.state_hash(),
            symptom: "x".to_owned(),
        },
        b"s",
        b"e",
    )
    .unwrap();

    // Content pack tien hoa.
    let mut moi = BTreeMap::new();
    moi.insert(
        "core".to_owned(),
        ("0.2.0".to_owned(), mow_math::StateHash([2u8; 32])),
    );
    let e = bundle
        .verify(&moi, mow_math::StateHash([9u8; 32]))
        .expect_err("phai tu choi");
    assert!(
        e.to_string().contains("không phải bằng chứng đã sửa"),
        "{e}"
    );
}
