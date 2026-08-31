//! # `mow-mind` — tầng nhận thức LLM cho NPC
//!
//! Crate này biến "NPC chạy theo lịch cứng" thành "NPC suy nghĩ", và nó làm
//! đúng một việc: **biến một quan sát thành một đề xuất hành động đã kiểm**.
//! Nó không thực hiện hành động, không đọc thế giới, không ghi belief.
//!
//! ## Bốn ràng buộc, và vì sao chúng nằm ở đây chứ không ở chỗ gọi
//!
//! Cả bốn đều là ràng buộc **cấu trúc**: chúng đúng vì không có đường nào khác
//! trong kiểu dữ liệu, chứ không vì có ai đó nhớ kiểm.
//!
//! 1. **Model không tự dựng quan sát** (`§10.4` bước 2). [`Observation`] do
//!    engine dựng và truyền vào [`prompt_of`]; prompt không đọc biến môi
//!    trường, không đọc tệp, không đọc đồng hồ. Và ở chiều ngược lại, hình dạng
//!    trả lời (`action`/`target`/`reason`) **không có ô nào** để model nhét vào
//!    một dữ kiện mới — mọi trường lạ bị bỏ, xem [`read_choice`]. Nếu model
//!    được phép tự mô tả nó thấy gì, nó sẽ thấy thứ nhân vật không thấy, và
//!    `§10.2` sụp ngay tại đó.
//! 2. **Model không khẳng định điều kiện tiên quyết** (`§10.4` bước 7).
//!    [`Choice`] là một **đề xuất**. Trường `reason` là lời kể, không có hiệu
//!    lực; `target` chỉ là một cái tên mà engine còn phải tự phân giải và có
//!    quyền từ chối. Crate này không hứa rằng hành động làm được — nó chỉ hứa
//!    rằng hành động **nằm trong tập được phép**.
//! 3. **Action registry** (`§10.5`). Model chỉ chọn được từ danh sách engine
//!    công bố. Một giá trị ngoài danh sách là **lỗi validate**
//!    ([`FallbackReason::NotInRegistry`]), không phải một hành động lạ được
//!    thực hiện. Và [`Choice::action`] trả về **đúng chuỗi trong registry**,
//!    không phải chuỗi model gõ — nên chỗ gọi không bao giờ nhận một biến thể
//!    chính tả.
//! 4. **Fallback là một nhánh có tên** (`§20.10`). Không có khối `catch` nào
//!    nuốt lỗi ở đây. Mỗi cách hỏng là một [`FallbackReason`] riêng, mỗi lần
//!    rơi là một [`FallbackNote`] trong sổ của [`Mind`], và
//!    [`Mind::fallbacks_total`] không bao giờ mất số đếm. Một exception đã nuốt
//!    là một thứ không có mặt ở đâu cả.
//!
//! ## Đường đi của một lượt suy nghĩ
//!
//! ```text
//! engine  →  Observation  →  prompt_of  →  ModelClient  →  read_choice  →  Decision
//!                              (xác định)     (§20.7)        (§10.5)         (§10.4/7)
//! ```
//!
//! [`Decision`] **luôn** có một [`Choice`] dùng được: `think` không trả lỗi ra
//! ngoài, vì một NPC không quyết định được thì thế giới đứng. Nhưng mọi lần rơi
//! về fallback đều nói rõ vì sao.
//!
//! ## Nối vào server
//!
//! [`bridge`] là chỗ crate này gặp [`mow_society::routine`]: fallback không
//! phải một hằng số vô hồn mà chính là **tầng Routine của `§10.3`** — khi model
//! im lặng, cư dân quay về nhịp sinh hoạt của vai nó và thế giới vẫn chạy.
//!
//! ```
//! use mow_llm::Gateway;
//! use mow_mind::{bridge, Mind};
//! use mow_society::routine::{Place, Role, Situation};
//!
//! # let situation = Situation {
//! #     tick: 40, ticks_per_day: 100, role: Role::Farmer,
//! #     hunger: 12, fatigue: 8, at: Place::Well, nearby: 1, nearest: Some(7),
//! # };
//! # let nearby = vec!["Doran".to_owned()];
//! # let recent: Vec<String> = Vec::new();
//! let mut mind = Mind::new(
//!     Box::new(Gateway::stub()),
//!     bridge::village_registry(),
//!     bridge::routine_fallback(&situation),
//! );
//! let obs = bridge::observation_of(&situation, "Mara", &nearby, &recent);
//! let decision = mind.think_with(&obs, &bridge::routine_fallback(&situation));
//! let intent = bridge::intent_of(decision.choice()); // `None` = engine từ chối
//! # assert!(intent.is_some());
//! ```
//!
//! ## Xác định
//!
//! [`prompt_of`] là hàm thuần của `(Observation, registry)`. Đó không phải sự
//! cầu kỳ: khóa của bản ghi `REPLAY` trong `mow_llm::Request::hash` gồm cả
//! chuỗi đã render, nên một prompt đổi giữa hai lần chạy là một bộ ghi không
//! bao giờ trúng — và một CI phải gọi mạng thật để xanh.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

pub mod bridge;
pub mod choice;
pub mod mind;
pub mod observation;
pub mod parse;
pub mod prompt;

pub use choice::{Choice, Decision, FallbackNote, FallbackReason};
pub use mind::{Mind, JOURNAL_CAP, ROUTE_ROLE};
pub use observation::Observation;
pub use parse::{read_choice, ReadError};
pub use prompt::{canonical_registry, prompt_of, MAX_RECENT, PROMPT_ID, PROMPT_VERSION};
