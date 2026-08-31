//! Kich ban khong gian (`PA-10`).
//!
//! Ba dieu can chung minh, va ca ba deu la loai loi lo ra rat muon neu khong
//! kiem som:
//!
//! 1. **Seam** — dia hinh lien tuc qua bien chunk.
//! 2. **Toa do xa** — the gioi van dung o quy mo ma `f64` da mat chinh xac.
//! 3. **Vong doi save** — dao, dat, luu, nap, replay cho **dung cung hash**.

use mow_core::{BranchId, Clock, Command, Sim, SimConfig, Value, WorldId};
use mow_math::{ChunkPos, WorldPos, WorldSeed};
use mow_scenario::testing::handlers;
use mow_spatial::{ChunkStore, Lod};
use mow_worldgen::{GenerationProfile, Worldgen};

const SIZE: i64 = 32;

fn wg() -> Worldgen {
    Worldgen::new(WorldSeed(4_242), GenerationProfile::default())
}

// ── Seam ─────────────────────────────────────────────────────────────────────

#[test]
fn dia_hinh_lien_tuc_qua_moi_bien_chunk_theo_ca_hai_truc() {
    let w = wg();
    for truc in 0..2 {
        for k in -6..6i64 {
            let bien = k * SIZE;
            let (a, b) = if truc == 0 {
                (
                    w.base_cell(bien - 1, 77).unwrap(),
                    w.base_cell(bien, 77).unwrap(),
                )
            } else {
                (
                    w.base_cell(77, bien - 1).unwrap(),
                    w.base_cell(77, bien).unwrap(),
                )
            };
            let buoc = (a.elevation.height_m - b.elevation.height_m).abs();
            assert!(
                buoc < 200,
                "bac thang {buoc} m ngay tai bien chunk {bien} (truc {truc})"
            );
        }
    }
}

#[test]
fn goc_bon_chunk_gap_nhau_van_lien_tuc() {
    // Diem ma bon chunk cham nhau la cho de lo nhat neu co khau mep.
    let w = wg();
    for (dx, dy) in [(-1i64, -1i64), (0, -1), (-1, 0), (0, 0)] {
        let c = w.base_cell(SIZE * 3 + dx, SIZE * 3 + dy).unwrap();
        assert!(c.elevation.height_m.abs() < 10_000);
    }
    let a = w.base_cell(SIZE * 3 - 1, SIZE * 3 - 1).unwrap();
    let d = w.base_cell(SIZE * 3, SIZE * 3).unwrap();
    assert!((a.elevation.height_m - d.elevation.height_m).abs() < 300);
}

// ── Toa do xa ────────────────────────────────────────────────────────────────

#[test]
fn the_gioi_van_dung_o_quy_mo_vuot_2_mu_53() {
    // 2^53 la gioi han ma `f64` con bieu dien chinh xac so nguyen. `§22.10`
    // doi he thong van dung qua diem do.
    //
    // Kiem o tang **nhieu**, khong phai o tang `BaseCell`: hai o dai duong sau
    // canh nhau co the giong het nhau mot cach hoan toan hop le, nen so
    // `BaseCell` se cho ket qua chap chon tuy vao dia hinh o day.
    use mow_worldgen::noise::lattice;

    let xa: i64 = 1 << 55;
    assert_ne!(
        lattice(1, "seam", xa, 0),
        lattice(1, "seam", xa + 1, 0),
        "hai toa do ke nhau o quy mo 2^55 bam ra cung gia tri — toa do da bi cat"
    );
    assert_ne!(lattice(1, "seam", xa, 0), lattice(1, "seam", -xa, 0));

    // Va toan bo pipeline van xac dinh o quy mo do.
    let w = wg();
    assert_eq!(w.base_cell(xa, -xa).unwrap(), w.base_cell(xa, -xa).unwrap());
}

#[test]
fn dia_hinh_van_bien_thien_o_quy_mo_xa() {
    // Neu toa do bi cat ve `f64`, mot vung rong se tro nen phang li — moi o
    // trong hang trieu o se ra cung mot gia tri. Bai nay bat dieu do.
    let w = wg();
    let xa: i64 = 1 << 55;
    let mut thay = std::collections::BTreeSet::new();
    for i in 0..40i64 {
        thay.insert(w.base_cell(xa + i * 337, 0).unwrap().elevation.height_m);
    }
    assert!(
        thay.len() > 10,
        "40 diem cach xa nhau chi cho {} do cao khac nhau — dia hinh bi phang li",
        thay.len()
    );
}

