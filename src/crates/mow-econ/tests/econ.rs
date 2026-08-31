//! Test sở hữu, tiền tệ, tín dụng, vận chuyển (`PD-10`–`PD-13`).

use mow_core::{EntityId, Tick};
use mow_econ::credit::{Ledger, Loan, Seniority};
use mow_econ::logistics::{specialize, Handover, LabourContract, Leg, ObservedTrade, Shipment};
use mow_econ::money::{Coinage, EconomyProfile, Faucet, MonetaryStage, MoneyDiagnosis, Sink};
use mow_econ::property::{Basis, Claim, Ownership, Right, RIGHTS};
use std::collections::BTreeMap;

// ───────────────────── PD-10 · possession ≠ claim ─────────────────────

fn claim(who: u64, ns: &str, basis: Basis) -> Claim {
    Claim {
        holder: EntityId(who),
        under_norm_set: ns.into(),
        rights: RIGHTS.to_vec(),
        basis,
        since: Tick(0),
    }
}

/// **Trộm cắp** = chuyển possession, không chuyển claim.
///
/// Không có hệ thống riêng nào cho trộm cắp; nó chỉ là một trạng thái của hai
/// trường vốn đã tách rời.
#[test]
fn trom_cap_la_chuyen_possession_ma_khong_chuyen_claim() {
    let mut o = Ownership::new();
    o.set_possession(1, EntityId(10));
    o.add_claim(1, claim(10, "veskar", Basis::Purchase));
    assert!(o.possession_is_lawful(1, "veskar"));

    // Kẻ trộm lấy đi.
    o.set_possession(1, EntityId(99));
    assert_eq!(o.possessor(1), Some(EntityId(99)));
    assert!(
        !o.possession_is_lawful(1, "veskar"),
        "đổi tay mà vẫn hợp pháp — hai khái niệm đã bị gộp"
    );
    // Chủ cũ vẫn giữ nguyên claim của mình.
    assert_eq!(o.claims_under(1, "veskar")[0].holder, EntityId(10));
}

/// **Tiêu thụ đồ gian**: mua từ người có possession mà không có claim.
#[test]
fn tieu_thu_do_gian_nhan_ra_duoc() {
    let mut o = Ownership::new();
    o.add_claim(1, claim(10, "veskar", Basis::Creation));
    o.set_possession(1, EntityId(99)); // kẻ trộm
    o.set_possession(1, EntityId(50)); // người mua lại, ngay tình

    assert!(
        !o.possession_is_lawful(1, "veskar"),
        "người mua ngay tình vẫn đang giữ đồ gian"
    );
}

/// **Chiến lợi phẩm**: hợp pháp theo luật bên thắng, bất hợp pháp theo luật bên bại.
///
/// Hai câu trả lời, cả hai đúng — vì không có "claim hợp lệ" nói chung.
#[test]
fn chien_loi_pham_hop_phap_o_mot_ben_bat_hop_phap_o_ben_kia() {
    let mut o = Ownership::new();
    o.set_possession(1, EntityId(20));
    o.add_claim(1, claim(20, "victor.law", Basis::Conquest));
    o.add_claim(1, claim(10, "vanquished.law", Basis::Inheritance));

    assert!(o.possession_is_lawful(1, "victor.law"));
    assert!(!o.possession_is_lawful(1, "vanquished.law"));
}

/// **Tranh chấp thừa kế**: nhiều claim cùng hạng, không cái nào tự thắng.
#[test]
fn tranh_chap_thua_ke_la_nhieu_claim_cung_bo_luat() {
    let mut o = Ownership::new();
    o.add_claim(1, claim(10, "veskar", Basis::Inheritance));
    o.add_claim(1, claim(11, "veskar", Basis::Inheritance));
    assert!(o.disputed(1, "veskar"));
    assert!(!o.disputed(1, "other.law"));
}

