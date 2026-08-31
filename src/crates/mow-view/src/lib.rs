//! # `mow-view` — read model, lọc **trước khi** dữ liệu rời khỏi máy chủ
//!
//! `idea.md §18.9` ràng buộc thứ ba, viết nguyên văn:
//!
//! > **Lọc ở phía máy chủ, không phải ẩn ở phía client.** Read model chỉ gửi
//! > những gì chế độ hiện tại được phép thấy. Ẩn bằng CSS nghĩa là dữ liệu đã
//! > nằm trong máy người chơi và bất kỳ ai mở devtool trình duyệt cũng đọc
//! > được — điều đó biến `§10.2` thành trang trí.
//!
//! ## Vì sao đây là một crate riêng chứ không phải một hàm trong server
//!
//! Vì cách hỏng không phải là "ai đó viết sai bộ lọc". Cách hỏng là **ai đó
//! thêm một trường mới vào payload và quên lọc nó**. Một hàm `filter()` nằm
//! cạnh chỗ dựng payload sẽ không ngăn được điều đó: cả hai đều là code trong
//! cùng một file, và trường mới chỉ cần được gán thẳng.
//!
//! Ở đây, kiểu duy nhất mà giao thức gửi đi là [`EntityView`], và **không có
//! cách nào dựng nó ngoài [`project`]**. Trường của nó riêng tư, constructor
//! không công khai. Thêm một trường mới nghĩa là phải sửa `project`, và `project`
//! nhận [`Lens`] làm tham số bắt buộc — nên câu hỏi "chế độ nào được thấy cái
//! này" xuất hiện ngay lúc gõ, không phải sáu tháng sau.
//!
//! ## Ba chế độ
//!
//! | Chế độ | Thấy gì |
//! |---|---|
//! | [`Mode::Embodied`] | Chỉ những gì avatar quan sát được hoặc tin |
//! | [`Mode::Observer`] | Sự thật của vùng đang xem, **có nhãn** phân biệt với belief |
//! | [`Mode::TrueGod`] | Mọi thứ, cộng provenance |
//!
//! Ràng buộc thứ nhất của `§18.9` — *"belief và sự thật không bao giờ được vẽ
//! giống nhau"* — được giữ bằng [`Certainty`], một trường **bắt buộc** trên mọi
//! giá trị gửi đi. Không có mặc định. Một giá trị không nói rõ nó là sự thật
//! hay phỏng đoán thì không đi qua được kiểu.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]

pub mod lens;
pub mod project;

pub use lens::{Lens, Mode};
pub use project::{
    project, project_presences, Certainty, EntityView, Field, PresenceView, WorldTruth,
};
