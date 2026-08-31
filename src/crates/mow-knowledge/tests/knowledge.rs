//! Test đồ thị tri thức, truyền dạy, sách, thể chế (`PD-16`, `PD-24`, `PD-25`).

use mow_core::EntityId;
use mow_knowledge::graph::{
    blockers, Blocker, KnowledgeGraph, Level, Node, Requirements, Understanding,
};
use mow_knowledge::school::{Archive, Examination, Institution, Lineage, Rejection};
use mow_knowledge::teaching::{read, teach, Corpus, Learner, Setting, Teacher, Text};
use std::collections::BTreeSet;

fn node(id: &str, school: &str, power: u16) -> Node {
    Node {
        id: id.into(),
        domain: "metallurgy".into(),
        school: school.into(),
        requirements: Requirements::default(),
        unlocks: vec![],
        secrecy: 0,
        teaching_difficulty: 200,
        failure_rate: 100,
        predictive_power: power,
    }
}

fn thep() -> Node {
    Node {
        requirements: Requirements {
            prerequisites: vec![("core.iron_smelting".into(), Level::Practiced)],
            evidence: vec!["core.observed_carburization".into()],
            materials: vec!["core.iron_ore".into(), "core.charcoal".into()],
            collaborators: 3,
            distinct_specialties: 2,
        },
        ..node("core.steel", "orthodox", 800)
    }
}

// ─────────────────── PD-16 · đồ thị và thang hiểu biết ───────────────────

/// **Nghe nói về cổng liên-world không đồng nghĩa biết xây cổng.**
#[test]
fn nghe_noi_khong_phai_la_biet() {
    assert!(!Level::HeardOf.can_practise());
    assert!(!Level::Conceptual.can_practise());
    assert!(Level::Practiced.can_practise());

    // Và làm được chưa chắc dạy được.
    assert!(!Level::Practiced.can_teach());
    assert!(Level::Proficient.can_teach());
}

/// **`tech_points` không phải tiền để mua node**: thiếu điều kiện thì không có
/// cách nào bù bằng điểm.
#[test]
fn khong_the_mua_node_bang_diem() {
    let n = thep();
    let ai_do = Understanding::new(); // chưa biết gì
    let can = blockers(&n, &ai_do, &BTreeSet::new(), &BTreeSet::new(), 0, 0);
    // Năm loại thiếu, và không loại nào là "chưa đủ điểm".
    assert_eq!(can.len(), 6, "{can:?}");
}

/// Chặn phải nói **thiếu cái gì**, không phải làm mờ một nút bấm.
#[test]
fn chan_phai_noi_ro_thieu_cai_gi() {
    let n = thep();
    let mut u = Understanding::new();
    u.set("core.iron_smelting", Level::HeardOf, 500, None);

    let can = blockers(
        &n,
        &u,
        &BTreeSet::from(["core.observed_carburization".to_owned()]),
        &BTreeSet::from(["core.iron_ore".to_owned(), "core.charcoal".to_owned()]),
        3,
        2,
    );
    assert_eq!(can.len(), 1);
    assert_eq!(
        can[0],
        Blocker::Prerequisite {
            node: "core.iron_smelting".into(),
            need: Level::Practiced,
            have: Level::HeardOf,
        }
    );
}

/// **Địa lý biến thành lịch sử**: không có mỏ thì không luyện được thép, dù giàu.
#[test]
fn khong_co_vat_lieu_thi_giau_may_cung_khong_lam_duoc() {
    let n = thep();
    let mut u = Understanding::new();
    u.set("core.iron_smelting", Level::Proficient, 900, None);

    let can = blockers(
        &n,
        &u,
        &BTreeSet::from(["core.observed_carburization".to_owned()]),
        &BTreeSet::new(), // không có quặng
        10,
        5,
    );
    assert!(can
        .iter()
        .any(|b| matches!(b, Blocker::MissingMaterial(m) if m == "core.iron_ore")));
}

