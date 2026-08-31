//! Test tính cách, danh tiếng và trao đổi xã hội (`PC-10`, `PC-11`, `PC-12`).

use mow_math::rng::streams;
use mow_math::{CanonicalHash, RngStreams, WorldSeed};
use mow_society::personality::{CauseKind, CauseRef, Personality, TraitField, Traits, Values};
use mow_society::reputation::{Norm, NormOrder, NormSet, Reputation};
use mow_society::social::{apply_outcome, volition, Bond, Exchange, ExchangeKind, SocialState};

fn rng(seed: u64) -> mow_math::DetRng {
    RngStreams::new(WorldSeed(seed)).stream(streams::SOCIAL_PERSONALITY)
}

// ─────────────────────────── PC-10 · tính cách ───────────────────────────

/// Lấy mẫu độc lập tạo ra những tổ hợp không tồn tại: người vừa rất tận tâm vừa
/// rất bất ổn. Test đo tương quan trên một quần thể để bắt lỗi đó.
#[test]
fn tan_tam_va_bat_on_tuong_quan_am() {
    let mut r = rng(7);
    let mau: Vec<Traits> = (0..2_000).map(|_| Traits::sample(&mut r)).collect();

    let n = mau.len() as i64;
    let tb = |f: fn(&Traits) -> u16| mau.iter().map(|t| i64::from(f(t))).sum::<i64>() / n;
    let (tb_c, tb_n) = (tb(|t| t.conscientiousness), tb(|t| t.neuroticism));

    // Hiệp phương sai, không cần chuẩn hóa: chỉ cần biết dấu.
    let cov: i64 = mau
        .iter()
        .map(|t| (i64::from(t.conscientiousness) - tb_c) * (i64::from(t.neuroticism) - tb_n))
        .sum::<i64>()
        / n;
    assert!(
        cov < -5_000,
        "tận tâm và bất ổn phải tương quan âm rõ rệt, cov = {cov}"
    );
}

#[test]
fn huong_ngoai_va_de_chiu_tuong_quan_duong() {
    let mut r = rng(11);
    let mau: Vec<Traits> = (0..2_000).map(|_| Traits::sample(&mut r)).collect();
    let n = mau.len() as i64;
    let tb = |f: fn(&Traits) -> u16| mau.iter().map(|t| i64::from(f(t))).sum::<i64>() / n;
    let (tb_e, tb_a) = (tb(|t| t.extraversion), tb(|t| t.agreeableness));
    let cov: i64 = mau
        .iter()
        .map(|t| (i64::from(t.extraversion) - tb_e) * (i64::from(t.agreeableness) - tb_a))
        .sum::<i64>()
        / n;
    assert!(
        cov > 5_000,
        "hướng ngoại và dễ chịu phải tương quan dương, cov = {cov}"
    );
}

/// Phân phối phải hình chuông. Nếu mọi nhân vật đều cực đoan thì ngôi làng
/// trông như một đoàn xiếc chứ không như một cộng đồng.
#[test]
fn phan_phoi_hinh_chuong_khong_phai_deu() {
    let mut r = rng(13);
    let mau: Vec<u16> = (0..5_000)
        .map(|_| Traits::sample(&mut r).openness)
        .collect();
    let giua = mau.iter().filter(|v| (300..=700).contains(*v)).count();
    let cuc = mau.iter().filter(|v| **v < 150 || **v > 850).count();
    assert!(
        giua > cuc * 4,
        "người ở giữa ({giua}) phải nhiều hơn hẳn người ở cực ({cuc})"
    );
}

/// Cùng seed ⇒ cùng quần thể. Không có điều này thì không có replay.
#[test]
fn lay_mau_xac_dinh_theo_seed() {
    let a: Vec<Traits> = (0..50).map(|_| Traits::sample(&mut rng(99))).collect();
    let b: Vec<Traits> = (0..50).map(|_| Traits::sample(&mut rng(99))).collect();
    assert_eq!(a, b);
    assert_ne!(a[0], Traits::sample(&mut rng(100)));
}

