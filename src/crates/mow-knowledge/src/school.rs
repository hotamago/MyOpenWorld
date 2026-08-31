//! Giáo dục, thi cử và lưu trữ (`idea.md §13.10`, `PD-25`).
//!
//! ## Ba hệ quả đáng chơi, và cả ba là **hệ quả** chứ không phải tính năng
//!
//! **1. Gác cửa là quyền lực.**
//!
//! > Ai được vào học quyết định ai có thể lên địa vị ở `§12.10`. Đóng cửa học
//! > viện với một tầng lớp là một hành động chính trị có hậu quả kéo dài nhiều
//! > thế hệ.
//!
//! [`Institution::admits`] vì thế nhận cả `class` lẫn `wealth` lẫn `patron`, và
//! không có đường nào để bỏ qua nó. Một học viện "mở cho tất cả" phải khai điều
//! đó ra bằng danh sách rỗng, chứ không phải bằng cách quên khai.
//!
//! **2. Kho lưu trữ có thể bị kiểm duyệt hoặc cháy.**
//!
//! Kết hợp với `§8.8` quy tắc 4, một trận hỏa hoạn ở thư viện lớn **xóa vĩnh
//! viễn** một nhánh tri thức. Ở đây [`Archive::censor`] và
//! [`crate::teaching::Corpus::burn`] là hai đường khác nhau tới cùng một kết
//! cục, và chúng để lại dấu vết khác nhau — kiểm duyệt thì sách còn mà không ai
//! được đọc; cháy thì sách không còn.
//!
//! **3. Chép sai sinh ra trường phái mới.**
//!
//! > Một bản sao có lỗi được dạy suốt trăm năm tạo ra một truyền thống phép
//! > thuật khác hẳn nguyên bản — và **cả hai bên đều tin mình mới là chính thống**.
//!
//! [`Lineage::divergence`] đo khoảng cách giữa hai dòng truyền thừa. Nó không
//! nói bên nào đúng, vì không có ai trong world biết điều đó.

use crate::graph::Level;
use crate::teaching::Text;
use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Một thể chế giáo dục.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Institution {
    /// Định danh.
    pub id: String,
    /// Chương trình: dạy những node nào.
    pub curriculum: Vec<String>,
    /// **Chỉ nhận những tầng lớp này.** Rỗng nghĩa là không xét tầng lớp.
    ///
    /// Rỗng phải là một lựa chọn *được khai*, không phải hệ quả của việc quên:
    /// đó là khác biệt giữa một học viện chủ trương mở cửa và một học viện chưa
    /// ai nghĩ tới chuyện đóng.
    pub admits_classes: Vec<String>,
    /// Học phí.
    pub tuition: i64,
    /// Cần người bảo trợ không.
    pub requires_patron: bool,
    /// Bậc tối thiểu để tốt nghiệp.
    pub graduation_level: Level,
    /// Kinh phí mỗi kỳ. Thiếu tiền thì chất lượng tụt.
    pub funding: i64,
    /// Kinh phí cần để chạy đủ.
    pub funding_needed: i64,
}

/// Vì sao một người không vào được.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rejection {
    /// Sai tầng lớp.
    WrongClass(String),
    /// Không đủ tiền.
    CannotAfford {
        /// Cần bao nhiêu.
        tuition: i64,
        /// Có bao nhiêu.
        wealth: i64,
    },
    /// Không có người bảo trợ.
    NoPatron,
}

impl Institution {
    /// Một người có vào được không, và **nếu không thì vì sao**.
    ///
    /// Trả `Vec` chứ không trả `bool`: một cánh cửa đóng mà không nói vì sao là
    /// một cánh cửa người chơi không biết cách mở, và `§18.13` nguyên tắc 3 nói
    /// mọi quyết định đều phải có affordance hỏi lý do.
    pub fn admits(&self, class: &str, wealth: i64, patron: Option<EntityId>) -> Vec<Rejection> {
        let mut ra = Vec::new();
        if !self.admits_classes.is_empty() && !self.admits_classes.contains(&class.to_owned()) {
            ra.push(Rejection::WrongClass(class.to_owned()));
        }
        if wealth < self.tuition {
            ra.push(Rejection::CannotAfford {
                tuition: self.tuition,
                wealth,
            });
        }
        if self.requires_patron && patron.is_none() {
            ra.push(Rejection::NoPatron);
        }
        ra
    }