/// Đủ người nhưng **thiếu chuyên môn khác nhau** là một kiểu thiếu riêng.
#[test]
fn du_nguoi_ma_thieu_chuyen_mon_van_khong_lam_duoc() {
    let n = thep();
    let mut u = Understanding::new();
    u.set("core.iron_smelting", Level::Proficient, 900, None);

    let can = blockers(
        &n,
        &u,
        &BTreeSet::from(["core.observed_carburization".to_owned()]),
        &BTreeSet::from(["core.iron_ore".to_owned(), "core.charcoal".to_owned()]),
        10, // thừa người
        1,  // nhưng ai cũng làm một nghề
    );
    assert_eq!(
        can,
        vec![Blocker::NotEnoughSpecialties { need: 2, have: 1 }]
    );
}

/// **Nhiều trường phái cùng tồn tại**, và thử nghiệm chọn ra cái dự báo tốt hơn
/// — chứ không tuyên bố cái nào đúng.
#[test]
fn thu_nghiem_chon_truong_phai_du_bao_tot_hon() {
    let mut g = KnowledgeGraph::new();
    g.add(node("core.phlogiston", "phlogiston", 300));
    g.add(node("core.oxidation", "oxidation", 900));

    assert_eq!(g.rival_schools("metallurgy").len(), 2);
    assert_eq!(g.best_predictor("metallurgy").unwrap().school, "oxidation");
}

/// Provenance: truy được một sai lệch về tận người dạy.
#[test]
fn provenance_truy_duoc_ve_nguoi_day() {
    let mut u = Understanding::new();
    u.set("core.steel", Level::Practiced, 600, Some(EntityId(42)));
    assert_eq!(u.learned_from("core.steel"), Some(EntityId(42)));
    assert_eq!(u.confidence("core.steel"), 600);
}

// ─────────────────── PD-16 · truyền dạy có hao hụt ───────────────────

fn thay(level: Level, pedagogy: u16, fidelity: u16) -> Teacher {
    Teacher {
        who: EntityId(1),
        level,
        pedagogy,
        fidelity,
    }
}

fn tro() -> Learner {
    Learner {
        who: EntityId(2),
        memory: 700,
        attention: 700,
        motivation: 800,
    }
}

fn lop_tot() -> Setting {
    Setting {
        shared_language: 1_000,
        trust: 900,
        tools: 800,
        practice_time: 800,
    }
}

/// **Người dạy có thể truyền sai**, và độ chính xác đi theo suốt đời.
#[test]
fn day_sai_thi_tro_hoc_duoc_cai_sai() {
    let n = node("core.steel", "orthodox", 800);
    let sai = teach(
        &n,
        &thay(Level::Mastered, 900, 300),
        &tro(),
        &lop_tot(),
        Level::Conceptual,
    );
    let dung = teach(
        &n,
        &thay(Level::Mastered, 900, 1_000),
        &tro(),
        &lop_tot(),
        Level::Conceptual,
    );

    assert!(sai.fidelity < dung.fidelity);
    assert_eq!(sai.level, dung.level, "cùng bậc, khác độ chính xác");
}

/// **Độ chính xác không bao giờ vượt nguồn.**
///
/// Muốn tăng thì phải nghiên cứu lại, không phải học lại.
#[test]
fn do_chinh_xac_khong_bao_gio_vuot_nguon() {
    let n = node("core.steel", "orthodox", 800);
    for f in [0u16, 200, 500, 900, 1_000] {
        let ra = teach(
            &n,
            &thay(Level::Mastered, 1_000, f),
            &tro(),
            &lop_tot(),
            Level::Conceptual,
        );
        assert!(
            ra.fidelity <= f,
            "học ra chính xác hơn thầy: {} > {f}",
            ra.fidelity
        );
    }
}

/// Chưa thành thạo thì **không dạy được**, và nói rõ vì sao.
#[test]
fn chua_thanh_thao_thi_khong_day_duoc() {
    let n = node("core.steel", "orthodox", 800);
    let ra = teach(
        &n,
        &thay(Level::Practiced, 1_000, 1_000),
        &tro(),
        &lop_tot(),
        Level::Unknown,
    );
    assert_eq!(ra.level, Level::Unknown);
    assert!(ra.reasons[0].contains("chưa dạy được"));
}

