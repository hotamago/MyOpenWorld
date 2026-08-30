use mow_core::{BranchId, Tick, WorldId};
use mow_obs::{log_event, Budget, Level, SimContext, Span};

fn ctx() -> SimContext {
    SimContext::new(BranchId(2), WorldId(3), Tick(4500))
}

#[test]
fn moi_dong_log_deu_co_branch_world_tick() {
    let r = log_event(Level::Info, ctx(), "vat pham bien mat");
    let j = r.to_json();
    assert!(j.contains("\"branch\":2"), "{j}");
    assert!(j.contains("\"world\":3"), "{j}");
    assert!(j.contains("\"tick\":4500"), "{j}");
}

#[test]
fn dang_nguoi_doc_cung_co_du_ngu_canh() {
    let s = log_event(Level::Warn, ctx(), "lam phat bat thuong").to_pretty();
    assert!(s.contains("b2 w3 t4500"), "{s}");
}

#[test]
fn truong_phu_sap_theo_khoa() {
    // Thu tu on dinh de diff hai log khong ra nhieu thay doi gia.
    let r = log_event(Level::Info, ctx(), "x")
        .field("z", 1)
        .field("a", 2)
        .field("m", 3);
    let j = r.to_json();
    let ia = j.find("\"a\"").unwrap();
    let im = j.find("\"m\"").unwrap();
    let iz = j.find("\"z\"").unwrap();
    assert!(ia < im && im < iz, "{j}");
}

#[test]
fn span_con_giu_nguyen_trace_id() {
    let goc = Span::root("apply_command", ctx());
    let con = goc.child("emit_event");
    assert_eq!(
        goc.trace_id, con.trace_id,
        "duong command -> event phai cung mot trace"
    );
    assert_eq!(con.parent, Some(goc.trace_id));
}

#[test]
fn span_goc_khac_nhau_thi_trace_khac_nhau() {
    let a = Span::root("a", ctx());
    let b = Span::root("b", ctx());
    assert_ne!(a.trace_id, b.trace_id);
}

#[test]
fn log_trong_span_mang_theo_trace_id() {
    let s = Span::root("apply_command", ctx());
    let j = log_event(Level::Info, ctx(), "xong").in_span(&s).to_json();
    assert!(j.contains("trace_id"), "{j}");
}

#[test]
fn ngan_sach_bao_ro_vuot_bao_nhieu_phan_tram() {
    // 3% la nhieu do dac, 300% la hoi quy can chan ngay — bao cao phai phan biet.
    let b = Budget {
        name: "tick_p99",
        limit: 100,
        unit: "ms",
    };
    assert!(b.check(100).is_ok());
    let e = b.check(400).unwrap_err();
    assert!(e.contains("300%"), "{e}");
    assert!(e.contains("tick_p99"), "{e}");
}