#[test]
fn chunk_o_toa_do_xa_van_tinh_dung() {
    let xa: i64 = 1 << 55;
    let p = WorldPos::new(xa + 5, -(xa) - 5, 0);
    let c = p.chunk_of(SIZE).unwrap();
    let goc = c.origin(SIZE).unwrap();
    let (lx, ly) = p.local_in_chunk(SIZE).unwrap();
    assert_eq!(goc.x + lx, p.x);
    assert_eq!(goc.y + ly, p.y);
}

#[test]
fn tran_toa_do_la_loi_khong_phai_quay_vong() {
    let p = WorldPos::new(i64::MAX, 0, 0);
    assert!(p.offset(mow_math::WorldVec::new(1, 0, 0)).is_err());
}

// ── Vong doi save ────────────────────────────────────────────────────────────

fn sim_moi() -> Sim {
    Sim::new(
        SimConfig {
            world: WorldId(1),
            branch: BranchId(1),
            seed: WorldSeed(4_242),
            clock: Clock::synchronous(),
        },
        handlers(),
    )
}

/// Mot chuoi thao tac co dinh: dao, dat, tao thuc the, tien thoi gian.
fn chay_chuoi(sim: &mut Sim, kho: &mut ChunkStore) {
    kho.load(
        ChunkPos {
            cx: 0,
            cy: 0,
            cz: 0,
        },
        Lod::Active,
    );

    for i in 0..8i64 {
        kho.write_cell(WorldPos::new(i, i, 0), "material", Value::from("dug"))
            .unwrap();
    }
    kho.write_cell(WorldPos::new(3, 3, 0), "structure", Value::from("wall"))
        .unwrap();

    sim.apply(&Command::new(
        "core.spawn",
        WorldId(1),
        mow_core::val! { "kind" => "entity", "name" => "Dao" },
    ))
    .unwrap();
    sim.advance(250).unwrap();
}

#[test]
fn dao_dat_luu_nap_replay_cho_cung_hash() {
    // Day la vong lap ma toan bo he thong save dung tren: neu no do, khong
    // save nao dang tin.
    let mut sim_a = sim_moi();
    let mut kho_a = ChunkStore::new(SIZE);
    chay_chuoi(&mut sim_a, &mut kho_a);

    let mut sim_b = sim_moi();
    let mut kho_b = ChunkStore::new(SIZE);
    chay_chuoi(&mut sim_b, &mut kho_b);

    assert_eq!(sim_a.state_hash(), sim_b.state_hash(), "sim lech");
    assert_eq!(kho_a.storage_hash(), kho_b.storage_hash(), "kho chunk lech");
}

#[test]
fn nap_lai_khong_lam_phinh_save() {
    // Tai roi bo tai nhieu lan khong duoc them gi vao phan luu tru.
    let mut kho = ChunkStore::new(SIZE);
    let mut sim = sim_moi();
    chay_chuoi(&mut sim, &mut kho);
    let sau_lan_dau = kho.stored_chunks();
    let hash = kho.storage_hash();

    for i in 0..1_000i64 {
        kho.load(
            ChunkPos {
                cx: i,
                cy: i,
                cz: 0,
            },
            Lod::Near,
        );
        kho.unload(ChunkPos {
            cx: i,
            cy: i,
            cz: 0,
        });
    }

    assert_eq!(kho.stored_chunks(), sau_lan_dau);
    assert_eq!(kho.storage_hash(), hash);
}

#[test]
fn dao_roi_lap_lai_tra_the_gioi_ve_nhu_cu() {
    let mut kho = ChunkStore::new(SIZE);
    let goc = kho.storage_hash();

    let at = WorldPos::new(12, 34, 0);
    kho.write_cell(at, "material", Value::from("dug")).unwrap();
    assert_ne!(kho.storage_hash(), goc);

    kho.revert_cell(at, "material").unwrap();
    assert_eq!(
        kho.storage_hash(),
        goc,
        "the gioi da duoc dua ve nguyen trang nhung save van khac"
    );
    assert_eq!(kho.stored_chunks(), 0);
}
