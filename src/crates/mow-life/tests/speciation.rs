//! Test hình thành loài mới (`§9.5.5`).

use mow_life::barrier::Reproductive;
use mow_life::speciation::{
    secondary_contact, Divergence, IsolatedPopulation, SpeciationRoute, CAP_DU_DE_VO_SINH,
};

fn tach(generations: u64, effective_size: u64, selection_differential: u32) -> IsolatedPopulation {
    IsolatedPopulation {
        id: "x".into(),
        route: SpeciationRoute::IsolationThenDivergence,
        effective_size,
        generations,
        selection_differential,
    }
}

/// **Bất tương hợp tăng theo bình phương, khác biệt tăng tuyến tính.**
///
/// Hai nhịp khác nhau là toàn bộ nội dung của mô hình. Nếu cả hai cùng tuyến
/// tính thì không có hiệu ứng snowball, và quá trình sẽ đều đặn một cách không
/// thực tế.
#[test]
fn bat_tuong_hop_tang_nhanh_hon_khac_biet() {
    let a = Divergence::after(&tach(200, 800, 60));
    let b = Divergence::after(&tach(400, 800, 60));

    // Khác biệt: gấp đôi.
    assert_eq!(b.fixed_differences, a.fixed_differences * 2);
    // Cặp bất tương hợp: gấp gần bốn.
    assert!(b.incompatible_pairs > a.incompatible_pairs * 3);
}

/// **Quần thể nhỏ phân kỳ nhanh hơn** — con đường 2 của `§9.5.5` cộng dồn.
#[test]
fn quan_the_nho_phan_ky_nhanh_hon() {
    let nho = Divergence::after(&tach(300, 200, 60));
    let lon = Divergence::after(&tach(300, 5_000, 60));
    assert!(nho.fixed_differences > lon.fixed_differences);
}

/// **Áp lực chọn lọc mạnh đẩy nhanh** — portal nhanh hơn một dãy núi.
#[test]
fn ap_luc_chon_loc_manh_day_nhanh_hon() {
    let world_khac = Divergence::after(&tach(300, 5_000, 200));
    let ben_kia_nui = Divergence::after(&tach(300, 5_000, 5));
    assert!(world_khac.fixed_differences > ben_kia_nui.fixed_differences);
}

/// Khả năng sinh sản **giảm dần**, không nhảy.
#[test]
fn kha_nang_sinh_san_giam_dan_khong_nhay() {
    let mut truoc = 1_001;
    for doi in [0, 100, 200, 400, 600, 800, 1_200] {
        let f = Divergence::after(&tach(doi, 800, 60)).hybrid_fertility();
        assert!(f <= truoc, "đời {doi}: {f} > {truoc}");
        truoc = f;
    }
    assert_eq!(truoc, 0, "đủ lâu thì vô sinh hẳn");
}

/// Vừa tách thì chưa có gì.
#[test]
fn vua_tach_thi_chua_co_gi() {
    let d = Divergence::after(&tach(0, 800, 60));
    assert_eq!(d.fixed_differences, 0);
    assert_eq!(d.incompatible_pairs, 0);
    assert_eq!(d.hybrid_fertility(), 1_000);
    assert_eq!(d.as_barrier(), Reproductive::FullyCompatible);
}

/// **Xếp đúng vào thang rào cản `§9.11.1`** khi phân kỳ tăng.
#[test]
fn xep_dung_vao_thang_rao_can_sinh_san() {
    let bac = |cap: u64| {
        Divergence {
            fixed_differences: 0,
            incompatible_pairs: cap,
        }
        .as_barrier()
    };
    assert_eq!(bac(0), Reproductive::FullyCompatible);
    assert_eq!(bac(30), Reproductive::FullyCompatible);
    assert_eq!(bac(300), Reproductive::ReducedViability);
    assert_eq!(bac(900), Reproductive::SterileHybrid);
    assert_eq!(bac(CAP_DU_DE_VO_SINH), Reproductive::Incompatible);
}

/// **Không có con lai nào thì không kết luận được gì** — mẫu rỗng, không phải
/// bằng chứng vô sinh.
#[test]
fn mau_rong_khong_phai_bang_chung_vo_sinh() {
    let khong_ai_thu = secondary_contact(&tach(600, 800, 60), 0);
    assert_eq!(khong_ai_thu.hybrids_born, 0);
    assert!(
        !khong_ai_thu.decline_is_measurable(),
        "không ai thử thì không kết luận được"
    );
}

/// Mẫu nhỏ cũng chưa đủ.
#[test]
fn mau_nho_cung_chua_du_de_ket_luan() {
    let vai_cap = secondary_contact(&tach(600, 800, 60), 5);
    assert!(vai_cap.hybrids_born < 20);
    assert!(!vai_cap.decline_is_measurable());
}

/// **Vẫn nhận ra nhau là họ hàng** ở thang thời gian này — chỗ bi kịch nằm.
#[test]
fn van_nhan_ra_nhau_la_ho_hang() {
    let g = secondary_contact(&tach(600, 800, 60), 400);
    assert!(g.still_recognisable);
    assert!(
        g.hybrids_born > 0,
        "vẫn lai được, chỉ là con lai vô sinh dần"
    );
}

/// **Sức sống giảm chậm hơn khả năng sinh sản.**
///
/// Con lai F1 thường sống khỏe và vô sinh, chứ không chết trong trứng. Nếu hai
/// con số này đi cùng nhịp thì con lai biến mất trước khi ai kịp phát hiện
/// vấn đề — và mất luôn phần "phát hiện ra sau một thế hệ".
#[test]
fn suc_song_giam_cham_hon_kha_nang_sinh_san() {
    let g = secondary_contact(&tach(800, 800, 60), 1_000);
    let ti_le_sinh_ra = g.hybrids_born * 1_000 / g.pairings;
    assert!(
        ti_le_sinh_ra > u64::from(g.fertile_permille()),
        "sinh ra {ti_le_sinh_ra}‰ nhưng chỉ {}‰ sinh sản tiếp",
        g.fertile_permille()
    );
}

/// Xác định: cùng đầu vào cho cùng biên bản.
#[test]
fn tiep_xuc_thu_cap_xac_dinh() {
    let p = tach(600, 800, 60);
    assert_eq!(secondary_contact(&p, 400), secondary_contact(&p, 400));
}
