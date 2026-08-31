//! Test khong gian.
//!
//! Bai quan trong nhat la `di_bo_ngan_gio_khong_lam_save_phinh` — do la
//! `§22.12`, va cach no bi vi pham thuong khong phai la luu het cac o ma la
//! luu mot ban ghi rang chunk da duoc ghe qua.

use mow_core::{EntityId, Value};
use mow_math::{ChunkPos, WorldPos};
use mow_spatial::{ChunkStore, Lod, Occupancy};

const SIZE: i64 = 32;

// ── §22.12 ───────────────────────────────────────────────────────────────────

#[test]
fn di_bo_ngan_gio_khong_lam_save_phinh() {
    let mut s = ChunkStore::new(SIZE);
    // Di qua 100 000 chunk ma khong doi gi ca.
    for i in 0..100_000i64 {
        s.load(
            ChunkPos {
                cx: i,
                cy: 0,
                cz: 0,
            },
            Lod::Active,
        );
        s.unload(ChunkPos {
            cx: i,
            cy: 0,
            cz: 0,
        });
    }
    assert_eq!(
        s.stored_chunks(),
        0,
        "di ngang qua chunk lam no lot vao save — §22.12 bi vi pham"
    );
    assert_eq!(s.resident_chunks(), 0);
}

#[test]
fn chi_chunk_da_doi_moi_vao_save() {
    let mut s = ChunkStore::new(SIZE);
    for i in 0..1_000i64 {
        s.load(
            ChunkPos {
                cx: i,
                cy: 0,
                cz: 0,
            },
            Lod::Near,
        );
    }
    assert_eq!(s.stored_chunks(), 0);

    s.write_cell(WorldPos::new(5, 5, 0), "material", Value::from("dirt"))
        .unwrap();
    assert_eq!(s.stored_chunks(), 1, "dao mot o phai tao dung mot chunk");
}

#[test]
fn hoan_nguyen_o_thi_chunk_bien_mat_khoi_save() {
    // Lap day roi hoan nguyen phai tra ve dung trang thai ban dau — neu khong,
    // mot the gioi da duoc dua ve nguyen trang van chiem cho mai mai.
    let mut s = ChunkStore::new(SIZE);
    let at = WorldPos::new(10, 20, 0);
    s.write_cell(at, "material", Value::from("stone")).unwrap();
    assert_eq!(s.stored_chunks(), 1);

    s.revert_cell(at, "material").unwrap();
    assert_eq!(s.stored_chunks(), 0, "chunk rong van con trong save");
    assert!(s.read_cell(at, "material").is_none());
}

#[test]
fn trang_thai_tai_khong_lot_vao_hash() {
    // Neu no lot vao, hai nguoi choi cung mot save nhung dung o hai cho se co
    // hai hash khac nhau, va determinism harness se do o moi lan chay.
    let mut a = ChunkStore::new(SIZE);
    let mut b = ChunkStore::new(SIZE);
    for st in [&mut a, &mut b] {
        st.write_cell(WorldPos::new(1, 1, 0), "m", Value::from("x"))
            .unwrap();
    }
    a.load(
        ChunkPos {
            cx: 50,
            cy: 50,
            cz: 0,
        },
        Lod::Active,
    );
    b.load(
        ChunkPos {
            cx: -50,
            cy: -50,
            cz: 0,
        },
        Lod::Far,
    );

    assert_eq!(a.storage_hash(), b.storage_hash());
}

#[test]
fn bo_tai_khong_lam_mat_delta() {
    let mut s = ChunkStore::new(SIZE);
    let at = WorldPos::new(3, 4, 0);
    s.load(at.chunk_of(SIZE).unwrap(), Lod::Active);
    s.write_cell(at, "m", Value::from("dug")).unwrap();
    s.unload(at.chunk_of(SIZE).unwrap());

    assert_eq!(
        s.read_cell(at, "m"),
        Some(&Value::from("dug")),
        "delta la luu tru, khong phai bo nho"
    );
}

