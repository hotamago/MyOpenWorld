//! Test soak và World Health Report (`PF-10`, `§P7.7`, `§P8.1`).

use mow_devtool::soak::{
    health_report, Explanations, HealthReport, MemoryTrace, Sample, SoakRun, DAO_DONG_CHO_PHEP,
    NAM_KHOI_DONG, SO_NAM, SO_WORLD,
};
use std::collections::BTreeMap;

fn mau(year: u32) -> Sample {
    Sample {
        year,
        population: 12_000,
        price_index: 1_000,
        money_supply: 500_000,
        knowledge_nodes: 240,
        events_per_day: 900,
        active_region_permille: 120,
        species_population: BTreeMap::from([("deer".to_owned(), 4_000), ("wolf".to_owned(), 300)]),
        rss_mb: 800,
        live_objects: 1_200_000,
        save_bytes: 40_000_000,
        events_total: 2_000_000,
        tick_p99_ms: 22,
        invariant_violations: 0,
        leaked_entities: 0,
    }
}

/// Một lần chạy khỏe mạnh: RAM phẳng sau khởi động.
fn chuoi_khoe() -> Vec<Sample> {
    (0..=SO_NAM)
        .step_by(10)
        .map(|y| Sample {
            rss_mb: if y <= NAM_KHOI_DONG {
                400 + u64::from(y) * 8
            } else {
                // Phẳng, có nhấp nhô nhẹ.
                800 + u64::from(y % 30)
            },
            ..mau(y)
        })
        .collect()
}

fn khong_giai_thich() -> Explanations {
    Explanations::default()
}

// ════════════════ rò rỉ đo bằng mặt bằng, không bằng độ dốc ════════════════

/// Chuỗi RAM phẳng sau khởi động thì đạt.
#[test]
fn ram_phang_sau_khoi_dong_thi_dat() {
    let r = health_report("gaia", &chuoi_khoe(), &khong_giai_thich()).unwrap();
    assert!(r.memory.has_plateaued());
    assert!(r.healthy(), "{:?}", r.warnings);
}

/// **Rò rỉ chậm vẫn là rò rỉ.**
///
/// Đây là bài trung tâm của `PF-10`: một trần dạng *"tăng dưới 50 MB mỗi
/// năm"* sẽ **cho qua** chuỗi này, và cho phép world 200 năm phình 10 GB.
#[test]
fn ro_ri_cham_van_bi_bat() {
    let ro: Vec<Sample> = (0..=SO_NAM)
        .step_by(10)
        .map(|y| Sample {
            // Chỉ 4 MB mỗi năm — dưới mọi trần "mỗi năm" hợp lý.
            rss_mb: 400 + u64::from(y) * 4,
            ..mau(y)
        })
        .collect();

    let tang_moi_nam = 4;
    assert!(tang_moi_nam < 50, "dưới trần 'mỗi năm' thường gặp");

    let r = health_report("gaia", &ro, &khong_giai_thich()).unwrap();
    assert!(!r.memory.has_plateaued());
    assert!(
        r.warnings.iter().any(|w| w.code == "memory.no_plateau"),
        "{:?}",
        r.warnings
    );
    assert!(!r.healthy());
}

/// **Nhấp nhô quanh mặt bằng không phải rò rỉ.**
///
/// RAM thật luôn dao động theo chunk đang giữ. Báo động ở đây sẽ làm cả cổng
/// này bị tắt trong một tuần.
#[test]
fn nhap_nho_quanh_mat_bang_khong_phai_ro_ri() {
    let nhap_nho: Vec<Sample> = (0..=SO_NAM)
        .step_by(10)
        .map(|y| Sample {
            rss_mb: 800 + u64::from(y % 7) * 10,
            ..mau(y)
        })
        .collect();
    assert!(MemoryTrace {
        rss_by_year: nhap_nho.iter().map(|s| (s.year, s.rss_mb)).collect(),
    }
    .has_plateaued());
}

/// RAM tăng **trong** giai đoạn khởi động là bình thường.
#[test]
fn ram_tang_trong_giai_doan_khoi_dong_la_binh_thuong() {
    let r = health_report("gaia", &chuoi_khoe(), &khong_giai_thich()).unwrap();
    let khoi_dong: Vec<u64> = r
        .memory
        .rss_by_year
        .iter()
        .filter(|(y, _)| *y <= NAM_KHOI_DONG)
        .map(|(_, m)| *m)
        .collect();
    assert!(khoi_dong.last() > khoi_dong.first(), "có tăng thật");
    assert!(r.memory.has_plateaued(), "nhưng không bị coi là rò");
}