/// **Không tin thầy thì nghe cũng như không.**
#[test]
fn khong_tin_thay_thi_hoc_khong_vao() {
    let n = node("core.steel", "orthodox", 800);
    let ngo_vuc = Setting {
        trust: 50,
        ..lop_tot()
    };
    let ra = teach(
        &n,
        &thay(Level::Mastered, 900, 900),
        &tro(),
        &ngo_vuc,
        Level::Unknown,
    );
    assert_eq!(ra.level, Level::Unknown, "ngờ vực mà vẫn học được");
    assert!(ra.reasons.iter().any(|r| r.contains("không tin thầy")));
}

/// Không học vượt thầy trong một buổi.
#[test]
fn khong_hoc_vuot_thay_trong_mot_buoi() {
    let n = node("core.steel", "orthodox", 800);
    let ra = teach(
        &n,
        &thay(Level::Proficient, 1_000, 1_000),
        &tro(),
        &lop_tot(),
        Level::Practiced,
    );
    assert!(ra.level <= Level::Proficient);
}

/// Truyền dạy **xác định**: cùng đầu vào, cùng kết quả.
#[test]
fn truyen_day_xac_dinh() {
    let n = node("core.steel", "orthodox", 800);
    let a = teach(
        &n,
        &thay(Level::Mastered, 800, 900),
        &tro(),
        &lop_tot(),
        Level::Conceptual,
    );
    let b = teach(
        &n,
        &thay(Level::Mastered, 800, 900),
        &tro(),
        &lop_tot(),
        Level::Conceptual,
    );
    assert_eq!(a, b);
}

// ─────────────────── PD-24 · vật phẩm mang thông tin ───────────────────

fn sach(fidelity: u16) -> Text {
    Text {
        id: 1,
        language: "old_veskaran".into(),
        script: "runic".into(),
        cipher: None,
        node: "core.steel".into(),
        fidelity,
        generation: 0,
        transcription_errors: 0,
        copied_from: None,
    }
}

/// Ba điều kiện đọc, thất bại theo **ba kiểu khác nhau**.
#[test]
fn ba_dieu_kien_doc_that_bai_theo_ba_kieu() {
    let s = Text {
        cipher: Some("temple_substitution".into()),
        ..sach(800)
    };
    let ngon_ngu = vec!["old_veskaran".to_owned()];
    let chu = vec!["runic".to_owned()];
    let khoa = vec!["temple_substitution".to_owned()];

    assert!(s.legible_to(&ngon_ngu, &chu, &khoa));
    assert!(!s.legible_to(&[], &chu, &khoa), "không biết tiếng");
    assert!(!s.legible_to(&ngon_ngu, &[], &khoa), "không biết chữ");
    assert!(!s.legible_to(&ngon_ngu, &chu, &[]), "không có khóa");
}

/// **Đọc là một lần truyền dạy có hao hụt** — và sách dạy kém hơn người.
///
/// Một cuốn sách phép cao cấp trong tay người thiếu nền tảng chỉ cho ra
/// `HEARD_OF`, không phải `PRACTICED`.
#[test]
fn sach_day_kem_hon_nguoi() {
    let n = node("core.steel", "orthodox", 800);
    let kem = Learner {
        who: EntityId(2),
        memory: 200,
        attention: 200,
        motivation: 300,
    };
    let u = Understanding::new();
    let ra = read(
        &sach(900),
        &n,
        &kem,
        &u,
        &["old_veskaran".to_owned()],
        &["runic".to_owned()],
        &[],
    )
    .expect("đọc được");
    assert!(
        ra.level <= Level::HeardOf,
        "người thiếu nền tảng học được quá nhiều"
    );
}