/// Thứ tự giá trị **là** tính cách: ai cũng coi trọng cả trung thành lẫn thật
/// thà, khác nhau ở chỗ khi hai thứ xung đột thì bỏ cái nào.
#[test]
fn gia_tri_giai_quyet_xung_dot_theo_thu_tu() {
    let trung_thanh_truoc = Values::new(["loyalty".into(), "honesty".into()]);
    let that_tha_truoc = Values::new(["honesty".into(), "loyalty".into()]);
    assert_eq!(
        trung_thanh_truoc.resolve("honesty", "loyalty"),
        Some("loyalty")
    );
    assert_eq!(
        that_tha_truoc.resolve("honesty", "loyalty"),
        Some("honesty")
    );
    // Không quan tâm cả hai ⇒ giá trị không quyết định gì, và nhân vật sẽ quyết
    // bằng thứ khác. Trả `Some` bừa ở đây là bịa ra một niềm tin họ không có.
    assert_eq!(trung_thanh_truoc.resolve("greed", "sloth"), None);
}

/// Lớp tự sự có thể **sai** về chính mình — và khoảng cách đó là dữ liệu cho
/// `PC-13`, không phải lỗi cần sửa.
#[test]
fn tu_su_co_the_mau_thuan_voi_dac_diem_that() {
    let goc = Traits {
        openness: 500,
        conscientiousness: 900,
        extraversion: 500,
        agreeableness: 500,
        neuroticism: 900,
    };
    let mut p = Personality::from_traits(goc);
    p.narrative.claims = vec!["dũng cảm".into(), "giữ lời".into()];

    let mt = p.self_contradictions();
    assert_eq!(mt, vec!["dũng cảm"], "chỉ 'dũng cảm' mới mâu thuẫn");

    // Một người tận tâm thật thì lời tự nhận 'giữ lời' không phải mâu thuẫn.
    // Đổi phải đi qua đường có nguyên nhân — đó là bất biến của `§20.11.4`.
    p.apply_change(
        100,
        TraitField::Conscientiousness,
        -800,
        CauseRef {
            event_seq: 1,
            kind: CauseKind::Addiction,
        },
    );
    assert!(p.self_contradictions().contains(&"giữ lời"));
}

#[test]
fn tinh_cach_vao_state_hash() {
    let a = Personality::sample(&mut rng(1));
    let mut b = a.clone();
    assert_eq!(a.state_hash(), b.state_hash());
    b.affect.valence = 1;
    assert_ne!(a.state_hash(), b.state_hash());
}

// ────────────────────────── PC-11 · danh tiếng ──────────────────────────

/// Danh tiếng là **khóa ba**: ai tin, về ai, về chuyện gì. Gộp lại thành "điểm
/// danh tiếng" toàn cục là xóa mất khả năng hai người bất đồng về cùng một
/// người thứ ba — thứ mà mọi tin đồn đều dựa vào.
#[test]
fn hai_nguoi_co_the_bat_dong_ve_nguoi_thu_ba() {
    let mut r = Reputation::new();
    r.observe(1, 3, "honesty", 800, 10);
    r.observe(2, 3, "honesty", -600, 10);
    assert!(r.get(1, 3, "honesty").unwrap().value > 0);
    assert!(r.get(2, 3, "honesty").unwrap().value < 0);

    let (min, max) = r.disagreement(3, "honesty").unwrap();
    assert!(max - min > 1_000, "bất đồng phải đo được, không bị gộp mất");
}

/// Một lời vu khống **không được** xóa cái đã thấy tận mắt.
#[test]
fn mot_loi_don_khong_lat_duoc_dieu_da_thay() {
    let mut r = Reputation::new();
    r.observe(1, 2, "honesty", 900, 10);
    r.hear(1, 2, "honesty", -900, 11);
    let b = r.get(1, 2, "honesty").unwrap();
    assert!(
        b.value > 500,
        "một lời đồn không được lật ngược điều đã thấy, value = {}",
        b.value
    );
    assert!(b.firsthand >= 1);
}