#[test]
fn ghi_o_am_vao_dung_chunk() {
    // `div_euclid` chu khong phai `/`: neu sai, o -1 va o 0 se roi vao cung
    // chunk con o -32 thi khong, va lech mot o quanh goc se khong ai thay.
    let mut s = ChunkStore::new(SIZE);
    s.write_cell(WorldPos::new(-1, -1, 0), "m", Value::from("a"))
        .unwrap();
    s.write_cell(WorldPos::new(0, 0, 0), "m", Value::from("b"))
        .unwrap();
    assert_eq!(
        s.stored_chunks(),
        2,
        "hai o nay phai thuoc hai chunk khac nhau"
    );
    assert_eq!(
        s.read_cell(WorldPos::new(-1, -1, 0), "m"),
        Some(&Value::from("a"))
    );
    assert_eq!(
        s.read_cell(WorldPos::new(0, 0, 0), "m"),
        Some(&Value::from("b"))
    );
}

#[test]
fn hash_doi_khi_delta_doi() {
    let mut s = ChunkStore::new(SIZE);
    let h0 = s.storage_hash();
    s.write_cell(WorldPos::new(1, 1, 0), "m", Value::from("x"))
        .unwrap();
    let h1 = s.storage_hash();
    assert_ne!(h0, h1);
    s.revert_cell(WorldPos::new(1, 1, 0), "m").unwrap();
    assert_eq!(s.storage_hash(), h0, "hoan nguyen phai tra hash ve nhu cu");
}

// ── Occupancy ────────────────────────────────────────────────────────────────

#[test]
fn di_chuyen_khong_de_lai_ban_sao() {
    let mut o = Occupancy::new();
    let e = EntityId(1);
    o.place(e, WorldPos::new(0, 0, 0));
    o.place(e, WorldPos::new(5, 5, 0));

    assert!(
        !o.is_occupied(WorldPos::new(0, 0, 0)),
        "thuc the con o cho cu"
    );
    assert_eq!(o.at(WorldPos::new(5, 5, 0)).collect::<Vec<_>>(), vec![e]);
    assert_eq!(o.len(), 1);
}

#[test]
fn nhieu_thuc_the_cung_mot_o() {
    let mut o = Occupancy::new();
    for i in 1..=3u64 {
        o.place(EntityId(i), WorldPos::new(2, 2, 0));
    }
    let ds: Vec<_> = o.at(WorldPos::new(2, 2, 0)).collect();
    assert_eq!(ds.len(), 3);
    // Thu tu on dinh theo dinh danh.
    assert_eq!(ds, vec![EntityId(1), EntityId(2), EntityId(3)]);
}

#[test]
fn truy_van_tam_nhin_cho_ket_qua_on_dinh() {
    let mut o = Occupancy::new();
    // Dat theo thu tu lon xon.
    for (i, (x, y)) in [(9i64, 9i64), (1, 1), (-3, 2), (0, 0), (4, -4)]
        .iter()
        .enumerate()
    {
        o.place(EntityId(i as u64 + 1), WorldPos::new(*x, *y, 0));
    }
    let a = o.in_range(WorldPos::new(0, 0, 0), 5);
    let b = o.in_range(WorldPos::new(0, 0, 0), 5);
    assert_eq!(a, b);
    assert!(!a.contains(&EntityId(1)), "(9,9) nam ngoai ban kinh 5");
    assert!(a.contains(&EntityId(4)), "(0,0) phai trong ban kinh");
}

#[test]
fn tang_khac_nhau_khong_thay_nhau() {
    let mut o = Occupancy::new();
    o.place(EntityId(1), WorldPos::new(0, 0, 0));
    o.place(EntityId(2), WorldPos::new(0, 0, 5));
    let ds = o.in_range(WorldPos::new(0, 0, 0), 10);
    assert_eq!(ds, vec![EntityId(1)], "tang khac phai vo hinh");
}

