//! Test kinh tế và đời sống.

use mow_math::{Money, Rate, WorldPos};
use mow_society::economy::{Market, Order, Recipe, Source};
use mow_society::household::{ContactGraph, Household, HouseholdStage, Place, PlaceKind};
use std::collections::BTreeMap;

fn mo_sat() -> Source {
    Source {
        id: "core.iron_vein".into(),
        yields: "core.iron_ore".into(),
        remaining: 100,
        capacity: 100,
        // Không tái tạo: một mỏ cạn thì cạn thật.
        regen: Rate::ZERO,
        carry: 0,
    }
}

fn rung() -> Source {
    Source {
        id: "core.forest".into(),
        yields: "core.wood".into(),
        remaining: 50,
        capacity: 500,
        // 1 đơn vị mỗi 1000 tick — nhỏ hơn một đơn vị mỗi tick rất nhiều.
        regen: Rate::new(1, 1_000).unwrap(),
        carry: 0,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PB-13 — tài nguyên có nguồn thật
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn khong_lay_duoc_nhieu_hon_tru_luong() {
    let mut m = mo_sat();
    assert_eq!(m.extract(150), 100, "lấy được nhiều hơn số có");
    assert_eq!(m.remaining, 0);
    assert_eq!(m.extract(10), 0, "mỏ cạn vẫn cho ra quặng");
}

#[test]
fn mo_can_thi_can_that() {
    let mut m = mo_sat();
    m.extract(100);
    m.regenerate(1_000_000);
    assert!(m.is_exhausted(), "mỏ không tái tạo lại mọc lại");
    assert_eq!(m.remaining, 0);
}

#[test]
fn rung_moc_lai_du_toc_do_nho_hon_mot_don_vi_moi_tick() {
    // Cùng bài học với tỉ lệ đột biến và với ngăn S/E/I/R: làm tròn về 0 thì
    // rừng không bao giờ mọc lại.
    let mut r = rung();
    let truoc = r.remaining;
    r.regenerate(10_000);
    assert_eq!(r.remaining, truoc + 10, "1/1000 mỗi tick bị làm tròn về 0");
}

#[test]
fn tai_tao_khong_vuot_suc_chua() {
    let mut r = rung();
    r.regenerate(100_000_000);
    assert_eq!(r.remaining, r.capacity);
}

#[test]
fn tai_tao_chia_nho_bang_tai_tao_mot_lan() {
    let mot_lan = {
        let mut r = rung();
        r.regenerate(30_000);
        r.remaining
    };
    let chia_nho = {
        let mut r = rung();
        for _ in 0..30 {
            r.regenerate(1_000);
        }
        r.remaining
    };
    assert_eq!(mot_lan, chia_nho);
}

#[test]
fn cong_thuc_bien_thu_nay_thanh_thu_khac() {
    let banh = Recipe {
        id: "core.bake_bread".into(),
        inputs: vec![("core.flour".into(), 2), ("core.water".into(), 1)],
        outputs: vec![("core.bread".into(), 3)],
        ticks: 600,
        skill_required: 20,
    };

    let mut kho: BTreeMap<String, i64> = BTreeMap::new();
    kho.insert("core.flour".into(), 5);
    kho.insert("core.water".into(), 5);

    assert!(banh.can_make(&kho));
    assert!(banh.apply(&mut kho));
    assert_eq!(kho["core.flour"], 3);
    assert_eq!(kho["core.bread"], 3);
}

#[test]
fn thieu_nguyen_lieu_thi_khong_lam_duoc() {
    // Bánh mì không xuất hiện từ hư không.
    let banh = Recipe {
        id: "core.bake_bread".into(),
        inputs: vec![("core.flour".into(), 2)],
        outputs: vec![("core.bread".into(), 3)],
        ticks: 600,
        skill_required: 20,
    };
    let mut kho = BTreeMap::new();
    assert!(!banh.can_make(&kho));
    assert!(!banh.apply(&mut kho));
    assert!(kho.is_empty(), "làm thất bại mà vẫn tạo ra sản phẩm");
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.35 — giá hình thành, không được đặt
// ─────────────────────────────────────────────────────────────────────────────

fn lenh(trader: u64, good: &str, qty: u32, gia: i64) -> Order {
    Order {
        trader,
        good: good.into(),
        quantity: qty,
        limit_price: Money::new(gia),
    }
}

#[test]
fn chua_ai_ban_thi_khong_co_gia() {
    // `None` là thông tin quan trọng: trả 0 sẽ biến "chưa ai biết" thành "miễn phí".
    let m = Market::new("core.village");
    assert_eq!(m.last("core.bread"), None);
}

#[test]
fn gia_khop_la_trung_diem_khong_phai_gia_mot_ben() {
    // Nếu luôn lấy giá người bán thì người mua không bao giờ được lợi, và không
    // ai có động cơ mặc cả.
    let mut m = Market::new("core.village");
    m.bid(lenh(1, "core.bread", 5, 12));
    m.ask(lenh(2, "core.bread", 5, 8));

    let gd = m.clear();
    assert_eq!(gd.len(), 1);
    assert_eq!(gd[0].price, Money::new(10));
    assert_eq!(m.last("core.bread"), Some(Money::new(10)));
}

#[test]
fn nguoi_tra_cao_nhat_khop_voi_nguoi_ban_re_nhat() {
    let mut m = Market::new("core.village");
    m.bid(lenh(1, "g", 1, 5));
    m.bid(lenh(2, "g", 1, 20));
    m.ask(lenh(3, "g", 1, 15));
    m.ask(lenh(4, "g", 1, 4));

    let gd = m.clear();
    assert_eq!(gd[0].buyer, 2, "người trả cao nhất phải khớp trước");
    assert_eq!(gd[0].seller, 4, "người bán rẻ nhất phải khớp trước");
}

#[test]
fn khong_khop_khi_nguoi_mua_tra_thap_hon_nguoi_ban_doi() {
    let mut m = Market::new("core.village");
    m.bid(lenh(1, "g", 5, 5));
    m.ask(lenh(2, "g", 5, 20));
    assert!(m.clear().is_empty());
    assert_eq!(m.open_orders(), 2, "lệnh không khớp phải còn nguyên");
}

#[test]
fn thu_tu_dat_lenh_khong_quyet_dinh_ai_khop_truoc() {
    let mk = |dao: bool| {
        let mut m = Market::new("v");
        let bids = [lenh(1, "g", 1, 10), lenh(2, "g", 1, 20)];
        let ds: Vec<_> = if dao {
            bids.iter().rev().cloned().collect()
        } else {
            bids.to_vec()
        };
        for b in ds {
            m.bid(b);
        }
        m.ask(lenh(9, "g", 1, 5));
        m.clear()[0].buyer
    };
    assert_eq!(mk(false), mk(true));
}

#[test]
fn khop_mot_phan_thi_phan_con_lai_van_cho() {
    let mut m = Market::new("v");
    m.bid(lenh(1, "g", 10, 10));
    m.ask(lenh(2, "g", 3, 5));
    let gd = m.clear();
    assert_eq!(gd[0].quantity, 3);
    assert_eq!(m.open_orders(), 1, "phần chưa khớp phải còn lại");
}

#[test]
fn chenh_lech_cung_cau_la_tin_hieu_cho_ai() {
    let mut m = Market::new("v");
    m.bid(lenh(1, "core.bread", 20, 10));
    m.ask(lenh(2, "core.bread", 5, 8));
    assert_eq!(m.imbalance("core.bread"), 15, "thiếu bánh");
    assert_eq!(m.imbalance("core.iron"), 0);
}

#[test]
fn hai_lang_co_gia_khac_nhau() {
    // `§12.17`: hàng hóa không teleport, nên giá phải khác nhau. Một thị trường
    // toàn cầu duy nhất sẽ xóa mất thương mại như một hoạt động.
    let mut a = Market::new("core.village_a");
    let mut b = Market::new("core.village_b");
    a.bid(lenh(1, "g", 1, 20));
    a.ask(lenh(2, "g", 1, 18));
    b.bid(lenh(3, "g", 1, 6));
    b.ask(lenh(4, "g", 1, 4));
    a.clear();
    b.clear();
    assert_ne!(a.last("g"), b.last("g"));
}

// ─────────────────────────────────────────────────────────────────────────────
// PB-14 — hộ gia đình và địa điểm
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ho_rong_thi_tu_tan() {
    // Một hộ không còn ai mà vẫn tồn tại sẽ giữ kho chung mãi mãi, và của cải
    // trong đó biến mất khỏi kinh tế.
    let mut h = Household::new(1, WorldPos::ORIGIN, [10, 11]);
    assert_eq!(h.stage, HouseholdStage::Forming);
    h.remove(10);
    assert!(!h.is_dissolved());
    h.remove(11);
    assert!(h.is_dissolved());
}

#[test]
fn thanh_vien_duyet_theo_thu_tu_dinh_danh() {
    let h = Household::new(1, WorldPos::ORIGIN, [30, 10, 20]);
    assert_eq!(h.members().collect::<Vec<_>>(), vec![10, 20, 30]);
    assert_eq!(h.size(), 3);
}

#[test]
fn gieng_co_hang_doi_that() {
    let mut g = Place::new("core.well", PlaceKind::Well, WorldPos::ORIGIN, 2, 10);

    // Hai người đầu được phục vụ ngay.
    assert_eq!(g.arrive(1, 0), 0);
    assert_eq!(g.arrive(2, 0), 0);
    // Người thứ ba phải chờ.
    assert_eq!(g.arrive(3, 0), 1);
    assert_eq!(g.arrive(4, 0), 2);
    assert_eq!(g.queue_len(), 2);

    // Sau 10 tick, hai người đầu xong và hai người sau vào.
    let xong = g.tick(10);
    assert_eq!(xong, vec![1, 2]);
    assert_eq!(g.queue_len(), 0);
    assert_eq!(g.present(), vec![3, 4]);
}

#[test]
fn nguoi_dung_cho_cung_duoc_tinh_la_co_mat() {
    // Ở một cái giếng đông, hàng chờ dài hơn chỗ múc nước nhiều lần. Chỉ đếm
    // người đang dùng sẽ bỏ sót phần lớn tiếp xúc.
    let mut g = Place::new("core.well", PlaceKind::Well, WorldPos::ORIGIN, 1, 10);
    for i in 1..=5 {
        g.arrive(i, 0);
    }
    assert_eq!(g.present().len(), 5);
}

#[test]
fn xep_hang_hai_lan_khong_nhan_doi() {
    let mut g = Place::new("core.well", PlaceKind::Well, WorldPos::ORIGIN, 1, 10);
    g.arrive(1, 0);
    g.arrive(2, 0);
    g.arrive(2, 0);
    assert_eq!(g.queue_len(), 1);
}

#[test]
fn roi_hang_duoc() {
    let mut g = Place::new("core.tavern", PlaceKind::Tavern, WorldPos::ORIGIN, 1, 100);
    g.arrive(1, 0);
    g.arrive(2, 0);
    assert!(g.leave(2));
    assert_eq!(g.queue_len(), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Đồ thị tiếp xúc nổi lên từ hành vi
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn do_thi_tiep_xuc_noi_len_tu_hanh_vi() {
    // Không được khai báo "hai người này quen nhau".
    let mut g = Place::new("core.well", PlaceKind::Well, WorldPos::ORIGIN, 3, 10);
    let mut ct = ContactGraph::new();
    assert!(ct.is_empty());

    for i in 1..=3 {
        g.arrive(i, 0);
    }
    ct.record(&g.present(), 10);

    assert_eq!(ct.contact_ticks(1, 2), 10);
    assert_eq!(ct.contact_ticks(2, 1), 10, "cạnh phải vô hướng");
    assert_eq!(ct.len(), 3, "ba người cho ba cặp");
}

#[test]
fn tiep_xuc_tich_luy_qua_nhieu_lan_gap() {
    let mut ct = ContactGraph::new();
    for _ in 0..5 {
        ct.record(&[1, 2], 10);
    }
    assert_eq!(ct.contact_ticks(1, 2), 50);
}

#[test]
fn dieu_tra_dich_te_doc_duoc_do_thi() {
    // "Ai đã ở gần bệnh nhân số 0 lâu nhất."
    let mut ct = ContactGraph::new();
    ct.record(&[1, 2], 100);
    ct.record(&[1, 3], 30);
    ct.record(&[1, 4], 60);

    let gan = ct.contacts_of(1);
    assert_eq!(gan[0], (2, 100));
    assert_eq!(gan[1], (4, 60));
    assert_eq!(gan[2], (3, 30));
}

#[test]
fn quen_dan_de_do_thi_khong_lon_vo_han() {
    // Không có bước này thì cuối cùng mọi người đều "quen" mọi người.
    let mut ct = ContactGraph::new();
    ct.record(&[1, 2], 10);
    ct.record(&[3, 4], 100);

    ct.decay(50);
    assert_eq!(ct.contact_ticks(1, 2), 0, "cặp yếu phải biến mất");
    assert_eq!(ct.contact_ticks(3, 4), 50);
    assert_eq!(ct.len(), 1);
}

#[test]
fn nguoi_moi_toi_khong_co_trong_do_thi() {
    // Và đó là cách người ta nhận ra họ.
    let mut ct = ContactGraph::new();
    ct.record(&[1, 2, 3], 100);
    assert!(ct.contacts_of(99).is_empty());
}
