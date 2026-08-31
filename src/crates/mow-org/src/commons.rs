//! Quản trị tài nguyên chung (`idea.md §12.12`, `PD-08`).
//!
//! > Rừng, đồng cỏ, hệ thống tưới, ngư trường và mạch mana là tài nguyên chung.
//! > Chúng **không mặc định bị khai thác tới cạn** và cũng **không bắt buộc phải
//! > tư hữu hóa**.
//!
//! Đây là điểm mà mô hình phổ thông ("bi kịch của cái chung") sai, và Ostrom
//! được giải Nobel vì chỉ ra chỗ sai. Cộng đồng tự quản trị được — nhưng chỉ khi
//! có đủ bảy yếu tố.
//!
//! ## Điều làm cho module này đáng viết
//!
//! > **Thiếu yếu tố nào thì thất bại theo kiểu tương ứng của yếu tố đó**, và
//! > người chơi có thể nhìn ra nguyên nhân. Một mạch mana cạn kiệt vì thiếu giám
//! > sát **khác hẳn** một mạch cạn vì hạn mức đặt sai.
//!
//! Nếu chỉ có một chỉ số "quản trị tốt/xấu" thì hai trường hợp trên cho ra cùng
//! một cái hồ cạn, và người chơi không học được gì. Với bảy yếu tố, mỗi cái hỏng
//! để lại một **dấu vết riêng**, và [`diagnose`] đọc dấu vết đó ngược về nguyên
//! nhân — đúng tinh thần `§18.13`: mọi con số đều bấm được về nguồn.

use serde::{Deserialize, Serialize};

/// Bảy yếu tố quản trị của Ostrom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Principle {
    /// 1. Ranh giới tài nguyên và nhóm được quyền dùng.
    Boundaries,
    /// 2. Hạn mức khai thác phù hợp điều kiện địa phương.
    QuotaFit,
    /// 3. Giám sát, và ai giám sát người giám sát.
    Monitoring,
    /// 4. Chế tài **tăng dần** thay vì trừng phạt nặng ngay lần đầu.
    GradedSanctions,
    /// 5. Giải quyết tranh chấp rẻ và nhanh.
    ConflictResolution,
    /// 6. Quyền của chính người bị ảnh hưởng được sửa luật.
    RightToOrganize,
    /// 7. Các tầng quản trị lồng nhau cho tài nguyên lớn.
    NestedTiers,
}

/// Bảy yếu tố, để lặp.
pub const PRINCIPLES: [Principle; 7] = [
    Principle::Boundaries,
    Principle::QuotaFit,
    Principle::Monitoring,
    Principle::GradedSanctions,
    Principle::ConflictResolution,
    Principle::RightToOrganize,
    Principle::NestedTiers,
];

