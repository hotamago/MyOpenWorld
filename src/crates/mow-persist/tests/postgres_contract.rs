//! Bộ test hợp đồng chạy trên **backend thứ hai** (`PC-20`, `plan.md §P3.4`).
//!
//! ## Điều bài này chứng minh
//!
//! Không phải "Postgres hoạt động". Mà là: **bộ test hợp đồng viết ở Giai đoạn 0
//! dùng lại được nguyên vẹn.**
//!
//! Chú ý dòng `contract::run_all(...)` bên dưới — nó gọi **đúng** hàm mà
//! `sqlite_contract.rs` gọi, không sửa một tham số nào. Nếu ở đây phải viết một
//! biến thể riêng, hoặc bỏ qua vài bài, thì hợp đồng đã bị viết quanh SQLite chứ
//! không phải quanh ngữ nghĩa — và lời hứa "hai backend tương đương" trở thành
//! một câu nói suông.
//!
//! ## Vì sao `#[ignore]`
//!
//! Bài này cần một Postgres đang chạy. Một test cần dịch vụ ngoài mà **fail**
//! khi không có dịch vụ sẽ khiến `cargo test` đỏ trên máy mọi người, và một bộ
//! test đỏ thường xuyên là một bộ test không ai đọc — lúc đó nó tệ hơn là không
//! có.
//!
//! ```bash
//! ./mow infra up
//! MOWTEST_POSTGRES_URL=postgres://mow:mow@localhost:15432/mow \
//!   cargo test -p mow-persist --features postgres -- --ignored --test-threads=1
//! ```
//!
//! `--test-threads=1` vì các bài dùng chung một cơ sở dữ liệu và mỗi bài đòi một
//! kho **rỗng, độc lập**; chạy song song thì chúng nhiễm nhau, và bộ test trở
//! nên vô giá trị theo cách rất khó nhận ra.

#![cfg(feature = "postgres")]

use mow_persist::contract;
use mow_persist::postgres::PostgresStore;

fn url() -> Option<String> {
    std::env::var("MOWTEST_POSTGRES_URL").ok()
}

#[test]
#[ignore = "cần Postgres: ./mow infra up, rồi đặt MOWTEST_POSTGRES_URL"]
fn hop_dong_store_chay_nguyen_ven_tren_postgres() {
    let Some(u) = url() else {
        panic!(
            "thiếu MOWTEST_POSTGRES_URL.\n\
             Bài này được đánh dấu `#[ignore]`, nên nếu bạn thấy dòng này thì bạn \
             đã chạy `--ignored` mà chưa dựng hạ tầng: `./mow infra up`."
        );
    };

    contract::run_all(|| {
        let mut s = PostgresStore::connect(&u).expect("kết nối Postgres");
        // Hợp đồng đòi một kho **rỗng, độc lập** mỗi lần gọi factory.
        s.truncate_all().expect("dọn sạch");
        s
    });
}
