//! Test diễn thế, loài xâm lấn và mầm bệnh qua cổng (`PE-12`).

use mow_eco::invasion::{assess, outbreak, Ecosystem, FoodWeb, Immunity, Virulence, NGUONG_XOA_SO};
use mow_eco::succession::{Event, Patch, Process, Stage};
use std::collections::BTreeSet;

fn tap(v: &[&str]) -> BTreeSet<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

// ───────────────────────── diễn thế theo thời gian ─────────────────────────

/// Chuỗi diễn thế đi đúng thứ tự `§9.10`.
#[test]
fn chuoi_dien_the_di_dung_thu_tu() {
    let mut s = Stage::BareGround;
    let mut duong = vec![s];
    while let Some(k) = s.next() {
        s = k;
        duong.push(s);
    }
    assert_eq!(
        duong,
        vec![
            Stage::BareGround,
            Stage::Grass,
            Stage::Shrub,
            Stage::Pioneer,
            Stage::MatureForest
        ]
    );
}

/// **Mỗi giai đoạn nuôi một tập loài khác nhau** — nếu không thì diễn thế chỉ
/// là một thanh tiến trình.
#[test]
fn moi_giai_doan_nuoi_mot_tap_loai_khac_nhau() {
    let cac = [
        Stage::BareGround,
        Stage::Grass,
        Stage::Shrub,
        Stage::Pioneer,
        Stage::MatureForest,
    ];
    for (i, a) in cac.iter().enumerate() {
        for b in &cac[i + 1..] {
            assert_ne!(
                tap(a.supports()),
                tap(b.supports()),
                "{a:?} và {b:?} nuôi cùng một tập loài"
            );
        }
    }
}

/// **Mất hàng chục tới hàng trăm năm** — không phải một mùa.
#[test]
fn rung_truong_thanh_mat_hang_tram_nam() {
    assert!(
        Stage::MatureForest.years_from_bare() >= 100,
        "rừng mọc lại nhanh thì quyết định phá rừng không có trọng lượng"
    );
    assert!(Stage::Grass.years_from_bare() < 10, "cỏ thì phủ nhanh");
}

/// Rừng bị đốt đi qua **đủ bốn giai đoạn** chứ không nhảy thẳng về rừng.
#[test]
fn rung_bi_dot_di_qua_du_bon_giai_doan() {
    let mut p = Patch::mature_forest(1);
    p.apply(&Event::Fire);
    assert_eq!(p.stage, Stage::BareGround);

    let mut chang = vec![p.stage];
    for _ in 0..200 {
        p.apply(&Event::Time { years: 1 });
        if *chang.last().unwrap() != p.stage {
            chang.push(p.stage);
        }
    }
    assert_eq!(
        chang,
        vec![
            Stage::BareGround,
            Stage::Grass,
            Stage::Shrub,
            Stage::Pioneer,
            Stage::MatureForest
        ]
    );
}

/// **Cháy khác phá rừng lấy đất**: tro là dinh dưỡng, phá thì mất lớp mặt.
#[test]
fn chay_khac_pha_rung_lay_dat() {
    let mut chay = Patch::mature_forest(1);
    let mut pha = Patch::mature_forest(2);
    chay.apply(&Event::Fire);
    pha.apply(&Event::Cleared);

    assert!(chay.soil > pha.soil);
    assert!(
        chay.processes.contains(&Process::SoilFormation),
        "cháy không giết hệ hình thành đất"
    );
    assert!(
        !pha.processes.contains(&Process::SoilFormation),
        "phá rừng lấy đất thì nén đất, hệ hình thành đất đứt"
    );
}

/// **Phá rừng làm xói mòn đất** ⇒ diễn thế chặn lại, chờ cũng không hồi.
#[test]
fn pha_rung_roi_xoi_mon_thi_cho_cung_khong_hoi() {
    let mut p = Patch::mature_forest(1);
    p.apply(&Event::Cleared);
    p.apply(&Event::Erosion { permille: 600 });

    for _ in 0..500 {
        p.apply(&Event::Time { years: 1 });
    }
    assert_ne!(
        p.stage,
        Stage::MatureForest,
        "mất hệ hình thành đất thì năm trăm năm cũng không đủ"
    );
    assert!(p.blocked_by().is_some());
}