/// **Chưa đủ mẫu thì chưa kết luận được** — và chưa kết luận khác với đã đạt.
#[test]
fn chua_du_mau_thi_chua_ket_luan_duoc() {
    let it = MemoryTrace {
        rss_by_year: vec![(60, 800), (70, 800)],
    };
    assert!(!it.has_plateaued());
}

/// Ngưỡng dao động có tên và kiểm được.
#[test]
fn nguong_dao_dong_co_ten_va_kiem_duoc() {
    let vua_du = MemoryTrace {
        rss_by_year: vec![(60, 1_000), (70, 1_000), (80, 1_140), (90, 1_140)],
    };
    // 140/1000 = 140‰ ≤ 150‰.
    assert_eq!(DAO_DONG_CHO_PHEP, 150);
    assert!(vua_du.has_plateaued());

    let vua_qua = MemoryTrace {
        rss_by_year: vec![(60, 1_000), (70, 1_000), (80, 1_200), (90, 1_200)],
    };
    assert!(!vua_qua.has_plateaued());
}

// ════════════════ cảnh báo nói triệu chứng ════════════════

/// **"Lạm phát không giải thích được"** — chữ *không giải thích được* là mấu chốt.
#[test]
fn lam_phat_khong_giai_thich_duoc_la_canh_bao() {
    let mut ch = chuoi_khoe();
    ch.last_mut().unwrap().price_index = 3_000; // giá gấp ba

    let r = health_report("gaia", &ch, &khong_giai_thich()).unwrap();
    let w = r
        .warnings
        .iter()
        .find(|w| w.code == "economy.unexplained_inflation")
        .expect("phải cảnh báo");
    assert!(w.blocking);
    assert!(
        w.symptom.contains("không truy được nguyên nhân"),
        "{}",
        w.symptom
    );
}

/// **Lạm phát có nguyên nhân truy được thì không phải cảnh báo.**
///
/// Đây là chỗ phân biệt một cổng có ích với một cổng gây phiền: một đợt lạm
/// phát sau khi mở mỏ bạc là mô phỏng đang chạy đúng.
#[test]
fn lam_phat_co_nguyen_nhan_thi_khong_phai_canh_bao() {
    let mut ch = chuoi_khoe();
    ch.last_mut().unwrap().price_index = 3_000;

    let co_ly_do = Explanations {
        inflation_causes: vec!["mỏ bạc Kesh mở năm 84".to_owned()],
        ..Explanations::default()
    };
    let r = health_report("gaia", &ch, &co_ly_do).unwrap();
    assert!(
        !r.warnings
            .iter()
            .any(|w| w.code == "economy.unexplained_inflation"),
        "{:?}",
        r.warnings
    );
    assert!(r.healthy());
}

/// **"Quần thể loài X sụp"** — và cảnh báo gọi đúng tên loài.
#[test]
fn quan_the_loai_sup_duoc_bao_dung_ten() {
    let mut ch = chuoi_khoe();
    ch.last_mut()
        .unwrap()
        .species_population
        .insert("wolf".to_owned(), 10);

    let r = health_report("gaia", &ch, &khong_giai_thich()).unwrap();
    let w = r
        .warnings
        .iter()
        .find(|w| w.code == "ecology.population_collapse")
        .expect("phải cảnh báo");
    assert!(w.symptom.contains("wolf"), "{}", w.symptom);
    assert!(w.blocking);
}

/// Quần thể sụp **có nguyên nhân** thì vẫn báo, nhưng không chặn.
///
/// Vẫn báo vì người đọc cần biết; không chặn vì săn hết sói là một chuyện có
/// thể xảy ra trong một thế giới chạy đúng.
#[test]
fn quan_the_sup_co_nguyen_nhan_thi_van_bao_nhung_khong_chan() {
    let mut ch = chuoi_khoe();
    ch.last_mut()
        .unwrap()
        .species_population
        .insert("wolf".to_owned(), 10);

    let co_ly_do = Explanations {
        population_causes: BTreeMap::from([(
            "wolf".to_owned(),
            "chính sách thưởng săn sói của Veskar từ năm 60".to_owned(),
        )]),
        ..Explanations::default()
    };
    let r = health_report("gaia", &ch, &co_ly_do).unwrap();
    let w = r
        .warnings
        .iter()
        .find(|w| w.code == "ecology.population_collapse")
        .expect("vẫn báo");
    assert!(!w.blocking, "nhưng không chặn");
    assert!(w.symptom.contains("Veskar"), "{}", w.symptom);
    assert!(r.healthy());
}