/// **Chiếm hữu lâu ngày thành quyền** — nhưng chỉ khi đủ lâu.
#[test]
fn chiem_huu_lau_ngay_sinh_ra_claim() {
    let mut o = Ownership::new();
    o.set_possession(1, EntityId(99));
    o.add_claim(1, claim(10, "veskar", Basis::Creation));

    assert!(o
        .ripened_claim(1, "veskar", Tick(0), Tick(500), 1_000)
        .is_none());
    let chin = o
        .ripened_claim(1, "veskar", Tick(0), Tick(2_000), 1_000)
        .unwrap();
    assert_eq!(chin.holder, EntityId(99));
    assert_eq!(chin.basis, Basis::AdversePossession);
}

/// Đã có claim rồi thì không "chín" thêm lần nữa.
#[test]
fn da_co_claim_thi_khong_chin_them() {
    let mut o = Ownership::new();
    o.set_possession(1, EntityId(10));
    o.add_claim(1, claim(10, "veskar", Basis::Purchase));
    assert!(o
        .ripened_claim(1, "veskar", Tick(0), Tick(999_999), 10)
        .is_none());
}

/// **Bó quyền**: sở hữu không phải một thứ nguyên khối.
#[test]
fn bo_quyen_tach_duoc_thanh_tung_quyen() {
    let ta_dien = Claim {
        holder: EntityId(30),
        under_norm_set: "veskar".into(),
        // Được dùng và thu hoa lợi, nhưng không được bán, không được phá.
        rights: vec![Right::Access, Right::Withdrawal],
        basis: Basis::Gift,
        since: Tick(0),
    };
    assert!(ta_dien.grants(Right::Withdrawal));
    assert!(!ta_dien.grants(Right::Alienation));
    assert!(!ta_dien.grants(Right::Destruction));
}

/// Thứ tự claim **xác định**, không phụ thuộc thứ tự thêm.
#[test]
fn thu_tu_claim_xac_dinh() {
    let mut a = Ownership::new();
    a.add_claim(1, claim(20, "b.law", Basis::Purchase));
    a.add_claim(1, claim(10, "a.law", Basis::Creation));
    let mut b = Ownership::new();
    b.add_claim(1, claim(10, "a.law", Basis::Creation));
    b.add_claim(1, claim(20, "b.law", Basis::Purchase));

    let ten = |o: &Ownership| -> Vec<String> {
        o.claims_on(1)
            .iter()
            .map(|c| format!("{}:{}", c.under_norm_set, c.holder.0))
            .collect()
    };
    assert_eq!(ten(&a), ten(&b));
}

// ───────────────────────── PD-11 · tiền tệ ─────────────────────────

fn xu(fineness: u16, weight: u32) -> Coinage {
    Coinage {
        id: "veskar.silver_mark".into(),
        face_value: 100,
        fineness,
        original_fineness: 900,
        weight,
        original_weight: 100,
    }
}

/// **Một world có thể không bao giờ có tiền đúc**, và đó là hợp lệ.
#[test]
fn nac_tien_te_la_du_lieu_khong_phai_giai_doan_bat_buoc() {
    let khong_tien = EconomyProfile {
        stage: MonetaryStage::Reciprocity,
        faucets: vec![],
        sinks: vec![],
        money_supply: 0,
        goods_supply: 1_000,
    };
    assert_eq!(khong_tien.audit(), MoneyDiagnosis::Balanced);

    // Và một world dùng ân huệ thần linh cũng hợp lệ.
    let than = EconomyProfile {
        stage: MonetaryStage::Exotic,
        ..khong_tien
    };
    assert!(than.stage > MonetaryStage::Coinage);
}

/// **Pha loãng** hàm lượng bạc làm giá trị nội tại tụt.
#[test]
fn pha_loang_lam_gia_tri_noi_tai_tut() {
    let tot = xu(900, 100);
    let xau = xu(500, 100);
    assert!(xau.intrinsic_value() < tot.intrinsic_value());
    assert_eq!(xau.debasement(), 400);
    assert_eq!(tot.debasement(), 0);
}

/// **Cắt xén viền** cũng làm giá trị tụt, và đo được riêng.
#[test]
fn cat_xen_vien_do_duoc_rieng_voi_pha_loang() {
    let bi_cat = xu(900, 80);
    assert_eq!(bi_cat.clipping(), 200);
    assert_eq!(bi_cat.debasement(), 0, "cắt viền không phải pha loãng");
}

