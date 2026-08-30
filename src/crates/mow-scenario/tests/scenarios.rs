//! Test bộ chạy kịch bản, cộng ba kịch bản khói.

use mow_scenario::testing::TestWorldFactory;
use mow_scenario::{run, Predicate, RunError, Scenario};
use std::collections::BTreeMap;

fn thu_muc_kich_ban() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/scenarios/smoke")
}

// ─────────────────────────────────────────────────────────────────────────────
// Ba kịch bản khói
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ba_kich_ban_khoi_deu_dat() {
    let dir = thu_muc_kich_ban();
    let mut so = 0;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("có thư mục kịch bản khói")
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();

    for p in entries {
        if p.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        let sc = Scenario::from_yaml(&text)
            .unwrap_or_else(|e| panic!("{} không phân tích được: {e}", p.display()));
        let rep = run(&sc, &TestWorldFactory)
            .unwrap_or_else(|e| panic!("{} không chạy được:\n{e}", p.display()));
        assert!(rep.passed(), "{}\n{rep}", p.display());
        so += 1;
    }
    assert_eq!(so, 3, "phải có đúng 3 kịch bản khói");
}

#[test]
fn kich_ban_cho_ket_qua_giong_nhau_qua_nhieu_lan_chay() {
    // Đây là điều kiện để một kịch bản có giá trị: chạy lại phải ra cùng thế
    // giới, cùng ràng buộc, cùng hash.
    let text = std::fs::read_to_string(thu_muc_kich_ban().join("bind_total_order.yaml")).unwrap();
    let sc = Scenario::from_yaml(&text).unwrap();

    let a = run(&sc, &TestWorldFactory).unwrap();
    let b = run(&sc, &TestWorldFactory).unwrap();
    assert_eq!(a.bindings, b.bindings);
    assert_eq!(a.state_hash, b.state_hash);
}

#[test]
fn bao_cao_ghi_lai_alias_tro_toi_id_nao() {
    // §P7.3 quy tắc 3.
    let text = std::fs::read_to_string(thu_muc_kich_ban().join("bind_total_order.yaml")).unwrap();
    let sc = Scenario::from_yaml(&text).unwrap();
    let rep = run(&sc, &TestWorldFactory).unwrap();
    assert!(rep.bindings.contains_key("@elder"));
    assert!(rep.to_string().contains("@elder →"), "{rep}");
}

// ─────────────────────────────────────────────────────────────────────────────
// Quy tắc của `bind`
// ─────────────────────────────────────────────────────────────────────────────

fn kich_ban(yaml: &str) -> Scenario {
    Scenario::from_yaml(yaml).expect("phân tích được")
}

#[test]
fn order_thieu_id_asc_bi_tu_choi() {
    // §P7.3 quy tắc 1. Đây là bài quan trọng nhất của cả file: thiếu vế phá
    // hòa thì kịch bản chập chờn, và kịch bản chập chờn dạy cả đội bỏ qua đỏ.
    let sc = kich_ban(
        r#"
scenario: thieu_pha_hoa
worldseed: "test:tiny_village"
bind:
  "@ai_do": { kind: entity, select: first, order: ["age desc"] }
then:
  - assert_entity_count: 5
"#,
    );
    let e = run(&sc, &TestWorldFactory).expect_err("phải bị từ chối");
    let s = e.to_string();
    assert!(s.contains("id asc"), "{s}");
}

#[test]
fn bo_chon_khong_khop_la_loi_khong_phai_bo_qua() {
    // §P7.3 quy tắc 2.
    let sc = kich_ban(
        r#"
scenario: khong_khop
worldseed: "test:tiny_village"
bind:
  "@rong": { kind: dragon, select: first, order: ["id asc"] }
then:
  - assert_entity_count: 5
"#,
    );
    let e = run(&sc, &TestWorldFactory).expect_err("phải là lỗi");
    assert!(matches!(e, RunError::NoMatch { .. }), "{e}");
}

#[test]
fn then_rong_bi_tu_choi() {
    let sc = kich_ban(
        r#"
scenario: khong_khang_dinh_gi
worldseed: "test:tiny_village"
then: []
"#,
    );
    let e = run(&sc, &TestWorldFactory).expect_err("phải bị từ chối");
    assert!(e.to_string().contains("luôn xanh"), "{e}");
}

#[test]
fn nth_dem_tu_1() {
    let sc = kich_ban(
        r#"
scenario: nth_tu_0
worldseed: "test:tiny_village"
bind:
  "@x": { kind: entity, select: nth, n: 0, order: ["id asc"] }
then:
  - assert_entity_count: 5
"#,
    );
    let e = run(&sc, &TestWorldFactory).expect_err("phải bị từ chối");
    assert!(e.to_string().contains("đếm từ 1"), "{e}");
}

#[test]
fn in_tro_toi_alias_khong_ton_tai_bi_bat() {
    let sc = kich_ban(
        r#"
scenario: in_sai
worldseed: "test:tiny_village"
bind:
  "@x": { kind: entity, in: "@khong_co", select: first, order: ["id asc"] }
then:
  - assert_entity_count: 5
"#,
    );
    let e = run(&sc, &TestWorldFactory).expect_err("phải bị từ chối");
    assert!(e.to_string().contains("@khong_co"), "{e}");
}

