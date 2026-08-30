//! Bus SQLite phai dat toan bo hop dong.
//! Khi NATS JetStream duoc them o `PC-20`, file cua no goi dung ham nay.

use mow_bus::{contract, sqlite::SqliteBus};

#[test]
fn sqlite_bus_dat_hop_dong() {
    contract::run_all(|| SqliteBus::in_memory().expect("mo duoc bus"));
}