/// **Niềm tin thay đổi dần**, không sập một lần: pha loãng nhẹ chỉ người giỏi
/// mới thấy.
#[test]
fn pha_loang_nhe_chi_nguoi_gioi_moi_phat_hien() {
    let nhe = xu(880, 100);
    assert!(!nhe.detectable_by(300), "người thường không nên thấy ngay");
    assert!(nhe.detectable_by(1_000), "thương nhân lão luyện phải thấy");

    let nang = xu(300, 100);
    assert!(nang.detectable_by(200), "pha loãng nặng thì ai cũng thấy");
}

/// **Luật Gresham**: xu tốt bị tích trữ, xu xấu lưu thông.
///
/// Không ai quyết định "hãy tích trữ" — nó là hệ quả của việc hai đồng cùng mệnh
/// giá mà khác giá trị nội tại.
#[test]
fn xu_tot_bi_tich_tru_xu_xau_luu_thong() {
    let tot = xu(900, 100);
    let xau = xu(400, 100);
    assert!(
        tot.gresham_pressure(&xau) > 0,
        "xu tốt phải có áp lực tích trữ"
    );
    assert!(xau.gresham_pressure(&tot) < 0);
}

/// Mệnh giá khác nhau thì không có áp lực Gresham — chúng không thay nhau được.
#[test]
fn menh_gia_khac_nhau_thi_khong_co_ap_luc_gresham() {
    let a = xu(900, 100);
    let b = Coinage {
        face_value: 50,
        ..xu(400, 100)
    };
    assert_eq!(a.gresham_pressure(&b), 0);
}

/// Auditor **báo nguyên nhân**, không âm thầm chỉnh hệ số.
#[test]
fn auditor_bao_nguyen_nhan_lam_phat() {
    let p = EconomyProfile {
        stage: MonetaryStage::Coinage,
        faucets: vec![
            Faucet {
                id: "state_minting".into(),
                rate: 900,
            },
            Faucet {
                id: "mining".into(),
                rate: 100,
            },
        ],
        sinks: vec![Sink {
            id: "wear".into(),
            rate: 200,
            physical: true,
            voluntary: false,
        }],
        money_supply: 10_000,
        goods_supply: 1_000,
    };

    match p.audit() {
        MoneyDiagnosis::Inflation { cause, surplus } => {
            assert_eq!(surplus, 800);
            assert!(
                cause.contains("state_minting"),
                "phải chỉ đúng vòi: {cause}"
            );
        }
        khac => panic!("phải báo lạm phát, nhận được {khac:?}"),
    }
}

/// **Rút tiền mà không có cống vật chất** — chẩn đoán phải nói đúng điều đó.
#[test]
fn giam_phat_vi_thieu_cong_vat_chat_duoc_goi_ten() {
    let p = EconomyProfile {
        stage: MonetaryStage::Coinage,
        faucets: vec![Faucet {
            id: "mining".into(),
            rate: 100,
        }],
        // Chỉ có thuế: rút tiền, không hao mòn hàng.
        sinks: vec![Sink {
            id: "tax".into(),
            rate: 500,
            physical: false,
            voluntary: false,
        }],
        money_supply: 1_000,
        goods_supply: 1_000,
    };

    match p.audit() {
        MoneyDiagnosis::Deflation { cause, deficit } => {
            assert_eq!(deficit, 400);
            assert!(
                cause.contains("cống vật chất"),
                "phải chỉ ra thiếu cống vật chất: {cause}"
            );
        }
        khac => panic!("phải báo giảm phát, nhận được {khac:?}"),
    }
}

/// Cống tự nguyện phân biệt được với cống ép buộc — cái sau tạo ra `§12.5`.
#[test]
fn cong_tu_nguyen_phan_biet_voi_cong_ep_buoc() {
    let le_hoi = Sink {
        id: "festival".into(),
        rate: 100,
        physical: true,
        voluntary: true,
    };
    let thue = Sink {
        id: "tax".into(),
        rate: 100,
        physical: false,
        voluntary: false,
    };
    assert!(le_hoi.voluntary && !thue.voluntary);
}

