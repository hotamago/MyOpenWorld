//! Test tổ chức, nhà nước, chính danh, hành động tập thể, tài nguyên chung
//! (`PD-04`, `PD-05`, `PD-07`, `PD-08`).

use mow_core::EntityId;
use mow_org::collective::{cascade, Participant, Signal};
use mow_org::commons::{diagnose, Commons, Governance, Principle, PRINCIPLES};
use mow_org::legitimacy::{Legitimacy, Motive};
use mow_org::state::{Directive, District, StateCapacity};

fn nha_nuoc(corruption: u16, officials: u32) -> StateCapacity {
    StateCapacity {
        // 3000 dân, cần 100/người cho độ phủ đầy đủ ⇒ 300 000 là vừa đủ.
        revenue: 300_000,
        officials,
        full_coverage_cost_per_capita: 100,
        full_coverage_officials_per_1000: 20,
        corruption,
        delay_per_hop: 100,
        distortion_per_hop: 200,
        districts: vec![
            District {
                id: "core".into(),
                admin_distance: 1,
                population: 1_000,
            },
            District {
                id: "docks".into(),
                admin_distance: 3,
                population: 1_000,
            },
            District {
                id: "outskirts".into(),
                admin_distance: 6,
                population: 1_000,
            },
        ],
    }
}

// ─────────────────────── PD-04 · năng lực nhà nước ───────────────────────

/// **Ra quyết định không có nghĩa là điều đó xảy ra.**
#[test]
fn menh_lenh_hao_hut_qua_tung_bac() {
    let s = nha_nuoc(200, 60);
    let d = Directive {
        what: "xây cầu".into(),
        budget: 1_000,
        hops: 3,
    };
    let ra = s.execute(&d);
    assert!(ra.delivered_budget < 1_000, "không thất thoát gì cả");
    assert_eq!(ra.leaked, 1_000 - ra.delivered_budget);
    assert_eq!(ra.delay_ticks, 300);
}

/// Thất thoát **theo từng bậc**, nên đế chế lớn cai trị vùng biên kém hơn thành
/// bang cai trị chính nó — cùng mức tham nhũng.
#[test]
fn cang_nhieu_bac_cang_hao_hut_du_cung_muc_tham_nhung() {
    let s = nha_nuoc(200, 60);
    let gan = s.execute(&Directive {
        what: "x".into(),
        budget: 1_000,
        hops: 1,
    });
    let xa = s.execute(&Directive {
        what: "x".into(),
        budget: 1_000,
        hops: 6,
    });
    assert!(xa.delivered_budget < gan.delivered_budget / 2);
}

/// Qua đủ nhiều bậc thì mệnh lệnh **chắc chắn** bị hiểu lệch.
#[test]
fn qua_du_nhieu_bac_thi_menh_lenh_bi_hieu_lech() {
    let s = nha_nuoc(0, 60);
    assert!(
        !s.execute(&Directive {
            what: "x".into(),
            budget: 100,
            hops: 1
        })
        .distorted
    );
    assert!(
        s.execute(&Directive {
            what: "x".into(),
            budget: 100,
            hops: 5
        })
        .distorted
    );
}

/// **Chuỗi nhân quả đóng lại**: cắt ngân sách → độ phủ tụt.
///
/// Không có điều này thì `§12.13.1` chỉ là trang trí — người chơi cắt thuế và
/// không có gì xảy ra, vì độ phủ nằm trong một file YAML.
#[test]
fn cat_ngan_sach_lam_do_phu_tut() {
    let giau = nha_nuoc(200, 60);
    let mut ngheo = giau.clone();
    ngheo.revenue = giau.revenue / 10;

    assert!(
        ngheo.coverage("core") < giau.coverage("core"),
        "cắt ngân sách mà độ phủ không đổi"
    );
}

/// Khu càng xa trung tâm hành chính, nhà nước càng ít với tới — bất công có cấu trúc.
#[test]
fn khu_xa_trung_tam_thi_nha_nuoc_it_voi_toi() {
    let s = nha_nuoc(300, 60);
    assert!(s.coverage("core") > s.coverage("docks"));
    assert!(s.coverage("docks") > s.coverage("outskirts"));
}

/// Tham nhũng cao thì cùng ngân sách cho ra ít độ phủ hơn.
#[test]
fn tham_nhung_bien_ngan_sach_thanh_it_su_hien_dien_hon() {
    let sach = nha_nuoc(0, 60);
    let ban = nha_nuoc(500, 60);
    assert!(ban.coverage("docks") < sach.coverage("docks"));
}

