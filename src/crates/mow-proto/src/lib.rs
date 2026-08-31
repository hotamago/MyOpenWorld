//! # `mow-proto` — kiểu sinh từ `proto/`
//!
//! Crate này **không có mã viết tay** ngoài file này. Mọi kiểu ở đây được sinh
//! ra từ `proto/*.proto` qua `make codegen`, và `plan.md §P4.1` nói rõ:
//!
//! > Không ai được viết tay một struct đã tồn tại ở phía kia.
//!
//! Lý do là một lỗi rất dễ mắc và rất khó thấy: khi hai bên của một hợp đồng
//! được viết tay hai lần, chúng khớp nhau vào ngày đầu tiên và trôi khỏi nhau
//! kể từ ngày thứ hai. Bug xuất hiện dưới dạng một trường im lặng bằng 0, không
//! phải dưới dạng một lỗi biên dịch.
//!
//! `build.rs` đọc `proto/descriptor_set.bin` đã commit, nên `cargo build` không
//! cần `protoc`. Xem lời giải thích đầy đủ ở đó.
//!
//! ## Vì sao ba module dưới đây tắt `missing_docs`
//!
//! Tài liệu của chúng nằm trong `.proto` và được `--include_source_info` mang
//! sang, nên phần *giải thích* không mất đi đâu cả. Nhưng những trường tầm
//! thường kiểu `EntityId.value` thì không có gì để nói, và bắt chúng có doc chỉ
//! đẻ ra một dòng lặp lại tên trường. Ràng buộc `deny(missing_docs)` vẫn giữ
//! nguyên cho mọi mã viết tay, tức là chỗ nó thật sự có tác dụng.

#![deny(missing_docs)]
#![allow(clippy::doc_markdown)]

/// Kiểu dùng chung.
pub mod common {
    /// `mow.common.v1`.
    #[allow(missing_docs)]
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/mow.common.v1.rs"));
    }
}

/// Hợp đồng chu trình nhận thức.
pub mod cognition {
    /// `mow.cognition.v1`.
    #[allow(missing_docs)]
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/mow.cognition.v1.rs"));
    }
}

/// Hợp đồng memory-service.
pub mod memory {
    /// `mow.memory.v1`.
    #[allow(missing_docs)]
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/mow.memory.v1.rs"));
    }
}