/// Chín lời đồn ngược lại thì sao? Kết quả đúng là **hoang mang**, không phải
/// **tin chắc điều ngược lại**.
///
/// Đây là chỗ mà một mô hình danh tiếng chỉ có một con số sẽ sai mà không ai
/// nhận ra: nó sẽ cho ra `-624`, và `-624` đọc lên là *"tôi biết chắc hắn là kẻ
/// lừa đảo"* — trong khi trạng thái thật của nhân vật là *"tôi không còn biết
/// phải nghĩ sao"*. Hai trạng thái đó dẫn tới hành vi hoàn toàn khác nhau, và
/// chỉ diễn đạt được khi `value` và `confidence` tách rời.
#[test]
fn nhieu_loi_don_nguoc_lai_tao_ra_hoang_mang_khong_phai_tin_chac() {
    let mut r = Reputation::new();
    r.observe(1, 2, "honesty", 900, 10);
    let chac_ban_dau = r.get(1, 2, "honesty").unwrap().confidence;

    for t in 11..20 {
        r.hear(1, 2, "honesty", -900, t);
    }
    let b = r.get(1, 2, "honesty").unwrap();

    assert!(
        b.confidence < chac_ban_dau,
        "bằng chứng mâu thuẫn phải làm giảm độ chắc chắn, {} → {}",
        chac_ban_dau,
        b.confidence
    );
    assert!(
        b.confidence < 150,
        "sau chín lời đồn ngược, nhân vật phải hoang mang, confidence = {}",
        b.confidence
    );
    // Và belief hoang mang thì gần như không ảnh hưởng tới quyết định — đó là
    // toàn bộ lý do tách `confidence` ra khỏi `value`.
    assert!(
        i64::from(b.value) * i64::from(b.confidence) / 1000 / 10 > -20,
        "một belief hoang mang không được nặng như một điều biết chắc"
    );
}

/// Nhưng belief chỉ dựa trên tin đồn thì **phải** dễ đổi — đó là cách một lời
/// vu khống bị lật lại được.
#[test]
fn belief_tin_don_de_doi_hon() {
    let mut r = Reputation::new();
    r.hear(1, 2, "honesty", -800, 10);
    let truoc = r.get(1, 2, "honesty").unwrap().value;
    r.observe(1, 2, "honesty", 800, 11);
    let sau = r.get(1, 2, "honesty").unwrap().value;
    assert!(
        sau - truoc > 400,
        "một lần thấy tận mắt phải lật được tin đồn"
    );
}

/// Độ chắc chắn tách khỏi giá trị: "tôi chắc chắn hắn lương thiện" và "tôi hơi
/// nghi hắn lương thiện" dẫn tới hành vi khác nhau dù cùng dấu.
#[test]
fn chac_chan_tang_theo_so_lan_quan_sat() {
    let mut r = Reputation::new();
    r.observe(1, 2, "honesty", 500, 1);
    let c1 = r.get(1, 2, "honesty").unwrap().confidence;
    for t in 2..10 {
        r.observe(1, 2, "honesty", 500, t);
    }
    assert!(r.get(1, 2, "honesty").unwrap().confidence > c1);
}

/// Chuẩn mực bậc hai là thứ khiến hợp tác bền vững được. Không có nó, trừng
/// phạt là hành động tốn kém mà không ai có động cơ làm.
#[test]
fn chuan_muc_bac_hai_ton_tai_rieng() {
    let mut ns = NormSet::new("core.village");
    ns.add(Norm {
        id: "core.no_theft".into(),
        order: NormOrder::First,
        act: "steal".into(),
        disapproval: 800,
        compliance: 900,
    });
    assert!(!ns.has_second_order_for("steal"));

    ns.add(Norm {
        id: "core.punish_the_silent".into(),
        order: NormOrder::Second,
        act: "steal".into(),
        disapproval: 500,
        compliance: 600,
    });
    assert!(ns.has_second_order_for("steal"));
    assert_eq!(ns.for_act("steal").len(), 2);
}

/// Phản đối cao mà tuân thủ thấp = chuẩn mực sắp sụp. Khoảng lệch **là** thông
/// tin; gộp hai trường thành một sẽ xóa mất nó.
#[test]
fn chuan_muc_sap_sup_nhan_ra_duoc() {
    let sap_sup = Norm {
        id: "core.no_smuggling".into(),
        order: NormOrder::First,
        act: "smuggle".into(),
        disapproval: 900,
        compliance: 200,
    };
    let vung = Norm {
        compliance: 900,
        ..sap_sup.clone()
    };
    assert!(sap_sup.is_collapsing());
    assert!(!vung.is_collapsing());
}

// ───────────────────────── PC-12 · trao đổi xã hội ─────────────────────────

fn rong() -> Reputation {
    Reputation::new()
}