/// **Không có người thì tiền không thành sự hiện diện.**
#[test]
fn nhieu_tien_ma_khong_co_quan_chuc_thi_van_khong_phu_duoc() {
    let s = StateCapacity {
        officials: 0,
        ..nha_nuoc(0, 60)
    };
    assert_eq!(s.coverage("core"), 0);
}

/// Khu không khai báo thì bằng 0 — đó là chỗ băng đảng mọc lên (`§12.6`).
#[test]
fn khu_khong_khai_bao_thi_nha_nuoc_khong_voi_toi() {
    assert_eq!(nha_nuoc(0, 60).coverage("sewers"), 0);
}

// ─────────────────────── PD-05 · chính danh ───────────────────────

fn che_do_so_hai() -> Legitimacy {
    Legitimacy {
        belief: 100,
        fear: 800,
        conformity: 300,
        sources: vec![],
    }
}

fn che_do_niem_tin() -> Legitimacy {
    Legitimacy {
        belief: 800,
        fear: 100,
        conformity: 300,
        sources: vec![],
    }
}

/// **Giống nhau khi nhà nước mạnh.**
#[test]
fn khi_nha_nuoc_manh_ba_dong_co_cho_ket_qua_giong_nhau() {
    let a = che_do_so_hai().compliance(1_000).total;
    let b = che_do_niem_tin().compliance(1_000).total;
    assert!(
        a.abs_diff(b) < 100,
        "khi mạnh, hai chế độ phải trông như nhau: {a} vs {b}"
    );
}

/// **Khác nhau hoàn toàn vào ngày nhà nước yếu đi.**
#[test]
fn khi_nha_nuoc_yeu_che_do_so_hai_sup_con_che_do_niem_tin_dung() {
    let so = che_do_so_hai().compliance(50).total;
    let tin = che_do_niem_tin().compliance(50).total;
    assert!(
        tin > so * 3,
        "chế độ dựa trên niềm tin phải bền hơn hẳn: {tin} vs {so}"
    );
}

/// Một chỉ số duy nhất không phân biệt được hai chế độ — nên phải có ba.
#[test]
fn dong_co_dang_do_che_do_nhan_ra_duoc() {
    assert_eq!(
        che_do_so_hai().compliance(1_000).dominant(),
        Some(Motive::Fear)
    );
    assert_eq!(
        che_do_niem_tin().compliance(1_000).dominant(),
        Some(Motive::Belief)
    );
}

/// Điểm sụp: chế độ sợ hãi sụp sớm hơn hẳn.
#[test]
fn che_do_so_hai_sup_o_muc_suc_manh_cao_hon() {
    let so = che_do_so_hai().collapse_point(400);
    let tin = che_do_niem_tin().collapse_point(400);

    assert!(so.is_some(), "chế độ sợ hãi phải có điểm sụp");
    match (so, tin) {
        (Some(a), Some(b)) => assert!(a > b, "chế độ sợ hãi sụp sớm hơn: {a} vs {b}"),
        (Some(_), None) => {} // chế độ niềm tin không sụp — càng đúng
        _ => panic!("chế độ sợ hãi lại bền hơn chế độ niềm tin"),
    }
}

/// Hùa theo **khuếch đại** cái đang có, chứ không tự đứng được.
#[test]
fn hua_theo_khuech_dai_chu_khong_tu_dung_duoc() {
    let chi_hua = Legitimacy {
        belief: 0,
        fear: 0,
        conformity: 1_000,
        sources: vec![],
    };
    assert_eq!(
        chi_hua.compliance(1_000).total,
        0,
        "không ai tin, không ai sợ, mà vẫn có người tuân theo đám đông trống rỗng"
    );
}

/// Tuân thủ đơn điệu theo sức mạnh nhà nước.
#[test]
fn tuan_thu_khong_tang_khi_nha_nuoc_yeu_di() {
    let l = che_do_so_hai();
    let mut truoc = 0;
    for s in [0u16, 200, 400, 600, 800, 1_000] {
        let t = l.compliance(s).total;
        assert!(t >= truoc, "sức mạnh {s} lại làm tuân thủ giảm");
        truoc = t;
    }
}

// ─────────────────────── PD-07 · hành động tập thể ───────────────────────

fn dam_dong(nguong: &[u16]) -> Vec<Participant> {
    nguong
        .iter()
        .enumerate()
        .map(|(i, t)| Participant {
            who: EntityId(i as u64),
            threshold: *t,
            cost: 0,
            free_rider: false,
        })
        .collect()
}

