//! Test luật, tội, chứng cứ và thẩm quyền (`PD-01`–`PD-03`, `PD-06`).

use mow_core::{EntityId, Tick};
use mow_law::crime::{Temptation, Witness};
use mow_law::norms::{
    governing_charge, immune, judge, Deed, Enforcement, Immunity, LegalOrder, NormSet, ProofMode,
    ProofRequirement, Rule, Sanction, SanctionKind, Scope,
};
use mow_law::trial::{proof_met, try_case, DoubleJeopardy, Evidence, Procedure, TrialContext};
use std::collections::BTreeMap;

fn trom(value_above: Option<i64>, chi_voi: &[&str]) -> Rule {
    Rule {
        act: "theft".into(),
        value_above,
        sanction: Sanction {
            kind: SanctionKind::Corporal,
            severity: 400,
        },
        proof_required: vec![
            ProofRequirement::WitnessCount(2),
            ProofRequirement::PhysicalEvidence,
        ],
        proof_mode: ProofMode::AnyOf,
        enforced_against: chi_voi.iter().map(|s| (*s).to_owned()).collect(),
    }
}

fn quoc_gia(id: &str, version: u32, precedence: u8, rules: Vec<Rule>) -> NormSet {
    NormSet {
        id: id.into(),
        version,
        precedence,
        scope: Scope {
            jurisdiction: format!("organization:{id}"),
            territorial: true,
            districts: vec!["core".into(), "docks".into(), "outskirts".into()],
            members: vec![],
        },
        rules,
        enforcement: Enforcement {
            agency: "veskar.city_watch".into(),
            coverage_by_district: BTreeMap::from([
                ("core".into(), 800),
                ("docks".into(), 250),
                ("outskirts".into(), 50),
            ]),
        },
    }
}

fn hanh_vi(district: &str, class: &str, value: i64) -> Deed {
    Deed {
        actor: EntityId(1),
        act: "theft".into(),
        value,
        district: district.into(),
        actor_class: class.into(),
        actor_groups: vec!["guild.smiths".into()],
    }
}

// ───────────────────────── PD-01 · norm_set ─────────────────────────

/// **Tội không phải thuộc tính của hành động.** Cùng một việc, hai nơi, hai kết quả.
#[test]
fn cung_hanh_vi_hop_phap_o_noi_nay_pham_phap_o_noi_kia() {
    let mut co_luat = LegalOrder::new();
    co_luat.add(quoc_gia("veskar", 3, 0, vec![trom(Some(50), &[])]));

    // Trong lãnh thổ Veskar: có cáo buộc.
    assert_eq!(judge(&co_luat, &hanh_vi("core", "commoner", 100)).len(), 1);

    // Ngoài lãnh thổ: không có gì cả — không phải "vô tội", mà là **không có
    // luật nào để nói tới**.
    let ngoai = Deed {
        district: "neighbor_land".into(),
        ..hanh_vi("core", "commoner", 100)
    };
    assert!(judge(&co_luat, &ngoai).is_empty());
}

/// Ngưỡng giá trị: trộm vặt không phải là trộm theo luật này.
#[test]
fn nguong_gia_tri_quyet_dinh_co_phai_toi_khong() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 3, 0, vec![trom(Some(50), &[])]));

    assert!(judge(&o, &hanh_vi("core", "commoner", 30)).is_empty());
    assert_eq!(judge(&o, &hanh_vi("core", "commoner", 51)).len(), 1);
}

/// **Luật áp dụng không đều là chuyện thường.** Người thường bị xử, quý tộc thì không.
#[test]
fn enforced_against_lam_bat_cong_thanh_cau_truc() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 3, 0, vec![trom(None, &["commoner"])]));

    assert_eq!(judge(&o, &hanh_vi("core", "commoner", 100)).len(), 1);
    assert!(
        judge(&o, &hanh_vi("core", "noble", 100)).is_empty(),
        "quý tộc bị áp cùng điều luật dành riêng cho thường dân"
    );
}

/// Độ phủ theo khu: cùng một tội, ở bến cảng gần như không bị phát hiện.
#[test]
fn do_phu_khac_nhau_theo_khu() {
    let ns = quoc_gia("veskar", 3, 0, vec![trom(None, &[])]);
    assert_eq!(ns.enforcement.coverage("core"), 800);
    assert_eq!(ns.enforcement.coverage("docks"), 250);
    assert_eq!(ns.enforcement.coverage("outskirts"), 50);
}

