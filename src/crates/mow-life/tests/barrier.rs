//! Test năm rào cản liên loài (`PE-15`, `§9.11`).

use mow_life::barrier::{
    coordination_cost, inapplicable_systems, time_gap, Axis, Barriers, Habitat, Lifespan, Range,
    Reproductive, Senses, SocialStructure, Territorial, BRIDGES,
};
use std::collections::BTreeSet;

fn kenh(v: &[&str]) -> Senses {
    Senses {
        channels: v.iter().map(|s| (*s).to_owned()).collect(),
    }
}

fn nguoi() -> Lifespan {
    Lifespan {
        years: 75,
        adult_at: 18,
    }
}

fn elf() -> Lifespan {
    Lifespan {
        years: 3_000,
        adult_at: 100,
    }
}

fn dai_on_hoa() -> Habitat {
    Habitat {
        temperature: Range::new(-500, 3_500),
        mana: Range::new(0, 4_000),
        atmosphere: Range::new(40, 100),
    }
}

// ───────────────── rào 1 · sinh sản (§9.11.1) ─────────────────

/// **Con lai vô sinh cho phép hôn nhân nhưng chặn dòng dõi.**
#[test]
fn con_lai_vo_sinh_cho_phep_hon_nhan_nhung_chan_dong_doi() {
    assert!(Reproductive::SterileHybrid.can_bear_offspring());
    assert!(!Reproductive::SterileHybrid.lineage_continues());
    // Khác hẳn với không lai được.
    assert!(!Reproductive::Incompatible.can_bear_offspring());
}

/// Giảm sức sống vẫn tiếp dòng được.
#[test]
fn giam_suc_song_van_tiep_dong_duoc() {
    assert!(Reproductive::ReducedViability.lineage_continues());
    assert!(Reproductive::FullyCompatible.lineage_continues());
}

// ───────────────── rào 2 · môi trường (§9.11.2) ─────────────────

/// **Không sống chung được ⇒ không bao giờ tranh chấp lãnh thổ.**
#[test]
fn khong_song_chung_duoc_thi_khong_bao_gio_tranh_chap() {
    let lua = Habitat {
        temperature: Range::new(6_000, 9_000),
        mana: Range::new(8_000, 12_000),
        atmosphere: Range::new(0, 20),
    };
    assert_eq!(
        Barriers::territorial_from(dai_on_hoa(), lua),
        Territorial::Disjoint
    );
}

/// **Chồng lấn hẹp ⇒ tranh chấp gay gắt**: cả hai cần đúng một dải.
#[test]
fn chong_lan_hep_thi_tranh_chap_gay_gat() {
    let a = Habitat {
        temperature: Range::new(0, 4_000),
        mana: Range::new(0, 4_000),
        atmosphere: Range::new(40, 100),
    };
    let b = Habitat {
        temperature: Range::new(3_900, 8_000),
        mana: Range::new(0, 4_000),
        atmosphere: Range::new(40, 100),
    };
    assert_eq!(
        Barriers::territorial_from(a, b),
        Territorial::NarrowContested
    );
}

/// Chồng lấn rộng thì chia nhau được.
#[test]
fn chong_lan_rong_thi_chia_nhau_duoc() {
    assert_eq!(
        Barriers::territorial_from(dai_on_hoa(), dai_on_hoa()),
        Territorial::BroadShared
    );
}

/// **Hai cực đối lập đều ít xung đột, vì lý do trái ngược nhau.**
///
/// Đây là lý do quan hệ lãnh thổ không phải một thang từ hòa bình tới chiến
/// tranh: xung đột nằm ở **giữa**, không ở một đầu.
#[test]
fn hai_cuc_doi_lap_deu_it_xung_dot() {
    let lua = Habitat {
        temperature: Range::new(6_000, 9_000),
        mana: Range::new(8_000, 12_000),
        atmosphere: Range::new(0, 20),
    };
    for t in [
        Barriers::territorial_from(dai_on_hoa(), lua),
        Barriers::territorial_from(dai_on_hoa(), dai_on_hoa()),
    ] {
        assert_ne!(t, Territorial::NarrowContested);
    }
}

// ───────────────── rào 3 · tri giác (§9.11.3) ─────────────────

