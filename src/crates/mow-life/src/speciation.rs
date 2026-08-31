//! Hình thành loài mới (`idea.md §9.5.5`, `PE-GATE`).
//!
//! > **Portal là cỗ máy tạo loài tốt nhất của thế giới này.** Một nhóm di cư
//! > sang world khác sống dưới trọng lực, khí quyển và mật độ mana khác; vài
//! > trăm năm sau cổng mở lại, hai bên gặp nhau ở một **vùng tiếp xúc thứ
//! > cấp** — vẫn nhận ra nhau là họ hàng, nhưng **con lai đã bắt đầu vô sinh**.
//!
//! ## Vì sao bất tương hợp phải là **cặp**, không phải một khoảng cách
//!
//! Cách rẻ là đo "khoảng cách di truyền" rồi bảo dưới ngưỡng thì lai được. Nó
//! chạy, và nó sai ở đúng chỗ thú vị: bất tương hợp thật (Bateson–Dobzhansky–
//! Muller) sinh ra từ việc **hai nhánh cùng tiến hóa theo hai hướng vô hại
//! riêng lẻ**, và chỉ khi gặp nhau trong một bộ gen thì hai allele mới đánh
//! nhau.
//!
//! Hệ quả quan sát được, và nó là hệ quả mà một mô hình khoảng cách không cho:
//! số cặp bất tương hợp tăng **theo bình phương** số khác biệt tích lũy, nên
//! cách ly hai lần lâu hơn cho ra bốn lần nhiều bất tương hợp. Đó gọi là hiệu
//! ứng "snowball", và nó là lý do quá trình này bắt đầu chậm rồi tăng tốc.
//!
//! ## Phân kỳ đo được, không phải một cờ
//!
//! `PE-GATE` đòi *"con lai giảm sinh sản **đo được**"*. Nên
//! [`Divergence::hybrid_fertility`] trả về một phần nghìn, và
//! [`secondary_contact`] trả về một biên bản đủ để một nhà tự nhiên học trong
//! game điền vào sổ: bao nhiêu con lai sinh ra, bao nhiêu con sinh sản tiếp.
//!
//! Tham khảo: Orr (1995) trên tích lũy bất tương hợp BDM.

use crate::barrier::Reproductive;
use serde::{Deserialize, Serialize};

/// Bốn con đường tạo loài mới (`§9.5.5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeciationRoute {
    /// Cách ly rồi phân kỳ — núi, biển, hoặc **một portal đóng lại**.
    IsolationThenDivergence,
    /// Trôi dạt trong quần thể nhỏ — nút thắt cổ chai cố định allele hiếm.
    DriftInSmallPopulation,
    /// Áp lực chọn lọc mới — khí hậu đổi, con mồi biến mất, trường mana mới.
    NewSelectionPressure,
    /// Tác nhân gây đột biến — dị thường mana, chất độc, thí nghiệm hỏng.
    Mutagen,
}

/// Một quần thể đang bị cách ly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolatedPopulation {
    /// Định danh.
    pub id: String,
    /// Cách ly bởi cái gì.
    pub route: SpeciationRoute,
    /// Kích cỡ quần thể hiệu dụng.
    ///
    /// Nhỏ thì trôi dạt nhanh — đó là con đường 2, và nó **cộng dồn** với con
    /// đường 1 chứ không thay thế.
    pub effective_size: u64,
    /// Số đời đã trôi qua trong cách ly.
    pub generations: u64,
    /// Áp lực chọn lọc khác biệt so với quần thể gốc, phần nghìn.
    ///
    /// World đích có trọng lực, khí quyển và mật độ mana khác thì con số này
    /// lớn — đó là lý do portal tạo loài nhanh hơn một dãy núi.
    pub selection_differential: u32,
}

/// Số khác biệt di truyền cố định, và số cặp bất tương hợp sinh ra từ đó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Divergence {
    /// Số locus đã cố định khác nhau giữa hai nhánh.
    pub fixed_differences: u64,
    /// Số **cặp** bất tương hợp BDM.
    pub incompatible_pairs: u64,
}

/// Bao nhiêu cặp khác biệt thì thật sự tương tác xấu, một phần bao nhiêu.
///
/// Đại đa số cặp allele khác nhau chung sống vô hại; chỉ một phần nhỏ đánh
/// nhau. Con số này quyết định **thang thời gian** của toàn bộ quá trình, nên
/// nó có tên và nằm một chỗ.
///
/// Hiệu chỉnh theo `§9.5.5`: *"**vài trăm năm** sau cổng mở lại … con lai đã
/// bắt đầu vô sinh"*. Ở 1/600, một nhánh tách 600 đời dưới áp lực chọn lọc của
/// một world khác cho con lai còn khoảng một nửa khả năng sinh sản — "đã bắt
/// đầu", chưa hẳn. Đây là một con số của thế giới này, không phải của Trái Đất:
/// tốc độ thật chậm hơn nhiều bậc, và tài liệu thiết kế đã chọn nhanh hơn một
/// cách có chủ ý để portal thành cỗ máy tạo loài chơi được.
pub const MOT_PHAN_BAO_NHIEU_CAP_LA_XAU: u64 = 600;

/// Số cặp bất tương hợp để con lai vô sinh hoàn toàn.
pub const CAP_DU_DE_VO_SINH: u64 = 1_000;

