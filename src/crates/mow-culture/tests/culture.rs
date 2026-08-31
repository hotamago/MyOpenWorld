//! Test thông điệp, tôn giáo, thế giới ngầm (`PD-09`, `PD-14`, `PD-15`).

use mow_core::EntityId;
use mow_culture::message::{
    consider, dominant_version, Bias, Reception, Rumour, SocialEvidence, Translation,
};
use mow_culture::religion::{conviction, credibility, Doctrine, Observance, Religion, Rite};
use mow_culture::underworld::{
    black_market_price, gambling_craving, recruitment_pool, Addiction, Cohort, Substance,
};

// ───────────────────────── PD-14 · thông điệp ─────────────────────────

fn tin(id: u64) -> Rumour {
    Rumour {
        id,
        about_event: 100,
        content: "lãnh chúa đã chết".into(),
        origin: EntityId(1),
        fidelity: 1_000,
        hops: 0,
        distortion_motive: 0,
    }
}

fn boi_canh() -> SocialEvidence {
    SocialEvidence {
        peers_adopting: 400,
        adopter_prestige: 400,
        observed_success: 400,
        ingroup: 400,
        instructor_expertise: 400,
        costly_display: 400,
    }
}

/// **Nghe được ≠ làm theo.**
#[test]
fn nghe_duoc_khac_lam_theo() {
    let mut r = Reception::new();
    r.hear(EntityId(5), 1);
    assert!(r.has_heard(EntityId(5), 1));
    assert!(!r.has_adopted(EntityId(5), 1), "nghe xong là làm theo ngay");

    r.adopt(EntityId(5), 1);
    assert!(r.has_adopted(EntityId(5), 1));
}

/// **Cùng một mạng lưới, nhiều tốc độ lan** — điều mà một hệ số duy nhất không
/// tạo ra được.
#[test]
fn cung_mot_boi_canh_cac_xu_huong_cho_ket_qua_khac_nhau() {
    // Bối cảnh: ai cũng làm, nhưng chưa ai chứng tỏ bằng hành động tốn kém.
    let thoi_trang_lan = SocialEvidence {
        peers_adopting: 900,
        costly_display: 0,
        ..boi_canh()
    };

    let theo_dam_dong = consider(Bias::Conformity, &thoi_trang_lan, 500);
    let theo_hy_sinh = consider(Bias::CostlySignal, &thoi_trang_lan, 500);

    assert!(theo_dam_dong.adopts, "thời trang phải lan khi ai cũng theo");
    assert!(
        !theo_hy_sinh.adopts,
        "tín ngưỡng không được lan chỉ vì đông người"
    );
}

/// Và ngược lại: hành động tốn kém thuyết phục được người theo `CostlySignal`
/// ngay cả khi chưa ai làm theo.
#[test]
fn hy_sinh_that_thuyet_phuc_duoc_du_chua_ai_theo() {
    let mot_nguoi_hy_sinh = SocialEvidence {
        peers_adopting: 0,
        costly_display: 950,
        ..boi_canh()
    };
    assert!(consider(Bias::CostlySignal, &mot_nguoi_hy_sinh, 500).adopts);
    assert!(!consider(Bias::Conformity, &mot_nguoi_hy_sinh, 500).adopts);
}

/// Quyết định làm theo luôn **giải thích được**.
#[test]
fn quyet_dinh_lam_theo_giai_thich_duoc() {
    let a = consider(Bias::Success, &boi_canh(), 500);
    assert!(!a.factors.is_empty());
    assert_eq!(a.score, a.factors.iter().map(|(_, v)| v).sum::<i64>());
    assert_eq!(a.bias, Bias::Success);
}

/// **Truyền là mất**: mỗi lần kể lại độ trung thực giảm.
#[test]
fn moi_lan_ke_lai_do_trung_thuc_giam() {
    let goc = tin(1);
    let l1 = goc.retell(2, 0);
    let l2 = l1.retell(3, 0);
    assert!(l1.fidelity < goc.fidelity);
    assert!(l2.fidelity < l1.fidelity);
    assert_eq!(l2.hops, 2);
}

/// **Tuyên truyền khác tam sao thất bản**: có động cơ thì mất nhanh hơn.
#[test]
fn co_dong_co_be_noi_dung_thi_mat_nhanh_hon() {
    let goc = tin(1);
    let vo_tinh = goc.retell(2, 0);
    let co_y = goc.retell(3, 900);
    assert!(co_y.fidelity < vo_tinh.fidelity);
}

