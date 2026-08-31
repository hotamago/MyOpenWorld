//! Bộ test hợp đồng chạy trên **backend thứ hai** (`PC-20`, `plan.md §P3.4`).
//!
//! Xem `crates/mow-persist/tests/postgres_contract.rs` để biết bài này chứng
//! minh điều gì và vì sao nó `#[ignore]`.
//!
//! ```bash
//! ./mow infra up
//! MOWTEST_NATS_URL=nats://localhost:14222 \
//!   cargo test -p mow-bus --features nats -- --ignored --test-threads=1
//! ```

#![cfg(feature = "nats")]

use mow_bus::contract;
use mow_bus::nats::NatsBus;

#[test]
#[ignore = "cần NATS: ./mow infra up, rồi đặt MOWTEST_NATS_URL"]
fn hop_dong_bus_chay_nguyen_ven_tren_nats() {
    let u = std::env::var("MOWTEST_NATS_URL").expect(
        "thiếu MOWTEST_NATS_URL — bài này `#[ignore]`, nên bạn đã chạy `--ignored` \
         mà chưa dựng hạ tầng: `./mow infra up`",
    );

    // Mỗi lần gọi factory dùng một stream **riêng**: hợp đồng đòi một bus rỗng,
    // độc lập, và JetStream giữ dữ liệu qua các lần chạy nên dùng chung tên
    // stream sẽ làm bài sau đọc phải rác của bài trước.
    //
    // Bộ đếm thôi thì chưa đủ, và đó là một lỗi đã thật sự cắn: bộ đếm bắt đầu
    // lại từ 0 ở **mỗi tiến trình**, nên lần chạy thứ hai dùng lại đúng những
    // tên `mowtest0`, `mowtest1`… mà lần trước đã để lại thông điệp trong đó.
    // Bài `thu_tu_trong_mot_chu_de` đỏ với một danh sách payload dài hơn nó
    // gửi. Nghĩa là bộ test này chỉ đúng ở lần chạy ĐẦU TIÊN trên một máy chủ
    // sạch — đúng cái điều kiện mà không ai kiểm.
    //
    // Mốc thời gian cho mỗi lần chạy một không gian tên riêng. Stream cũ đọng
    // lại trên máy chủ test; `./mow reset` xóa volume nếu cần dọn.
    let phien = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("đồng hồ hệ thống")
        .as_nanos();
    let dem = std::sync::atomic::AtomicU32::new(0);
    contract::run_all(|| {
        let n = dem.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let ten = format!("mowtest{phien}x{n}");
        NatsBus::connect(&u, &ten).expect("kết nối NATS")
    });
}
