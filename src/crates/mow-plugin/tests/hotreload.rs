//! Test vòng lặp phát triển content pack (`PF-03`, `§P10.7`, `§19.7.3`).

use mow_math::StateHash;
use mow_plugin::hotreload::{
    plan_reload, BuildKind, PackSnapshot, ReloadError, ReloadStep, TestReport,
};
use std::collections::BTreeMap;

fn bam(s: &str) -> StateHash {
    let mut h = mow_math::StateHasher::new();
    h.write_str(s);
    h.finish()
}

fn anh(version: &str, defs: &[(&str, &str)]) -> PackSnapshot {
    let definitions: BTreeMap<String, StateHash> = defs
        .iter()
        .map(|(id, noi_dung)| ((*id).to_owned(), bam(noi_dung)))
        .collect();
    let mut h = mow_math::StateHasher::new();
    for (k, v) in &definitions {
        h.write_str(k);
        h.write_hash(*v);
    }
    PackSnapshot {
        pack: "bakery".to_owned(),
        version: version.to_owned(),
        hash: h.finish(),
        definitions,
    }
}

// ───────────────────── nạp nóng chỉ ở dev ─────────────────────

/// **Bản phát hành không nạp nóng.**
#[test]
fn ban_phat_hanh_khong_nap_nong() {
    let a = anh("1.0.0", &[("bakery.bread", "v1")]);
    let b = anh("1.1.0", &[("bakery.bread", "v2")]);
    assert_eq!(
        plan_reload(&a, &b, BuildKind::Release).unwrap_err(),
        ReloadError::NotDevBuild
    );
    assert!(plan_reload(&a, &b, BuildKind::Dev).is_ok());
}

/// `BuildKind` là một kiểu, không phải một `bool` truyền quanh.
#[test]
fn build_kind_noi_ro_minh_la_gi() {
    assert!(BuildKind::Dev.allows_hot_reload());
    assert!(!BuildKind::Release.allows_hot_reload());
    // Test chạy ở dev build.
    assert_eq!(BuildKind::current(), BuildKind::Dev);
}

// ───────────────────── không ghi đè tại chỗ ─────────────────────

/// **Đổi nội dung mà không tăng version thì bị từ chối.**
///
/// Đây là chỗ `INV-22-9` được bảo vệ: event log ghi *"dùng `bakery.bread` v1"*.
/// Nếu v1 đổi nội dung tại chỗ thì replay cùng log ra kết quả khác, và không có
/// gì báo — thế giới vẫn chạy, save vẫn mở được, chỉ là hash không tái lập.
#[test]
fn doi_noi_dung_ma_khong_tang_version_thi_bi_tu_choi() {
    let a = anh("1.0.0", &[("bakery.bread", "v1")]);
    let b = anh("1.0.0", &[("bakery.bread", "da sua")]);
    assert!(matches!(
        plan_reload(&a, &b, BuildKind::Dev).unwrap_err(),
        ReloadError::VersionNotBumped { .. }
    ));
}

/// Không đổi gì thì version giữ nguyên cũng được, và kế hoạch rỗng.
#[test]
fn khong_doi_gi_thi_ke_hoach_rong() {
    let a = anh("1.0.0", &[("bakery.bread", "v1")]);
    let ke = plan_reload(&a, &a, BuildKind::Dev).unwrap();
    assert!(ke.is_empty());
    assert!(ke.affects().is_empty());
}

/// **Sửa một định nghĩa ⇒ version mới, bản cũ ở lại.**
#[test]
fn sua_dinh_nghia_thi_tao_version_moi_va_giu_ban_cu() {
    let a = anh("1.0.0", &[("bakery.bread", "v1")]);
    let b = anh("1.1.0", &[("bakery.bread", "v2")]);
    let ke = plan_reload(&a, &b, BuildKind::Dev).unwrap();

    assert_eq!(
        ke.steps,
        vec![ReloadStep::Supersede {
            id: "bakery.bread".to_owned(),
            old_version: "1.0.0".to_owned(),
            new_version: "1.1.0".to_owned(),
        }]
    );
    assert!(ke.preserves_replay());
}

/// **Không có bước nào ghi đè tại chỗ**, dù kế hoạch phức tạp thế nào.
#[test]
fn khong_co_buoc_nao_ghi_de_tai_cho() {
    let a = anh(
        "1.0.0",
        &[
            ("bakery.bread", "v1"),
            ("bakery.cake", "v1"),
            ("bakery.oven", "v1"),
        ],
    );
    let b = anh(
        "2.0.0",
        &[
            ("bakery.bread", "v2"), // sửa
            ("bakery.oven", "v1"),  // giữ
            ("bakery.pastry", "moi"), // thêm
                                    // bakery.cake bị xóa
        ],
    );
    let ke = plan_reload(&a, &b, BuildKind::Dev).unwrap();
    assert!(ke.preserves_replay());
    assert_eq!(ke.steps.len(), 3);
}