/// Khu chưa khai báo thì độ phủ **bằng 0**, không phải trung bình.
///
/// Lấy trung bình sẽ tạo ra một sự hiện diện mà ngân sách chưa bao giờ trả tiền.
#[test]
fn khu_chua_khai_bao_thi_khong_co_luc_luong() {
    let ns = quoc_gia("veskar", 3, 0, vec![]);
    assert_eq!(ns.enforcement.coverage("sewers"), 0);
}

/// Version của luật đi theo cáo buộc — đây là thứ `§22.49` cần.
#[test]
fn cao_buoc_mang_theo_version_luat() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 7, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];
    assert_eq!(c.norm_set_version, 7);
}

// ───────────────────────── PD-06 · đa tầng ─────────────────────────

/// Một hành vi vi phạm **nhiều** hệ luật cùng lúc.
#[test]
fn mot_hanh_vi_vi_pham_nhieu_he_luat() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 3, 1, vec![trom(None, &[])]));
    o.add(NormSet {
        id: "guild.smiths.code".into(),
        version: 2,
        precedence: 0, // luật phường hội thắng luật quốc gia trong nội bộ
        scope: Scope {
            jurisdiction: "organization:guild.smiths".into(),
            territorial: false,
            districts: vec![],
            members: vec!["guild.smiths".into()],
        },
        rules: vec![Rule {
            act: "theft".into(),
            value_above: None,
            sanction: Sanction {
                kind: SanctionKind::Exile,
                severity: 900,
            },
            proof_required: vec![ProofRequirement::WitnessCount(1)],
            proof_mode: ProofMode::AnyOf,
            enforced_against: vec![],
        }],
        enforcement: Enforcement::default(),
    });

    let cac = judge(&o, &hanh_vi("core", "commoner", 100));
    assert_eq!(cac.len(), 2, "phải thấy cả hai hệ luật");

    // Bậc nhỏ hơn thắng.
    let thang = governing_charge(&o, &hanh_vi("core", "commoner", 100)).unwrap();
    assert_eq!(thang.norm_set, "guild.smiths.code");
    assert_eq!(thang.sanction.kind, SanctionKind::Exile);
}

/// Luật theo **thành viên** đi theo người, luật theo **lãnh thổ** thì không.
///
/// Đây là thứ quyết định chạy trốn có tác dụng hay không.
#[test]
fn luat_theo_thanh_vien_di_theo_nguoi_qua_bien_gioi() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 3, 1, vec![trom(None, &[])]));
    o.add(NormSet {
        id: "guild.smiths.code".into(),
        version: 1,
        precedence: 0,
        scope: Scope {
            jurisdiction: "organization:guild.smiths".into(),
            territorial: false,
            districts: vec![],
            members: vec!["guild.smiths".into()],
        },
        rules: vec![trom(None, &[])],
        enforcement: Enforcement::default(),
    });

    let chay_tron = Deed {
        district: "far_away".into(),
        ..hanh_vi("core", "commoner", 100)
    };
    let cac = judge(&o, &chay_tron);
    assert_eq!(cac.len(), 1, "chỉ luật phường hội còn với tới");
    assert_eq!(cac[0].norm_set, "guild.smiths.code");
}

/// Kết quả phải **xác định**: hai bộ luật cùng bậc không được đổi thứ tự theo
/// thứ tự nạp content pack.
#[test]
fn thu_tu_cao_buoc_xac_dinh() {
    let mk = |id: &str| NormSet {
        id: id.into(),
        version: 1,
        precedence: 5,
        scope: Scope {
            jurisdiction: id.into(),
            territorial: true,
            districts: vec!["core".into()],
            members: vec![],
        },
        rules: vec![trom(None, &[])],
        enforcement: Enforcement::default(),
    };

    let mut a = LegalOrder::new();
    a.add(mk("zeta")).add(mk("alpha"));
    let mut b = LegalOrder::new();
    b.add(mk("alpha")).add(mk("zeta"));

    let ra_a: Vec<String> = judge(&a, &hanh_vi("core", "c", 1))
        .iter()
        .map(|c| c.norm_set.clone())
        .collect();
    let ra_b: Vec<String> = judge(&b, &hanh_vi("core", "c", 1))
        .iter()
        .map(|c| c.norm_set.clone())
        .collect();
    assert_eq!(ra_a, ra_b);
    assert_eq!(ra_a, vec!["alpha", "zeta"]);
}

/// Miễn trừ chặn được một hệ luật, **không phải tất cả**.
#[test]
fn mien_tru_chi_chan_dung_he_luat_da_cho() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 3, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    let mt = [Immunity {
        holder: EntityId(1),
        from_norm_set: "veskar".into(),
        basis: "envoy".into(),
    }];
    assert!(immune(&mt, EntityId(1), c));
    assert!(!immune(&mt, EntityId(2), c), "miễn trừ lan sang người khác");
}

