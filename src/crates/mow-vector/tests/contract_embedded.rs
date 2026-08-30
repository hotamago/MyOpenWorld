//! Chi muc nhung phai dat toan bo hop dong.
//! Khi Qdrant duoc them o `PC-20`, file cua no goi dung ham nay.

use mow_vector::{contract, embedded::EmbeddedIndex};

#[test]
fn embedded_dat_hop_dong() {
    contract::run_all(|| EmbeddedIndex::in_memory(contract::DIMENSION).expect("mo duoc chi muc"));
}

#[test]
fn luong_tu_hoa_giu_duoc_thu_hang() {
    // Lam tron sang i16 khong duoc dao thu tu cua nhung vector khac nhau ro rang.
    let a = mow_vector::quantize(&[1.0, 0.0, 0.0, 0.0]);
    let b = mow_vector::quantize(&[0.9, 0.1, 0.0, 0.0]);
    let c = mow_vector::quantize(&[0.0, 1.0, 0.0, 0.0]);
    let q = mow_vector::quantize(&[1.0, 0.0, 0.0, 0.0]);
    let sa = mow_vector::dot(&q, &a);
    let sb = mow_vector::dot(&q, &b);
    let sc = mow_vector::dot(&q, &c);
    assert!(sa > sb && sb > sc, "{sa} {sb} {sc}");
}

#[test]
fn vector_khong_bi_luong_tu_hoa_ve_0() {
    let v = mow_vector::quantize(&[1e-6, -2e-6, 0.0, 5e-7]);
    assert!(
        v.iter().any(|x| *x != 0),
        "chuan hoa phai theo max, khong theo thang tuyet doi"
    );
}
