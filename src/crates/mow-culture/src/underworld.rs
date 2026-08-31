//! Tổ chức tội phạm, chợ đen, nghiện (`idea.md §12.6`, `PD-09`).
//!
//! ## Băng đảng **chỉ là organization với charter bất hợp pháp**
//!
//! `§12.6.1` mở đầu bằng: *"Không cần loại entity mới."*
//!
//! Đó là câu đáng giữ nhất. Một `CriminalOrg` riêng biệt sẽ có luật riêng, cách
//! tuyển người riêng, cách chết riêng — và không cái nào nhất quán với tổ chức
//! hợp pháp. Rồi khi một băng đảng mua được chức quan, hai hệ thống phải nối vào
//! nhau, và chỗ nối đó là chỗ mọi thứ hỏng.
//!
//! Ở đây [`Syndicate`] chỉ là vài trường **thêm vào** một tổ chức bình thường.
//!
//! ## Tuyển mộ từ chính hệ quả của `§12.5.4`
//!
//! > Tuyển mộ từ nhóm dân cư có `belonging` thấp và **cơ hội hợp pháp thấp**.
//!
//! Nghĩa là: trừng phạt nặng → lưu đày, kỳ thị → mất cơ hội hợp pháp → nguồn
//! tuyển của băng đảng. Nhà nước càng nghiêm khắc mù quáng càng nuôi lớn thứ nó
//! đang chống. Vòng lặp đó không cần ai viết ra — [`recruitment_pool`] đọc đúng
//! những trường mà hình phạt đã để lại.
//!
//! ## Chợ đen **không cần hệ thống riêng**
//!
//! > Nó là thị trường ở `§12.2` cộng thêm phần bù rủi ro, xác suất bị bắt giữ
//! > hàng và giá phụ thuộc mức truy quét.
//!
//! Nên [`black_market_price`] là một hàm, không phải một cái chợ khác.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Nguồn thu của một băng đảng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Racket {
    /// Bảo kê.
    Protection,
    /// Buôn lậu.
    Smuggling,
    /// Trộm cắp có tổ chức.
    OrganisedTheft,
    /// Cho vay nặng lãi.
    Usury,
    /// Buôn người.
    Trafficking,
    /// Đánh bạc.
    Gambling,
}

/// Vài trường **thêm vào** một tổ chức bình thường để nó thành băng đảng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Syndicate {
    /// Định danh tổ chức — vẫn là một `organization` như mọi tổ chức khác.
    pub org: String,
    /// Lãnh thổ kiểm soát.
    pub territory: Vec<String>,
    /// Nguồn thu.
    pub rackets: Vec<Racket>,
    /// Luật nội bộ: mức trừng phạt kẻ chỉ điểm, `0`–`1000`.
    ///
    /// Cao thì im lặng tốt, nhưng cũng làm thành viên khó rời bỏ — và một tổ
    /// chức không ai rời được là một tổ chức không ai dám gia nhập.
    pub omerta: u16,
    /// Có nghi thức gia nhập tốn kém không (`§12.16` cùng cơ chế).
    pub initiation_rite: bool,
    /// **Cạnh hối lộ** nối sang tổ chức hợp pháp: quan chức nào đã mua được.
    pub bribed_officials: Vec<EntityId>,
}

/// Một nhóm dân cư, nhìn từ góc độ tuyển mộ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cohort {
    /// Bao nhiêu người.
    pub size: u32,
    /// Mức gắn bó với cộng đồng, `0`–`1000`.
    pub belonging: u16,
    /// **Cơ hội hợp pháp**, `0`–`1000`.
    ///
    /// Đây là trường mà hình phạt ở `§12.5.4` hạ xuống: lưu đày, kỳ thị, án tích.
    pub lawful_opportunity: u16,
}

/// Bao nhiêu người trong nhóm này là nguồn tuyển tiềm năng.
///
/// Hai điều kiện **cùng lúc**: ít gắn bó **và** ít cơ hội. Chỉ một cái thì
/// không đủ — một người nghèo mà gắn bó với xóm làng thì không đi làm cho băng
/// đảng, và một người lạc lõng mà có nghề tử tế cũng vậy.
pub fn recruitment_pool(c: &Cohort) -> u32 {
    let long_leo = i64::from(1_000 - c.belonging);
    let bi_tac = i64::from(1_000 - c.lawful_opportunity);
    // Tích hai phần nghìn cho ra một phần nghìn: chia **một** lần cho 1000, không
    // phải hai. Chia hai lần thì mọi nhóm đều ra 0, và băng đảng không bao giờ
    // tuyển được ai — một mô hình chạy êm và hoàn toàn vô dụng.
    let ti_le = long_leo * bi_tac / 1_000;
    u32::try_from(i64::from(c.size) * ti_le / 1_000).unwrap_or(0)
}