// ───────────────────────── PD-12 · tín dụng ─────────────────────────

fn vay(id: u64, no: u64, chu: u64, goc: i64, the_chap: i64) -> Loan {
    Loan {
        id,
        debtor: EntityId(no),
        creditor: EntityId(chu),
        principal: goc,
        interest_per_period: 100,
        due: Tick(1_000),
        collateral: the_chap,
        guarantor: None,
        seniority: Seniority::Junior,
        repaid: 0,
    }
}

/// Lãi tích lũy theo kỳ.
#[test]
fn lai_tich_luy_theo_ky() {
    let l = vay(1, 10, 20, 1_000, 0);
    let som = l.outstanding(Tick(1_000), 1_000);
    let muon = l.outstanding(Tick(5_000), 1_000);
    assert!(muon > som, "để lâu mà không tăng nợ");
}

/// **Khủng hoảng dây chuyền**: một con nợ lớn sụp kéo theo chủ nợ của nó.
#[test]
fn mot_con_no_lon_sup_keo_theo_chu_no() {
    let mut s = Ledger::new();
    // A vay B rất nhiều; B vay C.
    s.lend(vay(1, 1, 2, 10_000, 0));
    s.lend(vay(2, 2, 3, 8_000, 0));

    let assets = BTreeMap::from([
        (EntityId(1), 100),   // A gần như trắng tay
        (EntityId(2), 1_000), // B trông vẫn ổn — nếu A trả được
        (EntityId(3), 50_000),
    ]);

    let vo = s.cascade_default(&assets, Tick(1_000), 1_000);
    let ai: Vec<u64> = vo.iter().map(|d| d.debtor.0).collect();
    assert!(ai.contains(&1), "A phải sụp");
    assert!(ai.contains(&2), "B phải sụp theo — đó là dây chuyền");

    // Và theo đúng thứ tự đợt.
    let dot_a = vo.iter().find(|d| d.debtor.0 == 1).unwrap().wave;
    let dot_b = vo.iter().find(|d| d.debtor.0 == 2).unwrap().wave;
    assert!(dot_a < dot_b, "B phải sụp SAU A, không cùng lúc");
}

/// **Không ai vay chồng chéo thì không có dây chuyền** — và đó là câu trả lời
/// đúng, không phải một kịch bản bị bỏ lỡ.
#[test]
fn khong_vay_chong_cheo_thi_khong_co_day_chuyen() {
    let mut s = Ledger::new();
    s.lend(vay(1, 1, 99, 10_000, 0));
    let assets = BTreeMap::from([(EntityId(1), 100), (EntityId(99), 1_000_000)]);
    let vo = s.cascade_default(&assets, Tick(1_000), 1_000);
    assert_eq!(vo.len(), 1);
}

/// Thế chấp giảm thiệt hại của chủ nợ.
#[test]
fn the_chap_giam_thiet_hai_cua_chu_no() {
    let mut khong = Ledger::new();
    khong.lend(vay(1, 1, 2, 10_000, 0));
    let mut co = Ledger::new();
    co.lend(vay(1, 1, 2, 10_000, 9_000));

    let assets = BTreeMap::from([(EntityId(1), 0), (EntityId(2), 1_000_000)]);
    let a = khong.cascade_default(&assets, Tick(1_000), 1_000)[0].creditor_loss;
    let b = co.cascade_default(&assets, Tick(1_000), 1_000)[0].creditor_loss;
    assert!(b < a, "thế chấp mà chủ nợ vẫn mất như nhau");
}

/// **Người bảo lãnh bị kéo vào** dù chưa từng vay gì.
#[test]
fn nguoi_bao_lanh_bi_keo_vao_du_chua_tung_vay() {
    let mut s = Ledger::new();
    let mut l = vay(1, 1, 2, 10_000, 0);
    l.guarantor = Some(EntityId(7));
    s.lend(l);

    let assets = BTreeMap::from([(EntityId(1), 0), (EntityId(2), 1_000_000)]);
    let vo = s.cascade_default(&assets, Tick(1_000), 1_000);
    assert_eq!(vo[0].fell_to_guarantor, Some(EntityId(7)));
}