/// Khôi phục quá trình đúng chỗ thì diễn thế chạy lại — và **nói được chỗ nào**.
#[test]
fn khoi_phuc_dung_qua_trinh_thi_dien_the_chay_lai() {
    let mut p = Patch::mature_forest(1);
    p.apply(&Event::Cleared);
    p.apply(&Event::Erosion { permille: 600 });
    let ket = p.blocked_by().unwrap();
    assert!(ket.contains("đất"), "{ket}");

    p.apply(&Event::ProcessRestored(Process::SoilFormation));
    for _ in 0..800 {
        p.apply(&Event::Time { years: 1 });
    }
    assert_eq!(p.stage, Stage::MatureForest);
}

/// **Đất là ràng buộc chặt hơn đồng hồ diễn thế.**
///
/// Chuỗi giai đoạn cộng lại chỉ 179 năm, nhưng dựng lại lớp đất đã mất thì mất
/// hàng trăm năm nữa. Nếu hai con số này bằng nhau thì xói mòn không có hậu quả
/// riêng, và cả một loại quyết định — phá rừng ở đâu, phá thế nào — mất trọng
/// lượng.
#[test]
fn dat_la_rang_buoc_chat_hon_dong_ho_dien_the() {
    let mut mat_dat = Patch::mature_forest(1);
    mat_dat.apply(&Event::Cleared);
    mat_dat.apply(&Event::Erosion { permille: 600 });
    mat_dat.apply(&Event::ProcessRestored(Process::SoilFormation));

    let mut chi_chay = Patch::mature_forest(2);
    chi_chay.apply(&Event::Fire);

    let den_rung = |p: &mut Patch| -> u32 {
        for n in 1..2_000 {
            p.apply(&Event::Time { years: 1 });
            if p.stage == Stage::MatureForest {
                return n;
            }
        }
        u32::MAX
    };
    let a = den_rung(&mut mat_dat);
    let b = den_rung(&mut chi_chay);
    assert!(
        b <= Stage::MatureForest.years_from_bare() + 1,
        "cháy: {b} năm"
    );
    assert!(a > b * 3, "mất đất {a} năm, chỉ cháy {b} năm");
}

/// **Mất phát tán hạt thì diễn thế đứng lại** — hậu quả sinh thái đọc được.
#[test]
fn mat_phat_tan_hat_thi_dien_the_dung_lai() {
    let mut p = Patch::mature_forest(1);
    p.apply(&Event::Fire);
    p.apply(&Event::ProcessLost(Process::SeedDispersal));
    for _ in 0..300 {
        p.apply(&Event::Time { years: 1 });
    }
    assert_eq!(p.stage, Stage::BareGround);
    assert_eq!(p.blocked_by(), Some(Process::SeedDispersal.failure()));
}

/// Bốn quá trình, bốn kiểu hỏng riêng — không gộp.
#[test]
fn bon_qua_trinh_bon_kieu_hong_rieng() {
    let cac = [
        Process::Pollination,
        Process::Decomposition,
        Process::SeedDispersal,
        Process::SoilFormation,
    ];
    let mo_ta: BTreeSet<&str> = cac.iter().map(|p| p.failure()).collect();
    assert_eq!(mo_ta.len(), 4);
}

/// Diễn thế **xác định**: cùng chuỗi biến cố cho cùng kết quả.
#[test]
fn dien_the_xac_dinh() {
    let chuoi = [
        Event::Fire,
        Event::Time { years: 30 },
        Event::Erosion { permille: 100 },
        Event::Time { years: 80 },
    ];
    let mut a = Patch::mature_forest(1);
    let mut b = Patch::mature_forest(1);
    for e in &chuoi {
        a.apply(e);
        b.apply(e);
    }
    assert_eq!(a, b);
}

// ───────────────────── loài xâm lấn và mầm bệnh (§9.10.1) ─────────────────────

fn tho() -> FoodWeb {
    FoodWeb {
        predators: tap(&["predator.fox", "predator.lynx"]),
        prey: tap(&["grass"]),
        competitors: tap(&["grazer.native_hare"]),
    }
}

