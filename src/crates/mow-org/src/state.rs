//! Năng lực nhà nước và chuỗi ủy quyền (`idea.md §12.13.1`, `PD-04`).
//!
//! > Một quốc gia ra quyết định **không có nghĩa là điều đó xảy ra**.
//!
//! ```text
//! chức vụ → mệnh lệnh → ngân sách → quan chức → đơn vị thực thi → kết quả thực tế
//! ```
//!
//! Mỗi cạnh có độ trễ, thất thoát, thiếu năng lực, hiểu sai, và rủi ro người
//! được ủy quyền theo đuổi lợi ích riêng.
//!
//! ## Vì sao `coverage_by_district` phải **sinh ra**, không được viết tay
//!
//! `§12.5.1` cho phép khai `coverage_by_district: { docks: 0.25 }` trong YAML, và
//! `mow-law` đọc nó như một con số. Nhưng nếu con số đó **chỉ** là hằng số viết
//! tay thì cả `§12.13.1` trở thành trang trí: người chơi cắt ngân sách của đội
//! tuần tra và không có gì xảy ra, vì độ phủ nằm trong một file YAML mà ngân
//! sách không chạm tới.
//!
//! [`StateCapacity::coverage`] là hàm sinh ra con số đó từ ngân sách, số quan
//! chức, mức tham nhũng và khoảng cách tới trung tâm. Nhờ vậy chuỗi nhân quả
//! đóng lại: *cắt thuế → ít quan chức → độ phủ ở bến cảng tụt → trộm cắp tăng →
//! thương nhân bỏ đi → thuế còn ít hơn.*
//!
//! Đó là một vòng lặp phản hồi mà không ai phải viết riêng.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Một khu và những gì quyết định nhà nước với tới đó được bao nhiêu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct District {
    /// Định danh.
    pub id: String,
    /// Khoảng cách tới trung tâm quyền lực, tính bằng bậc hành chính.
    ///
    /// Không phải khoảng cách vật lý: một hòn đảo có phó vương riêng gần hơn một
    /// khu ổ chuột cách hoàng cung ba con phố mà không ai chịu trách nhiệm.
    pub admin_distance: u16,
    /// Dân số, để chia đầu người.
    pub population: u64,
}

/// Một mệnh lệnh đi xuống theo chuỗi ủy quyền.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directive {
    /// Nội dung.
    pub what: String,
    /// Ngân sách cấp cho nó.
    pub budget: i64,
    /// Số bậc phải đi qua.
    pub hops: u16,
}

/// Kết quả thật của một mệnh lệnh sau khi đi hết chuỗi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// Ngân sách còn lại tới nơi.
    pub delivered_budget: i64,
    /// Bao nhiêu tick mới tới nơi.
    pub delay_ticks: u64,
    /// Nội dung đã bị hiểu lệch chưa.
    pub distorted: bool,
    /// Phần đã bị giữ lại dọc đường.
    pub leaked: i64,
}

/// Năng lực thật của một nhà nước.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateCapacity {
    /// Tổng ngân sách mỗi kỳ.
    pub revenue: i64,
    /// Số quan chức có thật.
    pub officials: u32,
    /// Mức tham nhũng `0`–`1000`: phần bị giữ lại mỗi bậc.
    pub corruption: u16,
    /// Độ trễ mỗi bậc, tính bằng tick.
    pub delay_per_hop: u64,
    /// Xác suất hiểu lệch mỗi bậc, `0`–`1000`.
    pub distortion_per_hop: u16,
    /// Ngân sách cần cho **độ phủ đầy đủ**, tính trên mỗi đầu người.
    ///
    /// ## Vì sao phải khai hằng số này ra
    ///
    /// Bản đầu tiên tính độ phủ bằng `min(tiền trên đầu người, quan chức trên
    /// đầu người)` mà không có mốc quy chiếu nào, rồi chặn trần ở 1000. Kết quả:
    /// **mọi khu đều trả về 1000**. Không phải vì công thức sai dấu, mà vì không
    /// có gì nói cho nó biết "bao nhiêu là đủ" — nên bất kỳ ngân sách nào cũng
    /// vượt một cái trần vô định.
    ///
    /// Một mô hình mà mọi đầu vào cho cùng một đầu ra vẫn *chạy*, vẫn có test
    /// xanh nếu test chỉ kiểm "không panic". Nên mốc phải nằm ở đây, có tên, và
    /// content pack đặt được.
    pub full_coverage_cost_per_capita: i64,
    /// Số quan chức cần cho độ phủ đầy đủ, trên mỗi 1000 dân.
    ///
    /// Tiền và người là **hai điều kiện cần**, không thay thế nhau: ngân sách
    /// gấp mười không bù được việc không có ai đi tuần.
    pub full_coverage_officials_per_1000: u32,
    /// Các khu.
    pub districts: Vec<District>,
}