/// Đã xử ở nước A rồi vẫn có thể bị nước B xử — đó là xung đột thẩm quyền, không
/// phải lỗi.
#[test]
fn cam_xu_hai_lan_theo_tung_he_luat_khong_theo_toan_cuc() {
    let mut dj = DoubleJeopardy::new();
    dj.record(EntityId(1), "veskar", "theft");

    assert!(dj.already_tried(EntityId(1), "veskar", "theft"));
    assert!(
        !dj.already_tried(EntityId(1), "guild.smiths.code", "theft"),
        "một bản án ở nước A không chặn được phường hội xử"
    );
}

// ───────────────────────── PD-02 · đường đi của tội ─────────────────────────

fn cam_do() -> Temptation {
    Temptation {
        actor: EntityId(1),
        act: "theft".into(),
        need: 800,
        gain: 300,
        exposure: 500,
        capability: 900,
        believed_coverage: 500,
        moral_cost: 200,
        believed_sanction: 500,
    }
}

/// **Điểm quan trọng nhất của `§12.5.2`**: rủi ro tính theo belief, không theo
/// con số thật.
///
/// Một chính quyền chỉ cần làm cho người ta *tin* rằng mình giám sát chặt — và
/// điều đó lật được quyết định của một kẻ **ở mức cám dỗ vừa phải**.
#[test]
fn chinh_quyen_lam_nguoi_ta_tin_la_giam_duoc_toi_pham() {
    // Kẻ trộm cơ hội: không túng quẫn, chỉ thấy món hời.
    let ke_co_hoi = Temptation {
        need: 300,
        gain: 400,
        moral_cost: 300,
        ..cam_do()
    };

    let de = Temptation {
        believed_coverage: 100,
        ..ke_co_hoi.clone()
    };
    let doa = Temptation {
        believed_coverage: 950,
        ..ke_co_hoi
    };

    assert!(de.deliberate().will_act, "tin là không ai canh thì làm");
    assert!(!doa.deliberate().will_act, "tin là bị canh chặt thì thôi");

    // Và lực lượng **thật** không hề đổi giữa hai trường hợp — chỉ niềm tin đổi.
    assert_eq!(de.exposure, doa.exposure);
    assert_eq!(de.believed_sanction, doa.believed_sanction);
}

/// Nhưng răn đe **không** ngăn được người túng quẫn, và đó là một tính chất chứ
/// không phải một khiếm khuyết.
///
/// Nếu tăng độ phủ là ngăn được mọi tội, thì nạn đói ở `§12.2` không còn dẫn tới
/// trộm cắp, và cả chuỗi nhân quả "mất mùa → đói → trộm → bị đuổi" mà `§18.10`
/// lấy làm ví dụ sẽ không bao giờ xảy ra.
#[test]
fn ran_de_khong_ngan_duoc_nguoi_tung_quan() {
    let sap_chet_doi = Temptation {
        need: 950,
        moral_cost: 100,
        believed_coverage: 1_000,
        ..cam_do()
    };
    assert!(
        sap_chet_doi.deliberate().will_act,
        "người sắp chết đói phải vẫn trộm dù biết chắc sẽ bị bắt"
    );
}

/// Belief phải **đổi điểm một cách đơn điệu**: tin càng bị canh chặt, càng ít muốn.
#[test]
fn tin_bi_canh_cang_chat_thi_diem_cang_thap() {
    let mut truoc = i64::MAX;
    for cov in [0u16, 200, 400, 600, 800, 1_000] {
        let t = Temptation {
            believed_coverage: cov,
            ..cam_do()
        };
        let d = t.deliberate().score;
        assert!(d <= truoc, "độ phủ {cov} lại làm điểm tăng: {d} > {truoc}");
        truoc = d;
    }
}

/// Trộm món đắt hơn phải **rủi ro hơn**, không phải an toàn hơn.
///
/// Nếu mất mát khi bị bắt không tính món lợi bị tịch thu, thì món càng đắt càng
/// đáng trộm mà không thêm chút rủi ro nào.
#[test]
fn trom_mon_dat_thi_mat_nhieu_hon_khi_bi_bat() {
    let re = Temptation {
        gain: 50,
        ..cam_do()
    };
    let dat = Temptation {
        gain: 900,
        ..cam_do()
    };
    assert!(
        dat.expected_loss() > re.expected_loss(),
        "trộm món đắt lại không rủi ro hơn trộm món rẻ"
    );
}

