//! Di cư, tị nạn và ứng phó thảm họa (`idea.md §12.19`, `§12.20`, `PF-20`).
//!
//! ## Quyết định rời đi dựa trên **belief**, không dựa trên số liệu thật
//!
//! `§12.19` mở đầu bằng đúng câu đó, và nó là chỗ dễ làm sai nhất: một hàm
//! `should_migrate(world_state)` chạy tốt, cho kết quả hợp lý, và **sai về
//! nguyên tắc**. Người ta không di cư vì lương ở nơi đến thật sự cao hơn; họ
//! di cư vì họ **tin** là cao hơn — và khoảng cách giữa hai điều đó là toàn bộ
//! bi kịch của di cư lao động.
//!
//! Nên [`decide`] nhận [`Belief`], không nhận world state. Không có tham số
//! nào cho phép nó nhìn sự thật.
//!
//! ## Di cư là quyết định của **hộ gia đình hoặc mạng lưới**
//!
//! > gửi một người đi trước, những người sau đi theo
//!
//! Nên [`Household::decide`] có hai chế độ: người đi đầu chịu rủi ro cao và đi
//! khi kỳ vọng vừa đủ; người đi sau đi khi **người đi trước đã tới nơi**, và
//! rào cản của họ thấp hơn hẳn. Mô hình một-cá-nhân-một-quyết-định không tạo
//! ra được dòng người theo chuỗi mà mọi làn sóng di cư thật đều có.
//!
//! ## Cùng một trận động đất, hai kết cục
//!
//! `§12.20`, và đây là câu mà `PF-20` phải chứng minh:
//!
//! > Cùng một trận động đất chỉ gây thiệt hại cục bộ ở một xã hội có tổ chức,
//! > nhưng **làm sụp đổ một nhà nước đã mất chính danh** — và đó là **hệ quả
//! > tính ra được, không phải một quyết định của Director**.
//!
//! Nên [`respond`] nhận cường độ thiên tai và [`Capacity`], và kết cục rơi ra
//! từ phép tính. Không có tham số `should_collapse` nào.

use crate::legitimacy::Legitimacy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ══════════════════════ §12.19 · di cư ══════════════════════

/// Niềm tin của một người về nơi ở hiện tại và nơi định đến.
///
/// **Belief**, không phải sự thật. Mỗi trường là thứ người đó nghĩ, và nó sai
/// được — tin đồn phóng đại lương, một người quen kể chuyện thành công, một
/// nỗi sợ lớn hơn nguy hiểm thật.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Belief {
    /// Tin nơi đang ở an toàn tới đâu, phần nghìn.
    pub safety_here: u32,
    /// Tin nơi định đến an toàn tới đâu.
    pub safety_there: u32,
    /// Tin tiền công ở đây bao nhiêu.
    pub wage_here: u32,
    /// Tin tiền công ở đó bao nhiêu.
    pub wage_there: u32,
    /// Tin đường đi tốn bao nhiêu.
    pub journey_cost: u32,
    /// **Có người quen ở nơi đến không.**
    ///
    /// Trường quyết định nhất trong cả cấu trúc, và là lý do di cư chảy theo
    /// những dòng cố định thay vì tỏa đều tới mọi nơi giàu hơn.
    pub has_contact_there: bool,
}

/// Vai trò trong một chuỗi di cư (`§12.19`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Người đi đầu — chịu rủi ro cao, chưa có ai ở nơi đến.
    Pioneer,
    /// Người đi theo — đi khi người đi trước đã tới nơi.
    Follower,
}

/// Ngưỡng lợi ích để một người đi đầu quyết định rời đi, phần nghìn.
///
/// Cao, vì họ đi vào chỗ không quen ai. Đây là con số làm cho làn sóng di cư
/// **bắt đầu chậm** — và bắt đầu chậm rồi tăng vọt là hình dạng thật của mọi
/// dòng di cư có thật.
pub const NGUONG_NGUOI_DI_DAU: i64 = 400;

/// Ngưỡng cho người đi theo. Thấp hơn hẳn: đã có người đón.
pub const NGUONG_NGUOI_DI_SAU: i64 = 100;