/// Ranh giới của `PC-12`: cùng tình huống ⇒ cùng ý chí, không lời gọi mô hình.
/// Không có tính chất này thì người chơi không học được luật nào cả.
#[test]
fn y_chi_la_ham_thuan_lap_lai_duoc() {
    let mut s = SocialState::new();
    s.set_bond(
        2,
        1,
        Bond {
            affinity: 300,
            obligation: 0,
            trust: 500,
        },
    );
    let ex = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Request,
        cost: 100,
    };
    let a = volition(&ex, &s, &rong());
    let b = volition(&ex, &s, &rong());
    assert_eq!(a, b);
}

/// Có đi có lại: làm ơn trước thì lần sau được đồng ý. Không có `obligation`,
/// giúp đỡ là mất mát thuần túy và xã hội chỉ có bạn thân với người dưng.
#[test]
fn lam_on_truoc_thi_lan_sau_de_duoc_dong_y() {
    let mut s = SocialState::new();
    // Người lạ, đòi hỏi lớn ⇒ từ chối.
    let xin = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Request,
        cost: 400,
    };
    assert!(!volition(&xin, &s, &rong()).accepts);

    // 1 tặng 2 một món hậu hĩnh trước.
    let tang = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Gift,
        cost: 600,
    };
    let v = volition(&tang, &s, &rong());
    assert!(
        v.accepts,
        "món quà hậu hĩnh phải được nhận: {:?}",
        v.factors
    );
    apply_outcome(&tang, &v, &mut s);

    // Giờ 2 mắc nợ 1.
    assert!(
        s.bond(2, 1).obligation > 0,
        "người nhận quà phải mang nợ nghĩa, obligation = {}",
        s.bond(2, 1).obligation
    );
    // Và cùng lời xin đó, giờ được đồng ý.
    assert!(
        volition(&xin, &s, &rong()).accepts,
        "làm ơn trước rồi mà lần sau vẫn bị từ chối thì có đi có lại là vô nghĩa"
    );
}

/// **Càng hào phóng càng bị từ chối** là điều ngược đời mà một mô hình chỉ có
/// một trường `cost` sẽ tạo ra: nếu cái giá luôn do người nhận trả, thì tặng
/// một món càng quý càng dễ bị cự tuyệt.
#[test]
fn tang_cang_hau_cang_de_duoc_nhan() {
    let s = SocialState::new();
    let nho = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Gift,
        cost: 50,
    };
    let lon = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Gift,
        cost: 900,
    };
    assert!(
        volition(&lon, &s, &rong()).score > volition(&nho, &s, &rong()).score,
        "món quà lớn hơn phải hấp dẫn hơn, không phải khó chấp nhận hơn"
    );

    // Còn lời xin thì ngược lại: xin càng nhiều càng khó được cho.
    let xin_it = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Request,
        cost: 50,
    };
    let xin_nhieu = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Request,
        cost: 900,
    };
    assert!(volition(&xin_it, &s, &rong()).score > volition(&xin_nhieu, &s, &rong()).score);
}

/// Kẻ bị ép **không mang ơn** kẻ ép mình. Nếu phục tùng dưới sức ép cũng sinh
/// nợ nghĩa, thì bắt nạt trở thành một cách xây dựng quan hệ.
#[test]
fn phuc_tung_duoi_suc_ep_khong_sinh_no_nghia() {
    let mut s = SocialState::new();
    s.set_bond(
        2,
        1,
        Bond {
            affinity: 0,
            obligation: 0,
            trust: 0,
        },
    );
    let ep = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Threat,
        cost: 300,
    };
    let v = volition(&ep, &s, &rong());
    assert!(v.accepts);
    apply_outcome(&ep, &v, &mut s);
    assert!(
        s.bond(2, 1).obligation <= 0,
        "nạn nhân không được mang ơn kẻ đe dọa, obligation = {}",
        s.bond(2, 1).obligation
    );
}

/// Đe dọa **thành công** vẫn làm hỏng quan hệ. Một hệ thống chỉ tính "được việc
/// hay không" sẽ khiến đe dọa luôn là nước đi tối ưu, và cả thế giới thành côn đồ.
#[test]
fn de_doa_thanh_cong_van_pha_quan_he() {
    let mut s = SocialState::new();
    s.set_bond(
        2,
        1,
        Bond {
            affinity: 500,
            obligation: 0,
            trust: 500,
        },
    );
    let ex = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Threat,
        cost: 100,
    };
    let v = volition(&ex, &s, &rong());
    assert!(
        v.accepts,
        "đe dọa rẻ với người không đủ mạnh phải có tác dụng"
    );

    let truoc = s.bond(2, 1).affinity;
    apply_outcome(&ex, &v, &mut s);
    assert!(
        s.bond(2, 1).affinity < truoc,
        "đe dọa thành công vẫn phải làm nạn nhân ghét kẻ ép mình: {} → {}",
        truoc,
        s.bond(2, 1).affinity
    );
}