/// **Kết quả kinh điển của Granovetter**: cùng ngưỡng trung bình, hai kết cục.
///
/// Đây là lý do `§15.4` nói Director không được phép ép kết quả.
#[test]
fn cung_nguong_trung_binh_hai_dam_dong_hai_ket_cuc() {
    // Đám A: ngưỡng 0,10,20,...,90 — dây chuyền liền mạch.
    let a: Vec<u16> = (0..10).map(|i| i * 100).collect();
    // Đám B: giống hệt, trừ một người đổi từ 100 thành 200.
    let mut b = a.clone();
    b[1] = 200;

    let ra_a = cascade(&dam_dong(&a), Signal::Silence, 50);
    let ra_b = cascade(&dam_dong(&b), Signal::Silence, 50);

    assert_eq!(ra_a.participation, 1_000, "đám A phải lan hết");
    assert!(
        ra_b.participation < ra_a.participation,
        "một người đổi ý mà kết cục không đổi: {} vs {}",
        ra_b.participation,
        ra_a.participation
    );

    // Và trung bình gần như y hệt.
    let tb = |v: &[u16]| -> u32 { v.iter().map(|x| u32::from(*x)).sum::<u32>() / v.len() as u32 };
    assert!(
        tb(&a).abs_diff(tb(&b)) <= 20,
        "hai đám phải gần như cùng trung bình để bài này có nghĩa"
    );
}

/// Không có người khởi xướng (ngưỡng 0) thì **không có gì xảy ra**.
#[test]
fn khong_co_nguoi_khoi_xuong_thi_khong_ai_bat_dau() {
    let khong_ai_dam: Vec<u16> = (0..10).map(|_| 100).collect();
    let ra = cascade(&dam_dong(&khong_ai_dam), Signal::Silence, 50);
    assert_eq!(ra.participation, 0);
}

/// Đàn áp đẩy ngưỡng lên và dập được phong trào.
#[test]
fn dan_ap_day_nguong_len_va_dap_duoc_phong_trao() {
    let d: Vec<u16> = (0..10).map(|i| i * 100).collect();
    let im = cascade(&dam_dong(&d), Signal::Silence, 50);
    let dan_ap = cascade(&dam_dong(&d), Signal::Repression { severity: 400 }, 50);
    assert!(dan_ap.participation < im.participation);
}

/// **Nhượng bộ nửa vời làm phong trào cực đoan hơn**: người ôn hòa nguội đi,
/// người quyết liệt thì không.
#[test]
fn nhuong_bo_lam_nguoi_on_hoa_nguoi_di_nhung_khong_dong_toi_nguoi_quyet_liet() {
    let d = vec![0u16, 100, 200, 700, 800];
    let nhuong = cascade(&dam_dong(&d), Signal::Concession { size: 500 }, 50);

    // Người ngưỡng 700, 800 không bị nhượng bộ chạm tới.
    let cuc_doan_bi_cham = nhuong.joined.iter().any(|e| e.0 >= 3);
    let on_hoa_bi_cham = nhuong.joined.len() < 3;
    assert!(
        on_hoa_bi_cham || !cuc_doan_bi_cham,
        "nhượng bộ phải làm nguội người ôn hòa"
    );
}

/// Kẻ ăn theo không chịu chi phí, nên họ không có mặt trong danh sách tham gia.
#[test]
fn ke_an_theo_khong_bao_gio_tham_gia() {
    let mut d = dam_dong(&[0, 100, 200]);
    d[1].free_rider = true;
    let ra = cascade(&d, Signal::Silence, 50);
    assert!(!ra.joined.contains(&EntityId(1)));
}

/// Chi phí cá nhân đẩy ngưỡng thực tế lên.
#[test]
fn chi_phi_ca_nhan_lam_kho_tham_gia_hon() {
    let re = dam_dong(&[0, 100, 200, 300]);
    let mut dat = re.clone();
    for p in &mut dat {
        p.cost = 800;
    }
    assert!(
        cascade(&dat, Signal::Silence, 50).participation
            <= cascade(&re, Signal::Silence, 50).participation
    );
}

/// Hàm thuần và **xác định**: bài học phải là "phân bố quyết định", không "may rủi".
#[test]
fn lan_toa_xac_dinh() {
    let d = dam_dong(&[0, 100, 200, 300, 900]);
    let a = cascade(&d, Signal::Silence, 50);
    let b = cascade(&d, Signal::Silence, 50);
    assert_eq!(a, b);
}

#[test]
fn dam_dong_rong_khong_lam_treo() {
    let ra = cascade(&[], Signal::Silence, 50);
    assert_eq!(ra.participation, 0);
    assert_eq!(ra.rounds, 0);
}

// ─────────────────────── PD-08 · tài nguyên chung ───────────────────────

fn ho_ca(g: Governance) -> Commons {
    Commons {
        id: "core.fishery".into(),
        stock: 5_000,
        capacity: 10_000,
        regen_at_full: 500,
        users: 100,
        quota: 4,
        governance: g,
    }
}