/// Một quyết định di cư, kèm lý do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// Có đi không.
    pub leaves: bool,
    /// Điểm kỳ vọng đã tính.
    pub expected_gain: i64,
    /// Ngưỡng áp dụng.
    pub threshold: i64,
    /// Vai trò.
    pub role: Role,
}

/// Quyết định rời đi — **chỉ từ belief** (`§12.19`).
///
/// Chữ ký không nhận world state. Đó là cách quy tắc được thi hành bằng kiểu
/// chứ không bằng kỷ luật: không có đường nào để hàm này nhìn sự thật.
pub fn decide(b: &Belief, role: Role) -> Decision {
    let loi_luong = i64::from(b.wage_there) - i64::from(b.wage_here);
    let loi_an_toan = i64::from(b.safety_there) - i64::from(b.safety_here);
    // Người quen ở nơi đến **cắt chi phí đường đi**: có chỗ ở nhờ, có người
    // giới thiệu việc. Đây là cơ chế thật đằng sau chuỗi di cư, không phải một
    // khoản thưởng gắn thêm.
    let chi_phi = if b.has_contact_there {
        i64::from(b.journey_cost) / 3
    } else {
        i64::from(b.journey_cost)
    };
    let ky_vong = loi_luong + loi_an_toan - chi_phi;

    let nguong = match role {
        Role::Pioneer => NGUONG_NGUOI_DI_DAU,
        Role::Follower => NGUONG_NGUOI_DI_SAU,
    };
    Decision {
        leaves: ky_vong >= nguong,
        expected_gain: ky_vong,
        threshold: nguong,
        role,
    }
}

/// Một hộ gia đình quyết định di cư (`§12.19`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Household {
    /// Bao nhiêu người.
    pub size: u32,
    /// Đã có ai đi trước và tới nơi chưa.
    pub pioneer_arrived: bool,
    /// Niềm tin chung của hộ.
    pub belief: Belief,
}

impl Household {
    /// Hộ này gửi bao nhiêu người đi, đợt này.
    ///
    /// **Gửi một người đi trước, những người sau đi theo** — không phải cả hộ
    /// cùng đi một lúc. Cả hộ cùng đi là mô hình sai: nó bỏ mất chuỗi, và với
    /// nó là bỏ mất kiều hối, môi giới việc làm, và toàn bộ cộng đồng ly tán.
    pub fn decide(&self) -> (u32, Role) {
        if !self.pioneer_arrived {
            let d = decide(&self.belief, Role::Pioneer);
            return (u32::from(d.leaves), Role::Pioneer);
        }
        // Người đi trước đã tới: những người còn lại có "người quen ở nơi đến".
        let co_nguoi_don = Belief {
            has_contact_there: true,
            ..self.belief
        };
        let d = decide(&co_nguoi_don, Role::Follower);
        if d.leaves {
            (self.size.saturating_sub(1), Role::Follower)
        } else {
            (0, Role::Follower)
        }
    }
}

/// Một cộng đồng ly tán (`§12.19`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diaspora {
    /// Từ đâu đến.
    pub origin: String,
    /// Đang ở đâu.
    pub host: String,
    /// Bao nhiêu người.
    pub population: u64,
    /// Kiều hối gửi về mỗi năm.
    pub remittances_per_year: u64,
    /// Còn giữ ngôn ngữ gốc không.
    pub keeps_language: bool,
    /// Bao nhiêu thế hệ đã ở đây.
    pub generations: u32,
}

impl Diaspora {
    /// **Lòng trung thành kép** — nguồn nghi kỵ chính trị (`§12.19`).
    ///
    /// Tính từ những thứ **quan sát được từ bên ngoài**: gửi tiền về, giữ tiếng
    /// nói, mới tới. Đó là chủ đích — nghi kỵ nảy sinh từ dấu hiệu bên ngoài
    /// chứ không từ lòng người thật, và một cộng đồng hòa nhập hoàn toàn vẫn bị
    /// nghi nếu vẫn nói tiếng cũ ở nhà.
    pub fn dual_loyalty_suspicion(&self) -> u32 {
        let mut d = 0;
        if self.remittances_per_year > 0 {
            d += 300;
        }
        if self.keeps_language {
            d += 300;
        }
        // Nghi kỵ giảm dần theo đời, nhưng **không về 0**: `§12.19` gọi đây là
        // "một nguồn nghi kỵ chính trị rất thật", và nó thật vì nó dai.
        d += 400u32.saturating_sub(self.generations.saturating_mul(100));
        d.min(1_000)
    }