/// Tội phạm hoàn hảo: chỉ cần **một** khâu bị vô hiệu, không cần cả ba.
#[test]
fn mot_khau_bi_vo_hieu_la_rui_ro_bang_khong() {
    let khong_ai_thay = Temptation {
        exposure: 0,
        ..cam_do()
    };
    assert_eq!(khong_ai_thay.perceived_catch_chance(), 0);
    assert_eq!(khong_ai_thay.expected_loss(), 0);

    let khong_ai_bat = Temptation {
        believed_coverage: 0,
        ..cam_do()
    };
    assert_eq!(khong_ai_bat.perceived_catch_chance(), 0);
}

/// Năng lực **cho phép**, không **thúc đẩy**: muốn mà không làm được thì không làm.
#[test]
fn nang_luc_cho_phep_chu_khong_thuc_day() {
    let vung = cam_do();
    let vung_ve = Temptation {
        capability: 50,
        ..cam_do()
    };

    assert!(vung.deliberate().will_act);
    let yeu = vung_ve.deliberate();
    assert!(
        yeu.score < vung.deliberate().score,
        "kẻ vụng về phải khó thành công hơn"
    );
}

/// Chi phí đạo đức ngăn được một người có đủ mọi điều kiện khác.
#[test]
fn chi_phi_dao_duc_ngan_duoc_nguoi_co_du_dieu_kien() {
    let co_luong_tam = Temptation {
        moral_cost: 1_000,
        ..cam_do()
    };
    assert!(!co_luong_tam.deliberate().will_act);
}

/// Quyết định luôn **giải thích được** (`§18.13`).
#[test]
fn quyet_dinh_pham_toi_luon_giai_thich_duoc() {
    let i = cam_do().deliberate();
    assert!(!i.factors.is_empty());
    for ten in ["đang thiếu thốn", "rủi ro ước lượng", "chi phí đạo đức"] {
        assert!(
            i.factors.iter().any(|f| f.label == ten),
            "thiếu phần `{ten}`"
        );
    }
}

// ───────────────────────── PD-03 · chứng cứ ─────────────────────────

fn nhan_chung(id: u64, tin: Option<u64>, dong_co: i16) -> Evidence {
    Evidence::Testimony(Witness {
        who: EntityId(id),
        believes_actor: tin.map(EntityId),
        confidence: 700,
        motive_to_testify: dong_co,
    })
}

fn vat_chung(het_han: u64) -> Evidence {
    Evidence::Physical {
        what: "dấu chân bùn".into(),
        decays_at: Tick(het_han),
        destroyed: false,
    }
}

/// **Cả làng đều biết nhưng không ai dám làm chứng** — không viết riêng dòng nào.
#[test]
fn ca_lang_deu_biet_nhung_khong_ai_dam_lam_chung() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 1, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    // Năm người thấy, tất cả đều sợ.
    let so: Vec<Evidence> = (10..15).map(|i| nhan_chung(i, Some(1), -500)).collect();
    assert!(
        !proof_met(c, &so, Tick(100)),
        "sợ hãi không ngăn được lời khai"
    );

    // Hai người dám nói thì đủ.
    let dam: Vec<Evidence> = (10..12).map(|i| nhan_chung(i, Some(1), 400)).collect();
    assert!(proof_met(c, &dam, Tick(100)));
}

/// **Phi tang** là một nước đi hợp lệ: chứng cứ có thời hạn và phá hủy được.
#[test]
fn chung_cu_het_han_hoac_bi_pha_huy_thi_khong_dung_duoc() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 1, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    let vc = vec![vat_chung(200)];
    assert!(proof_met(c, &vc, Tick(100)), "còn hạn thì dùng được");
    assert!(!proof_met(c, &vc, Tick(300)), "hết hạn mà vẫn dùng được");

    let da_pha = vec![Evidence::Physical {
        what: "dấu chân bùn".into(),
        decays_at: Tick(1_000),
        destroyed: true,
    }];
    assert!(!proof_met(c, &da_pha, Tick(100)));
}

/// Phép truy vấn sự thật **có counter** — nếu không, tư pháp thành tuyệt đối.
#[test]
fn phep_truy_van_su_that_bi_hoa_giai_thi_vo_nghia() {
    let phep = Evidence::TruthSpell {
        says_guilty: true,
        countered: false,
    };
    assert!(phep.is_available(Tick(0)));

    let bi_chan = Evidence::TruthSpell {
        says_guilty: true,
        countered: true,
    };
    assert!(
        !bi_chan.is_available(Tick(0)),
        "giới quyền lực nghiên cứu cách chống lại, và nó phải có tác dụng"
    );
}