/// **Cùng một loài, hai world, hai kết luận** — xâm lấn không phải thuộc tính
/// của loài.
#[test]
fn cung_mot_loai_hai_world_hai_ket_luan() {
    let co_cao = Ecosystem {
        present: tap(&["grass", "predator.fox"]),
        carrying_capacity: 10_000,
    };
    let khong_cao = Ecosystem {
        present: tap(&["grass"]),
        carrying_capacity: 10_000,
    };
    assert!(!assess("grazer.rabbit", &tho(), &co_cao).will_explode());
    assert!(assess("grazer.rabbit", &tho(), &khong_cao).will_explode());
}

/// Không có thức ăn thì không bùng nổ, dù cũng không có thiên địch.
#[test]
fn khong_co_thuc_an_thi_khong_bung_no() {
    let tram = Ecosystem {
        present: tap(&["rock"]),
        carrying_capacity: 10,
    };
    assert!(!assess("grazer.rabbit", &tho(), &tram).will_explode());
}

/// **Thả con gì vào thì hết bùng nổ** — câu hỏi mà một cờ `is_invasive` xóa mất.
#[test]
fn noi_duoc_tha_con_gi_vao_thi_het_bung_no() {
    let khong_cao = Ecosystem {
        present: tap(&["grass"]),
        carrying_capacity: 10_000,
    };
    let r = assess("grazer.rabbit", &tho(), &khong_cao);
    assert_eq!(
        r.missing_predators(&tho()),
        tap(&["predator.fox", "predator.lynx"])
    );
}

/// Loài mới đẩy loài bản địa cùng ổ sinh thái.
#[test]
fn loai_moi_day_loai_ban_dia_cung_o_sinh_thai() {
    let co_ban_dia = Ecosystem {
        present: tap(&["grass", "grazer.native_hare"]),
        carrying_capacity: 5_000,
    };
    let r = assess("grazer.rabbit", &tho(), &co_ban_dia);
    assert_eq!(r.competitors_displaced, tap(&["grazer.native_hare"]));
}

/// **Chưa từng phơi nhiễm là thảm họa**, không phải "miễn dịch thấp".
#[test]
fn chua_tung_phoi_nhiem_khac_han_mien_dich_thap() {
    let v = Virulence(150);
    let da_gap_mien_dich_thap = Immunity {
        pathogen: "redcough".into(),
        ever_exposed: true,
        herd_permille: 100,
    };
    let chua_tung = Immunity {
        pathogen: "redcough".into(),
        ever_exposed: false,
        herd_permille: 0,
    };

    let a = outbreak(v, &da_gap_mien_dich_thap, Some(7));
    let b = outbreak(v, &chua_tung, Some(7));
    assert!(b.mortality_permille > a.mortality_permille * 3);
    assert!(!a.civilization_ending);
    assert!(
        b.civilization_ending,
        "một mầm bệnh tầm thường ở world nguồn là thảm họa ở world đích"
    );
    assert!(b.mortality_permille >= NGUONG_XOA_SO);
}

/// Miễn dịch cộng đồng cao thì gần như không ai chết.
#[test]
fn mien_dich_cong_dong_cao_thi_gan_nhu_khong_ai_chet() {
    let o = outbreak(
        Virulence(150),
        &Immunity {
            pathogen: "redcough".into(),
            ever_exposed: true,
            herd_permille: 950,
        },
        None,
    );
    assert!(o.mortality_permille < 10, "{o:?}");
}

/// **Truy ngược được về đúng một chuyến đi qua cổng.**
#[test]
fn dich_truy_nguoc_duoc_ve_mot_chuyen_di_qua_cong() {
    let o = outbreak(
        Virulence(200),
        &Immunity {
            pathogen: "redcough".into(),
            ever_exposed: false,
            herd_permille: 0,
        },
        Some(7_741),
    );
    assert_eq!(
        o.arrived_via,
        Some(7_741),
        "§6.2 bước 8 ghi lại thứ đi cùng chính là để trả lời câu này"
    );
}