impl StateCapacity {
    /// Cho một mệnh lệnh chạy hết chuỗi ủy quyền.
    ///
    /// Thất thoát tính **theo từng bậc**, không phải một lần ở cuối. Khác biệt
    /// đó là toàn bộ lý do một đế chế lớn cai trị vùng biên kém hơn một thành
    /// bang cai trị chính nó: cùng mức tham nhũng, nhưng nhiều bậc hơn.
    pub fn execute(&self, d: &Directive) -> Outcome {
        let mut con_lai = d.budget;
        let mut lech = false;

        for buoc in 0..d.hops {
            let giu = con_lai * i64::from(self.corruption) / 1_000;
            con_lai -= giu;
            // Hiểu lệch **tích lũy**: qua đủ nhiều bậc thì gần như chắc chắn.
            // Dùng ngưỡng xác định thay vì ngẫu nhiên để `§22.9` giữ được.
            if i64::from(self.distortion_per_hop) * i64::from(buoc + 1) >= 1_000 {
                lech = true;
            }
        }

        Outcome {
            delivered_budget: con_lai,
            delay_ticks: self.delay_per_hop * u64::from(d.hops),
            distorted: lech,
            leaked: d.budget - con_lai,
        }
    }

    /// **Độ phủ cưỡng chế thật ở một khu**, `0`–`1000`.
    ///
    /// Đây là hàm mà `mow-law` đáng lẽ đọc thay vì đọc hằng số. Nó là **giá trị
    /// nhỏ hơn** trong hai điều kiện cần:
    ///
    /// - **tiền tới nơi trên đầu người**, sau khi đi qua các bậc hành chính;
    /// - **quan chức trên đầu người**.
    ///
    /// Lấy `min` chứ không phải trung bình: hai thứ này không bù cho nhau. Ngân
    /// sách gấp mười không tạo ra một người đi tuần, và một nghìn quan chức
    /// không được trả lương thì không đi tuần.
    ///
    /// Trả `0` cho khu không khai báo: một khu nhà nước chưa từng nhắc tới là
    /// một khu nhà nước không với tới, và đó là chỗ `§12.6` băng đảng mọc lên.
    pub fn coverage(&self, district_id: &str) -> u16 {
        let Some(d) = self.districts.iter().find(|d| d.id == district_id) else {
            return 0;
        };
        if d.population == 0 || self.officials == 0 {
            return 0;
        }

        let tong_dan: i64 = self
            .districts
            .iter()
            .map(|x| i64::try_from(x.population).unwrap_or(0))
            .sum();
        if tong_dan <= 0 {
            return 0;
        }
        let dan = i64::try_from(d.population).unwrap_or(1).max(1);

        // Ngân sách chia theo dân số, rồi đi qua các bậc hành chính. Đây là chỗ
        // khoảng cách hành chính biến thành bất công đo được.
        let phan = self.revenue * dan / tong_dan;
        let toi_noi = self
            .execute(&Directive {
                what: "policing".into(),
                budget: phan,
                hops: d.admin_distance,
            })
            .delivered_budget;

        let tien = if self.full_coverage_cost_per_capita <= 0 {
            1_000
        } else {
            toi_noi * 1_000 / dan / self.full_coverage_cost_per_capita
        };

        // Quan chức chia theo dân số.
        let quan_chuc_khu = i64::from(self.officials) * dan / tong_dan;
        let nguoi = if self.full_coverage_officials_per_1000 == 0 {
            1_000
        } else {
            quan_chuc_khu * 1_000 * 1_000 / dan / i64::from(self.full_coverage_officials_per_1000)
        };

        u16::try_from(tien.min(nguoi).clamp(0, 1_000)).unwrap_or(0)
    }

    /// Độ phủ mọi khu — dạng mà `mow-law` nhận trực tiếp.
    pub fn coverage_map(&self) -> BTreeMap<String, u16> {
        self.districts
            .iter()
            .map(|d| (d.id.clone(), self.coverage(&d.id)))
            .collect()
    }
}