/// `fidelity` **không phải** "đúng hay sai".
///
/// Một phiên bản trung thực với lời kể gốc vẫn có thể sai, nếu người kể đầu tiên
/// đã nhìn nhầm — nên không có trường nào tên `is_true`.
#[test]
fn fidelity_khong_phai_dung_hay_sai() {
    let j = serde_json::to_string(&tin(1)).unwrap();
    for cam in ["is_true", "truth", "correct"] {
        assert!(
            !j.contains(cam),
            "Rumour có trường `{cam}` — nó không nên biết sự thật"
        );
    }
}

/// **Yuu không quyết định phiên bản nào thắng** — số người làm theo quyết định.
#[test]
fn phien_ban_thang_la_phien_ban_nhieu_nguoi_lam_theo() {
    let a = tin(1);
    let b = Rumour {
        content: "lãnh chúa vẫn sống".into(),
        ..tin(2)
    };

    let mut r = Reception::new();
    for i in 0..3 {
        r.hear(EntityId(i), 1);
        r.adopt(EntityId(i), 1);
    }
    r.hear(EntityId(9), 2);

    let cac = [a, b];
    let thang = dominant_version(&cac, &r).unwrap();
    assert_eq!(thang.id, 1);
}

/// **Dịch sai là một nguồn xung đột**, và không ai cố tình nói dối.
#[test]
fn dich_sai_lam_hong_noi_dung_ma_khong_ai_noi_doi() {
    let tot = Translation {
        from: "old_veskaran".into(),
        to: "common".into(),
        mutual_intelligibility: 900,
        translator_skill: 900,
    };
    let te = Translation {
        mutual_intelligibility: 200,
        translator_skill: 300,
        ..tot.clone()
    };

    assert!(te.fidelity_after(1_000) < tot.fidelity_after(1_000));
    assert!(
        te.is_dangerous(1_000, 500),
        "một câu thần chú dịch tệ phải là nguồn tai nạn hợp lý"
    );
    assert!(!tot.is_dangerous(1_000, 500));
}

// ───────────────────────── PD-15 · tôn giáo ─────────────────────────

fn giao_hoi() -> Religion {
    Religion {
        id: "church.of_veskar".into(),
        worships: "deity.the_forge".into(),
        doctrines: vec![
            Doctrine {
                id: "d.creation".into(),
                derives_from: vec![],
                interpreters: vec![EntityId(1)],
            },
            Doctrine {
                id: "d.succession".into(),
                derives_from: vec!["d.creation".into()],
                interpreters: vec![EntityId(1), EntityId(2)],
            },
        ],
        rites: vec![
            Rite {
                id: "r.sermon".into(),
                cost: 1,
                hard_to_fake: false,
                public: true,
            },
            Rite {
                id: "r.pilgrimage".into(),
                cost: 900,
                hard_to_fake: true,
                public: true,
            },
        ],
        holy_sites: vec!["site.great_forge".into()],
        clergy: vec![EntityId(1), EntityId(2)],
    }
}

/// **Belief tách khỏi việc vị thần có thật hay không.**
///
/// Giáo hội chỉ giữ một cái *tên*; nó không có cách nào hỏi xem tên đó ứng với ai.
#[test]
fn giao_hoi_khong_co_duong_nao_hoi_than_co_that_khong() {
    let g = giao_hoi();
    let j = serde_json::to_string(&g).unwrap();
    assert!(j.contains("deity.the_forge"));
    // Không có `EntityId` nào của thần trong cấu trúc — chỉ có tên.
    assert!(!j.contains("deity_entity"));
    assert!(!j.contains("\"exists\""));
}

/// **Nghi lễ tốn kém là bằng chứng, giảng đạo thì không.**
#[test]
fn giang_dao_khong_tao_ra_bang_chung_hanh_huong_thi_co() {
    let g = giao_hoi();
    let sermon = &g.rites[0];
    let pilgrimage = &g.rites[1];

    let nghe = Observance {
        who: EntityId(5),
        rite: "r.sermon".into(),
        paid: 1,
        witnesses: 500,
    };
    let di = Observance {
        who: EntityId(5),
        rite: "r.pilgrimage".into(),
        paid: 900,
        witnesses: 5,
    };

    assert!(
        credibility(pilgrimage, &di) > credibility(sermon, &nghe) * 5,
        "giảng đạo trước 500 người vẫn không bằng đi bộ ba tháng"
    );
}