/// `AllOf` khắt khe hơn `AnyOf`, và khác biệt đó phải đo được.
#[test]
fn proof_mode_quyet_dinh_muc_kho_cua_viec_buoc_toi() {
    let mut o = LegalOrder::new();
    let mut r = trom(None, &[]);
    r.proof_mode = ProofMode::AllOf;
    o.add(quoc_gia("veskar", 1, 0, vec![r]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    // Chỉ có nhân chứng: `AnyOf` thì đủ, `AllOf` thì không.
    let chi_nhan_chung: Vec<Evidence> = (10..12).map(|i| nhan_chung(i, Some(1), 400)).collect();
    assert!(!proof_met(c, &chi_nhan_chung, Tick(10)));

    let mut du = chi_nhan_chung;
    du.push(vat_chung(1_000));
    assert!(proof_met(c, &du, Tick(10)));
}

// ───────────────────────── xét xử ─────────────────────────

/// **Án oan**: tòa kết tội người không làm, và không có gì trong hệ thống ngăn
/// điều đó — vì `try_case` không biết ai là thủ phạm thật.
#[test]
fn an_oan_dien_dat_duoc_vi_toa_khong_biet_su_that() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 1, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    // Hai nhân chứng cùng tin nhầm là người số 9 đã làm.
    let vu_khong: Vec<Evidence> = (10..12).map(|i| nhan_chung(i, Some(9), 800)).collect();
    let v = try_case(
        EntityId(9),
        c,
        &vu_khong,
        Procedure::Evidentiary,
        Tick(10),
        &TrialContext::default(),
    );

    assert!(v.guilty, "chứng cứ đủ thì tòa kết tội");
    // Nhưng thủ phạm thật là người số 1.
    assert!(
        !v.was_correct(Some(EntityId(1))),
        "phán quyết này phải bị audit chấm là sai"
    );
}

/// **Tội phạm hoàn hảo**: đúng người, nhưng không đủ chứng cứ.
#[test]
fn toi_pham_hoan_hao_thi_toa_khong_ket_toi_duoc() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 1, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    let v = try_case(
        EntityId(1),
        c,
        &[],
        Procedure::Evidentiary,
        Tick(10),
        &TrialContext::default(),
    );
    assert!(!v.guilty);
    assert!(
        !v.was_correct(Some(EntityId(1))),
        "tha bổng đúng thủ phạm cũng là một phán quyết sai so với sự thật"
    );
}

/// Tra tấn cho ra án oan **một cách có hệ thống**, và mô hình phải nói được điều đó.
#[test]
fn tra_tan_ket_toi_bat_ke_co_lam_hay_khong() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 1, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    let ctx = TrialContext {
        pain_applied: 900,
        defendant_endurance: 300,
        ..TrialContext::default()
    };
    // Không một mẩu chứng cứ nào.
    let v = try_case(EntityId(9), c, &[], Procedure::Torture, Tick(10), &ctx);
    assert!(v.guilty);
    assert_eq!(v.evidence_accepted, 0, "kết tội mà không có chứng cứ nào");
}

/// Đấu thần thánh: kẻ mạnh hơn được coi là đúng.
#[test]
fn dau_than_thanh_quyet_theo_suc_manh() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 1, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    let yeu = TrialContext {
        defendant_strength: 100,
        accuser_strength: 900,
        ..TrialContext::default()
    };
    assert!(try_case(EntityId(9), c, &[], Procedure::TrialByCombat, Tick(0), &yeu).guilty);

    let khoe = TrialContext {
        defendant_strength: 900,
        accuser_strength: 100,
        ..TrialContext::default()
    };
    assert!(
        !try_case(
            EntityId(9),
            c,
            &[],
            Procedure::TrialByCombat,
            Tick(0),
            &khoe
        )
        .guilty
    );
}

/// Mọi phán quyết đều **giải thích được**.
#[test]
fn phan_quyet_luon_kem_ly_do() {
    let mut o = LegalOrder::new();
    o.add(quoc_gia("veskar", 1, 0, vec![trom(None, &[])]));
    let c = &judge(&o, &hanh_vi("core", "commoner", 100))[0];

    for p in [
        Procedure::Evidentiary,
        Procedure::Compurgation,
        Procedure::TrialByCombat,
        Procedure::Torture,
        Procedure::ElderCouncil,
    ] {
        let v = try_case(EntityId(9), c, &[], p, Tick(0), &TrialContext::default());
        assert!(!v.reasons.is_empty(), "thủ tục {p:?} không nêu lý do");
    }
}
