//! Test di cư và ứng phó thảm họa (`PF-20`, `§12.19`, `§12.20`).

use mow_org::crisis::{
    decide, displacement_belief, flows, respond, Belief, Capacity, Diaspora, Household, Magnitude,
    Role, NGUONG_SUP_DO,
};
use mow_org::legitimacy::{Legitimacy, Source};

fn tin_binh_thuong() -> Belief {
    Belief {
        safety_here: 600,
        safety_there: 700,
        wage_here: 300,
        wage_there: 800,
        journey_cost: 400,
        has_contact_there: false,
    }
}

fn duoc_tin() -> Legitimacy {
    Legitimacy {
        belief: 700,
        fear: 100,
        conformity: 200,
        sources: vec![Source::Performance],
    }
}

fn dua_tren_so_hai() -> Legitimacy {
    Legitimacy {
        belief: 100,
        fear: 700,
        conformity: 200,
        sources: vec![Source::Tradition],
    }
}

// ══════════════════ §12.19 · di cư ══════════════════

/// **Quyết định dựa trên belief, không dựa trên số liệu thật.**
///
/// Bài này khẳng định một quyết định API: `decide` không nhận world state, nên
/// hai người có cùng niềm tin quyết định giống nhau **dù sự thật khác nhau**.
#[test]
fn quyet_dinh_dua_tren_belief_khong_dua_tren_su_that() {
    // Nền: tiền công thật ở nơi đến là 800 — không đủ để một người đi đầu đi.
    assert!(!decide(&tin_binh_thuong(), Role::Pioneer).leaves);

    // Tin đồn phóng đại lên 1000. Sự thật không đổi; quyết định đổi.
    let tin_sai = Belief {
        wage_there: 1_000,
        ..tin_binh_thuong()
    };
    let d = decide(&tin_sai, Role::Pioneer);
    assert!(d.leaves, "người ta đi vì tin, không vì đúng");
    assert_eq!(d.expected_gain, 1_000 - 300 + 700 - 600 - 400);
}

/// **Người quen ở nơi đến là yếu tố quyết định nhất.**
///
/// Đó là lý do di cư chảy theo những dòng cố định thay vì tỏa đều tới mọi nơi
/// giàu hơn.
#[test]
fn nguoi_quen_o_noi_den_la_yeu_to_quyet_dinh_nhat() {
    let xa_la = Belief {
        wage_there: 900,
        journey_cost: 450,
        ..tin_binh_thuong()
    };
    let co_nguoi = Belief {
        has_contact_there: true,
        ..xa_la
    };
    assert!(!decide(&xa_la, Role::Pioneer).leaves);
    assert!(decide(&co_nguoi, Role::Pioneer).leaves);
}

/// **Người đi đầu chịu rào cản cao hơn người đi sau.**
#[test]
fn nguoi_di_dau_chiu_rao_can_cao_hon_nguoi_di_sau() {
    let vua_du = Belief {
        wage_there: 700,
        ..tin_binh_thuong()
    };
    let a = decide(&vua_du, Role::Pioneer);
    let b = decide(&vua_du, Role::Follower);
    assert_eq!(a.expected_gain, b.expected_gain, "cùng kỳ vọng");
    assert!(!a.leaves);
    assert!(b.leaves, "nhưng ngưỡng khác nhau");
}

/// **Gửi một người đi trước, những người sau đi theo** — không phải cả hộ.
#[test]
fn ho_gia_dinh_gui_mot_nguoi_di_truoc_roi_ca_nha_theo_sau() {
    let ho = Household {
        size: 5,
        pioneer_arrived: false,
        belief: Belief {
            wage_there: 1_000,
            ..tin_binh_thuong()
        },
    };
    let (so_nguoi, vai) = ho.decide();
    assert_eq!(so_nguoi, 1);
    assert_eq!(vai, Role::Pioneer);

    // Người đó tới nơi, cả nhà theo sau.
    let sau = Household {
        pioneer_arrived: true,
        ..ho
    };
    let (so_nguoi, vai) = sau.decide();
    assert_eq!(so_nguoi, 4);
    assert_eq!(vai, Role::Follower);
}

/// Kỳ vọng quá thấp thì cả người đi đầu cũng không đi.
#[test]
fn ky_vong_qua_thap_thi_khong_ai_di() {
    let khong_dang = Household {
        size: 5,
        pioneer_arrived: false,
        belief: Belief {
            wage_there: 310,
            journey_cost: 900,
            ..tin_binh_thuong()
        },
    };
    assert_eq!(khong_dang.decide().0, 0);
}