/// Giá chợ đen = giá hợp pháp + phần bù rủi ro.
///
/// **Không phải một cái chợ khác** — chỉ là giá của cùng thị trường ở `§12.2`,
/// cộng thêm ba thứ:
///
/// - `crackdown` càng gắt càng đắt: người bán đòi bù cho rủi ro bị bắt;
/// - `seizure_rate` càng cao càng đắt: hàng mất dọc đường phải tính vào giá;
/// - hàng bị cấm hẳn thì đắt hơn hàng chỉ bị đánh thuế.
///
/// Hệ quả rơi ra: **truy quét mạnh làm lợi nhuận tăng**, và đó là vòng phản hồi
/// ở `§12.6.3` — cấm đoán → chợ đen → lợi nhuận lớn → tổ chức mạnh hơn.
pub fn black_market_price(
    lawful_price: i64,
    crackdown: u16,
    seizure_rate: u16,
    outright_banned: bool,
) -> i64 {
    let bu_rui_ro = lawful_price * i64::from(crackdown) / 1_000;
    // Hàng bị tịch thu phải được bù bởi những chuyến trót lọt.
    let bu_tich_thu = if seizure_rate >= 1_000 {
        lawful_price * 10
    } else {
        lawful_price * i64::from(seizure_rate) / i64::from(1_000 - seizure_rate).max(1)
    };
    let bu_cam = if outright_banned { lawful_price / 2 } else { 0 };
    lawful_price + bu_rui_ro + bu_tich_thu + bu_cam
}

/// Một chất gây nghiện (`§12.6.2`).
///
/// Ánh xạ thẳng vào `§9.8.6` — không có hệ thống "nghiện" riêng, chỉ là một item
/// có dược lý.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Substance {
    /// Định danh vật phẩm.
    pub id: String,
    /// Liều chuẩn.
    pub dose: i64,
    /// Tác dụng kéo dài bao nhiêu tick.
    pub duration: u64,
    /// Mức tăng dung nạp mỗi liều, `0`–`1000`.
    pub tolerance_gain: u16,
    /// Mức tăng lệ thuộc mỗi liều, `0`–`1000`.
    pub dependence_gain: u16,
    /// Độc tính cộng vào `toxin_load` mỗi liều.
    pub toxicity: i64,
}

/// Trạng thái nghiện của một người.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Addiction {
    /// Dung nạp: cần bao nhiêu liều mới có tác dụng như cũ, `0`–`1000`.
    pub tolerance: u16,
    /// Lệ thuộc, `0`–`1000`.
    pub dependence: u16,
    /// Độc tích lũy.
    pub toxin_load: i64,
    /// Đã bao lâu chưa dùng.
    pub ticks_since_dose: u64,
}

impl Addiction {
    /// Dùng một liều.
    pub fn take(&mut self, s: &Substance) {
        self.tolerance = self.tolerance.saturating_add(s.tolerance_gain).min(1_000);
        self.dependence = self.dependence.saturating_add(s.dependence_gain).min(1_000);
        self.toxin_load += s.toxicity;
        self.ticks_since_dose = 0;
    }

    /// Liều **thật sự cần** để có tác dụng, sau khi đã dung nạp.
    ///
    /// Đây là chỗ độc tính tích lũy trở nên chết người mà không cần một cơ chế
    /// riêng: càng nghiện càng phải dùng nhiều, càng dùng nhiều càng độc.
    pub fn effective_dose(&self, s: &Substance) -> i64 {
        s.dose * (1_000 + i64::from(self.tolerance)) / 1_000
    }

    /// Mức vật vã hiện tại, `0`–`1000`.
    pub fn withdrawal(&self, s: &Substance) -> u16 {
        if self.ticks_since_dose < s.duration {
            return 0;
        }
        let qua_han = self.ticks_since_dose - s.duration;
        let m = i64::from(self.dependence) * i64::try_from(qua_han.min(10_000)).unwrap_or(0)
            / i64::try_from(s.duration.max(1)).unwrap_or(1)
            / 10;
        u16::try_from(m.clamp(0, 1_000)).unwrap_or(1_000)
    }
}

/// **Cờ bạc dùng cùng khung**: phần thưởng biến thiên tạo `craving` mà không cần chất.
///
/// Trả về mức thèm, `0`–`1000`. Điểm mấu chốt là `variance`: một trò luôn thắng
/// hoặc luôn thua đều không gây nghiện. Chính sự **không đoán trước được** mới
/// gây nghiện, và đó là lý do nhà cái thiết kế tỉ lệ thắng chứ không thiết kế
/// phần thưởng.
pub fn gambling_craving(sessions: u32, variance: u16, near_misses: u32) -> u16 {
    let lap = i64::from(sessions.min(200)) * 3;
    let bien_thien = i64::from(variance);
    // Suýt thắng có sức kéo mạnh hơn cả thắng thật — một hiệu ứng có thật, và là
    // lý do máy đánh bạc được thiết kế để suýt thắng thường xuyên.
    let suyt = i64::from(near_misses.min(200)) * 4;
    let m = (lap + suyt) * bien_thien / 1_000;
    u16::try_from(m.clamp(0, 1_000)).unwrap_or(1_000)
}