#[test]
fn buoc_khong_biet_thi_liet_ke_buoc_da_co() {
    let sc = kich_ban(
        r#"
scenario: buoc_la
worldseed: "test:tiny_village"
when:
  - lam_phep_thuat: { x: 1 }
then:
  - assert_entity_count: 5
"#,
    );
    let e = run(&sc, &TestWorldFactory).expect_err("phải lỗi");
    let s = e.to_string();
    assert!(s.contains("lam_phep_thuat"), "{s}");
    assert!(s.contains("core.spawn"), "lỗi phải liệt kê bước đã có: {s}");
}

#[test]
fn khang_dinh_khong_biet_thi_truot_chu_khong_am_tham_xanh() {
    let sc = kich_ban(
        r#"
scenario: khang_dinh_la
worldseed: "test:tiny_village"
then:
  - assert_dieu_gi_do: { x: 1 }
"#,
    );
    let rep = run(&sc, &TestWorldFactory).unwrap();
    assert!(!rep.passed(), "khẳng định không biết phải trượt");
}

#[test]
fn assert_invariants_go_nham_id_thi_truot() {
    // Gõ nhầm `INV-22-4` thành `INV-22-04` sẽ làm khẳng định luôn xanh nếu
    // runner chỉ lọc rồi thấy danh sách rỗng.
    let sc = kich_ban(
        r#"
scenario: go_nham_id
worldseed: "test:tiny_village"
then:
  - assert_invariants: { ids: ["INV-22-999"] }
"#,
    );
    let rep = run(&sc, &TestWorldFactory).unwrap();
    assert!(!rep.passed(), "id bất biến không tồn tại phải làm trượt");
    assert!(rep.assertions[0].detail.contains("INV-22-999"));
}

#[test]
fn run_until_het_han_ma_vi_tu_chua_dung_thi_loi() {
    let sc = kich_ban(
        r#"
scenario: het_han
worldseed: "test:tiny_village"
when:
  - run_until: { predicate: "event.kind == 'khong.bao.gio'", max_ticks: 5 }
then:
  - assert_entity_count: 5
"#,
    );
    let e = run(&sc, &TestWorldFactory).expect_err("phải hết hạn");
    assert!(matches!(e, RunError::Timeout { .. }), "{e}");
}

// ─────────────────────────────────────────────────────────────────────────────
// Vị từ
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn phan_tich_vi_tu() {
    let p = Predicate::parse("event.kind == 'crime.committed' && event.actor == @aren").unwrap();
    assert_eq!(p.paths(), vec!["event.actor", "event.kind"]);
    assert_eq!(p.aliases(), vec!["@aren"]);
}

#[test]
fn vi_tu_danh_gia_dung() {
    use mow_scenario::Val;
    let mut ctx = BTreeMap::new();
    ctx.insert("tick".to_owned(), Val::Int(100));
    ctx.insert("event.kind".to_owned(), Val::Text("a.b".to_owned()));
    let al = BTreeMap::new();

    assert!(Predicate::parse("tick >= 100").unwrap().eval(&ctx, &al));
    assert!(!Predicate::parse("tick > 100").unwrap().eval(&ctx, &al));
    assert!(Predicate::parse("event.kind == 'a.b'")
        .unwrap()
        .eval(&ctx, &al));
    assert!(Predicate::parse("tick < 50 || tick == 100")
        .unwrap()
        .eval(&ctx, &al));
    assert!(!Predicate::parse("tick < 50 && tick == 100")
        .unwrap()
        .eval(&ctx, &al));
}

#[test]
fn so_sanh_voi_truong_khong_ton_tai_luon_sai_ke_ca_khac() {
    // Nếu `!=` với trường không tồn tại trả `true`, thì một lỗi chính tả trong
    // tên trường sẽ làm `run_until` dừng ở tick đầu tiên và kịch bản xanh mà
    // chẳng chứng minh gì.
    let ctx = BTreeMap::new();
    let al = BTreeMap::new();
    assert!(!Predicate::parse("event.kind == 'x'")
        .unwrap()
        .eval(&ctx, &al));
    assert!(
        !Predicate::parse("event.kind != 'x'")
            .unwrap()
            .eval(&ctx, &al),
        "`!=` với trường không tồn tại phải là sai, không phải đúng"
    );
}

#[test]
fn vi_tu_sai_cu_phap_bi_bat() {
    assert!(Predicate::parse("tick").is_err());
    assert!(Predicate::parse("tick ==").is_err());
    assert!(Predicate::parse("tick == 'chua dong").is_err());
}

#[test]
fn vi_tu_co_ngoac() {
    use mow_scenario::Val;
    let mut ctx = BTreeMap::new();
    ctx.insert("a".to_owned(), Val::Int(1));
    ctx.insert("b".to_owned(), Val::Int(2));
    let al = BTreeMap::new();
    assert!(Predicate::parse("(a == 1 || b == 99) && b == 2")
        .unwrap()
        .eval(&ctx, &al));
    assert!(!Predicate::parse("(a == 9 || b == 99) && b == 2")
        .unwrap()
        .eval(&ctx, &al));
}
