//! SQLite phai dat toan bo hop dong.
//!
//! Khi Postgres duoc them o `PC-20`, file tuong duong cua no goi dung
//! `contract::run_all` nay, khong sua mot dong.

use mow_persist::{contract, SqliteStore};

#[test]
fn sqlite_dat_toan_bo_hop_dong() {
    contract::run_all(|| SqliteStore::in_memory().expect("mo duoc kho trong bo nho"));
}

#[test]
fn khong_co_cot_so_thuc_nao() {
    // §P10.2.1: khong co cot `real` hay `double precision` tren duong commit.
    let s = SqliteStore::in_memory().unwrap();
    let vi_pham = s.kiem_tra_khong_co_cot_thuc().unwrap();
    assert!(
        vi_pham.is_empty(),
        "cot so thuc tren duong commit: {vi_pham:?}"
    );
}

/// **Cận trên `u64::MAX` nghĩa là "không có cận trên"**, không phải "rỗng".
///
/// `EventSeq` là `u64` còn cột SQL là `i64`. Ép thẳng thì `u64::MAX` thành
/// `-1`, và `seq < -1` không khớp gì cả — truy vấn trả về **rỗng** trong khi
/// chỗ gọi tin là nó vừa đọc hết nhật ký.
///
/// Đây đúng lớp lỗi mà cả dự án này chống: không panic, không sai kiểu, chỉ là
/// một câu trả lời sai và im lặng.
#[test]
fn can_tren_khong_gioi_han_khong_tra_ve_rong() {
    use mow_core::{BranchId, EventSeq, Tick, WorldId};
    use mow_persist::sqlite::SqliteStore;
    use mow_persist::store::{BranchRecord, EventRecord, Store};

    let mut s = SqliteStore::in_memory().unwrap();
    s.create_branch(&BranchRecord {
        id: BranchId(1),
        parent: None,
        fork_tick: Tick(0),
        label: "gốc".into(),
    })
    .unwrap();
    s.append_events(&[EventRecord {
        seq: EventSeq(1),
        branch: BranchId(1),
        world: WorldId(1),
        tick: Tick(10),
        kind: "core.founded".into(),
        actor: 0,
        subject: 0,
        payload: Vec::new(),
        cause: None,
        law_version: None,
        norm_set_version: None,
    }])
    .unwrap();

    let het = s
        .read_events(BranchId(1), EventSeq(0), EventSeq(u64::MAX))
        .unwrap();
    assert_eq!(het.len(), 1, "cận trên vô hạn phải đọc được mọi event");
}
