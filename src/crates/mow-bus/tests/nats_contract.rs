//! Bộ test hợp đồng chạy trên **backend thứ hai** (`PC-20`, `plan.md §P3.4`).
//!
//! Xem `crates/mow-persist/tests/postgres_contract.rs` để biết bài này chứng
//! minh điều gì và vì sao nó `#[ignore]`.
//!
//! ```bash
//! ./mow infra up
//! MOW_NATS_URL=nats://localhost:4222 \
//!   cargo test -p mow-bus --features nats -- --ignored --test-threads=1
//! ```

#![cfg(feature = "nats")]

use mow_bus::contract;
use mow_bus::nats::NatsBus;

#[test]
#[ignore = "cần NATS: ./mow infra up, rồi đặt MOW_NATS_URL"]
fn hop_dong_bus_chay_nguyen_ven_tren_nats() {
    let u = std::env::var("MOW_NATS_URL").expect(
        "thiếu MOW_NATS_URL — bài này `#[ignore]`, nên bạn đã chạy `--ignored` \
         mà chưa dựng hạ tầng: `./mow infra up`",
    );

    // Mỗi lần gọi factory dùng một stream **riêng**: hợp đồng đòi một bus rỗng,
    // độc lập, và JetStream giữ dữ liệu qua các lần chạy nên dùng chung tên
    // stream sẽ làm bài sau đọc phải rác của bài trước.
    let dem = std::sync::atomic::AtomicU32::new(0);
    contract::run_all(|| {
        let n = dem.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let ten = format!("mowtest{n}");
        NatsBus::connect(&u, &ten).expect("kết nối NATS")
    });
}