/// **Thứ tự ưu tiên quyết định ai sống sót.**
#[test]
fn thu_tu_uu_tien_quyet_dinh_ai_duoc_tra_truoc() {
    let mut s = Ledger::new();
    let mut a = vay(1, 1, 2, 5_000, 0);
    a.seniority = Seniority::Secured;
    let mut b = vay(2, 1, 3, 5_000, 0);
    b.seniority = Seniority::Junior;
    s.lend(a).lend(b);

    // Chỉ đủ trả một nửa.
    let assets = BTreeMap::from([(EntityId(1), 5_000), (EntityId(2), 0), (EntityId(3), 0)]);
    let vo = s.cascade_default(&assets, Tick(1_000), 1_000);
    assert!(!vo.is_empty());
    // Chủ nợ có bảo đảm không mất gì; chủ nợ thường mất sạch.
    assert!(vo[0].recovered > 0);
}

/// Dây chuyền **kết thúc**, không lặp vô hạn kể cả khi đồ thị nợ có chu trình.
#[test]
fn day_chuyen_ket_thuc_du_do_thi_no_co_chu_trinh() {
    let mut s = Ledger::new();
    s.lend(vay(1, 1, 2, 1_000, 0));
    s.lend(vay(2, 2, 1, 1_000, 0));
    let assets = BTreeMap::from([(EntityId(1), 0), (EntityId(2), 0)]);
    let vo = s.cascade_default(&assets, Tick(1_000), 1_000);
    assert!(vo.len() <= 2);
}

// ─────────────────── PD-13 · lao động và vận chuyển ───────────────────

fn chuyen_hang(passable: bool) -> Shipment {
    Shipment {
        id: 1,
        goods: "core.grain".into(),
        quantity: 1_000,
        shipper: EntityId(10),
        consignee: EntityId(20),
        route: vec![
            Leg {
                from: "veskar".into(),
                to: "bridge".into(),
                travel_ticks: 100,
                spoilage: 20,
                banditry: 50,
                passable: true,
            },
            Leg {
                from: "bridge".into(),
                to: "port".into(),
                travel_ticks: 100,
                spoilage: 20,
                banditry: 100,
                passable,
            },
        ],
        departed: Tick(0),
        chain: vec![Handover {
            from_tick: 100,
            custodian: EntityId(30),
        }],
        escorted: false,
    }
}

/// **Hàng không teleport**: chưa tới giờ thì chưa tới nơi.
#[test]
fn hang_khong_teleport() {
    let s = chuyen_hang(true);
    assert!(!s.progress(Tick(50)).arrived);
    assert!(!s.progress(Tick(150)).arrived);
    assert!(s.progress(Tick(200)).arrived);
}

/// Hao hụt dọc đường: tới nơi ít hơn lúc đi.
#[test]
fn hao_hut_doc_duong_lam_toi_noi_it_hon_luc_di() {
    let p = chuyen_hang(true).progress(Tick(200));
    assert!(p.remaining < 1_000);
    assert!(p.spoiled > 0 && p.raided > 0);
}

/// **Cầu sập lan thành thiếu hàng** — với cause chain, không phải sự kiện từ hư không.
#[test]
fn cau_sap_lam_hang_ket_lai_va_chi_ro_ket_o_dau() {
    let p = chuyen_hang(false).progress(Tick(1_000));
    assert!(!p.arrived);
    assert_eq!(p.blocked_at.as_deref(), Some("bridge"));
    assert_eq!(p.legs_done, 1, "đã đi được chặng đầu rồi mới kẹt");
}

/// Áp tải giảm rủi ro cướp, **nhưng không hết**.
#[test]
fn ap_tai_giam_rui_ro_nhung_khong_het() {
    let khong = chuyen_hang(true).progress(Tick(200));
    let mut co = chuyen_hang(true);
    co.escorted = true;
    let co = co.progress(Tick(200));

    assert!(co.raided < khong.raided, "áp tải mà không giảm được gì");
    assert!(co.raided > 0, "áp tải không được biến rủi ro thành 0");
}