    /// Chất lượng giảng dạy, `0`–`1000`, theo mức kinh phí.
    ///
    /// Cắt kinh phí không đóng cửa trường; nó làm trường dạy kém đi, và hậu quả
    /// chỉ hiện ra một thế hệ sau. Đó là kiểu hậu quả mà một trò chơi hay để
    /// người chơi tự phát hiện.
    pub fn quality(&self) -> u16 {
        if self.funding_needed <= 0 {
            return 1_000;
        }
        u16::try_from((self.funding * 1_000 / self.funding_needed).clamp(0, 1_000)).unwrap_or(0)
    }
}

/// Một kỳ thi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Examination {
    /// Thi node nào.
    pub node: String,
    /// Cần bậc nào để đỗ.
    pub pass_level: Level,
    /// Thiên vị theo tầng lớp, `-1000`..`1000`.
    ///
    /// Dương là được nâng đỡ, âm là bị dìm. Một kỳ thi "khách quan" là kỳ thi
    /// khai `0` — và việc phải khai nó ra chính là điểm: thiên vị trong thi cử
    /// là dữ liệu của thể chế, không phải một điều bí mật của engine.
    pub class_bias: Vec<(String, i16)>,
}

impl Examination {
    /// Một người có đỗ không.
    pub fn passes(&self, level: Level, class: &str) -> bool {
        let thien_vi = self
            .class_bias
            .iter()
            .find(|(c, _)| c == class)
            .map_or(0, |(_, b)| *b);

        // Thiên vị đủ mạnh nâng được người kém một bậc lên, hoặc dìm người đủ
        // giỏi xuống. Đó là toàn bộ cơ chế "gác cửa là quyền lực".
        let bac = level as i32 + i32::from(thien_vi) / 500;
        bac >= self.pass_level as i32
    }
}

/// Kho lưu trữ.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Archive {
    /// Định danh.
    pub id: String,
    /// Sách đang giữ.
    pub holdings: Vec<Text>,
    /// Node bị **cấm đọc**, dù sách vẫn còn.
    ///
    /// Tách khỏi việc đốt: kiểm duyệt để lại sách mà không ai được đọc, và một
    /// chế độ sụp đổ có thể mở lại kho. Đốt thì không mở lại được gì.
    pub censored: Vec<String>,
}

impl Archive {
    /// Sách đọc được — trừ những gì bị kiểm duyệt.
    pub fn accessible(&self) -> Vec<&Text> {
        self.holdings
            .iter()
            .filter(|t| !self.censored.contains(&t.node))
            .collect()
    }

    /// Cấm một node. Trả về số sách bị khóa lại.
    pub fn censor(&mut self, node: &str) -> usize {
        if !self.censored.contains(&node.to_owned()) {
            self.censored.push(node.to_owned());
        }
        self.holdings.iter().filter(|t| t.node == node).count()
    }

    /// Bỏ cấm.
    pub fn uncensor(&mut self, node: &str) {
        self.censored.retain(|c| c != node);
    }
}

/// Một dòng truyền thừa: chuỗi bản sao mà một trường phái dạy theo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineage {
    /// Tên trường phái.
    pub school: String,
    /// Bản đang được dùng làm chính bản.
    pub canonical: Text,
}

impl Lineage {
    /// Hai dòng truyền thừa đã **lệch nhau** bao nhiêu.
    ///
    /// Không nói bên nào đúng — **không ai trong world biết điều đó**. Cả hai
    /// bên đều tin mình mới là chính thống, và đó là trạng thái đúng: chỉ có
    /// người chơi ở chế độ True God mới đối chiếu được với bản gốc.
    pub fn divergence(&self, other: &Lineage) -> u32 {
        self.canonical
            .transcription_errors
            .abs_diff(other.canonical.transcription_errors)
            + u32::from(self.canonical.fidelity.abs_diff(other.canonical.fidelity))
    }

    /// Hai dòng có còn là **cùng một tri thức** không.
    ///
    /// Quá một ngưỡng thì chúng đã tách thành hai truyền thống khác nhau — và
    /// lúc đó chữa lành không còn là việc sửa một bản chép, mà là một cuộc ly giáo.
    pub fn still_same_tradition(&self, other: &Lineage, threshold: u32) -> bool {
        self.canonical.node == other.canonical.node && self.divergence(other) < threshold
    }
}