/// Không giải mã nổi thì **chưa bắt đầu học được** — khác với học thất bại.
#[test]
fn khong_giai_ma_noi_thi_tra_none() {
    let n = node("core.steel", "orthodox", 800);
    let u = Understanding::new();
    assert!(read(&sach(900), &n, &tro(), &u, &[], &[], &[]).is_none());
}

/// **Sao chép sinh lỗi tích lũy.**
#[test]
fn sao_chep_sinh_loi_tich_luy() {
    let goc = sach(1_000);
    let doi1 = goc.copy(2, 900);
    let doi2 = doi1.copy(3, 900);

    assert!(doi1.fidelity < goc.fidelity);
    assert!(doi2.fidelity < doi1.fidelity);
    assert_eq!(doi2.generation, 2);
    assert!(doi2.transcription_errors > doi1.transcription_errors);
    assert_eq!(doi2.copied_from, Some(2));
}

/// Thợ chép giỏi mất ít hơn, **nhưng không bao giờ mất 0**.
#[test]
fn tho_chep_gioi_van_lam_mat_it_nhieu() {
    let goc = sach(1_000);
    let gioi = goc.copy(2, 1_000);
    let vung = goc.copy(3, 100);
    assert!(gioi.fidelity > vung.fidelity);
    assert!(
        gioi.fidelity < goc.fidelity,
        "bản sao hoàn hảo tuyệt đối làm trôi dạt văn bản biến mất"
    );
}

/// **Tri thức mất thật được**: đốt hết sách và không ai còn biết.
#[test]
fn dot_het_sach_va_khong_ai_biet_thi_tri_thuc_bien_mat() {
    let mut c = Corpus {
        texts: vec![sach(900), Text { id: 2, ..sach(800) }],
        minds: vec![(EntityId(5), "core.steel".into(), Level::Conceptual)],
    };
    assert!(c.knowledge_survives("core.steel"));

    let da_dot = c.burn(|t| t.node == "core.steel");
    assert_eq!(da_dot, 2);
    assert!(
        !c.knowledge_survives("core.steel"),
        "người còn sống chỉ ở bậc CONCEPTUAL thì không giữ được tri thức"
    );
}

/// Nhưng còn một người **làm được** thì tri thức chưa mất.
#[test]
fn con_mot_nguoi_lam_duoc_thi_tri_thuc_chua_mat() {
    let mut c = Corpus {
        texts: vec![sach(900)],
        minds: vec![(EntityId(5), "core.steel".into(), Level::Proficient)],
    };
    c.burn(|_| true);
    assert!(c.knowledge_survives("core.steel"));
}

/// Phê bình văn bản: tìm bản gần nguyên tác nhất trong mười bản chép tay.
#[test]
fn tim_duoc_ban_gan_nguyen_tac_nhat() {
    let goc = sach(1_000);
    let tot = goc.copy(2, 1_000);
    let te = goc.copy(3, 0);
    let c = Corpus {
        texts: vec![te, tot.clone()],
        minds: vec![],
    };
    assert_eq!(c.best_copy("core.steel").unwrap().id, tot.id);
}

// ─────────────────── PD-25 · giáo dục và lưu trữ ───────────────────

fn hoc_vien(classes: &[&str], tuition: i64) -> Institution {
    Institution {
        id: "veskar.academy".into(),
        curriculum: vec!["core.steel".into()],
        admits_classes: classes.iter().map(|c| (*c).to_owned()).collect(),
        tuition,
        requires_patron: false,
        graduation_level: Level::Proficient,
        funding: 1_000,
        funding_needed: 1_000,
    }
}

/// **Gác cửa là quyền lực**: đóng cửa với một tầng lớp là hành động chính trị.
#[test]
fn gac_cua_quyet_dinh_ai_len_duoc_dia_vi() {
    let chi_quy_toc = hoc_vien(&["noble"], 0);
    assert!(chi_quy_toc.admits("noble", 1_000, None).is_empty());
    assert_eq!(
        chi_quy_toc.admits("commoner", 1_000, None),
        vec![Rejection::WrongClass("commoner".into())]
    );
}