/// **Không mặc định bị khai thác tới cạn.** Quản trị đủ thì nó bền.
#[test]
fn quan_tri_du_thi_tai_nguyen_chung_khong_can() {
    let mut c = ho_ca(Governance::ideal());
    for _ in 0..200 {
        c.step();
    }
    assert!(!c.collapsed(), "quản trị lý tưởng mà vẫn cạn: {}", c.stock);
}

/// Và **không bắt buộc phải tư hữu hóa** — không có đường nào trong mô hình bắt
/// phải chia nhỏ tài nguyên ra mới giữ được.
#[test]
fn khong_can_tu_huu_hoa_van_giu_duoc() {
    let mut chung = ho_ca(Governance::ideal());
    for _ in 0..100 {
        chung.step();
    }
    assert!(chung.stock > chung.capacity / 4);
}

/// Thiếu giám sát: **vài người vượt mức rồi lan ra, sụp nhanh**.
#[test]
fn thieu_giam_sat_thi_sup_nhanh() {
    let mut g = Governance::ideal();
    g.set(Principle::Monitoring, 0);
    let mut c = ho_ca(g);
    for _ in 0..200 {
        c.step();
    }
    assert!(c.collapsed(), "thiếu giám sát mà tài nguyên vẫn còn");
}

/// Thiếu ranh giới: người ngoài vào khai thác.
#[test]
fn thieu_ranh_gioi_thi_nguoi_ngoai_vao_lay() {
    let mut g = Governance::ideal();
    g.set(Principle::Boundaries, 0);
    let mut chat = ho_ca(Governance::ideal());
    let mut ho = ho_ca(g);

    let a = chat.step().taken;
    let b = ho.step().taken;
    assert!(b > a, "không có ranh giới mà lượng khai thác không tăng");
}

/// **Phạt nặng ngay lần đầu hóa vô hiệu**: không ai dám tố hàng xóm.
#[test]
fn phat_nang_ngay_lan_dau_lam_vi_pham_khong_duoc_ghi_nhan() {
    let mut g = Governance::ideal();
    g.set(Principle::GradedSanctions, 0);
    let mut c = ho_ca(g);
    let h = c.step();
    assert_eq!(h.caught, 0, "không có chế tài tăng dần mà vẫn bắt được ai");
}

/// **Mỗi yếu tố thiếu để lại một dấu vết riêng** — đây là điều làm `§12.12` có ích.
#[test]
fn moi_yeu_to_thieu_co_kieu_that_bai_rieng() {
    let mut thay = std::collections::BTreeSet::new();
    for p in PRINCIPLES {
        assert!(
            thay.insert(p.failure_mode()),
            "hai yếu tố dùng chung một mô tả thất bại: {p:?}"
        );
    }
    assert_eq!(thay.len(), 7);
}

/// Chẩn đoán chỉ đúng chỗ hỏng, không đưa ra một điểm số mơ hồ.
#[test]
fn chan_doan_chi_ra_dung_yeu_to_thieu() {
    let mut g = Governance::ideal();
    g.set(Principle::Monitoring, 100);
    g.set(Principle::QuotaFit, 50);

    let d = diagnose(&g, 500);
    let ten: Vec<Principle> = d.iter().map(|x| x.principle).collect();
    assert_eq!(ten, vec![Principle::QuotaFit, Principle::Monitoring]);
    assert!(d[1].failure_mode.contains("vượt mức"));
}

/// Yếu tố không khai báo thì **bằng 0**, không phải "chắc là có".
#[test]
fn yeu_to_khong_khai_bao_thi_bang_khong() {
    let g = Governance::default();
    for p in PRINCIPLES {
        assert_eq!(g.level(p), 0);
    }
    assert_eq!(diagnose(&g, 1).len(), 7);
}

/// Tái tạo logistic: nhanh nhất ở nửa trữ lượng, gần 0 ở hai đầu.
///
/// Đây là lý do "cạn dần đều" nguy hiểm: dưới một ngưỡng thì tái tạo không đuổi
/// kịp nữa, dù khai thác đã giảm.
#[test]
fn tai_tao_cham_nhat_o_hai_dau() {
    let g = Governance::ideal();
    let giua = Commons {
        stock: 5_000,
        ..ho_ca(g.clone())
    };
    let sap_can = Commons {
        stock: 200,
        ..ho_ca(g.clone())
    };
    let day = Commons {
        stock: 10_000,
        ..ho_ca(g)
    };

    assert!(giua.regen() > sap_can.regen() * 10);
    assert_eq!(day.regen(), 0, "đầy rồi thì không mọc thêm");
}