/// **Có những từ không có vật quy chiếu ở phía bên kia.**
#[test]
fn co_nhung_node_khong_day_truc_tiep_duoc() {
    let cam_mana = kenh(&["sight.visible", "mana.gradient"]);
    let khong = kenh(&["sight.visible"]);
    let bang = vec![
        ("spell.ley_sense".to_owned(), "mana.gradient".to_owned()),
        ("craft.masonry".to_owned(), "sight.visible".to_owned()),
    ];

    let khong_day_duoc = cam_mana.unteachable_to(&khong, &bang);
    assert_eq!(khong_day_duoc.len(), 1);
    assert_eq!(khong_day_duoc[0].node, "spell.ley_sense");
    assert_eq!(khong_day_duoc[0].missing_channel, "mana.gradient");
}

/// **"Không dạy được" luôn kèm lối đi** — nó là rào, không phải tường.
#[test]
fn khong_day_duoc_luon_kem_loi_di() {
    let a = kenh(&["mana.gradient"]);
    let b = kenh(&[]);
    let r = a.unteachable_to(
        &b,
        &[("spell.ley_sense".to_owned(), "mana.gradient".to_owned())],
    );
    assert_eq!(r[0].bridges, BRIDGES);
    assert_eq!(r[0].bridges.len(), 3);
}

/// Dịch ngôn ngữ không đủ, nhưng **đội hỗn hợp nhìn được nhiều hơn**.
#[test]
fn doi_hon_hop_nhin_duoc_nhieu_hon_va_tra_gia_bang_chi_phi_phoi_hop() {
    let a = kenh(&["sight.visible", "mana.gradient"]);
    let b = kenh(&["sight.visible", "echolocation"]);
    assert_eq!(a.union_with(&b).len(), 3, "nhìn được 3 kênh thay vì 2");
    assert!(coordination_cost(&a, &b) > 0, "và trả giá bằng hiểu lầm");
}

/// Cùng bộ giác quan thì không tốn chi phí phối hợp.
#[test]
fn cung_bo_giac_quan_thi_khong_ton_chi_phi_phoi_hop() {
    let a = kenh(&["sight.visible", "hearing"]);
    assert_eq!(coordination_cost(&a, &a), 0);
}

/// Không chung kênh nào thì chi phí tối đa.
#[test]
fn khong_chung_kenh_nao_thi_chi_phi_toi_da() {
    assert_eq!(coordination_cost(&kenh(&["a"]), &kenh(&["b"])), 1_000);
}

// ───────────────── rào 4 · thời gian (§9.11.4) ─────────────────

/// **Con người vượt lên vì họ thay thế hệ** — số học, không phải trí tuệ.
#[test]
fn nguoi_thay_the_he_nhanh_hon_elf() {
    let g = time_gap(nguoi(), elf(), 100);
    assert_eq!(
        g.generations_per_long_life, 166,
        "một đời elf là 166 khoảng cách thế hệ người"
    );
    assert!(g.ratio_permille > 39_000);
}

/// **"Hòa ước một trăm năm" là một đời người và một giấc ngủ ngắn.**
#[test]
fn hoa_uoc_mot_tram_nam_lech_nghia_giua_hai_loai() {
    let g = time_gap(nguoi(), elf(), 100);
    let (phia_ngan, phia_dai) = g.treaty_meaning;
    assert!(phia_ngan > 1_000, "hơn cả một đời người: {phia_ngan}‰");
    assert!(phia_dai < 50, "chưa tới 5% một đời elf: {phia_dai}‰");
}

/// **Cá nhân là kho lưu trữ** — một vụ ám sát xóa sạch một thư viện.
#[test]
fn ca_nhan_song_lau_la_mot_kho_luu_tru() {
    let g = time_gap(nguoi(), elf(), 100);
    assert_eq!(
        g.individual_as_archive, 40,
        "một elf giữ tri thức qua 40 đời người"
    );
}

/// **Người đứng đầu không chết thì đường thăng tiến đóng lại.**
#[test]
fn nguoi_dung_dau_khong_chet_thi_duong_thang_tien_dong_lai() {
    assert!(time_gap(nguoi(), elf(), 100).gerontocracy_risk);
    // Hai loài tuổi thọ tương đương thì không có vấn đề này.
    assert!(!time_gap(nguoi(), nguoi(), 100).gerontocracy_risk);
}

/// **Bi kịch là số học** — không cần viết cốt truyện cho nó.
#[test]
fn quan_he_lien_loai_la_bi_kich_co_san() {
    assert!(time_gap(nguoi(), elf(), 100).tragedy_is_arithmetic);
    assert!(!time_gap(nguoi(), nguoi(), 100).tragedy_is_arithmetic);
}

// ───────────────── rào 5 · cấu trúc xã hội (§9.11.5) ─────────────────