/// **Không có `faith_point`**: hứa suông không tính.
#[test]
fn hua_suong_khong_tao_ra_bang_chung() {
    let g = giao_hoi();
    let noi_ma_khong_lam = Observance {
        who: EntityId(5),
        rite: "r.pilgrimage".into(),
        paid: 0,
        witnesses: 1_000,
    };
    assert_eq!(credibility(&g.rites[1], &noi_ma_khong_lam), 0);
}

/// **Bằng chứng không ai thấy thì không thuyết phục ai.**
#[test]
fn hy_sinh_kin_dao_khong_thuyet_phuc_duoc_nguoi_khac() {
    let g = giao_hoi();
    let mot_minh = Observance {
        who: EntityId(5),
        rite: "r.pilgrimage".into(),
        paid: 900,
        witnesses: 0,
    };
    assert_eq!(credibility(&g.rites[1], &mot_minh), 0);
}

/// Tin theo cộng dồn **bằng chứng**, không cộng dồn số buổi giảng.
#[test]
fn tin_theo_cong_don_bang_chung_khong_cong_don_buoi_giang() {
    let g = giao_hoi();
    let nhieu_buoi_giang: Vec<Observance> = (0..50)
        .map(|i| Observance {
            who: EntityId(i),
            rite: "r.sermon".into(),
            paid: 1,
            witnesses: 100,
        })
        .collect();
    let mot_lan_hanh_huong = vec![Observance {
        who: EntityId(1),
        rite: "r.pilgrimage".into(),
        paid: 900,
        witnesses: 10,
    }];

    assert!(
        conviction(&g.rites, &mot_lan_hanh_huong) > conviction(&g.rites, &nhieu_buoi_giang),
        "năm mươi buổi giảng thuyết phục hơn một chuyến hành hương"
    );
}

/// **Ly giáo cần có tranh chấp quyền diễn giải**, không có thì không tách được.
#[test]
fn ly_giao_can_tranh_chap_quyen_dien_giai() {
    let g = giao_hoi();
    assert!(!g.contested("d.creation"), "chỉ một người diễn giải");
    assert!(g.contested("d.succession"));

    assert!(
        g.schism("d.creation", EntityId(1), "church.new").is_none(),
        "tách được mà không có bất đồng nào"
    );
    let moi = g
        .schism("d.succession", EntityId(2), "church.reformed")
        .unwrap();
    assert_eq!(moi.clergy, vec![EntityId(2)]);
}

/// Nhánh ly khai **nhất quán hơn** bản gốc — chỉ một người diễn giải.
#[test]
fn nhanh_ly_khai_nhat_quan_hon_ban_goc() {
    let g = giao_hoi();
    let moi = g
        .schism("d.succession", EntityId(2), "church.reformed")
        .unwrap();
    assert!(!moi.contested("d.succession"));
    assert!(g.contested("d.succession"), "bản gốc vẫn còn tranh chấp");
}

/// Người ngoài không tách được giáo hội.
#[test]
fn nguoi_ngoai_khong_tach_duoc_giao_hoi() {
    let g = giao_hoi();
    assert!(g.schism("d.succession", EntityId(99), "x").is_none());
}

// ───────────────────────── PD-09 · thế giới ngầm ─────────────────────────

/// **Tuyển mộ từ chính hệ quả của `§12.5.4`**: cần cả hai điều kiện cùng lúc.
#[test]
fn tuyen_mo_can_ca_it_gan_bo_lan_it_co_hoi() {
    let ca_hai = Cohort {
        size: 1_000,
        belonging: 100,
        lawful_opportunity: 100,
    };
    let ngheo_ma_gan_bo = Cohort {
        belonging: 900,
        ..ca_hai
    };
    let lac_long_ma_co_nghe = Cohort {
        lawful_opportunity: 900,
        ..ca_hai
    };

    assert!(recruitment_pool(&ca_hai) > 500);
    assert!(
        recruitment_pool(&ngheo_ma_gan_bo) < 200,
        "người nghèo mà gắn bó với xóm làng không đi làm cho băng đảng"
    );
    assert!(recruitment_pool(&lac_long_ma_co_nghe) < 200);
}

