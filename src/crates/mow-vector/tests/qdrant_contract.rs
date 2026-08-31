//! Bộ test hợp đồng chạy trên **backend thứ hai** (`PC-20`, `plan.md §P3.4`).
//!
//! Xem `crates/mow-persist/tests/postgres_contract.rs` để biết bài này chứng
//! minh điều gì và vì sao nó `#[ignore]`.
//!
//! ```bash
//! ./mow infra up
//! MOW_QDRANT_URL=http://localhost:6333 \
//!   cargo test -p mow-vector --features qdrant -- --ignored --test-threads=1
//! ```

#![cfg(feature = "qdrant")]

use mow_vector::contract;
use mow_vector::qdrant::QdrantIndex;

#[test]
#[ignore = "cần Qdrant: ./mow infra up, rồi đặt MOW_QDRANT_URL"]
fn hop_dong_chi_muc_chay_nguyen_ven_tren_qdrant() {
    let u = std::env::var("MOW_QDRANT_URL").expect(
        "thiếu MOW_QDRANT_URL — bài này `#[ignore]`, nên bạn đã chạy `--ignored` \
         mà chưa dựng hạ tầng: `./mow infra up`",
    );

    contract::run_all(|| {
        let mut i =
            QdrantIndex::connect(&u, "mowtest", contract::DIMENSION).expect("kết nối Qdrant");
        // Hợp đồng đòi một chỉ mục **rỗng** mỗi lần gọi factory.
        mow_vector::VectorIndex::clear(&mut i).expect("dọn sạch");
        i
    });
}