/// **Loài đơn-lưỡng bội không vận hành theo cùng vật lý xã hội.**
#[test]
fn loai_don_luong_boi_khong_ap_duoc_mo_hinh_ho_gia_dinh() {
    let v = inapplicable_systems(SocialStructure::Haplodiploid);
    assert!(!v.is_empty());
    assert!(v.iter().any(|s| s.contains("§12.9")));
    assert!(v.iter().any(|s| s.contains("§12.5.2")));
    assert!(v.iter().any(|s| s.contains("norm_set")));
}

/// Loài lưỡng bội thì mọi hệ thống đã có đều áp được.
#[test]
fn loai_luong_boi_thi_moi_he_thong_da_co_deu_ap_duoc() {
    assert!(inapplicable_systems(SocialStructure::Diploid).is_empty());
}

/// Quần thể hợp nhất phá thêm cả `§12.10` — **danh sách cụ thể, không một cờ**.
#[test]
fn quan_the_hop_nhat_pha_them_ca_duong_thang_tien() {
    let a = inapplicable_systems(SocialStructure::Haplodiploid);
    let b = inapplicable_systems(SocialStructure::Colonial);
    assert!(b.len() > a.len());
    assert!(b.iter().any(|s| s.contains("§12.10")));
}

// ───────────── năm rào **độc lập**: điều mà một chỉ số làm mất ─────────────

fn nguoi_elf() -> Barriers {
    Barriers {
        reproductive: Reproductive::SterileHybrid,
        territorial: Territorial::BroadShared,
        perceptual_gap: BTreeSet::new(),
        temporal: time_gap(nguoi(), elf(), 100),
        social_mismatch: false,
    }
}

fn nguoi_kien_nhan() -> Barriers {
    Barriers {
        reproductive: Reproductive::Incompatible,
        territorial: Territorial::BroadShared,
        perceptual_gap: BTreeSet::new(),
        temporal: time_gap(nguoi(), nguoi(), 100),
        social_mismatch: true,
    }
}

/// **Hai cặp loài "cùng khó" chắn ở hai chỗ khác nhau.**
///
/// Đây là test trung tâm của `PE-15`. Nếu năm rào bị gộp thành một chỉ số thì
/// hai cặp này ra cùng một con số, và cả hai cốt truyện — một bên cưới được và
/// chết trước, một bên không ánh xạ nổi khái niệm hôn nhân — bị nhập làm một.
#[test]
fn hai_cap_loai_cung_kho_nhung_chan_o_hai_cho_khac_nhau() {
    let a = nguoi_elf();
    let b = nguoi_kien_nhan();
    assert_eq!(a.summary().len(), b.summary().len(), "cùng số rào");
    assert!(
        !a.same_shape_as(&b),
        "cùng số rào mà chắn ở chỗ khác — một chỉ số duy nhất sẽ nói chúng giống nhau"
    );
    assert_eq!(
        a.summary(),
        BTreeSet::from([Axis::Reproductive, Axis::Temporal])
    );
    assert_eq!(
        b.summary(),
        BTreeSet::from([Axis::Reproductive, Axis::Social])
    );
}

/// Vượt được rào này mà không vượt được rào kia — `§9.11` nói thẳng vậy.
#[test]
fn vuot_duoc_rao_nay_ma_khong_vuot_duoc_rao_kia() {
    let mut b = nguoi_elf();
    // Chữa được rào sinh sản bằng phép thuật; rào thời gian vẫn nguyên.
    b.reproductive = Reproductive::FullyCompatible;
    assert_eq!(b.summary(), BTreeSet::from([Axis::Temporal]));
}

/// Không rào nào chắn thì tập rỗng — và tập rỗng là một câu trả lời hợp lệ.
#[test]
fn khong_rao_nao_chan_thi_tap_rong() {
    let b = Barriers {
        reproductive: Reproductive::FullyCompatible,
        territorial: Territorial::BroadShared,
        perceptual_gap: BTreeSet::new(),
        temporal: time_gap(nguoi(), nguoi(), 100),
        social_mismatch: false,
    };
    assert!(b.summary().is_empty());
}

/// `summary` **không** trả về một con số.
///
/// Test này khẳng định một quyết định API. Nếu ai đó đổi nó thành `u32` thì
/// mọi chỗ gọi sẽ so bằng `<`, và câu hỏi *"chắn ở đâu"* — câu duy nhất người
/// chơi hành động theo được — biến mất.
#[test]
fn summary_khong_tra_ve_mot_con_so() {
    let s: BTreeSet<Axis> = nguoi_elf().summary();
    assert!(s.contains(&Axis::Reproductive));
}
