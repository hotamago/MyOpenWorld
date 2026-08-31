//! Test khung xem chuỗi nhân quả (`PC-16`, `§18.10`).

use mow_core::{BranchId, Event, EventKind, EventSeq, Tick, Value, WorldId};
use mow_obs::ChainView;

fn ev(seq: u64, tick: u64, kind: &str, cause: Option<u64>) -> Event {
    Event {
        seq: EventSeq(seq),
        branch: BranchId(1),
        world: WorldId(1),
        tick: Tick(tick),
        kind: EventKind::of(kind),
        actor: None,
        subject: None,
        payload: Value::Int(0),
        cause: cause.map(EventSeq),
        law_version: None,
        norm_set_version: None,
    }
}

/// Câu chuyện: mất mùa → đói → trộm → bị bắt → bị đuổi khỏi làng.
fn nan_doi() -> Vec<Event> {
    vec![
        ev(1, 100, "world.harvest.failed", None),
        ev(2, 200, "life.need.hunger.critical", Some(1)),
        ev(3, 210, "action.theft.committed", Some(2)),
        ev(4, 215, "society.crime.witnessed", Some(3)),
        ev(5, 300, "society.trial.verdict", Some(4)),
        ev(6, 310, "society.exile.enacted", Some(5)),
        // Một nhánh khác cũng từ vụ trộm.
        ev(7, 220, "society.rumor.spread", Some(3)),
    ]
}

/// Lời hứa của `§23`: từ một biến cố lớn, truy được về tận nguyên nhân.
#[test]
fn truy_tu_bien_co_lon_ve_tan_nguyen_nhan() {
    let v = ChainView::new(nan_doi());
    let c = v.chain(EventSeq(6), 10).expect("có sự kiện");

    let nguoc: Vec<u64> = c.backward.iter().map(|l| l.seq.0).collect();
    assert_eq!(nguoc, vec![5, 4, 3, 2, 1], "gần trước xa sau");
    assert!(!c.truncated_backward, "đã tới khởi nguồn thật");
}

#[test]
fn goc_re_xa_nhat_la_mat_mua() {
    let v = ChainView::new(nan_doi());
    assert_eq!(v.root_cause(EventSeq(6)), Some(EventSeq(1)));
}

/// Chiều xuôi phải thấy **cả hai** nhánh hệ quả, không chỉ nhánh chính.
#[test]
fn chieu_xuoi_thay_moi_nhanh_he_qua() {
    let v = ChainView::new(nan_doi());
    let c = v.chain(EventSeq(3), 10).unwrap();
    let xuoi: Vec<u64> = c.forward.iter().map(|l| l.seq.0).collect();
    assert!(
        xuoi.contains(&4) && xuoi.contains(&7),
        "thiếu nhánh: {xuoi:?}"
    );
    assert!(xuoi.contains(&6), "hệ quả xa phải tới được");
}

/// Duyệt bề rộng: mắt xích gần hiện trước mắt xích xa.
#[test]
fn he_qua_gan_hien_truoc_he_qua_xa() {
    let v = ChainView::new(nan_doi());
    let c = v.chain(EventSeq(3), 10).unwrap();
    let sau: Vec<i32> = c.forward.iter().map(|l| l.depth).collect();
    assert!(
        sau.windows(2).all(|w| w[0] <= w[1]),
        "không theo bề rộng: {sau:?}"
    );
}

/// "Đây là khởi nguồn" và "tôi ngừng tìm ở đây" trông giống hệt nhau trên màn
/// hình nếu không nói ra.
#[test]
fn cat_ngan_vi_do_sau_phai_noi_ro_khong_gia_lam_khoi_nguon() {
    let v = ChainView::new(nan_doi());

    let cat = v.chain(EventSeq(6), 2).unwrap();
    assert_eq!(cat.backward.len(), 2);
    assert!(cat.truncated_backward, "cắt ngắn mà không nói");

    let du = v.chain(EventSeq(6), 10).unwrap();
    assert!(!du.truncated_backward, "tới khởi nguồn mà lại bảo là cắt");
}

