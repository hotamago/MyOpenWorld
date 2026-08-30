//! Bộ test hợp đồng — **dùng lại nguyên vẹn cho backend thứ hai** (`PC-20`).
//!
//! Đây là lý do crate này tồn tại dưới dạng trait thay vì một struct SQLite
//! trần trụi. Khi Postgres được thêm vào ở Giai đoạn C, nó phải vượt qua đúng
//! những hàm dưới đây, **không sửa một dòng nào**. Nếu phải sửa thì trait đã rò
//! rỉ chi tiết cài đặt, và đó chính là phát hiện mà bộ test này tồn tại để tạo
//! ra.
//!
//! Cách dùng:
//!
//! ```no_run
//! use mow_persist::{contract, SqliteStore};
//!
//! #[test]
//! fn sqlite_dat_hop_dong() {
//!     contract::run_all(|| SqliteStore::in_memory().unwrap());
//! }
//! ```

use crate::store::{BranchRecord, EventRecord, Snapshot, Store};
use mow_core::{BranchId, EventSeq, Tick, WorldId};
use mow_math::StateHash;

const B1: BranchId = BranchId(1);
const B2: BranchId = BranchId(2);
const W1: WorldId = WorldId(1);

fn ev(branch: BranchId, seq: u64, tick: u64, kind: &str) -> EventRecord {
    EventRecord {
        seq: EventSeq(seq),
        branch,
        world: W1,
        tick: Tick(tick),
        kind: kind.to_owned(),
        actor: 0,
        subject: 0,
        payload: format!("payload-{seq}").into_bytes(),
        cause: None,
        law_version: None,
    }
}

/// Chạy toàn bộ hợp đồng.
///
/// `factory` phải trả về một kho **rỗng, độc lập** mỗi lần gọi. Nếu hai lần gọi
/// dùng chung state thì các bài dưới đây sẽ nhiễm nhau và bộ test trở thành vô
/// giá trị theo cách rất khó nhận ra.
pub fn run_all<S: Store, F: Fn() -> S>(factory: F) {
    ghi_them_va_doc_lai(&factory);
    doc_khoang_dung_bien(&factory);
    next_seq_dung_khi_rong_va_khi_co(&factory);
    ghi_theo_lo_la_nguyen_tu(&factory);
    nhanh_khac_nhau_khong_lan_sang_nhau(&factory);
    anh_chup_lay_ban_gan_nhat_khong_vuot_qua(&factory);
    anh_chup_ghi_de_cung_tick(&factory);
    dong_doi_tu_con_ve_goc(&factory);
    dong_doi_khong_treo_khi_dag_hong(&factory);
    payload_la_byte_duc(&factory);
}

/// Ghi rồi đọc lại phải ra đúng cái đã ghi, từng trường một.
pub fn ghi_them_va_doc_lai<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    let mut e = ev(B1, 0, 10, "core.entity.spawned");
    e.actor = 42;
    e.subject = 7;
    e.cause = Some(EventSeq(0));
    e.law_version = Some(3);
    s.append_events(&[e.clone()]).unwrap();

    let doc = s.read_events(B1, EventSeq(0), EventSeq(100)).unwrap();
    assert_eq!(doc.len(), 1, "hợp đồng: ghi một, đọc một");
    assert_eq!(doc[0], e, "hợp đồng: mọi trường phải đi và về nguyên vẹn");
}

/// `[from, to)` — mở ở đầu phải, đóng ở đầu trái.
///
/// Biên nửa mở là quy ước, và quy ước chỉ có giá trị khi được kiểm. Một backend
/// hiểu thành `[from, to]` sẽ trả thừa đúng một sự kiện ở mỗi lần đọc, và lỗi
/// đó biểu hiện thành replay lệch một bước — cực khó truy.
pub fn doc_khoang_dung_bien<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    let evs: Vec<_> = (0..5).map(|i| ev(B1, i, i * 10, "x")).collect();
    s.append_events(&evs).unwrap();

    let a = s.read_events(B1, EventSeq(1), EventSeq(3)).unwrap();
    assert_eq!(
        a.iter().map(|e| e.seq.0).collect::<Vec<_>>(),
        vec![1, 2],
        "hợp đồng: khoảng phải là nửa mở [from, to)"
    );

    let rong = s.read_events(B1, EventSeq(2), EventSeq(2)).unwrap();
    assert!(rong.is_empty(), "hợp đồng: [n, n) phải rỗng");
}