impl Divergence {
    /// Tính phân kỳ sau `generations` đời cách ly.
    ///
    /// Khác biệt tích lũy **tuyến tính** theo thời gian; cặp bất tương hợp tăng
    /// **theo bình phương**. Hai nhịp khác nhau là toàn bộ nội dung của mô hình
    /// — nếu cả hai cùng tuyến tính thì không có hiệu ứng snowball, và quá
    /// trình sẽ đều đặn một cách không thực tế.
    pub fn after(p: &IsolatedPopulation) -> Divergence {
        // Trôi dạt cố định allele nhanh hơn ở quần thể nhỏ, và chọn lọc khác
        // biệt đẩy thêm. Cả hai đường của `§9.5.5` cộng dồn ở đây.
        let troi_dat = p.generations * 1_000 / p.effective_size.clamp(1, 10_000);
        let chon_loc = p.generations * u64::from(p.selection_differential) / 1_000;
        let khac_biet = troi_dat + chon_loc;

        Divergence {
            fixed_differences: khac_biet,
            // k(k−1)/2 cặp, chia cho tỉ lệ cặp thật sự xấu.
            incompatible_pairs: khac_biet.saturating_mul(khac_biet.saturating_sub(1))
                / 2
                / MOT_PHAN_BAO_NHIEU_CAP_LA_XAU,
        }
    }

    /// Khả năng sinh sản của con lai, phần nghìn.
    ///
    /// Giảm dần chứ không nhảy: `§9.5.5` viết *"con lai **đã bắt đầu** vô
    /// sinh"*, và chữ "bắt đầu" là chỗ toàn bộ bi kịch chính trị nằm ở đó —
    /// hai bên còn nhận ra nhau là họ hàng, còn cưới nhau, và mới phát hiện ra
    /// vấn đề sau một thế hệ.
    pub fn hybrid_fertility(self) -> u32 {
        let mat = self.incompatible_pairs.saturating_mul(1_000) / CAP_DU_DE_VO_SINH;
        u32::try_from(1_000u64.saturating_sub(mat)).unwrap_or(0)
    }

    /// Xếp vào thang rào cản sinh sản của `§9.11.1`.
    pub fn as_barrier(self) -> Reproductive {
        match self.hybrid_fertility() {
            950..=1_000 => Reproductive::FullyCompatible,
            300..=949 => Reproductive::ReducedViability,
            1..=299 => Reproductive::SterileHybrid,
            _ => Reproductive::Incompatible,
        }
    }
}

/// Biên bản một vùng tiếp xúc thứ cấp (`§9.5.5`).
///
/// Đây là thứ một nhà tự nhiên học trong game điền vào sổ — mọi trường đều
/// **đếm được bằng quan sát**, không có trường nào chỉ engine mới biết.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecondaryContact {
    /// Cách ly bao nhiêu đời.
    pub generations_apart: u64,
    /// Hai bên còn nhận ra nhau là họ hàng không.
    ///
    /// Gần như luôn **có** ở thang thời gian này, và đó chính là chỗ đau: nếu
    /// họ đã thành hai loài trông khác hẳn nhau thì không ai kỳ vọng lai được,
    /// và không có bi kịch nào cả.
    pub still_recognisable: bool,
    /// Số cặp thử sinh con.
    pub pairings: u64,
    /// Số con lai sinh ra.
    pub hybrids_born: u64,
    /// Số con lai sinh sản tiếp được.
    pub hybrids_fertile: u64,
    /// Phân kỳ đo được.
    pub divergence: Divergence,
}

impl SecondaryContact {
    /// Tỉ lệ con lai sinh sản được, phần nghìn — **con số đo được**.
    pub fn fertile_permille(&self) -> u32 {
        if self.hybrids_born == 0 {
            return 0;
        }
        u32::try_from(self.hybrids_fertile * 1_000 / self.hybrids_born).unwrap_or(1_000)
    }

    /// Đã đo được sự sụt giảm chưa.
    ///
    /// Cần **cả hai**: có con lai để đo, và tỉ lệ sinh sản thấp hơn hẳn mức
    /// cùng loài. Không có con lai nào thì không kết luận được gì — đó là mẫu
    /// rỗng, không phải bằng chứng vô sinh.
    pub fn decline_is_measurable(&self) -> bool {
        self.hybrids_born >= 20 && self.fertile_permille() < 900
    }
}

/// Hai nhánh gặp lại nhau sau cách ly (`§9.5.5`).
///
/// `pairings` là số cặp thật sự thử, và số con lai sinh ra suy từ khả năng
/// sinh sản — nên hàm này **xác định**: cùng đầu vào cho cùng biên bản.
pub fn secondary_contact(p: &IsolatedPopulation, pairings: u64) -> SecondaryContact {
    let d = Divergence::after(p);
    let f = u64::from(d.hybrid_fertility());

    // Bất tương hợp làm hỏng cả sức sống phôi lẫn khả năng sinh sản, nhưng
    // hỏng sinh sản **trước**: con lai đời F1 thường sống khỏe và vô sinh, chứ
    // không chết trong trứng. Đó là lý do hai con số dưới đây tách rời.
    let ti_le_sinh_duoc = 500 + f / 2; // sức sống giảm chậm hơn
    let sinh_ra = pairings * ti_le_sinh_duoc / 1_000;
    let sinh_san_tiep = sinh_ra * f / 1_000;

    SecondaryContact {
        generations_apart: p.generations,
        still_recognisable: d.fixed_differences < 100_000,
        pairings,
        hybrids_born: sinh_ra,
        hybrids_fertile: sinh_san_tiep,
        divergence: d,
    }
}