impl Principle {
    /// **Kiểu thất bại riêng** của yếu tố này.
    ///
    /// Đây là bảng làm cho `§12.12` có ích thay vì chỉ đúng: mỗi yếu tố thiếu để
    /// lại một dấu vết khác nhau, nên người chơi nhìn cái hồ cạn là đoán được
    /// nguyên nhân — và sửa đúng chỗ.
    pub fn failure_mode(self) -> &'static str {
        match self {
            // Không biết ai được dùng ⇒ người ngoài vào lấy, dân bản địa không
            // ngăn được vì không có tư cách gì để ngăn.
            Principle::Boundaries => "người ngoài vào khai thác, không ai có tư cách ngăn",
            // Hạn mức đặt sai ⇒ cạn dần đều, kể cả khi mọi người đều tuân thủ.
            // Đây là kiểu thất bại nguy hiểm nhất vì nó **trông như đang ổn**.
            Principle::QuotaFit => "cạn dần đều dù ai cũng tuân thủ hạn mức",
            // Không giám sát ⇒ vài người vượt mức, phần còn lại thấy thế và
            // cũng vượt. Sụp nhanh, không đều.
            Principle::Monitoring => "vài người vượt mức, rồi lan ra, sụp nhanh và không đều",
            // Phạt nặng ngay lần đầu ⇒ không ai dám báo cáo hàng xóm, nên vi
            // phạm không bao giờ được ghi nhận. Nghiêm khắc quá hóa vô hiệu.
            Principle::GradedSanctions => "không ai dám tố hàng xóm, vi phạm không được ghi nhận",
            // Tranh chấp không giải quyết được ⇒ xung đột tích lại thành thù,
            // và hợp tác chết trước tài nguyên.
            Principle::ConflictResolution => {
                "tranh chấp tích thành thù, hợp tác tan trước tài nguyên"
            }
            // Không được sửa luật ⇒ quy tắc lỗi thời vẫn còn hiệu lực, và người
            // ta bỏ tuân thủ vì nó vô lý chứ không vì tham.
            Principle::RightToOrganize => {
                "quy tắc lỗi thời vẫn hiệu lực, người ta bỏ tuân vì vô lý"
            }
            // Không có tầng lồng nhau ⇒ tài nguyên lớn bị quản như tài nguyên
            // nhỏ; quyết định ở làng này phá kế hoạch của làng kia.
            Principle::NestedTiers => "làng này quyết định phá kế hoạch của làng kia",
        }
    }
}

/// Chế độ quản trị của một tài nguyên chung.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Governance {
    /// Yếu tố nào đang **có**, `0`–`1000` cho mỗi cái.
    ///
    /// Thang liên tục chứ không phải bật/tắt: giám sát *một phần* là chuyện
    /// thường nhất trên đời, và nó cho kết quả khác hẳn cả không giám sát lẫn
    /// giám sát chặt.
    pub strength: Vec<(Principle, u16)>,
}

impl Governance {
    /// Mức của một yếu tố. Không khai báo thì **bằng 0**.
    pub fn level(&self, p: Principle) -> u16 {
        self.strength
            .iter()
            .find(|(k, _)| *k == p)
            .map_or(0, |(_, v)| *v)
    }

    /// Đặt mức.
    pub fn set(&mut self, p: Principle, v: u16) -> &mut Governance {
        self.strength.retain(|(k, _)| *k != p);
        self.strength.push((p, v));
        self.strength.sort_by_key(|(k, _)| *k);
        self
    }

    /// Chế độ quản trị lý tưởng — dùng làm mốc trong test và trong UI.
    pub fn ideal() -> Governance {
        let mut g = Governance::default();
        for p in PRINCIPLES {
            g.set(p, 1_000);
        }
        g
    }
}

/// Một tài nguyên chung.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commons {
    /// Định danh.
    pub id: String,
    /// Trữ lượng hiện tại.
    pub stock: i64,
    /// Trữ lượng tối đa.
    pub capacity: i64,
    /// Tái tạo mỗi kỳ ở mức trữ lượng đầy.
    pub regen_at_full: i64,
    /// Số người dùng.
    pub users: u32,
    /// Hạn mức khai báo, mỗi người mỗi kỳ.
    pub quota: i64,
    /// Quản trị.
    pub governance: Governance,
}

/// Kết quả một kỳ khai thác.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Harvest {
    /// Đã lấy đi bao nhiêu.
    pub taken: i64,
    /// Tái tạo được bao nhiêu.
    pub regrown: i64,
    /// Trữ lượng sau kỳ.
    pub stock_after: i64,
    /// Bao nhiêu người vượt hạn mức.
    pub violators: u32,
    /// Bao nhiêu vi phạm bị **phát hiện**.
    pub caught: u32,
}