#[test]
fn go_thuc_the_thi_don_sach_o() {
    let mut o = Occupancy::new();
    o.place(EntityId(1), WorldPos::new(7, 7, 0));
    o.remove(EntityId(1));
    assert!(o.is_empty());
    assert!(!o.is_occupied(WorldPos::new(7, 7, 0)));
    assert_eq!(o.position_of(EntityId(1)), None);
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.14 — chuyen LOD khong lam mat gi
// ─────────────────────────────────────────────────────────────────────────────

use mow_spatial::lod::{
    lod_for_distance, relative_cost, transition, Aggregate, Conserved, LodError,
};
use std::collections::BTreeSet;

fn lang() -> Conserved {
    Conserved {
        population: 240,
        casualties: 12,
        resources: 5_000,
        relationships: 890,
        projects: 3,
        knowledge: 47,
    }
}

#[test]
fn chuyen_muc_bao_toan_thi_duoc() {
    let mut a = Aggregate::new(Lod::Active, lang());
    let ai_con = (1..=240u64).collect::<BTreeSet<_>>();
    transition(&mut a, Lod::Far, lang(), &ai_con).expect("bao toan thi phai duoc");
    assert_eq!(a.lod, Lod::Far);
}

#[test]
fn mat_dan_so_khi_ha_lod_la_bug_cua_engine() {
    let mut a = Aggregate::new(Lod::Active, lang());
    let mut sau = lang();
    sau.population -= 5;

    let e = transition(&mut a, Lod::Far, sau, &BTreeSet::new()).expect_err("phai tu choi");
    let s = e.to_string();
    assert!(s.contains("population"), "{s}");
    assert!(
        s.contains("bug cua engine") || s.contains("bug của engine"),
        "{s}"
    );
    assert_eq!(
        a.lod,
        Lod::Active,
        "chuyen muc that bai ma van doi trang thai"
    );
}

#[test]
fn nguoi_chet_cung_phai_bao_toan() {
    // Bo ho ra khoi phep dem se khien mot tran dich trong nhu the dan so "boc
    // hoi", va phep kiem bao toan se bao dong gia sau moi tham hoa.
    let mut a = Aggregate::new(Lod::Active, lang());
    let mut sau = lang();
    sau.casualties = 0;
    assert!(transition(&mut a, Lod::Far, sau, &BTreeSet::new()).is_err());
}

#[test]
fn moi_dai_luong_deu_duoc_kiem() {
    let goc = lang();
    for (ten, sua) in [
        (
            "population",
            Conserved {
                population: 1,
                ..goc
            },
        ),
        (
            "casualties",
            Conserved {
                casualties: 1,
                ..goc
            },
        ),
        (
            "resources",
            Conserved {
                resources: 1,
                ..goc
            },
        ),
        (
            "relationships",
            Conserved {
                relationships: 1,
                ..goc
            },
        ),
        ("projects", Conserved { projects: 1, ..goc }),
        (
            "knowledge",
            Conserved {
                knowledge: 1,
                ..goc
            },
        ),
    ] {
        let a = Aggregate::new(Lod::Active, goc);
        let lech = a.verify_against(&sua);
        assert_eq!(lech.len(), 1, "{ten} khong duoc kiem");
        assert_eq!(lech[0].quantity, ten);
    }
}

#[test]
fn thuc_the_ghim_khong_bi_gop() {
    // Mot vi vua khong duoc bien thanh "mot phan cua dan so 10 000".
    let mut a = Aggregate::new(Lod::Active, lang());
    a.pin(7);
    assert!(a.is_pinned(7));

    let khong_co_vua: BTreeSet<u64> = (1..=6).collect();
    let e = transition(&mut a, Lod::Far, lang(), &khong_co_vua).expect_err("phai tu choi");
    assert_eq!(e, LodError::PinnedLost(7));
}

#[test]
fn ghim_roi_bo_ghim_duoc() {
    let mut a = Aggregate::new(Lod::Far, lang());
    a.pin(7);
    assert!(a.unpin(7));
    assert!(!a.unpin(7));
    transition(&mut a, Lod::Active, lang(), &BTreeSet::new()).expect("khong con ai ghim");
}

#[test]
fn gop_hai_vung_thi_cong_don_dai_luong() {
    let a = lang();
    let b = Conserved {
        population: 10,
        casualties: 1,
        resources: 100,
        relationships: 5,
        projects: 0,
        knowledge: 2,
    };
    let c = a.merge(b);
    assert_eq!(c.population, 250);
    assert_eq!(c.knowledge, 49);
}

#[test]
fn muc_chi_tiet_theo_khoang_cach_toi_tieu_diem() {
    // `§8.4`: doi camera KHONG doi muc mo phong.
    assert_eq!(lod_for_distance(0, 3), Lod::Active);
    assert_eq!(lod_for_distance(3, 3), Lod::Active);
    assert_eq!(lod_for_distance(4, 3), Lod::Near);
    assert_eq!(lod_for_distance(12, 3), Lod::Near);
    assert_eq!(lod_for_distance(13, 3), Lod::Far);
}

#[test]
fn far_phai_re_hon_active_vai_bac_do_lon() {
    // Neu khong, LOD khong giai quyet duoc gi.
    assert!(relative_cost(Lod::Active) >= relative_cost(Lod::Far) * 500);
    assert!(relative_cost(Lod::Near) < relative_cost(Lod::Active));
}
