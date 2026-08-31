//! Loài xâm lấn và mầm bệnh đi qua cổng (`idea.md §9.10.1`, `§6.4`, `PE-12`).
//!
//! > Một mầm bệnh mà dân bản địa chưa từng có miễn dịch có thể **xóa sổ cả một
//! > nền văn minh nhanh hơn bất kỳ đội quân nào**. Đây là lý do `§6.4` tồn tại,
//! > và là một trong những hệ quả đáng sợ nhất mà việc mở cổng có thể gây ra —
//! > **thường là ngoài ý muốn của người mở**.
//!
//! Cụm cuối là yêu cầu thiết kế thật sự. Hậu quả phải đến từ **cơ chế**, không
//! từ một Director quyết định rằng bây giờ là lúc có dịch. Nên ở đây:
//!
//! - Một loài bùng nổ vì **thiếu thiên địch ở đích**, tính được từ dữ liệu
//!   chuỗi thức ăn, không từ một cờ `is_invasive`.
//! - Một mầm bệnh giết nhiều vì **quần thể đích chưa từng phơi nhiễm**, tính
//!   được từ lịch sử miễn dịch, không từ một con số `lethality`.
//!
//! Cả hai đều **truy ngược được về đúng một chuyến đi qua cổng**, vì `§6.2`
//! bước 8 bắt buộc ghi lại những gì đã đi cùng.
//!
//! ## Vì sao không có cờ `is_invasive`
//!
//! Xâm lấn **không phải thuộc tính của loài**. Cùng một con thỏ là loài bản địa
//! ở nơi có cáo và là thảm họa ở nơi không có. Một cờ trên định nghĩa loài sẽ
//! nói sai ở một trong hai nơi, và nó cũng xóa mất câu hỏi thú vị nhất: *thả
//! con gì vào thì hết bùng nổ*.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Vị trí của một loài trong chuỗi thức ăn ở một hệ sinh thái.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoodWeb {
    /// Loài nào ăn loài này.
    pub predators: BTreeSet<String>,
    /// Loài này ăn gì.
    pub prey: BTreeSet<String>,
    /// Loài nào cạnh tranh cùng nguồn thức ăn.
    pub competitors: BTreeSet<String>,
}

/// Hệ sinh thái đích, nhìn từ góc độ một loài mới tới.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ecosystem {
    /// Loài đang có mặt.
    pub present: BTreeSet<String>,
    /// Sức tải cho nhóm sinh thái này.
    pub carrying_capacity: u64,
}

/// Rủi ro bùng nổ của một loài mới tới.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvasionRisk {
    /// Loài nào.
    pub species: String,
    /// Thiên địch của nó **có mặt ở đích** không.
    pub predators_present: BTreeSet<String>,
    /// Thức ăn của nó có sẵn không.
    pub prey_available: bool,
    /// Loài bản địa nào bị cạnh tranh trực tiếp.
    pub competitors_displaced: BTreeSet<String>,
}

impl InvasionRisk {
    /// Có bùng nổ không.
    ///
    /// Điều kiện: **có thức ăn và không thiên địch**. Hai vế, và cả hai đều đọc
    /// từ dữ liệu hệ sinh thái đích — nên cùng một loài cho ra hai kết luận
    /// khác nhau ở hai world, đúng như thực tế.
    pub fn will_explode(&self) -> bool {
        self.prey_available && self.predators_present.is_empty()
    }

    /// Thả con gì vào thì hết bùng nổ.
    ///
    /// Đây là câu hỏi mà một cờ `is_invasive` xóa mất. Trả về thiên địch đang
    /// thiếu — và việc thả chúng vào lại là một quyết định có hậu quả riêng.
    pub fn missing_predators(&self, web: &FoodWeb) -> BTreeSet<String> {
        web.predators
            .difference(&self.predators_present)
            .cloned()
            .collect()
    }
}

/// Đánh giá rủi ro của một loài đi qua cổng.
pub fn assess(species: &str, web: &FoodWeb, dest: &Ecosystem) -> InvasionRisk {
    InvasionRisk {
        species: species.to_owned(),
        predators_present: web.predators.intersection(&dest.present).cloned().collect(),
        prey_available: web.prey.is_empty() || web.prey.iter().any(|p| dest.present.contains(p)),
        competitors_displaced: web
            .competitors
            .intersection(&dest.present)
            .cloned()
            .collect(),
    }
}

/// Lịch sử phơi nhiễm của một quần thể với một mầm bệnh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Immunity {
    /// Mầm bệnh nào.
    pub pathogen: String,
    /// Quần thể đã từng gặp chưa.
    ///
    /// **Chưa từng gặp** là điều kiện của thảm họa, không phải "miễn dịch thấp".
    /// Khác biệt này quan trọng: một quần thể có miễn dịch một phần chịu tổn
    /// thất nặng, còn một quần thể **chưa từng phơi nhiễm** thì sụp.
    pub ever_exposed: bool,
    /// Miễn dịch cộng đồng, phần nghìn.
    pub herd_permille: u32,
}

/// Hậu quả của một mầm bệnh mới tới.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outbreak {
    /// Mầm bệnh nào.
    pub pathogen: String,
    /// Tỉ lệ tử vong dự kiến, phần nghìn dân.
    pub mortality_permille: u32,
    /// Có phải là mức xóa sổ nền văn minh không.
    pub civilization_ending: bool,
    /// Chuyến đi qua cổng nào mang nó tới — **truy ngược được**.
    pub arrived_via: Option<u64>,
}

/// Ngưỡng gọi là "xóa sổ nền văn minh", phần nghìn dân.
///
/// 300‰ — bậc độ lớn của những đợt dịch thật sự làm sụp cấu trúc xã hội, chứ
/// không chỉ làm dân số giảm. Dưới mức đó thì xã hội tổn thất nặng nhưng còn
/// vận hành; trên mức đó thì thiết chế, nghề, và trí nhớ tập thể đứt gãy.
pub const NGUONG_XOA_SO: u32 = 300;

/// Độc lực nội tại của một mầm bệnh, phần nghìn — trên quần thể **đã** phơi nhiễm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Virulence(pub u32);

/// Hậu quả của việc một mầm bệnh đi qua cổng.
///
/// Nhân tố quyết định **không phải** độc lực mà là chỗ quần thể đích đã từng
/// gặp nó chưa: một mầm bệnh tầm thường ở world nguồn có thể là thảm họa ở
/// world đích, và đó chính là điều `§9.10.1` mô tả.
pub fn outbreak(v: Virulence, im: &Immunity, arrived_via: Option<u64>) -> Outbreak {
    let co_ban = if im.ever_exposed {
        // Đã gặp: miễn dịch cộng đồng cắt bớt.
        u64::from(v.0) * u64::from(1_000 - im.herd_permille.min(1_000)) / 1_000
    } else {
        // **Chưa từng gặp**: không có miễn dịch cộng đồng nào để cắt, và độc
        // lực biểu hiện gấp bội vì mọi lứa tuổi cùng nhiễm một lúc — không có
        // thế hệ nào đã qua bệnh để chăm thế hệ đang bệnh.
        u64::from(v.0) * 3
    };
    let m = u32::try_from(co_ban.min(1_000)).unwrap_or(1_000);
    Outbreak {
        pathogen: im.pathogen.clone(),
        mortality_permille: m,
        civilization_ending: m >= NGUONG_XOA_SO,
        arrived_via,
    }
}
