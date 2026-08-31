//! Test genesis va worldseed.
//!
//! Bai quan trong nhat: `genesis_de_lai_nhat_ky_day_du_tu_tick_0` — do la
//! §22.28, va la ly do genesis khong duoc di duong tat.

use mow_core::{BranchId, Clock, Sim, SimConfig, WorldId};
use mow_math::{CanonicalHash, StateHash, StateHasher, WorldSeed};
use mow_scenario::genesis;
use mow_scenario::testing::handlers;
use mow_scenario::worldseed::{Lockfile, Worldseed};

fn doc_worldseed() -> Worldseed {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content/core/worldseeds/tiny_village.yaml");
    let text = std::fs::read_to_string(&p).expect("co worldseed");
    Worldseed::from_yaml(&text).expect("phan tich duoc")
}

fn sim_rong(seed: u64) -> Sim {
    Sim::new(
        SimConfig {
            world: WorldId(1),
            branch: BranchId(1),
            seed: WorldSeed(seed),
            clock: Clock::synchronous(),
        },
        handlers(),
    )
}

// ── §22.28 ───────────────────────────────────────────────────────────────────

#[test]
fn genesis_de_lai_nhat_ky_day_du_tu_tick_0() {
    let ws = doc_worldseed();
    let mut sim = sim_rong(ws.resolved_seed());
    let r = genesis::run(&mut sim, &ws).expect("genesis chay duoc");

    assert_eq!(r.commands, 5);
    assert_eq!(sim.store().len(), 5);

    // Moi thuc the deu co su kien sinh ra — cau hoi "vi sao co ngoi lang o day"
    // tra loi duoc, thay vi cut o cho "ngoi lang da ton tai".
    assert_eq!(r.events, 5);
    assert!(
        sim.log().iter().all(|e| e.tick.0 == 0),
        "genesis phai o tick 0"
    );
    assert!(sim.log().iter().all(|e| e.kind.0 == "core.entity.spawned"));
}

#[test]
fn the_gioi_moi_tao_khong_vi_pham_bat_bien_nao() {
    use mow_core::invariant::Cost;
    use mow_core::InvariantRunner;

    let ws = doc_worldseed();
    let mut sim = sim_rong(ws.resolved_seed());
    genesis::run(&mut sim, &ws).unwrap();

    let rep = sim.check(&InvariantRunner::standard(Cost::Expensive));
    assert!(rep.is_clean(), "{rep}");
}

#[test]
fn ten_giai_duoc_thanh_dinh_danh_that() {
    let ws = doc_worldseed();
    let mut sim = sim_rong(ws.resolved_seed());
    let r = genesis::run(&mut sim, &ws).unwrap();

    let lang = r.named["village"];
    let aren = r.named["aren"];
    assert_ne!(lang, aren);
    // `within: $village` phai giai ra id that cua lang.
    assert_eq!(
        sim.store().attr_int(aren, "core.within"),
        Some(lang.get() as i64)
    );
}

#[test]
fn tro_toi_ten_chua_dat_thi_bi_bat_luc_validate() {
    // Bat o validate chu khong phai luc chay: mot worldseed do vo giua chung
    // de lai mot the gioi hong mot nua.
    let ws = Worldseed::from_yaml(
        r#"
id: "x"
generation_profile: "gaia"
genesis:
  - command: core.spawn
    args: { kind: entity, within: $chua_co }
"#,
    )
    .unwrap();
    let e = ws.validate().expect_err("phai loi");
    assert!(e.iter().any(|m| m.contains("chua_co")), "{e:?}");
}

#[test]
fn trung_ten_bi_bat() {
    let ws = Worldseed::from_yaml(
        r#"
id: "x"
generation_profile: "gaia"
genesis:
  - command: core.spawn
    name: a
    args: { kind: entity }
  - command: core.spawn
    name: a
    args: { kind: entity }
"#,
    )
    .unwrap();
    assert!(ws.validate().is_err());
}

#[test]
fn buoc_that_bai_thi_bao_ro_buoc_thu_may() {
    let ws = Worldseed::from_yaml(
        r#"
id: "x"
generation_profile: "gaia"
genesis:
  - command: core.spawn
    args: { kind: entity }
  - command: khong.ton.tai
    args: {}
"#,
    )
    .unwrap();
    let mut sim = sim_rong(1);
    let e = genesis::run(&mut sim, &ws).expect_err("phai loi");
    let s = e.to_string();
    assert!(s.contains("buoc 1") || s.contains("bước 1"), "{s}");
    assert!(s.contains("khong.ton.tai"), "{s}");
}