/// `next_seq` trên nhánh rỗng là 0; sau khi ghi tới `n` thì là `n+1`.
pub fn next_seq_dung_khi_rong_va_khi_co<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    assert_eq!(s.next_seq(B1).unwrap(), EventSeq(0));
    s.append_events(&[ev(B1, 0, 0, "x"), ev(B1, 1, 1, "x")])
        .unwrap();
    assert_eq!(s.next_seq(B1).unwrap(), EventSeq(2));
    // Nhánh chưa có gì vẫn phải là 0, không phải kế thừa của nhánh khác.
    assert_eq!(s.next_seq(B2).unwrap(), EventSeq(0));
}

/// Một lô hỏng thì **không** sự kiện nào của lô đó được ghi.
///
/// Đây là bài quan trọng nhất trong cả bộ. Một giao dịch sinh nhiều sự kiện;
/// nếu nửa số đó vào được nhật ký còn nửa kia thì không, nhật ký mô tả một
/// thế giới chưa từng tồn tại — và replay sẽ dựng lại đúng cái thế giới sai đó.
pub fn ghi_theo_lo_la_nguyen_tu<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    s.append_events(&[ev(B1, 0, 0, "x")]).unwrap();

    // Lô sau có một sự kiện trùng khóa chính, nên cả lô phải bị từ chối.
    let lo = vec![ev(B1, 1, 1, "hop_le"), ev(B1, 0, 0, "trung_khoa")];
    let kq = s.append_events(&lo);
    assert!(kq.is_err(), "hợp đồng: lô có phần tử hỏng phải thất bại");

    let con_lai = s.read_events(B1, EventSeq(0), EventSeq(100)).unwrap();
    assert_eq!(
        con_lai.len(),
        1,
        "hợp đồng: lô thất bại không được ghi một phần — thấy {} sự kiện",
        con_lai.len()
    );
}

/// Hai nhánh có không gian số thứ tự riêng và không thấy nhau.
pub fn nhanh_khac_nhau_khong_lan_sang_nhau<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    s.append_events(&[ev(B1, 0, 0, "cua_b1"), ev(B1, 1, 1, "cua_b1")])
        .unwrap();
    s.append_events(&[ev(B2, 0, 0, "cua_b2")]).unwrap();

    let a = s.read_events(B1, EventSeq(0), EventSeq(100)).unwrap();
    let b = s.read_events(B2, EventSeq(0), EventSeq(100)).unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 1);
    assert!(a.iter().all(|e| e.kind == "cua_b1"));
    assert!(b.iter().all(|e| e.kind == "cua_b2"));
}

/// Lấy ảnh chụp gần nhất **không vượt quá** tick đã hỏi.
///
/// "Không vượt quá" chứ không phải "gần nhất": khôi phục về tick 50 mà nhận
/// được ảnh chụp ở tick 60 nghĩa là bạn khôi phục về tương lai, và mọi sự kiện
/// giữa 50 và 60 sẽ bị áp lần thứ hai.
pub fn anh_chup_lay_ban_gan_nhat_khong_vuot_qua<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    for t in [10u64, 20, 30] {
        s.put_snapshot(&Snapshot {
            branch: B1,
            world: W1,
            tick: Tick(t),
            event_count: t,
            state_hash: StateHash([t as u8; 32]),
            blob: vec![t as u8],
        })
        .unwrap();
    }

    let g = s.latest_snapshot(B1, Tick(25)).unwrap().expect("phải có");
    assert_eq!(
        g.tick,
        Tick(20),
        "hợp đồng: không được vượt quá tick đã hỏi"
    );

    let dung = s.latest_snapshot(B1, Tick(30)).unwrap().expect("phải có");
    assert_eq!(dung.tick, Tick(30), "hợp đồng: bằng đúng tick thì lấy được");

    assert!(
        s.latest_snapshot(B1, Tick(5)).unwrap().is_none(),
        "hợp đồng: trước ảnh đầu tiên thì không có gì"
    );

    assert_eq!(
        g.state_hash,
        StateHash([20u8; 32]),
        "hash phải đi và về nguyên vẹn"
    );
}