    /// Cộng đồng này môi giới việc làm cho người mới tới — cơ chế của chuỗi.
    pub fn eases_arrival(&self) -> bool {
        self.population >= 100
    }
}

// ══════════════════════ §12.20 · thảm họa ══════════════════════

/// Bảy năng lực ứng phó (`§12.20`). **Tất cả đều có thể thiếu.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Capacity {
    /// Cảnh báo sớm.
    pub warning: u32,
    /// Sơ tán.
    pub evacuation: u32,
    /// Nơi trú.
    pub shelter: u32,
    /// Kho dự phòng.
    pub stockpile: u32,
    /// Lực lượng cứu hộ.
    pub rescue: u32,
    /// Mạng lưới tình nguyện.
    pub volunteers: u32,
    /// Năng lực tái thiết.
    pub reconstruction: u32,
}

impl Capacity {
    /// Một xã hội có tổ chức: mọi năng lực đều khá.
    pub fn organised() -> Capacity {
        Capacity {
            warning: 800,
            evacuation: 750,
            shelter: 700,
            stockpile: 700,
            rescue: 750,
            volunteers: 800,
            reconstruction: 700,
        }
    }

    /// Một nhà nước đã mất chính danh: hình thức còn, thực chất không.
    ///
    /// Chú ý `warning` vẫn cao — hệ thống cảnh báo là hạ tầng kỹ thuật, nó
    /// không mất đi khi chính quyền mất uy tín. Cái mất là **người ta có nghe
    /// theo hay không**, và đó là [`Legitimacy`], không phải năng lực.
    pub fn hollowed() -> Capacity {
        Capacity {
            warning: 700,
            evacuation: 200,
            shelter: 150,
            stockpile: 100,
            rescue: 200,
            volunteers: 250,
            reconstruction: 100,
        }
    }
}

/// Cường độ thiên tai, thang mở.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Magnitude(pub u32);

/// Kết cục của một thảm họa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aftermath {
    /// Thương vong, phần nghìn dân vùng bị nạn.
    pub casualties_permille: u32,
    /// Bao nhiêu phần nghìn dân phải rời đi.
    pub displaced_permille: u32,
    /// Bao nhiêu năm để tái thiết.
    pub rebuild_years: u32,
    /// Mức tuân thủ sau thảm họa, phần nghìn.
    ///
    /// Dùng `Compliance::total` của `§12.13.2` chứ không dựng một thang chính
    /// danh riêng: một thảm họa không làm người ta *"bớt tin"* một cách trừu
    /// tượng, nó làm họ **thôi nghe theo** — và cái đo được là cái sau.
    pub compliance_after: u16,
    /// **Nhà nước có sụp không** — tính ra, không phải quyết định.
    pub state_collapses: bool,
}

/// Dưới mức này thì nhà nước không còn cưỡng chế được gì.
pub const NGUONG_SUP_DO: u32 = 200;