// ── Worldseed va lockfile ────────────────────────────────────────────────────

#[test]
fn cung_worldseed_cho_cung_seed_so_hoc() {
    let a = doc_worldseed();
    let b = doc_worldseed();
    assert_eq!(a.resolved_seed(), b.resolved_seed());
    assert_ne!(a.resolved_seed(), 0);
}

#[test]
fn doi_version_thi_doi_the_gioi() {
    let mut a = doc_worldseed();
    let mut b = doc_worldseed();
    a.version = 1;
    b.version = 2;
    assert_ne!(a.resolved_seed(), b.resolved_seed());
}

#[test]
fn seed_khai_bao_tuong_minh_thi_duoc_ton_trong() {
    let mut ws = doc_worldseed();
    ws.seed = Some(12_345);
    assert_eq!(ws.resolved_seed(), 12_345);
}

fn hash_cua(ws: &Worldseed) -> StateHash {
    let mut h = StateHasher::with_domain("mow.worldseed.hash.v1");
    ws.canonical_hash(&mut h);
    h.finish()
}

fn lock_cho(ws: &Worldseed, packs: Vec<(String, String, StateHash)>) -> Lockfile {
    Lockfile {
        worldseed_id: ws.id.clone(),
        worldseed_version: ws.version,
        worldseed_hash: hash_cua(ws),
        resolved_seed: ws.resolved_seed(),
        generation_profile: ws.generation_profile.clone(),
        profile_hash: StateHash([7u8; 32]),
        packs,
    }
}

#[test]
fn lockfile_khop_thi_cho_load() {
    let ws = doc_worldseed();
    let packs = vec![("core".into(), "0.1.0".into(), StateHash([1u8; 32]))];
    let l = lock_cho(&ws, packs.clone());
    l.verify(&ws, StateHash([7u8; 32]), &packs).expect("khop");
}

#[test]
fn worldseed_bi_sua_sau_khi_tao_world_thi_bi_bat() {
    // Neu khong bat, mot world "cung ten" se sinh ra dia hinh khac va khong ai
    // biet vi sao.
    let ws = doc_worldseed();
    let packs = vec![("core".into(), "0.1.0".into(), StateHash([1u8; 32]))];
    let l = lock_cho(&ws, packs.clone());

    let mut da_sua = doc_worldseed();
    da_sua.genesis.pop();

    let e = l
        .verify(&da_sua, StateHash([7u8; 32]), &packs)
        .expect_err("phai tu choi");
    assert!(e.iter().any(|m| m.contains("worldseed")), "{e:?}");
}

#[test]
fn pack_thieu_thua_hoac_doi_deu_bi_bat() {
    let ws = doc_worldseed();
    let goc = vec![("core".into(), "0.1.0".into(), StateHash([1u8; 32]))];
    let l = lock_cho(&ws, goc.clone());

    // Thieu.
    assert!(l.verify(&ws, StateHash([7u8; 32]), &[]).is_err());

    // Doi hash.
    let doi = vec![("core".into(), "0.2.0".into(), StateHash([2u8; 32]))];
    assert!(l.verify(&ws, StateHash([7u8; 32]), &doi).is_err());

    // Thua — mot pack moi co the dang ky luat moi.
    let mut thua = goc.clone();
    thua.push(("mypack".into(), "1.0.0".into(), StateHash([3u8; 32])));
    let e = l
        .verify(&ws, StateHash([7u8; 32]), &thua)
        .expect_err("phai tu choi");
    assert!(e.iter().any(|m| m.contains("mypack")), "{e:?}");
}

#[test]
fn profile_doi_thi_bi_bat() {
    let ws = doc_worldseed();
    let packs = vec![("core".into(), "0.1.0".into(), StateHash([1u8; 32]))];
    let l = lock_cho(&ws, packs.clone());
    let e = l
        .verify(&ws, StateHash([8u8; 32]), &packs)
        .expect_err("phai tu choi");
    assert!(e.iter().any(|m| m.contains("profile")), "{e:?}");
}
