//! Test sinh mệnh.
//!
//! Bài quan trọng nhất: [`ra_khoi_tam_nhin_khong_phai_mot_cach_bat_tu`]. Nó
//! kiểm chứng `§22.24` ở đúng chỗ mà bất biến đó tồn tại để bảo vệ.

use mow_core::{ClockDomain, Tick};
use mow_life::body::{Injury, InjuryKind};
use mow_life::{
    BodyPlan, Genome, Homeostasis, LifeStage, LifeStages, Need, SenescenceModel, Tissue, SCALE,
};
use mow_math::{CanonicalHash, DetRng, Prob, Rate, RngStreams, WorldSeed};
use rand::SeedableRng;

fn doi() -> Need {
    // Tụt hết một thanh đầy trong 100 000 tick.
    Need::full("core.hunger", Rate::new(-SCALE, 100_000).unwrap(), Tick(0))
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.24 — tích phân đóng
// ─────────────────────────────────────────────────────────────────────────────

/// **Bài quan trọng nhất.** Một thực thể rời khỏi tầm nhìn rồi quay lại phải
/// đói đúng bằng thực thể chưa từng rời đi.
#[test]
fn ra_khoi_tam_nhin_khong_phai_mot_cach_bat_tu() {
    // A ở mức Active suốt: được `settle` mỗi 100 tick.
    let mut a = doi();
    for t in (100..=50_000).step_by(100) {
        a.settle(Tick(t)).unwrap();
    }

    // B ở mức Far: không ai đụng vào nó suốt 50 000 tick.
    let mut b = doi();
    b.settle(Tick(50_000)).unwrap();

    assert_eq!(
        a.value_at(Tick(50_000)).unwrap(),
        b.value_at(Tick(50_000)).unwrap(),
        "LOD làm lệch giá trị nhu cầu — ra khỏi tầm nhìn trở thành một cách bất tử"
    );
}

#[test]
fn gia_tri_la_ham_cua_thoi_gian_khong_can_cap_nhat() {
    let n = doi();
    // Không gọi `settle` lần nào.
    assert_eq!(n.value_at(Tick(0)).unwrap(), SCALE);
    assert_eq!(n.value_at(Tick(50_000)).unwrap(), SCALE / 2);
    assert_eq!(n.value_at(Tick(100_000)).unwrap(), 0);
}

#[test]
fn khong_tut_duoi_0() {
    let n = doi();
    assert_eq!(n.value_at(Tick(1_000_000)).unwrap(), 0);
}

#[test]
fn hoi_ve_qua_khu_khong_ngoai_suy_nguoc() {
    // Ngoại suy ngược sẽ cho những con số vô lý ở biên fork nhánh.
    let mut n = doi();
    n.settle(Tick(50_000)).unwrap();
    assert_eq!(n.value_at(Tick(10)).unwrap(), n.value);
}

#[test]
fn danh_thuc_theo_nguong_thay_vi_kiem_tra_moi_tick() {
    // Đây là thứ khiến "không tick per-entity" khả thi.
    let n = doi();
    let t = n.next_threshold_tick(SCALE / 4, Tick(0)).expect("sẽ tới");

    assert!(
        n.value_at(Tick(t.0 - 1)).unwrap() > SCALE / 4,
        "đánh thức muộn"
    );
    assert!(n.value_at(t).unwrap() <= SCALE / 4, "đánh thức sớm");
}

#[test]
fn nguong_khong_bao_gio_toi_thi_khong_dat_lich() {
    let no_du = Need::full("core.hunger", Rate::ZERO, Tick(0));
    assert_eq!(no_du.next_threshold_tick(0, Tick(0)), None);
}

#[test]
fn an_uong_bo_sung_dung_luc() {
    let mut n = doi();
    n.settle(Tick(80_000)).unwrap();
    assert_eq!(n.value, 2_000);
    n.replenish(5_000, Tick(80_000)).unwrap();
    assert_eq!(n.value_at(Tick(80_000)).unwrap(), 7_000);
}

#[test]
fn moi_nhu_cau_khai_bao_mien_dong_ho() {
    // `§4.5`: thiếu miền thì cái đói nhảy sai khi qua cổng.
    assert_eq!(doi().domain, ClockDomain::Proper);
}

#[test]
fn danh_thuc_som_nhat_trong_moi_nhu_cau() {
    let mut h = Homeostasis::new();
    h.insert(Need::full(
        "core.hunger",
        Rate::new(-SCALE, 100_000).unwrap(),
        Tick(0),
    ));
    h.insert(Need::full(
        "core.sleep",
        Rate::new(-SCALE, 20_000).unwrap(),
        Tick(0),
    ));

    let nguong = vec![
        ("core.hunger".to_owned(), SCALE / 2),
        ("core.sleep".to_owned(), SCALE / 2),
    ];
    let t = h.next_wakeup(&nguong, Tick(0)).expect("có");

    // Giấc ngủ tụt nhanh hơn năm lần, nên nó chạm ngưỡng trước.
    let ngu = h.get("core.sleep").unwrap();
    assert!(
        ngu.value_at(Tick(t.0 - 1)).unwrap() > SCALE / 2,
        "đánh thức muộn"
    );
    assert!(ngu.value_at(t).unwrap() <= SCALE / 2, "đánh thức sớm");

    // Và cái đói thì chưa chạm — nên đánh thức là vì giấc ngủ.
    let doi = h.get("core.hunger").unwrap();
    assert!(doi.value_at(t).unwrap() > SCALE / 2);
}

#[test]
fn nhu_cau_sap_theo_id_de_hash_on_dinh() {
    let mk = |dao: bool| {
        let mut h = Homeostasis::new();
        let ds = ["core.thirst", "core.hunger", "core.sleep"];
        for id in if dao {
            ds.iter().rev().copied().collect::<Vec<_>>()
        } else {
            ds.to_vec()
        } {
            h.insert(Need::full(id, Rate::per_tick(-1), Tick(0)));
        }
        h.state_hash()
    };
    assert_eq!(mk(false), mk(true));
}

// ─────────────────────────────────────────────────────────────────────────────
// §9.5.2 — bộ gen nén
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bo_gen_la_24_byte_bat_ke_loai_phuc_tap_the_nao() {
    assert_eq!(std::mem::size_of::<Genome>(), 24);
}

#[test]
fn tinh_trang_la_ham_thuan_cua_seed() {
    let g = Genome::founder(12_345, 1);
    assert_eq!(g.trait_value("height"), g.trait_value("height"));
    assert_ne!(g.trait_value("height"), g.trait_value("strength"));
    // Cùng cá thể luôn có cùng chiều cao, kể cả sau save/load trên máy khác.
    assert_eq!(
        Genome::founder(12_345, 1).trait_value("height"),
        g.trait_value("height")
    );
}

#[test]
fn anh_em_ruot_khac_nhau() {
    let cha = Genome::founder(1, 1);
    let me = Genome::founder(2, 1);
    let a = Genome::breed(cha, me, 100);
    let b = Genome::breed(cha, me, 200);
    assert_ne!(a.genotype_seed, b.genotype_seed);
    assert_eq!(a.lineage, b.lineage, "anh em ruột có cùng dòng dõi");
}

#[test]
fn the_he_tang_dan() {
    let cha = Genome::founder(1, 1);
    let me = Genome::founder(2, 1);
    let con = Genome::breed(cha, me, 1);
    let chau = Genome::breed(con, Genome::founder(3, 1), 2);
    assert_eq!(con.generation, 1);
    assert_eq!(chau.generation, 2);
}

#[test]
fn seed_tai_to_hop_khong_phu_thuoc_thu_tu_xu_ly() {
    // Nếu nó phụ thuộc một bộ đếm, thứ tự các ca sinh trong một tick sẽ quyết
    // định con nào giống ai — và thứ tự đó không phải một phần của thế giới.
    let w = WorldSeed(7);
    let a = Genome::founder(11, 1);
    let b = Genome::founder(22, 1);
    let s1 = mow_life::recombination_seed(w, a, b, 500);
    // Một ca sinh khác chen vào giữa.
    let _ = mow_life::recombination_seed(w, Genome::founder(33, 1), b, 500);
    let s2 = mow_life::recombination_seed(w, a, b, 500);
    assert_eq!(s1, s2);
}

#[test]
fn dot_bien_xay_ra_o_quy_mo_quan_the() {
    // Nối tiếp bài `mutation_rate_khong_bi_lam_tron_ve_0` của `mow-math`: ở đây
    // ta kiểm rằng tỉ lệ đó thật sự chạm tới bộ gen.
    let rate = Prob::from_sci(21, 9).unwrap();
    let mut rng: DetRng = DetRng::seed_from_u64(42);
    let goc = Genome::founder(1, 1);

    let mut so_doi = 0;
    for _ in 0..2_000 {
        // 20 000 locus × 200 thế hệ cho mỗi cá thể.
        if goc.mutate(rate, 20_000 * 200, &mut rng).genotype_seed != goc.genotype_seed {
            so_doi += 1;
        }
    }
    assert!(
        so_doi > 0,
        "đột biến không bao giờ xảy ra — tỉ lệ đã bị làm tròn về 0"
    );
    assert!(
        so_doi < 2_000,
        "đột biến xảy ra ở MỌI cá thể — tỉ lệ bị thổi phồng"
    );
}

#[test]
fn khong_dot_bien_thi_bo_gen_nguyen_ven() {
    let mut rng: DetRng = DetRng::seed_from_u64(1);
    let g = Genome::founder(5, 1);
    assert_eq!(g.mutate(Prob::NEVER, 1_000_000, &mut rng), g);
}

#[test]
fn dong_doi_gan_thi_tuong_dong_cao() {
    let cha = Genome::founder(0xAAAA_AAAA_AAAA_AAAA, 1);
    let me = Genome::founder(0xAAAA_AAAA_AAAA_AAAB, 1);
    let anh = Genome::breed(cha, me, 1);
    let em = Genome::breed(cha, me, 2);
    let nguoi_la = Genome::founder(0x1234_5678_9ABC_DEF0, 1);

    assert!(
        anh.lineage_similarity(em) > anh.lineage_similarity(nguoi_la),
        "anh em phải giống nhau hơn người lạ"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §9.5.6 — lão hóa
// ─────────────────────────────────────────────────────────────────────────────

fn nguoi() -> SenescenceModel {
    SenescenceModel::Gompertz {
        baseline_ppm_per_year: 200,
        doubling_years: 8,
    }
}

#[test]
fn gompertz_nguy_co_tang_theo_ham_mu() {
    let m = nguoi();
    let a = m.annual_mortality(20);
    let b = m.annual_mortality(28);
    let c = m.annual_mortality(36);
    assert!(b > a);
    assert!(c > b);
    // Mỗi 8 năm nhân đôi: khoảng cách phải tăng dần, không tuyến tính.
    assert!(
        c.raw() - b.raw() > b.raw() - a.raw(),
        "tăng tuyến tính, không phải hàm mũ"
    );
}

#[test]
fn lao_hoa_khong_dang_ke_thi_nguy_co_khong_doi() {
    let m = SenescenceModel::Negligible { annual_ppm: 500 };
    assert_eq!(m.annual_mortality(10), m.annual_mortality(500));
    assert!(m.is_negligible());
}

#[test]
fn khong_loai_nao_bat_tu_tuyet_doi() {
    // Một loài bất tử tuyệt đối sẽ tích lũy vô hạn và nuốt cả thế giới.
    let m = SenescenceModel::Negligible { annual_ppm: 500 };
    assert!(m.annual_mortality(0) > Prob::NEVER);
}

#[test]
fn tuoi_tho_ky_vong_tinh_duoc() {
    // 20% nguy cơ mỗi năm là mốc "sắp hết".
    let nguong = Prob::from_ppm(200_000).unwrap();
    let tuoi = nguoi().age_at_risk(nguong).expect("người thì có");
    assert!(
        (60..120).contains(&tuoi),
        "tuổi thọ kỳ vọng {tuoi} không hợp lý"
    );

    // Loài lão hóa không đáng kể không bao giờ tới ngưỡng đó.
    let tien = SenescenceModel::Negligible { annual_ppm: 500 };
    assert_eq!(tien.age_at_risk(nguong), None);
}

#[test]
fn lao_hoa_tac_dong_qua_effect_khong_ghi_thang_chi_so() {
    // `§22.20`: effect chỉ tác động qua modifier pipeline.
    let s = LifeStages {
        maturity_years: 18,
        elder_years: 60,
    };
    assert_eq!(
        mow_life::senescence_effect(s, 10),
        Some("core.effect.immature")
    );
    assert_eq!(mow_life::senescence_effect(s, 30), None);
    assert_eq!(mow_life::senescence_effect(s, 70), Some("core.effect.aged"));
}

#[test]
fn giai_doan_doi_theo_moc_cua_loai() {
    let s = LifeStages {
        maturity_years: 18,
        elder_years: 60,
    };
    assert_eq!(s.stage_at(17), LifeStage::Juvenile);
    assert_eq!(s.stage_at(18), LifeStage::Adult);
    assert_eq!(s.stage_at(60), LifeStage::Elder);
}

// ─────────────────────────────────────────────────────────────────────────────
// §9.4 — thương tích theo bộ phận
// ─────────────────────────────────────────────────────────────────────────────

fn thuong(kind: InjuryKind, severity: u8) -> Injury {
    Injury {
        kind,
        severity,
        at_tick: 0,
        infected: false,
    }
}

#[test]
fn vitality_la_suy_ra_khong_phai_luu() {
    // Nếu lưu, sẽ có hai nguồn sự thật và chúng sẽ lệch nhau.
    let mut b = BodyPlan::humanoid();
    assert_eq!(b.vitality(), 100);
    b.injure("core.arm_left", &thuong(InjuryKind::Cut, 50));
    assert!(b.vitality() < 100);
}

#[test]
fn mat_mot_tay_giam_mot_nua_kha_nang_cam_nam_chu_khong_mat_han() {
    let mut b = BodyPlan::humanoid();
    assert_eq!(b.function_level("manipulation"), 100);
    b.injure("core.arm_left", &thuong(InjuryKind::Severed, 100));
    assert_eq!(
        b.function_level("manipulation"),
        50,
        "mất một tay phải giảm một nửa, không phải mất hẳn"
    );
}

#[test]
fn chat_canh_tay_thi_mat_luon_ban_tay() {
    let mut b = BodyPlan::humanoid();
    b.injure("core.arm_right", &thuong(InjuryKind::Severed, 100));
    assert!(
        b.part("core.hand_right").unwrap().is_severed(),
        "bàn tay phải mất theo"
    );
    assert!(
        !b.part("core.hand_left").unwrap().is_severed(),
        "tay kia không được ảnh hưởng"
    );
}

#[test]
fn mat_mau_giet_nguoi_ma_khong_bo_phan_nao_bi_pha_huy() {
    let mut b = BodyPlan::humanoid();
    b.blood = 0;
    assert!(b.is_dead());
    assert!(
        b.parts().all(|p| !p.is_severed()),
        "không bộ phận nào mất mà vẫn chết"
    );
}

#[test]
fn dau_lam_ngat_chu_khong_giet() {
    let mut b = BodyPlan::humanoid();
    b.pain = 90;
    assert!(b.is_unconscious());
    assert!(!b.is_dead(), "đau không giết ai");
}

#[test]
fn mat_bo_phan_sinh_tu_la_chet_ngay() {
    let mut b = BodyPlan::humanoid();
    b.injure("core.heart", &thuong(InjuryKind::Pierce, 100));
    assert!(!b.is_dead(), "đâm thủng chưa phải mất hẳn");
    b.injure("core.heart", &thuong(InjuryKind::Severed, 100));
    assert!(b.is_dead());
}

#[test]
fn vet_thuong_nhiem_trung_la_trang_thai_rieng_khong_phai_muc_do_nang_hon() {
    // Một vết xước nhiễm trùng giết người, còn một vết chém sạch thì lành.
    let mut b = BodyPlan::humanoid();
    b.injure(
        "core.leg_left",
        &Injury {
            kind: InjuryKind::Cut,
            severity: 5,
            at_tick: 0,
            infected: true,
        },
    );
    let p = b.part("core.leg_left").unwrap();
    assert!(p.injuries[0].infected);
    assert_eq!(
        p.efficiency(),
        95,
        "nhiễm trùng không tự nó làm giảm hiệu quả"
    );
}

#[test]
fn xuong_lanh_cham_hon_da_nhieu_lan() {
    assert!(Tissue::Bone.heal_ticks() > Tissue::Skin.heal_ticks() * 10);
    assert!(Tissue::Nerve.heal_ticks() > Tissue::Organ.heal_ticks() * 10);
}

#[test]
fn chi_mo_co_mach_moi_chay_mau() {
    assert!(Tissue::Skin.bleeds());
    assert!(!Tissue::Bone.bleeds());
    let mut b = BodyPlan::humanoid();
    b.injure("core.head", &thuong(InjuryKind::Cut, 60));
    assert!(!b.is_bleeding(), "xương sọ không phải mô chảy máu");
    b.injure("core.arm_left", &thuong(InjuryKind::Cut, 60));
    assert!(b.is_bleeding());
}

#[test]
fn giai_phau_khac_thi_cho_mac_khac() {
    // `PB-21` dựa vào điều này: một loài bốn tay tự nhiên có bốn chỗ đeo găng
    // mà không cần sửa engine.
    let bon_tay = BodyPlan::new(
        ["a", "b", "c", "d"]
            .iter()
            .map(|s| mow_life::BodyPart {
                id: format!("mypack.arm_{s}"),
                tissue: Tissue::Muscle,
                functions: vec![("manipulation".to_owned(), 25)],
                vital: false,
                injuries: Vec::new(),
                parent: None,
            })
            .collect(),
    );
    assert_eq!(bon_tay.function_level("manipulation"), 100);
    assert_eq!(bon_tay.parts().count(), 4);
}

#[test]
fn cay_bo_phan_hong_khong_lam_tran_ngan_xep() {
    // Một cây tự trỏ vào mình đến từ save hỏng hoặc migration sai.
    let mut b = BodyPlan::new(vec![mow_life::BodyPart {
        id: "x.a".to_owned(),
        tissue: Tissue::Muscle,
        functions: vec![],
        vital: false,
        injuries: Vec::new(),
        parent: Some("x.a".to_owned()),
    }]);
    b.injure("x.a", &thuong(InjuryKind::Severed, 100));
    assert!(b.part("x.a").unwrap().is_severed());
}

#[test]
fn vitality_lay_gia_tri_nho_nhat_khong_phai_trung_binh() {
    // Một người mất 90% máu đang hấp hối; trung bình hóa sẽ vẽ ra một thanh máu
    // đầy ba phần tư ngay trước khi họ chết.
    let mut b = BodyPlan::humanoid();
    b.blood = 10;
    assert_eq!(b.vitality(), 10);
}

#[test]
fn rng_stream_cua_lao_hoa_tach_khoi_dot_bien() {
    let s = RngStreams::new(WorldSeed(1));
    use rand::Rng;
    let mut a = s.stream(mow_math::rng::streams::LIFE_MORTALITY);
    let mut b = s.stream(mow_math::rng::streams::LIFE_MUTATION);
    let va: u64 = a.gen();
    let vb: u64 = b.gen();
    assert_ne!(va, vb);
}
