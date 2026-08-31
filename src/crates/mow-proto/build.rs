//! Sinh kiểu Rust từ **descriptor set**, không phải từ `protoc`.
//!
//! ## Vì sao không gọi thẳng `protoc` ở đây
//!
//! `prost_build::compile_protos` cần một `protoc` trên máy. Điều đó có ba hệ quả
//! khó chịu, và cái thứ ba là cái thật sự đau:
//!
//! 1. Ai clone repo rồi gõ `cargo build` phải đi cài thêm một thứ.
//! 2. CI phải cài nó ở mọi job, kể cả job chỉ chạy `cargo test`.
//! 3. **Hai version `protoc` khác nhau sinh ra mã hơi khác nhau.** Đủ để
//!    `--check` đỏ trên máy này và xanh trên máy kia mà không ai sửa gì, và
//!    người gặp phải sẽ mất một buổi để tin rằng mình không điên.
//!
//! Nên chuỗi thật là: `make codegen` chạy **một** `protoc` (bản khóa trong
//! lockfile Python) để sinh ra `proto/descriptor_set.bin`; file đó được commit;
//! và mọi build sau đó chỉ đọc nó. `prost_build::compile_fds` làm được đúng
//! điều này, nên build Rust không cần công cụ ngoài nào cả.

use prost::Message as _;
use std::path::PathBuf;

fn main() {
    let goc = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/mow-proto phải nằm trong workspace")
        .to_path_buf();
    let fds = goc.join("proto/descriptor_set.bin");

    println!("cargo:rerun-if-changed={}", fds.display());

    let Ok(bytes) = std::fs::read(&fds) else {
        // Chưa chạy `make codegen`. Sinh một module rỗng thay vì fail: một
        // checkout sạch phải build được trước khi ai đó biết phải chạy gì.
        ghi_rong();
        return;
    };

    let set = prost_types::FileDescriptorSet::decode(&bytes[..])
        .expect("descriptor_set.bin hỏng — chạy lại `make codegen`");

    let mut cfg = prost_build::Config::new();
    cfg.out_dir(std::env::var("OUT_DIR").expect("cargo đặt OUT_DIR"));
    cfg.compile_fds(set)
        .expect("sinh kiểu Rust từ descriptor set");
}

/// Không có descriptor set: để lại một file rỗng cho `include!` khỏi gãy.
fn ghi_rong() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo đặt OUT_DIR"));
    for f in [
        "mow.common.v1.rs",
        "mow.cognition.v1.rs",
        "mow.memory.v1.rs",
    ] {
        let _ = std::fs::write(
            out.join(f),
            "// chưa chạy `make codegen` — chạy nó rồi build lại.\n",
        );
    }
}