/// Vi phạm bất biến **luôn** chặn.
#[test]
fn vi_pham_bat_bien_luon_chan() {
    let mut ch = chuoi_khoe();
    ch.last_mut().unwrap().invariant_violations = 1;
    let r = health_report("gaia", &ch, &khong_giai_thich()).unwrap();
    assert!(!r.healthy());
    assert!(r.warnings.iter().any(|w| w.code == "invariant.violated"));
}

/// Rò entity chặn — chúng vẫn ăn ngân sách mỗi tick.
#[test]
fn ro_entity_chan() {
    let mut ch = chuoi_khoe();
    ch.last_mut().unwrap().leaked_entities = 42;
    let r = health_report("gaia", &ch, &khong_giai_thich()).unwrap();
    assert!(r.warnings.iter().any(|w| w.code == "entity.leaked"));
}

/// **Thế giới đứng im** cũng là một lỗi — và một lỗi rất dễ trôi qua.
#[test]
fn the_gioi_dung_im_cung_la_mot_loi() {
    let mut ch = chuoi_khoe();
    ch.last_mut().unwrap().events_per_day = 0;
    let r = health_report("gaia", &ch, &khong_giai_thich()).unwrap();
    assert!(r.warnings.iter().any(|w| w.code == "world.stalled"));
}

/// Mỗi cảnh báo nói được **triệu chứng**, không chỉ một con số.
#[test]
fn moi_canh_bao_noi_duoc_trieu_chung() {
    let mut ch = chuoi_khoe();
    let cuoi = ch.last_mut().unwrap();
    cuoi.invariant_violations = 2;
    cuoi.leaked_entities = 5;
    cuoi.events_per_day = 0;

    let r = health_report("gaia", &ch, &khong_giai_thich()).unwrap();
    assert!(r.warnings.len() >= 3);
    for w in &r.warnings {
        assert!(
            w.symptom.len() > 20,
            "cảnh báo `{}` chỉ có con số, không có triệu chứng: {}",
            w.code,
            w.symptom
        );
    }
}

// ════════════════ đo save theo event ════════════════

/// Save đo theo **byte trên mỗi event**, không theo tổng (`§P8.1`).
#[test]
fn save_do_theo_byte_moi_event() {
    let r = health_report("gaia", &chuoi_khoe(), &khong_giai_thich()).unwrap();
    assert_eq!(r.bytes_per_event(), 20);
}

/// Không có event nào thì không chia — không panic.
#[test]
fn khong_co_event_thi_khong_chia() {
    let mut ch = chuoi_khoe();
    ch.last_mut().unwrap().events_total = 0;
    let r = health_report("gaia", &ch, &khong_giai_thich()).unwrap();
    assert_eq!(r.bytes_per_event(), 0);
}

// ════════════════ ba world ════════════════

fn bao_cao(world: &str) -> HealthReport {
    health_report(world, &chuoi_khoe(), &khong_giai_thich()).unwrap()
}

/// Ba world đều xanh thì đêm xanh.
#[test]
fn ba_world_deu_xanh_thi_dem_xanh() {
    let dem = SoakRun {
        reports: vec![bao_cao("gaia"), bao_cao("abyss"), bao_cao("celestia")],
    };
    assert_eq!(dem.reports.len(), SO_WORLD);
    assert!(dem.passed());
    assert!(dem.blockers().is_empty());
}

/// **Một world đỏ là cả đêm đỏ** — trung bình ba world sẽ giấu đúng cái hỏng.
#[test]
fn mot_world_do_la_ca_dem_do() {
    let mut hong = chuoi_khoe();
    hong.last_mut().unwrap().invariant_violations = 1;

    let dem = SoakRun {
        reports: vec![
            bao_cao("gaia"),
            health_report("abyss", &hong, &khong_giai_thich()).unwrap(),
            bao_cao("celestia"),
        ],
    };
    assert!(!dem.passed());
    let chan = dem.blockers();
    assert_eq!(chan.len(), 1);
    assert_eq!(chan[0].0, "abyss", "và nói rõ world nào");
}

/// **Thiếu world cũng là đỏ** — hai world chạy xong không phải là đạt.
#[test]
fn thieu_world_cung_la_do() {
    let dem = SoakRun {
        reports: vec![bao_cao("gaia"), bao_cao("abyss")],
    };
    assert!(!dem.passed(), "chạy thiếu một world không phải là đạt");
}

/// Chuỗi mẫu rỗng thì không dựng được báo cáo — và nói `None`, không bịa.
#[test]
fn chuoi_mau_rong_thi_khong_dung_duoc_bao_cao() {
    assert!(health_report("gaia", &[], &khong_giai_thich()).is_none());
}