/// Chụp lại cùng một tick thì ghi đè, không nhân bản.
pub fn anh_chup_ghi_de_cung_tick<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    let mk = |b: u8| Snapshot {
        branch: B1,
        world: W1,
        tick: Tick(10),
        event_count: 10,
        state_hash: StateHash([b; 32]),
        blob: vec![b],
    };
    s.put_snapshot(&mk(1)).unwrap();
    s.put_snapshot(&mk(2)).unwrap();
    let g = s.latest_snapshot(B1, Tick(10)).unwrap().unwrap();
    assert_eq!(g.blob, vec![2u8], "hợp đồng: ảnh chụp mới thay ảnh cũ");
}

/// Dòng dõi đi từ chính nhánh đó ngược về gốc, đúng thứ tự.
pub fn dong_doi_tu_con_ve_goc<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    s.create_branch(&BranchRecord {
        id: BranchId(1),
        parent: None,
        fork_tick: Tick(0),
        label: "goc".into(),
    })
    .unwrap();
    s.create_branch(&BranchRecord {
        id: BranchId(2),
        parent: Some(BranchId(1)),
        fork_tick: Tick(100),
        label: "con".into(),
    })
    .unwrap();
    s.create_branch(&BranchRecord {
        id: BranchId(3),
        parent: Some(BranchId(2)),
        fork_tick: Tick(200),
        label: "chau".into(),
    })
    .unwrap();

    let d = s.ancestry(BranchId(3)).unwrap();
    assert_eq!(
        d.iter().map(|b| b.id.get()).collect::<Vec<_>>(),
        vec![3, 2, 1],
        "hợp đồng: dòng dõi từ chính nó ngược về gốc"
    );
    assert_eq!(
        d[0].fork_tick,
        Tick(200),
        "hợp đồng: fork_tick phải giữ được — mọi truy vấn ký ức cần nó"
    );
}

/// DAG hỏng không được làm treo. Thà cắt còn hơn treo cả tiến trình.
pub fn dong_doi_khong_treo_khi_dag_hong<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    // Nhánh trỏ về chính nó. Không tạo được bằng đường bình thường, nhưng một
    // file save hỏng hoặc một migration sai thì có thể.
    let tu_tro = BranchRecord {
        id: BranchId(1),
        parent: Some(BranchId(1)),
        fork_tick: Tick(0),
        label: "hong".into(),
    };
    if s.create_branch(&tu_tro).is_err() {
        return; // Backend chặn được từ tầng lược đồ thì càng tốt.
    }
    let d = s.ancestry(BranchId(1)).unwrap();
    assert!(d.len() <= 2, "hợp đồng: vòng lặp dòng dõi phải bị cắt");
}

/// Payload là byte đục: tầng lưu trữ không được diễn giải nội dung.
pub fn payload_la_byte_duc<S: Store, F: Fn() -> S>(f: &F) {
    let mut s = f();
    // Byte không phải UTF-8, có cả byte 0 — thứ sẽ hỏng nếu backend nào đó lỡ
    // đưa payload qua một cột TEXT.
    let tho = vec![0u8, 0xff, 0x00, 0x80, b'a', 0x00];
    let mut e = ev(B1, 0, 0, "x");
    e.payload = tho.clone();
    s.append_events(&[e]).unwrap();

    let doc = s.read_events(B1, EventSeq(0), EventSeq(1)).unwrap();
    assert_eq!(
        doc[0].payload, tho,
        "hợp đồng: payload phải là byte đục, không qua bất kỳ diễn giải nào"
    );
}