impl Commons {
    /// Tái tạo theo đường cong logistic: nhanh nhất ở nửa trữ lượng, gần 0 ở
    /// hai đầu.
    ///
    /// Đây là chỗ "cạn dần đều" trở nên nguy hiểm: dưới một ngưỡng nào đó, tái
    /// tạo chậm tới mức không đuổi kịp dù khai thác đã giảm — tài nguyên sập
    /// **sau khi** người ta đã bắt đầu thận trọng.
    pub fn regen(&self) -> i64 {
        if self.capacity <= 0 || self.stock <= 0 {
            return 0;
        }
        let s = self.stock.min(self.capacity);
        // 4·r·s·(K−s)/K²  — cực đại đúng bằng `regen_at_full` tại s = K/2.
        4 * self.regen_at_full * s / self.capacity * (self.capacity - s) / self.capacity
    }

    /// Chạy một kỳ.
    ///
    /// **Xác định.** Mức vi phạm là hàm của các yếu tố quản trị, không của một
    /// lần tung xúc xắc — bài học phải là "quản trị quyết định kết quả", không
    /// phải "may rủi".
    pub fn step(&mut self) -> Harvest {
        let g = &self.governance;

        // Giám sát yếu ⇒ vượt mức. Chế tài tăng dần làm người ta dám tố cáo,
        // nên nó **nhân lên** hiệu quả giám sát chứ không cộng vào.
        let giam_sat = i64::from(g.level(Principle::Monitoring))
            * i64::from(g.level(Principle::GradedSanctions))
            / 1_000;

        // Ranh giới yếu ⇒ người ngoài vào, tính như người dùng thêm.
        let nguoi_ngoai =
            i64::from(self.users) * (1_000 - i64::from(g.level(Principle::Boundaries))) / 1_000;

        // Tranh chấp không giải quyết được và không được sửa luật ⇒ người ta bỏ
        // tuân thủ vì thấy vô lý, không vì tham.
        let chan_nan = (2_000
            - i64::from(g.level(Principle::ConflictResolution))
            - i64::from(g.level(Principle::RightToOrganize)))
            / 2;

        let ti_le_vuot = i64::midpoint(1_000 - giam_sat, chan_nan).clamp(0, 1_000);
        let violators = u32::try_from(i64::from(self.users) * ti_le_vuot / 1_000).unwrap_or(0);
        let caught = u32::try_from(i64::from(violators) * giam_sat / 1_000).unwrap_or(0);

        // Người tuân thủ lấy đúng hạn mức; người vượt lấy gấp đôi; người ngoài
        // không có hạn mức nào cả nên lấy gấp ba.
        let tuan = i64::from(self.users) - i64::from(violators);
        let taken = (tuan * self.quota
            + i64::from(violators) * self.quota * 2
            + nguoi_ngoai * self.quota * 3)
            .min(self.stock.max(0));

        let regrown = self.regen();
        self.stock = (self.stock - taken + regrown).clamp(0, self.capacity);

        Harvest {
            taken,
            regrown,
            stock_after: self.stock,
            violators,
            caught,
        }
    }

    /// Tài nguyên này đã sụp chưa.
    pub fn collapsed(&self) -> bool {
        self.stock * 100 < self.capacity * 5
    }
}

/// Một chẩn đoán: yếu tố nào yếu, và nó gây ra kiểu hỏng nào.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnosis {
    /// Yếu tố.
    pub principle: Principle,
    /// Mức hiện tại.
    pub level: u16,
    /// Kiểu thất bại tương ứng.
    pub failure_mode: &'static str,
}

/// Đọc ngược từ chế độ quản trị ra nguyên nhân sẽ hỏng.
///
/// `§18.13` nguyên tắc 2: mọi con số đều bấm được về nguồn. Người chơi nhìn cái
/// hồ cạn, bấm vào, và thấy *"thiếu giám sát"* chứ không thấy *"quản trị: 0.4"*.
pub fn diagnose(g: &Governance, threshold: u16) -> Vec<Diagnosis> {
    PRINCIPLES
        .iter()
        .filter(|p| g.level(**p) < threshold)
        .map(|p| Diagnosis {
            principle: *p,
            level: g.level(*p),
            failure_mode: p.failure_mode(),
        })
        .collect()
}
