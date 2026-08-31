//! # `mow-config` — cấu hình có kiểm tra
//!
//! Thứ tự layer, sau ghi đè trước (`plan.md §P6.1`):
//!
//! ```text
//! config/base.yaml → config/<env>.yaml → biến môi trường MOW_* → tham số dòng lệnh
//! ```
//!
//! Hai quy tắc, và cả hai đều là phản ứng với một lỗi cụ thể:
//!
//! **Khởi động thất bại nhanh.** Config sai thì tiến trình thoát ngay với đường
//! dẫn tới đúng field, không chạy tiếp với giá trị mặc định. Chạy tiếp với mặc
//! định là cách một máy chủ chạy suốt ba tuần với `llm_mode: STUB` mà không ai
//! nhận ra, cho tới khi có người hỏi vì sao NPC không nói gì.
//!
//! **Bí mật chỉ ở `.env`.** File `config/*.yaml` được commit, nên không bao giờ
//! chứa API key hay DSN có mật khẩu. [`AppConfig::validate`] có một bước quét
//! phát hiện thứ trông giống bí mật lọt vào YAML, và nó **từ chối khởi động**
//! chứ không chỉ cảnh báo — vì một cảnh báo trong log khởi động là thứ không ai
//! đọc.
//!
//! ## Về "yaml auto generate struct"
//!
//! Hướng đi là ngược lại: **struct Rust là nguồn**, JSON Schema được sinh ra từ
//! nó bằng `schemars`. Sinh struct từ YAML sẽ làm trình biên dịch không còn
//! kiểm được gì, và một lỗi chính tả trong tên field sẽ thành một field mới im
//! lặng thay vì một lỗi biên dịch.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// Biến thể lỗi cấu hình mang **đường dẫn field** theo yêu cầu của
// `§P6.1`. Thu nhỏ nó lại nghĩa là bỏ bớt thông tin đó, và đánh đổi sai
// chiều: cấu hình được đọc đúng một lần lúc khởi động, còn một thông báo
// khó hiểu thì làm mất cả buổi của người dùng.
#![allow(clippy::result_large_err)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

pub mod dotenv;
pub mod error;
pub mod schema;

pub use error::{ConfigError, ConfigResult};
pub use schema::{
    AppConfig, BudgetConfig, ContentConfig, EmbeddingConfig, EmbeddingMode, LlmConfig, LlmMode,
    LogFormat, ObservabilityConfig, PersistenceConfig, SimConfig, VectorConfig,
};

use figment::providers::{Env, Format, Yaml};
use figment::Figment;
use std::path::Path;

/// Nạp config theo đúng thứ tự layer.
///
/// `root` là thư mục `config/`. `env` là tên môi trường (`dev`, `test`, `prod`).
pub fn load(root: impl AsRef<Path>, env: &str) -> ConfigResult<AppConfig> {
    let root = root.as_ref();
    let base = root.join("base.yaml");
    let theo_env = root.join(format!("{env}.yaml"));

    if !base.exists() {
        return Err(ConfigError::Missing(base.display().to_string()));
    }

    let mut fig = Figment::new().merge(Yaml::file(&base));
    // File theo môi trường là tùy chọn: một môi trường không cần ghi đè gì thì
    // không cần một file rỗng chỉ để tồn tại.
    if theo_env.exists() {
        fig = fig.merge(Yaml::file(&theo_env));
    }
    // `MOW_DATABASE__URL` → `database.url`. Dấu gạch dưới đôi làm dấu phân cấp
    // vì tên field có thể chứa một gạch dưới đơn.
    fig = fig.merge(Env::prefixed("MOW_").split("__"));

    let mut cfg: AppConfig = fig.extract().map_err(ConfigError::from)?;

    // `env` là **dẫn xuất**, không phải cấu hình: nó luôn bằng tên môi trường
    // đã thật sự được nạp.
    //
    // Không có dòng này thì `MOW_ENV` làm hai việc mâu thuẫn nhau. Nó chọn file
    // (chỗ gọi đọc nó để truyền vào `env`), *và* nó là một field nên lớp biến
    // môi trường ghi đè lên YAML. Đặt `MOW_ENV=dev` rồi gọi `load(root, "test")`
    // cho ra một config đã nạp `test.yaml` nhưng **tự khai là `dev`** — tức là
    // đúng cái nhầm lẫn mà field này tồn tại để chống.
    //
    // Chuyện đó đã xảy ra thật: container `toolbox` đặt `MOW_ENV=dev`, và bài
    // test nạp môi trường `test` đỏ ở đó trong khi xanh trên máy thật.
    env.clone_into(&mut cfg.env);

    cfg.validate()?;
    Ok(cfg)
}

/// Nạp rồi thoát tiến trình nếu sai.
///
/// Dùng ở `main()` của mọi binary. In ra `stderr` đường dẫn field sai rồi thoát
/// với mã 78 (`EX_CONFIG` của `sysexits.h`) — mã riêng để script điều phối phân
/// biệt được "cấu hình sai" với "chương trình lỗi".
pub fn load_or_exit(root: impl AsRef<Path>, env: &str) -> AppConfig {
    match load(root, env) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cấu hình không hợp lệ:\n{e}");
            std::process::exit(78);
        }
    }
}
