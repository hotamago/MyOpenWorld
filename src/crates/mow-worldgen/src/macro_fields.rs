//! Trường vĩ mô, lấy mẫu từ lưới phân cấp **liên tục qua biên** (`§7.4`).
//!
//! Ba trường ở đây là nền của mọi thứ khác, và cả ba có cùng một tính chất:
//! chúng được định nghĩa trên một lưới thô hơn nhiều so với ô thế giới, nên hai
//! chunk kề nhau hỏi cùng một câu và nhận cùng một câu trả lời.

use crate::noise::{fbm, value, Octave};
use crate::profile::GenerationProfile;
use mow_math::Fx;

/// Tiềm năng lục địa, Q16.16 trong `[-1, 1]`.
///
/// Dương là đất, âm là biển. Đây là trường quyết định hình dạng lục địa, và
/// vì nó liên tục nên đường bờ biển cũng liên tục — không có bậc thang ở biên
/// chunk.
pub fn continental(seed: u64, p: &GenerationProfile, x: i64, y: i64) -> Fx {
    // Hai tầng: hình dạng lục địa lớn, cộng một tầng nhỏ hơn tạo bán đảo và
    // vịnh. Nhiều hơn hai sẽ làm bờ biển vụn thành quần đảo khắp nơi.
    fbm(
        seed,
        "macro.continental",
        x,
        y,
        &[
            Octave {
                cell: p.continental_cell.max(1),
                amplitude: Fx::ONE,
            },
            Octave {
                cell: (p.continental_cell / 3).max(1),
                amplitude: Fx::from_frac(1, 3).unwrap_or(Fx::ZERO),
            },
        ],
    )
}

/// Tọa độ khí hậu, Q16.16 trong `[-1, 1]`.
///
/// `-1` là cực lạnh, `0` là ấm, `1` là cực nóng. `§7.4` yêu cầu ghi rõ: đây là
/// **trường khí hậu procedural**, không phải vĩ độ thiên văn. Đừng suy ra ngày
/// dài đêm ngắn hay góc mặt trời từ nó — thế giới này là mặt phẳng, không phải
/// hình cầu, và giả vờ ngược lại sẽ dẫn tới một chuỗi hệ quả sai.
pub fn climate_coord(seed: u64, p: &GenerationProfile, x: i64, y: i64) -> Fx {
    // Dải chính chạy theo `y` — cho bản đồ có "phương bắc lạnh" đọc được — cộng
    // một tầng nhiễu để ranh giới không phải là những đường ngang thẳng tắp.
    let cell = p.climate_cell.max(1);
    let dai = Fx::from_frac(y.rem_euclid(cell * 2) - cell, cell).unwrap_or(Fx::ZERO);
    // Nhiễu ở một phần tư biên độ: đủ để bẻ cong ranh giới, không đủ để tạo
    // ra một ốc đảo băng giữa sa mạc.
    let nhieu =
        Fx::from_raw(value(seed, "macro.climate_wobble", x, y, (cell / 2).max(1)).raw() / 4);
    dai.add(nhieu)
        .unwrap_or(dai)
        .clamp(Fx::from_int(-1).unwrap_or(Fx::ZERO), Fx::ONE)
}

/// Ước lượng khoảng cách tới biển, Q16.16 trong `[0, 1]`.
///
/// `0` là sát biển, `1` là sâu trong lục địa. Ước lượng **từ các ô thô lân
/// cận** chứ không phải bằng cách tìm đường tới nước gần nhất — tìm đường sẽ
/// cần biết cả bản đồ, và cả bản đồ thì không tồn tại vì thế giới sinh lười.
pub fn continentality(seed: u64, p: &GenerationProfile, x: i64, y: i64) -> Fx {
    let cell = p.continental_cell.max(1);
    let mut tong = Fx::ZERO;
    let mut n = 0i64;
    // Lấy mẫu một lưới 5×5 ô thô quanh vị trí. Bán kính này đủ để "sâu trong
    // lục địa" có nghĩa mà vẫn rẻ — 25 lần băm, không phải một phép tìm đường.
    for dy in -2..=2 {
        for dx in -2..=2 {
            let c = continental(seed, p, x + dx * cell, y + dy * cell);
            if c > Fx::ZERO {
                tong = tong.add(Fx::ONE).unwrap_or(tong);
            }
            n += 1;
        }
    }
    Fx::from_frac(tong.round_int(), n.max(1)).unwrap_or(Fx::ZERO)
}

/// Cường độ nâng kiến tạo, Q16.16 trong `[0, 1]` (`§7.3` bước 2).
///
/// Cao dọc theo các đường đứt gãy giả lập. Dùng nhiễu dạng gờ nên kết quả là
/// những **dải** liên tục chứ không phải những đốm rời — đó là khác biệt giữa
/// một dãy núi và một đám gò.
pub fn uplift(seed: u64, p: &GenerationProfile, x: i64, y: i64) -> Fx {
    let r = crate::noise::ridged(seed, "macro.uplift", x, y, p.mountain_cell.max(1));
    r.clamp(Fx::ZERO, Fx::ONE)
}