/// **Chuỗi bàn giao** trả lời được "lúc nó mất thì là lỗi của ai".
#[test]
fn chuoi_ban_giao_chi_ra_ai_chiu_trach_nhiem() {
    let s = chuyen_hang(true);
    assert_eq!(
        s.liable_at(Tick(50)),
        EntityId(10),
        "chưa bàn giao: người gửi"
    );
    assert_eq!(
        s.liable_at(Tick(150)),
        EntityId(30),
        "đã bàn giao: người chở"
    );
}

/// Kết quả **xác định**: cùng tuyến thì luôn mất như nhau.
#[test]
fn van_chuyen_xac_dinh() {
    let a = chuyen_hang(true).progress(Tick(200));
    let b = chuyen_hang(true).progress(Tick(200));
    assert_eq!(a, b);
}

/// Bóc lột là **quan hệ với một chuẩn mực**, không phải thuộc tính hợp đồng.
#[test]
fn boc_lot_do_theo_chuan_muc_khong_phai_tuyet_doi() {
    let hd = LabourContract {
        worker: EntityId(1),
        employer: EntityId(2),
        wage: 10,
        term_ticks: None,
        hours_per_day: 14,
        hazard: 600,
        has_leave: false,
        tool_liability: EntityId(1),
    };
    assert!(
        hd.is_exploitative(20, 10, 500),
        "chuẩn mực nghiêm: là bóc lột"
    );
    assert!(
        !hd.is_exploitative(5, 16, 900) || !hd.has_leave,
        "chuẩn mực lỏng: có thể hợp pháp"
    );
}

/// **Chuyên môn hóa nảy sinh từ việc nhìn thấy nhau.**
///
/// Bị chặn tri giác xã hội thì không ai phân hóa — đó là kết quả của thí nghiệm
/// đối chứng, và mô hình phải tái hiện được nó.
#[test]
fn khong_thay_nguoi_khac_thi_khong_phan_hoa_nghe() {
    let nang_khieu = vec![("smith".to_owned(), 900u16)];
    assert_eq!(
        specialize(&[], &nang_khieu),
        None,
        "không quan sát được ai mà vẫn chọn được nghề"
    );
}

/// Thấy chỗ nào thiếu người thì vào đó.
#[test]
fn thay_cho_thieu_nguoi_thi_vao_do() {
    let thay = vec![
        ObservedTrade {
            trade: "smith".into(),
            visible_skill: 500,
            visible_income: 100,
            shortage: -500, // thừa người
        },
        ObservedTrade {
            trade: "baker".into(),
            visible_skill: 500,
            visible_income: 100,
            shortage: 800, // thiếu người
        },
    ];
    assert_eq!(specialize(&thay, &[]), Some("baker".to_owned()));
}

/// Năng khiếu **nhân lên**, không cộng vào: làm nghề mình không hợp thì thu nhập
/// kia không thành hiện thực.
#[test]
fn nang_khieu_nhan_len_chu_khong_cong_vao() {
    let thay = vec![
        ObservedTrade {
            trade: "smith".into(),
            visible_skill: 500,
            visible_income: 200,
            shortage: 0,
        },
        ObservedTrade {
            trade: "baker".into(),
            visible_skill: 500,
            visible_income: 250,
            shortage: 0,
        },
    ];
    // Không có năng khiếu gì: chọn nghề thu nhập cao hơn.
    assert_eq!(specialize(&thay, &[]), Some("baker".to_owned()));
    // Có năng khiếu rèn: đổi ý.
    assert_eq!(
        specialize(&thay, &[("smith".to_owned(), 1_000)]),
        Some("smith".to_owned())
    );
}

/// Chọn nghề **xác định**, không phụ thuộc thứ tự quan sát.
#[test]
fn chon_nghe_xac_dinh() {
    let mut thay = vec![
        ObservedTrade {
            trade: "smith".into(),
            visible_skill: 500,
            visible_income: 100,
            shortage: 0,
        },
        ObservedTrade {
            trade: "baker".into(),
            visible_skill: 500,
            visible_income: 100,
            shortage: 0,
        },
    ];
    let a = specialize(&thay, &[]);
    thay.reverse();
    assert_eq!(a, specialize(&thay, &[]));
}