/// Thêm id mới **không** đụng tới thế giới đang chạy.
#[test]
fn them_id_moi_khong_dung_toi_the_gioi_dang_chay() {
    let a = anh("1.0.0", &[("bakery.bread", "v1")]);
    let b = anh("1.1.0", &[("bakery.bread", "v1"), ("bakery.cake", "moi")]);
    let ke = plan_reload(&a, &b, BuildKind::Dev).unwrap();

    assert_eq!(
        ke.steps,
        vec![ReloadStep::Add {
            id: "bakery.cake".to_owned()
        }]
    );
    assert!(
        ke.affects().is_empty(),
        "không event nào tham chiếu một id chưa từng tồn tại"
    );
}

/// **Xóa một định nghĩa thành tombstone, không thành biến mất.**
///
/// Xóa thẳng để lại tham chiếu treo, và chúng lộ ra rải rác hàng giờ sau chứ
/// không lộ ra lúc nạp.
#[test]
fn xoa_dinh_nghia_thanh_tombstone_khong_thanh_bien_mat() {
    let a = anh("1.0.0", &[("bakery.bread", "v1"), ("bakery.cake", "v1")]);
    let b = anh("2.0.0", &[("bakery.bread", "v1")]);
    let ke = plan_reload(&a, &b, BuildKind::Dev).unwrap();

    assert_eq!(
        ke.steps,
        vec![ReloadStep::Tombstone {
            id: "bakery.cake".to_owned(),
            removed_at: "2.0.0".to_owned(),
        }]
    );
    assert_eq!(ke.affects(), vec!["bakery.cake"]);
}

/// Kế hoạch nói rõ **id nào** đổi, không chỉ "có gì đổi".
#[test]
fn ke_hoach_noi_ro_id_nao_doi() {
    let a = anh(
        "1.0.0",
        &[
            ("bakery.bread", "v1"),
            ("bakery.cake", "v1"),
            ("bakery.oven", "v1"),
        ],
    );
    let b = anh(
        "1.1.0",
        &[
            ("bakery.bread", "v2"),
            ("bakery.cake", "v1"),
            ("bakery.oven", "v1"),
        ],
    );
    let ke = plan_reload(&a, &b, BuildKind::Dev).unwrap();
    assert_eq!(ke.affects(), vec!["bakery.bread"], "chỉ một cái đổi");
}

/// Kế hoạch **xác định**: cùng hai ảnh chụp cho cùng kế hoạch.
#[test]
fn ke_hoach_xac_dinh() {
    let a = anh("1.0.0", &[("bakery.b", "1"), ("bakery.a", "1")]);
    let b = anh("1.1.0", &[("bakery.b", "2"), ("bakery.a", "2")]);
    assert_eq!(
        plan_reload(&a, &b, BuildKind::Dev).unwrap(),
        plan_reload(&a, &b, BuildKind::Dev).unwrap()
    );
    // Và thứ tự bước theo id, không theo thứ tự chèn.
    let ke = plan_reload(&a, &b, BuildKind::Dev).unwrap();
    assert_eq!(ke.steps[0].id(), "bakery.a");
}

// ───────────────────── `pack test` ─────────────────────

/// **Pack không khai test nào là một phát hiện, không phải một điểm đạt.**
#[test]
fn pack_khong_khai_test_nao_la_mot_phat_hien() {
    let trong = TestReport {
        pack: "bakery".to_owned(),
        scenarios: Vec::new(),
    };
    assert!(trong.has_no_tests());
    // `passed()` trên tập rỗng là `true` theo toán học — nên chỗ gọi **phải**
    // hỏi `has_no_tests()` riêng. Bài này ghim đúng cái bẫy đó.
    assert!(trong.passed());
}

/// Trượt thì nói rõ kịch bản nào.
#[test]
fn truot_thi_noi_ro_kich_ban_nao() {
    let r = TestReport {
        pack: "bakery".to_owned(),
        scenarios: vec![
            ("smoke/genesis.yaml".to_owned(), true),
            ("bakery/bread_rises.yaml".to_owned(), false),
        ],
    };
    assert!(!r.passed());
    assert_eq!(r.failures(), vec!["bakery/bread_rises.yaml"]);
    assert!(!r.has_no_tests());
}