/// Học viện mở cửa phải **khai điều đó ra**, không phải quên khai.
#[test]
fn mo_cua_cho_tat_ca_la_mot_lua_chon_duoc_khai() {
    let mo = hoc_vien(&[], 0);
    assert!(mo.admits("commoner", 0, None).is_empty());
    assert!(mo.admits("noble", 0, None).is_empty());
}

/// Cánh cửa đóng phải **nói vì sao**, và có thể vì nhiều lý do cùng lúc.
#[test]
fn cua_dong_phai_noi_vi_sao_va_co_the_nhieu_ly_do() {
    let kho = Institution {
        requires_patron: true,
        ..hoc_vien(&["noble"], 500)
    };
    let ly_do = kho.admits("commoner", 100, None);
    assert_eq!(ly_do.len(), 3, "{ly_do:?}");
}

/// **Cắt kinh phí không đóng cửa trường** — nó làm trường dạy kém đi.
#[test]
fn cat_kinh_phi_lam_chat_luong_tut_chu_khong_dong_cua() {
    let du = hoc_vien(&[], 0);
    let thieu = Institution {
        funding: 200,
        ..hoc_vien(&[], 0)
    };
    assert_eq!(du.quality(), 1_000);
    assert_eq!(thieu.quality(), 200);
    // Vẫn nhận học trò.
    assert!(thieu.admits("commoner", 0, None).is_empty());
}

/// Thi cử thiên vị: nâng người kém, dìm người giỏi.
#[test]
fn thien_vi_trong_thi_cu_la_du_lieu_cua_the_che() {
    let ky_thi = Examination {
        node: "core.steel".into(),
        pass_level: Level::Proficient,
        class_bias: vec![("noble".into(), 600), ("commoner".into(), -600)],
    };

    // Cùng bậc Practiced: quý tộc đỗ, thường dân trượt.
    assert!(ky_thi.passes(Level::Practiced, "noble"));
    assert!(!ky_thi.passes(Level::Practiced, "commoner"));

    // Kỳ thi khách quan thì cả hai như nhau.
    let cong_bang = Examination {
        class_bias: vec![],
        ..ky_thi
    };
    assert_eq!(
        cong_bang.passes(Level::Practiced, "noble"),
        cong_bang.passes(Level::Practiced, "commoner")
    );
}

/// **Kiểm duyệt khác đốt**: sách còn mà không ai được đọc, và mở lại được.
#[test]
fn kiem_duyet_khac_dot_vi_mo_lai_duoc() {
    let mut kho = Archive {
        id: "great_library".into(),
        holdings: vec![
            sach(900),
            Text {
                id: 2,
                node: "core.other".into(),
                ..sach(800)
            },
        ],
        censored: vec![],
    };
    assert_eq!(kho.accessible().len(), 2);

    let bi_khoa = kho.censor("core.steel");
    assert_eq!(bi_khoa, 1);
    assert_eq!(kho.accessible().len(), 1);

    // Một chế độ sụp đổ có thể mở lại kho.
    kho.uncensor("core.steel");
    assert_eq!(kho.accessible().len(), 2);
}

/// **Chép sai sinh ra trường phái mới**, và không ai biết bên nào chính thống.
#[test]
fn chep_sai_sinh_ra_truong_phai_moi() {
    let goc = sach(1_000);
    let mut a = goc.copy(2, 950);
    let mut b = goc.copy(3, 300);
    for i in 0..5 {
        a = a.copy(10 + i, 950);
        b = b.copy(20 + i, 300);
    }

    let chinh_thong = Lineage {
        school: "orthodox".into(),
        canonical: a,
    };
    let di_giao = Lineage {
        school: "reformed".into(),
        canonical: b,
    };

    assert!(chinh_thong.divergence(&di_giao) > 100);
    assert!(
        !chinh_thong.still_same_tradition(&di_giao, 100),
        "hai dòng đã tách mà vẫn bị coi là một truyền thống"
    );
    // Và không có hàm nào nói bên nào đúng — vì không ai trong world biết.
}