/// Ứng phó một thảm họa (`§12.20`).
///
/// **Không có tham số `should_collapse`.** Kết cục rơi ra từ cường độ, năng
/// lực và tính chính danh — đó là điều `§12.20` gọi là *"hệ quả tính ra được,
/// không phải một quyết định của Director"*.
pub fn respond(m: Magnitude, cap: &Capacity, leg: &Legitimacy, state_strength: u16) -> Aftermath {
    let cuong_do = i64::from(m.0);

    // Cảnh báo chỉ có tác dụng nếu người ta **nghe theo**. Đây là chỗ chính
    // danh vào phép tính, và là chỗ hai xã hội cùng có còi báo động cho hai
    // kết quả khác nhau.
    //
    // Dùng `compliance()` chứ không dùng riêng `belief`: trong một cuộc sơ tán,
    // tuân vì sợ và tuân vì thấy hàng xóm đi cũng làm người ta rời khỏi nhà.
    // `state_strength` là năng lực cưỡng chế **lúc thảm họa xảy ra**. Nó vào
    // đây vì một trận động đất phá luôn khả năng đi bắt người: đường sập, lính
    // cũng là nạn nhân. Đó là lý do một chế độ dựa trên sợ hãi mất tuân thủ
    // nhanh hơn một chế độ được tin.
    let tuan = i64::from(leg.compliance(state_strength).total);
    let canh_bao_hieu_luc = i64::from(cap.warning) * tuan / 1_000;
    let so_tan_hieu_luc = i64::from(cap.evacuation) * tuan / 1_000;

    let giam_thuong_vong = (canh_bao_hieu_luc + so_tan_hieu_luc + i64::from(cap.rescue)) / 3;
    let thuong_vong = ((cuong_do - giam_thuong_vong).max(0) * 1_000 / 1_000).min(1_000);

    // Mất nhà thì phải đi, trừ khi có nơi trú và kho dự phòng.
    let cho_o = i64::from(cap.shelter) + i64::from(cap.stockpile);
    let phai_roi = (cuong_do * 2 - cho_o).clamp(0, 1_000);

    // Tái thiết: năng lực càng thấp càng lâu, và nó tăng phi tuyến.
    let nam_tai_thiet = if cap.reconstruction == 0 {
        99
    } else {
        u32::try_from(cuong_do * 100 / i64::from(cap.reconstruction)).unwrap_or(99)
    };

    // Tuân thủ trả giá cho thương vong và cho việc không tái thiết nổi.
    let mat = u16::try_from(thuong_vong / 2).unwrap_or(u16::MAX)
        + u16::try_from(nam_tai_thiet.min(200)).unwrap_or(200);
    let sau = u16::try_from(tuan).unwrap_or(1_000).saturating_sub(mat);

    Aftermath {
        casualties_permille: u32::try_from(thuong_vong).unwrap_or(1_000),
        displaced_permille: u32::try_from(phai_roi).unwrap_or(1_000),
        rebuild_years: nam_tai_thiet,
        compliance_after: sau,
        state_collapses: u32::from(sau) < NGUONG_SUP_DO,
    }
}

/// Người phải rời đi sau thảm họa trở thành áp lực di cư (`§12.19` ↔ `§12.20`).
///
/// Nối hai nửa của module: một trận động đất không chỉ giết người, nó **đẩy
/// người đi** — và những người đó mang theo một `Belief` đã bị thảm họa làm
/// méo, nên họ đi cả khi nơi đến không thật sự tốt hơn.
pub fn displacement_belief(a: &Aftermath, base: &Belief) -> Belief {
    Belief {
        // Nơi này vừa sập: niềm tin vào an toàn ở đây rơi theo thương vong.
        safety_here: base
            .safety_here
            .saturating_sub(a.casualties_permille.saturating_mul(2)),
        // Và tiền công ở đây rơi theo mức tàn phá.
        wage_here: base.wage_here.saturating_sub(a.displaced_permille / 2),
        ..*base
    }
}

/// Ai đi đâu sau một thảm họa — dòng người theo **mạng lưới có sẵn**.
///
/// `diasporas` là những cộng đồng ly tán đã có. Người ta không tỏa đều tới mọi
/// nơi an toàn; họ đi tới nơi có người quen. Đó là lý do một trận động đất ở
/// một tỉnh làm đông thêm đúng vài khu phố ở vài thành phố cụ thể.
pub fn flows(displaced: u64, diasporas: &[Diaspora]) -> BTreeMap<String, u64> {
    let don_duoc: Vec<&Diaspora> = diasporas.iter().filter(|d| d.eases_arrival()).collect();
    if don_duoc.is_empty() {
        return BTreeMap::new();
    }
    let tong: u64 = don_duoc.iter().map(|d| d.population).sum();
    if tong == 0 {
        return BTreeMap::new();
    }
    don_duoc
        .iter()
        .map(|d| (d.host.clone(), displaced * d.population / tong))
        .collect()
}
