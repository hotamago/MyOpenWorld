//! Bước 3 — độ cao nhiều tần số (`§7.3`).

use crate::macro_fields::{continental, uplift};
use crate::noise::fbm;
use crate::profile::GenerationProfile;
use mow_math::{CanonicalHash, Fx, StateHasher};

/// Độ cao và các đại lượng suy ra từ nó.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elevation {
    /// Độ cao so với mực biển, mét. Âm là dưới nước.
    pub height_m: i64,
    /// Độ dốc cục bộ, mét trên mỗi ô. Luôn không âm.
    pub slope: i64,
    /// Có nằm dưới mực biển không.
    pub submerged: bool,
}

impl CanonicalHash for Elevation {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.height_m);
        h.write_i64(self.slope);
        h.write_bool(self.submerged);
    }
}

/// Độ cao thô tại một ô, chưa tính độ dốc.
///
/// Tách ra thành hàm riêng vì [`sample`] cần gọi nó cho bốn ô lân cận để tính
/// độ dốc; nếu gộp, mỗi ô sẽ tính lại độ dốc của láng giềng và chi phí thành
/// hàm mũ theo bán kính.
pub fn height_at(seed: u64, p: &GenerationProfile, x: i64, y: i64) -> i64 {
    let cont = continental(seed, p, x, y);

    // Thềm lục địa: đáy biển không phải là địa hình cạn bị dìm xuống. Nhân
    // phần âm với một hệ số lớn hơn cho ta vực sâu ngoài khơi thay vì một cái
    // hồ nông mênh mông.
    let nen_m = if cont >= Fx::ZERO {
        cont.scale_int(600).unwrap_or(Fx::ZERO).round_int()
    } else {
        cont.scale_int(2_400).unwrap_or(Fx::ZERO).round_int()
    };

    // Địa hình chi tiết, chỉ có ý nghĩa trên đất liền.
    let chi_tiet = fbm(seed, "elev", x, y, &p.elevation_octaves()).round_int();

    // Nâng kiến tạo tạo dãy núi, và nó **chỉ tác động ở nơi đã là đất**: một
    // đường đứt gãy dưới đáy biển không nên đội lên thành đảo ở khắp nơi, nếu
    // không bản đồ sẽ đầy quần đảo vô nghĩa.
    let nui_m = if cont > Fx::ZERO {
        let u = uplift(seed, p, x, y);
        // u² làm sườn núi dốc hơn ở gần đỉnh — dãy núi có hình dáng thay vì là
        // một cái nêm tuyến tính.
        u.mul(u)
            .unwrap_or(Fx::ZERO)
            .mul(cont)
            .unwrap_or(Fx::ZERO)
            .scale_int(p.elevation_amplitudes_m[1])
            .unwrap_or(Fx::ZERO)
            .round_int()
    } else {
        0
    };

    p.sea_level_m
        .saturating_add(nen_m)
        .saturating_add(chi_tiet)
        .saturating_add(nui_m)
}

/// Lấy mẫu độ cao đầy đủ.
pub fn sample(seed: u64, p: &GenerationProfile, x: i64, y: i64) -> Elevation {
    let h = height_at(seed, p, x, y);

    // Độ dốc từ hiệu hữu hạn trên bốn hàng xóm. Dùng bốn thay vì tám vì đường
    // chéo có bước dài hơn `1`, và bỏ qua điều đó sẽ làm dốc bị ước lượng cao
    // ở mọi sườn chéo.
    let hx = height_at(seed, p, x + 1, y) - height_at(seed, p, x - 1, y);
    let hy = height_at(seed, p, x, y + 1) - height_at(seed, p, x, y - 1);
    let slope = (hx.abs().max(hy.abs())) / 2;

    Elevation {
        height_m: h,
        slope,
        submerged: h < p.sea_level_m,
    }
}