/// Nguyên nhân nằm ngoài lát cắt cũng là cắt ngắn, cùng lý do.
#[test]
fn nguyen_nhan_ngoai_lat_cat_cung_la_cat_ngan() {
    // Chỉ lấy nửa sau của câu chuyện.
    let v = ChainView::new(nan_doi().into_iter().filter(|e| e.seq.0 >= 4));
    let c = v.chain(EventSeq(6), 10).unwrap();
    assert_eq!(
        c.backward.iter().map(|l| l.seq.0).collect::<Vec<_>>(),
        vec![5, 4]
    );
    assert!(c.truncated_backward);
}

/// Danh sách hệ quả rỗng phải đọc được đúng: "chưa có, **tính tới đây**".
#[test]
fn danh_sach_he_qua_rong_van_noi_ro_da_quet_toi_dau() {
    let v = ChainView::new(nan_doi());
    let c = v.chain(EventSeq(6), 10).unwrap();
    assert!(c.forward.is_empty());
    assert_eq!(c.scanned_to, EventSeq(7), "không nói đã quét tới đâu");
}

/// **Chỉ event có thật trong log.** Hỏi một seq không tồn tại thì không có gì.
///
/// Không trả về một mắt xích rỗng, không bịa một lời giải thích. `§22.17`.
#[test]
fn chi_hien_event_co_that() {
    let v = ChainView::new(nan_doi());
    assert!(v.chain(EventSeq(999), 10).is_none());
}

/// Version của luật **và** của chuẩn mực đều phải có mặt (`§22.49`).
#[test]
fn boi_canh_mang_ca_version_luat_lan_chuan_muc() {
    let mut e = ev(1, 10, "action.theft.committed", None);
    e.law_version = Some(7);
    e.norm_set_version = Some(3);

    let v = ChainView::new([e]);
    let c = v.chain(EventSeq(1), 1).unwrap();
    assert_eq!(c.root.law_version, Some(7));
    assert_eq!(
        c.root.norm_set_version,
        Some(3),
        "thiếu chuẩn mực thì không trả lời được vì sao cả làng phản ứng như thế"
    );
}

/// Mỗi mắt xích mang `tick` để "nhảy tới đúng lúc".
#[test]
fn moi_mat_xich_mang_tick_de_nhay_toi() {
    let v = ChainView::new(nan_doi());
    let c = v.chain(EventSeq(6), 10).unwrap();
    assert_eq!(c.root.tick, 310);
    assert!(c.backward.iter().all(|l| l.tick > 0));
}

/// Dòng thời gian để vẽ: ngược (xa→gần), gốc, rồi xuôi.
#[test]
fn dong_thoi_gian_de_ve_dung_thu_tu() {
    let v = ChainView::new(nan_doi());
    let c = v.chain(EventSeq(3), 2).unwrap();
    let t: Vec<u64> = c.timeline().iter().map(|l| l.seq.0).collect();
    assert_eq!(t[0], 1, "xa nhất trước");
    assert_eq!(t[2], 3, "gốc ở giữa");
    assert!(t.len() > 3);
}

/// Nhật ký hỏng có thể chứa chu trình. Đi vòng mãi ở một công cụ chẩn đoán là
/// cách tệ nhất để phát hiện điều đó.
#[test]
fn chu_trinh_trong_nhat_ky_khong_lam_treo() {
    let v = ChainView::new([ev(1, 1, "a", Some(2)), ev(2, 2, "b", Some(1))]);
    assert!(v.root_cause(EventSeq(1)).is_some());
    assert!(v.chain(EventSeq(1), 100).is_some());
}

/// Chỉ mục là **chỉ đọc**: dựng hai lần từ cùng dữ liệu cho cùng kết quả.
#[test]
fn truy_chuoi_lap_lai_duoc() {
    let a = ChainView::new(nan_doi()).chain(EventSeq(3), 5).unwrap();
    let b = ChainView::new(nan_doi()).chain(EventSeq(3), 5).unwrap();
    assert_eq!(a, b);
}

/// Thứ tự đầu vào không đổi kết quả — nhật ký có thể đọc lên theo thứ tự nào.
#[test]
fn thu_tu_dau_vao_khong_doi_ket_qua() {
    let mut nguoc = nan_doi();
    nguoc.reverse();
    let a = ChainView::new(nan_doi()).chain(EventSeq(3), 5).unwrap();
    let b = ChainView::new(nguoc).chain(EventSeq(3), 5).unwrap();
    assert_eq!(a, b);
}