/// Danh tiếng phải thắng được thiện cảm: tin ai đó là kẻ lừa đảo là một lý do
/// để từ chối, dù quý họ.
#[test]
fn danh_tieng_xau_lat_duoc_thien_cam() {
    let mut s = SocialState::new();
    s.set_bond(
        2,
        1,
        Bond {
            affinity: 400,
            obligation: 0,
            trust: 300,
        },
    );
    let ex = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Bargain,
        cost: 200,
    };
    assert!(volition(&ex, &s, &rong()).accepts);

    let mut r = Reputation::new();
    for t in 1..12 {
        r.observe(2, 1, "honesty", -900, t);
    }
    let v = volition(&ex, &s, &r);
    assert!(
        !v.accepts,
        "biết chắc là kẻ lừa đảo mà vẫn đồng ý: {:?}",
        v.factors
    );
}

/// Mọi quyết định phải **giải thích được** (`§18.13`). Một con số không có phân
/// rã là một lời phán, và người chơi không học được gì từ lời phán.
#[test]
fn y_chi_luon_giai_thich_duoc() {
    let mut s = SocialState::new();
    s.set_bond(
        2,
        1,
        Bond {
            affinity: -200,
            obligation: 50,
            trust: 100,
        },
    );
    let ex = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Request,
        cost: 300,
    };
    let v = volition(&ex, &s, &rong());

    assert!(!v.factors.is_empty());
    assert_eq!(
        v.score,
        v.factors.iter().map(|(_, x)| x).sum::<i64>(),
        "tổng phải bằng đúng các phần — nếu không, có một số hạng giấu mặt"
    );
    for ten in ["thiện cảm", "cái giá"] {
        assert!(
            v.factors.iter().any(|(n, _)| *n == ten),
            "thiếu phần `{ten}`"
        );
    }
}

/// Quan hệ **có hướng**: Aren quý Bram nhiều hơn Bram quý Aren là chuyện rất
/// thường, và là nguồn của kịch tính.
#[test]
fn quan_he_co_huong_khong_doi_xung() {
    let mut s = SocialState::new();
    s.set_bond(
        1,
        2,
        Bond {
            affinity: 700,
            obligation: 0,
            trust: 600,
        },
    );
    s.set_bond(
        2,
        1,
        Bond {
            affinity: 100,
            obligation: 0,
            trust: 200,
        },
    );
    assert_eq!(s.asymmetry(1, 2), 600);
    assert_eq!(s.asymmetry(2, 1), -600);
}

/// Vòng lặp phải khép: đề nghị → ý chí → kết quả → quan hệ đổi. Không có bước
/// cuối thì mọi cuộc trò chuyện đều là lần đầu tiên.
#[test]
fn tu_choi_lien_tuc_lam_xau_quan_he() {
    let mut s = SocialState::new();
    let ex = Exchange {
        from: 1,
        to: 2,
        kind: ExchangeKind::Request,
        cost: 900,
    };
    for _ in 0..10 {
        let v = volition(&ex, &s, &rong());
        assert!(!v.accepts);
        apply_outcome(&ex, &v, &mut s);
    }
    assert!(
        s.bond(1, 2).affinity < 0,
        "bị từ chối mãi thì quan hệ phải xấu đi"
    );
}

#[test]
fn social_state_vao_state_hash() {
    let mut a = SocialState::new();
    a.set_bond(
        1,
        2,
        Bond {
            affinity: 10,
            obligation: 0,
            trust: 0,
        },
    );
    let mut b = SocialState::new();
    b.set_bond(
        1,
        2,
        Bond {
            affinity: 10,
            obligation: 0,
            trust: 0,
        },
    );
    assert_eq!(a.state_hash(), b.state_hash());

    // Chiều ngược lại là một quan hệ khác, không phải cùng một quan hệ.
    b.set_bond(
        2,
        1,
        Bond {
            affinity: 10,
            obligation: 0,
            trust: 0,
        },
    );
    assert_ne!(a.state_hash(), b.state_hash());
}
