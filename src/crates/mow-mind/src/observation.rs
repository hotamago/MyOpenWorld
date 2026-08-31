//! [`Observation`] — thứ duy nhất nhân vật biết (`§10.4` bước 2).
//!
//! ## Vì sao struct này phải nghèo nàn
//!
//! Cám dỗ lớn nhất khi viết tầng nhận thức là cho model "thêm một chút ngữ
//! cảnh": vài dòng lore, tình hình cả làng, chuyện đang xảy ra ở đầu kia bản
//! đồ. Mỗi lần thêm như vậy đều rẻ, và tổng của chúng là một nhân vật biết
//! những thứ nó chưa bao giờ nhìn thấy — thứ `§10.2` gọi là xóa ranh giới giữa
//! ground truth và belief.
//!
//! Nên struct này cố tình chỉ có bảy trường, và **engine phải tự lọc** trước
//! khi dựng nó. Không có trường `world`, không có trường `sim`, không có con
//! trỏ nào để prompt builder lần ngược ra sự thật. Thứ không có mặt ở đây thì
//! không có đường vào prompt.
//!
//! ## Vì sao toàn là chuỗi
//!
//! Đây là dữ liệu đi vào một prompt, không phải dữ liệu đi vào mô phỏng. Giữ
//! `Place` hay `Role` dạng enum ở đây sẽ buộc crate này biết bảng địa điểm của
//! thế giới, và một thế giới mới với địa điểm mới sẽ phải sửa `mow-mind`. Việc
//! đổi từ enum sang tên nằm ở [`crate::bridge`], nơi nó thuộc về.

use serde::{Deserialize, Serialize};

/// Những gì một nhân vật cảm nhận được tại một tick.
///
/// **Do engine dựng**, không bao giờ do model dựng. Thứ tự các trường ở đây
/// cũng là thứ tự chúng xuất hiện trong prompt ([`crate::prompt_of`]) — đổi thứ
/// tự khai báo là đổi prompt, và là đổi khóa của mọi bản ghi `REPLAY`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// Tên nhân vật đang suy nghĩ.
    pub self_name: String,
    /// Vai của nó trong làng, ví dụ `farmer`.
    pub role: String,
    /// Mức đói: `0` là no, càng cao càng đói. Số nguyên (`§P10.2.1`).
    pub hunger: i64,
    /// Buổi trong ngày, ví dụ `morning`. Là **nhãn**, không phải đồng hồ: một
    /// tick tuyệt đối trong prompt sẽ làm hỏng cache ở `§20.8`.
    pub time_of_day: String,
    /// Nơi nhân vật đang đứng, ví dụ `well`.
    pub at: String,
    /// Tên những người nhân vật **nhìn thấy** ngay lúc này.
    ///
    /// Engine đã lọc theo giác quan rồi; crate này không lọc lại và cũng không
    /// có gì để lọc bằng. Prompt sắp xếp danh sách này trước khi in, nên một
    /// engine gom tên từ một `HashSet` vẫn cho ra prompt xác định.
    pub nearby: Vec<String>,
    /// Vài việc vừa xảy ra với nhân vật, **cũ trước mới sau**.
    ///
    /// Không sắp xếp: đây là một dòng thời gian, và sắp xếp một dòng thời gian
    /// là nói dối về thứ tự sự kiện. Tính xác định vẫn giữ nguyên vì `Vec` đã
    /// có thứ tự — cùng một `Vec` luôn cho cùng một prompt.
    pub recent: Vec<String>,
}

impl Observation {
    /// Dựng một quan sát tối thiểu: ai, làm vai gì, đang ở đâu.
    ///
    /// Có mặt để test và để chỗ gọi không phải gõ bảy trường khi chỉ cần ba.
    #[must_use]
    pub fn new(self_name: &str, role: &str, at: &str) -> Observation {
        Observation {
            self_name: self_name.to_owned(),
            role: role.to_owned(),
            hunger: 0,
            time_of_day: String::new(),
            at: at.to_owned(),
            nearby: Vec::new(),
            recent: Vec::new(),
        }
    }
}