/// **Lòng trung thành kép** tính từ dấu hiệu bên ngoài, không từ lòng người.
#[test]
fn long_trung_thanh_kep_tinh_tu_dau_hieu_ben_ngoai() {
    let moi_toi = Diaspora {
        origin: "veskar".into(),
        host: "tolm".into(),
        population: 5_000,
        remittances_per_year: 40_000,
        keeps_language: true,
        generations: 1,
    };
    assert!(moi_toi.dual_loyalty_suspicion() >= 900);

    // Đã bốn đời, không gửi tiền, không giữ tiếng: nghi kỵ gần hết.
    let da_hoa_nhap = Diaspora {
        remittances_per_year: 0,
        keeps_language: false,
        generations: 4,
        ..moi_toi.clone()
    };
    assert_eq!(da_hoa_nhap.dual_loyalty_suspicion(), 0);

    // Nhưng **giữ tiếng nói thôi cũng đủ bị nghi**, dù đã bốn đời.
    let van_noi_tieng_cu = Diaspora {
        remittances_per_year: 0,
        keeps_language: true,
        generations: 4,
        ..moi_toi
    };
    assert_eq!(van_noi_tieng_cu.dual_loyalty_suspicion(), 300);
}

// ══════════════════ §12.20 · thảm họa ══════════════════

/// **Cùng một trận động đất, hai kết cục.**
///
/// Đây là bài trung tâm của `PF-20`. Cường độ y hệt; khác nhau chỉ ở năng lực
/// và tính chính danh — và kết cục rơi ra từ phép tính, không từ một quyết
/// định của Director.
#[test]
fn cung_mot_tran_dong_dat_hai_ket_cuc() {
    let dong_dat = Magnitude(700);

    let co_to_chuc = respond(dong_dat, &Capacity::organised(), &duoc_tin(), 800);
    let da_muc_nat = respond(dong_dat, &Capacity::hollowed(), &dua_tren_so_hai(), 200);

    assert!(!co_to_chuc.state_collapses, "{co_to_chuc:?}");
    assert!(da_muc_nat.state_collapses, "{da_muc_nat:?}");
    assert!(da_muc_nat.casualties_permille > co_to_chuc.casualties_permille * 2);
    assert!(da_muc_nat.rebuild_years > co_to_chuc.rebuild_years * 2);
}

/// **Không có tham số `should_collapse`.**
///
/// Bài này khẳng định một quyết định API: `respond` không nhận cờ nào cho
/// phép chỗ gọi quyết định trước kết cục.
#[test]
fn khong_co_tham_so_quyet_dinh_truoc_ket_cuc() {
    // Cùng năng lực và cùng chính danh cho cùng kết quả, mọi lần.
    let a = respond(Magnitude(500), &Capacity::organised(), &duoc_tin(), 800);
    let b = respond(Magnitude(500), &Capacity::organised(), &duoc_tin(), 800);
    assert_eq!(a, b);
}

/// **Có còi báo động mà không ai nghe thì còi vô dụng.**
///
/// [`Capacity::hollowed`] cố tình giữ `warning` cao: hệ thống cảnh báo là hạ
/// tầng kỹ thuật, nó không mất khi chính quyền mất uy tín. Cái mất là người ta
/// có nghe theo hay không.
#[test]
fn co_coi_bao_dong_ma_khong_ai_nghe_thi_coi_vo_dung() {
    let canh_bao_tot = Capacity {
        warning: 900,
        evacuation: 900,
        ..Capacity::organised()
    };
    let duoc_nghe = respond(Magnitude(700), &canh_bao_tot, &duoc_tin(), 800);
    let khong_ai_nghe = respond(Magnitude(700), &canh_bao_tot, &dua_tren_so_hai(), 100);

    assert!(
        khong_ai_nghe.casualties_permille > duoc_nghe.casualties_permille,
        "cùng năng lực cảnh báo, khác mức tuân thủ: {} so với {}",
        khong_ai_nghe.casualties_permille,
        duoc_nghe.casualties_permille
    );
}

/// **Chế độ dựa trên sợ hãi mất tuân thủ nhanh hơn** khi nhà nước yếu đi.
///
/// Nối thẳng vào `§12.13.2`: động đất phá luôn khả năng đi bắt người, và ba
/// động cơ phản ứng khác nhau với cùng một sự suy yếu.
#[test]
fn che_do_dua_tren_so_hai_mat_tuan_thu_nhanh_hon() {
    let m = Magnitude(600);
    let cap = Capacity::organised();

    let tin_manh = respond(m, &cap, &duoc_tin(), 800);
    let tin_yeu = respond(m, &cap, &duoc_tin(), 100);
    let so_manh = respond(m, &cap, &dua_tren_so_hai(), 800);
    let so_yeu = respond(m, &cap, &dua_tren_so_hai(), 100);

    let sut_vi_tin = tin_yeu
        .casualties_permille
        .saturating_sub(tin_manh.casualties_permille);
    let sut_vi_so = so_yeu
        .casualties_permille
        .saturating_sub(so_manh.casualties_permille);
    assert!(
        sut_vi_so > sut_vi_tin,
        "sợ sụt {sut_vi_so}, tin sụt {sut_vi_tin}"
    );
}