/// Nhà nước nghiêm khắc mù quáng **nuôi lớn thứ nó đang chống**.
#[test]
fn trung_phat_lam_mat_co_hoi_hop_phap_thi_bang_dang_lon_len() {
    let truoc = Cohort {
        size: 1_000,
        belonging: 300,
        lawful_opportunity: 700,
    };
    // Sau khi bị lưu đày và kỳ thị: mất cả gắn bó lẫn cơ hội.
    let sau = Cohort {
        belonging: 100,
        lawful_opportunity: 100,
        ..truoc
    };
    assert!(recruitment_pool(&sau) > recruitment_pool(&truoc) * 3);
}

/// **Chợ đen không cần hệ thống riêng** — chỉ là giá cộng phần bù.
#[test]
fn cho_den_la_gia_cong_phan_bu_rui_ro() {
    let hop_phap = 100;
    let khong_truy_quet = black_market_price(hop_phap, 0, 0, false);
    assert_eq!(
        khong_truy_quet, hop_phap,
        "không rủi ro thì không có phần bù"
    );

    let bi_truy_quet = black_market_price(hop_phap, 800, 300, true);
    assert!(bi_truy_quet > hop_phap * 2);
}

/// **Truy quét mạnh làm lợi nhuận tăng** — vòng phản hồi ở `§12.6.3`.
#[test]
fn truy_quet_manh_lam_loi_nhuan_tang() {
    let mut truoc = 0;
    for gat in [0u16, 200, 400, 600, 800, 1_000] {
        let g = black_market_price(100, gat, 200, true);
        assert!(g >= truoc, "truy quét gắt hơn mà giá lại giảm");
        truoc = g;
    }
}

/// Nghiện: **càng nghiện càng phải dùng nhiều, càng dùng nhiều càng độc.**
#[test]
fn cang_nghien_cang_phai_dung_nhieu_va_cang_doc() {
    let chat = Substance {
        id: "core.dreamleaf".into(),
        dose: 100,
        duration: 1_000,
        tolerance_gain: 50,
        dependence_gain: 40,
        toxicity: 10,
    };
    let mut a = Addiction::default();
    let lieu_dau = a.effective_dose(&chat);

    for _ in 0..10 {
        a.take(&chat);
    }
    assert!(a.effective_dose(&chat) > lieu_dau);
    assert_eq!(a.toxin_load, 100);
    assert!(a.dependence > 0);
}

/// Vật vã tăng theo thời gian nhịn, và chỉ bắt đầu sau khi thuốc hết tác dụng.
#[test]
fn vat_va_chi_bat_dau_sau_khi_thuoc_het_tac_dung() {
    let chat = Substance {
        id: "core.dreamleaf".into(),
        dose: 100,
        duration: 1_000,
        tolerance_gain: 50,
        dependence_gain: 200,
        toxicity: 10,
    };
    let mut a = Addiction::default();
    for _ in 0..5 {
        a.take(&chat);
    }

    a.ticks_since_dose = 500;
    assert_eq!(a.withdrawal(&chat), 0, "còn tác dụng thì chưa vật");

    a.ticks_since_dose = 3_000;
    let som = a.withdrawal(&chat);
    a.ticks_since_dose = 9_000;
    assert!(a.withdrawal(&chat) > som);
}

/// **Cờ bạc dùng cùng khung**, và biến thiên là thứ gây nghiện — không phải phần thưởng.
#[test]
fn co_bac_gay_nghien_vi_bien_thien_khong_phai_phan_thuong() {
    // Trò luôn thắng hoặc luôn thua: không biến thiên, không nghiện.
    assert_eq!(gambling_craving(200, 0, 100), 0);

    let bien_thien_cao = gambling_craving(200, 1_000, 100);
    assert!(bien_thien_cao > 500);
}

/// **Suýt thắng kéo mạnh hơn cả thắng thật.**
#[test]
fn suyt_thang_keo_manh_hon_ca_thang_that() {
    let khong_suyt = gambling_craving(50, 800, 0);
    let nhieu_suyt = gambling_craving(50, 800, 150);
    assert!(nhieu_suyt > khong_suyt);
}
