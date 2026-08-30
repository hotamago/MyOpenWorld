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