/// Thiên tai nhẹ thì xã hội nào cũng chịu được.
#[test]
fn thien_tai_nhe_thi_xa_hoi_nao_cung_chiu_duoc() {
    let nhe = Magnitude(50);
    assert!(!respond(nhe, &Capacity::organised(), &duoc_tin(), 800).state_collapses);
    assert!(!respond(nhe, &Capacity::hollowed(), &duoc_tin(), 600).state_collapses);
}

/// **Không có năng lực tái thiết thì không tái thiết** — nhưng một nhà nước
/// được tin vẫn đứng vững trên một vùng đổ nát.
///
/// Hai kết quả này tách nhau là chủ đích: `§12.20` nói năng lực và chính danh
/// là hai thứ, nên mất một cái không tự động kéo theo cái kia. Một vùng không
/// dựng lại nổi dưới một chính quyền còn được nghe theo là một tình huống có
/// thật, và nó khác hẳn một cuộc sụp đổ.
#[test]
fn khong_co_nang_luc_tai_thiet_thi_khong_tai_thiet() {
    let co = respond(Magnitude(500), &Capacity::organised(), &duoc_tin(), 800);
    let khong = respond(
        Magnitude(500),
        &Capacity {
            reconstruction: 0,
            ..Capacity::organised()
        },
        &duoc_tin(),
        800,
    );

    assert_eq!(khong.rebuild_years, 99, "không dựng lại nổi");
    assert!(khong.rebuild_years > co.rebuild_years);
    assert!(
        khong.compliance_after < co.compliance_after,
        "và trả giá bằng tuân thủ"
    );
    assert!(
        !khong.state_collapses,
        "nhưng một nhà nước được tin vẫn đứng vững trên đống đổ nát"
    );
}

/// Ngưỡng sụp đổ có tên và kiểm được.
#[test]
fn nguong_sup_do_co_ten_va_kiem_duoc() {
    let ra = respond(
        Magnitude(900),
        &Capacity::hollowed(),
        &dua_tren_so_hai(),
        100,
    );
    assert!(u32::from(ra.compliance_after) < NGUONG_SUP_DO);
    assert!(ra.state_collapses);
}

// ══════════════════ nối hai nửa ══════════════════

/// **Thảm họa đẩy người đi** — và niềm tin của họ đã bị nó làm méo.
#[test]
fn tham_hoa_day_nguoi_di_va_lam_meo_niem_tin() {
    let ra = respond(
        Magnitude(700),
        &Capacity::hollowed(),
        &dua_tren_so_hai(),
        200,
    );
    let sau = displacement_belief(&ra, &tin_binh_thuong());

    assert!(sau.safety_here < tin_binh_thuong().safety_here);
    assert!(sau.wage_here <= tin_binh_thuong().wage_here);

    // Và người trước đó không định đi thì bây giờ đi.
    let truoc = decide(&tin_binh_thuong(), Role::Pioneer);
    let bay_gio = decide(&sau, Role::Pioneer);
    assert!(bay_gio.expected_gain > truoc.expected_gain);
}

/// **Người ta đi tới nơi có người quen**, không tỏa đều tới mọi nơi an toàn.
#[test]
fn nguoi_ta_di_toi_noi_co_nguoi_quen() {
    let cac = vec![
        Diaspora {
            origin: "veskar".into(),
            host: "tolm".into(),
            population: 3_000,
            remittances_per_year: 100,
            keeps_language: true,
            generations: 2,
        },
        Diaspora {
            origin: "veskar".into(),
            host: "arren".into(),
            population: 1_000,
            remittances_per_year: 50,
            keeps_language: true,
            generations: 2,
        },
        // Quá nhỏ để đón ai.
        Diaspora {
            origin: "veskar".into(),
            host: "kesh".into(),
            population: 10,
            remittances_per_year: 0,
            keeps_language: false,
            generations: 1,
        },
    ];

    let dong = flows(8_000, &cac);
    assert_eq!(dong.len(), 2, "kesh quá nhỏ để đón ai: {dong:?}");
    assert_eq!(dong["tolm"], 6_000);
    assert_eq!(dong["arren"], 2_000);
}

/// Không có cộng đồng nào đón thì không có dòng nào — người ta ở lại hoặc
/// đi bừa, và cả hai đều không phải chuyện module này quyết.
#[test]
fn khong_co_cong_dong_nao_don_thi_khong_co_dong_nao() {
    assert!(flows(8_000, &[]).is_empty());
}
