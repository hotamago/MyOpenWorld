//! Bước 4 — thủy văn phân cấp (`§7.3`, `§7.4`).
//!
//! Đây là chỗ mà nguyên tắc "đặc trưng lớn quyết định ở lưới thô" trả công rõ
//! nhất, và cũng là chỗ dễ làm sai nhất.
//!
//! **Cách sai:** cho mỗi ô nhìn tám hàng xóm rồi chảy xuống ô thấp nhất. Nghe
//! hợp lý và cho kết quả trông ổn *trong một chunk*. Nhưng dòng chảy khi đó là
//! kết quả của một phép duyệt cục bộ, và ở biên chunk hai bên có thể quyết định
//! hai hướng mâu thuẫn — con sông chảy tới mép rồi biến mất, hoặc tệ hơn, chảy
//! ngược lên.
//!
//! **Cách ở đây:** mỗi **ô lưu vực thô** có một outlet xác định, tính bằng một
//! hàm thuần của tọa độ ô đó. Dòng chảy cục bộ hướng về outlet của lưu vực chứa
//! nó. Hai chunk kề nhau nằm trong cùng lưu vực nên hỏi cùng một outlet và nhận
//! cùng một hướng — **không cần chunk nào biết chunk kia tồn tại**.

use crate::climate::Climate;
use crate::elevation::{height_at, Elevation};
use crate::macro_fields::continental;
use crate::profile::GenerationProfile;
use mow_math::{CanonicalHash, Fx, StateHasher};

/// Một lưu vực thô.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Basin {
    /// Tọa độ ô lưu vực.
    pub cell_x: i64,
    /// Tọa độ ô lưu vực.
    pub cell_y: i64,
    /// Ô thế giới nơi lưu vực này thoát nước ra.
    pub outlet_x: i64,
    /// Ô thế giới nơi lưu vực này thoát nước ra.
    pub outlet_y: i64,
    /// Thứ tự thoát nước: 0 là đổ thẳng ra biển, lớn hơn là đổ vào lưu vực khác.
    pub drain_order: u32,
}

/// Hướng và cường độ dòng chảy tại một ô.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flow {
    /// Hướng theo `x`: `-1`, `0` hoặc `1`.
    pub dx: i8,
    /// Hướng theo `y`.
    pub dy: i8,
    /// Lượng nước tích lũy, đơn vị tùy ý nhưng so sánh được.
    pub accumulation: u32,
    /// Ô này có phải lòng sông không.
    pub is_river: bool,
    /// Ô này có phải mặt hồ hoặc biển không.
    pub is_water_body: bool,
}

impl CanonicalHash for Flow {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(i64::from(self.dx));
        h.write_i64(i64::from(self.dy));
        h.write_u64(u64::from(self.accumulation));
        h.write_bool(self.is_river);
        h.write_bool(self.is_water_body);
    }
}

/// Tám hướng lân cận, **theo thứ tự cố định**.
///
/// Ở mức module chứ không ở trong hàm: thứ tự này là một phần của luật, không
/// phải của cách viết vòng lặp. Nếu hai hàng xóm bằng điểm thì cái được xét
/// trước thắng, nên đổi thứ tự ở đây là đổi hình dạng của các lưu vực.
const HUONG: [(i64, i64); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Cạnh của một ô lưu vực, tính bằng ô thế giới.
///
/// Đủ lớn để một lưu vực chứa nhiều chunk — nếu nhỏ hơn chunk thì lợi ích
/// "không đứt ở biên chunk" biến mất.
fn basin_cell(p: &GenerationProfile) -> i64 {
    (p.continental_cell / 4).max(64)
}

/// Khoảng cách lấy mẫu hai bờ, tính bằng ô.
///
/// Một ô là quá gần: nhiễu địa hình ở bước sóng ngắn lấn át chênh lệch thật, và
/// gần như ô nào cũng có lúc trông như đáy một cái rãnh. Hai ô đủ xa để đọc
/// được hình dạng thung lũng mà vẫn đủ gần để lòng sông không phình ra.
const BANK_PROBE: i64 = 2;

/// Chênh cao tối thiểu của bờ so với lòng, tính bằng mét.
const BANK_RISE_M: i64 = 1;

/// Ô này có nằm dưới đáy một rãnh theo hướng chảy không.
///
/// Hướng chảy `(dx, dy)` cho hướng vuông góc `(-dy, dx)`. Nước khoét thành lòng
/// thì **cả hai** bờ phải cao hơn; chỉ một bên cao hơn là một sườn dốc, và nước
/// trên sườn dốc thì chảy tràn chứ không thành sông.
///
/// `(0, 0)` — ô nằm ngay tại outlet — không có hướng nào để xét, và một điểm
/// duy nhất thì không làm nên một con sông.
fn in_channel(seed: u64, p: &GenerationProfile, x: i64, y: i64, dx: i8, dy: i8) -> bool {
    if dx == 0 && dy == 0 {
        return false;
    }
    let (px, py) = (-i64::from(dy), i64::from(dx));
    let here = crate::elevation::height_at(seed, p, x, y);
    let left = crate::elevation::height_at(seed, p, x + px * BANK_PROBE, y + py * BANK_PROBE);
    let right = crate::elevation::height_at(seed, p, x - px * BANK_PROBE, y - py * BANK_PROBE);
    left - here >= BANK_RISE_M && right - here >= BANK_RISE_M
}

/// Lưu vực chứa một ô thế giới.
pub fn basin_of(seed: u64, p: &GenerationProfile, x: i64, y: i64) -> Basin {
    let cell = basin_cell(p);
    let bx = x.div_euclid(cell);
    let by = y.div_euclid(cell);

    // Outlet: tìm ô lưu vực lân cận có "thế nước" thấp nhất, rồi đặt outlet ở
    // biên chung. Xét cả tám hướng, và **duyệt theo thứ tự cố định** — nếu hai
    // hàng xóm bằng điểm, cái nào được xét trước sẽ thắng, và thứ tự đó phải là
    // một phần của luật chứ không phải của cách viết vòng lặp.
    let ta = the_nuoc(seed, p, bx, by);
    let mut tot_nhat = ta;
    let mut huong = (0i64, 0i64);

    for (dx, dy) in HUONG {
        let v = the_nuoc(seed, p, bx + dx, by + dy);
        if v < tot_nhat {
            tot_nhat = v;
            huong = (dx, dy);
        }
    }

    let outlet_x = bx * cell + cell / 2 + huong.0 * (cell / 2);
    let outlet_y = by * cell + cell / 2 + huong.1 * (cell / 2);

    // Thứ tự thoát nước: 0 nếu outlet đã ở dưới mực biển. Không lần theo chuỗi
    // lưu vực — làm thế sẽ cần duyệt không giới hạn và phá tính "hàm thuần của
    // tọa độ". Một xấp xỉ ở đây là đủ, vì thứ tự chỉ dùng để phân biệt sông
    // lớn với sông nhỏ.
    let drain_order = u32::from(the_nuoc(seed, p, bx + huong.0, by + huong.1) > 0);

    Basin {
        cell_x: bx,
        cell_y: by,
        outlet_x,
        outlet_y,
        drain_order,
    }
}

/// "Thế nước" của một ô lưu vực: độ cao trung tâm cộng ưu tiên hướng ra biển.
///
/// Hàm thuần của tọa độ ô lưu vực, nên hai chunk bất kỳ trong cùng lưu vực
/// luôn tính ra cùng một giá trị.
fn the_nuoc(seed: u64, p: &GenerationProfile, bx: i64, by: i64) -> i64 {
    let cell = basin_cell(p);
    let cx = bx * cell + cell / 2;
    let cy = by * cell + cell / 2;
    let h = height_at(seed, p, cx, cy);
    let c = continental(seed, p, cx, cy);
    // Ô ngoài biển có thế rất thấp, nên nước luôn tìm được đường ra.
    if c < Fx::ZERO {
        h - 100_000
    } else {
        h
    }
}

/// Lấy mẫu dòng chảy tại một ô.
pub fn sample(
    seed: u64,
    p: &GenerationProfile,
    x: i64,
    y: i64,
    elev: &Elevation,
    climate: &Climate,
) -> Flow {
    if elev.submerged {
        return Flow {
            dx: 0,
            dy: 0,
            accumulation: u32::MAX,
            is_river: false,
            is_water_body: true,
        };
    }

    let b = basin_of(seed, p, x, y);

    // Hướng: về phía outlet của lưu vực. Chuẩn hóa về `{-1, 0, 1}` bằng dấu.
    let dx = (b.outlet_x - x).signum() as i8;
    let dy = (b.outlet_y - y).signum() as i8;

    // Tích lũy: xấp xỉ bằng khoảng cách đã đi trong lưu vực nhân lượng mưa.
    // Xấp xỉ chứ không phải tích lũy thật, vì tích lũy thật cần duyệt toàn bộ
    // thượng nguồn — thứ không tồn tại trong một thế giới sinh lười.
    let cell = basin_cell(p);
    let da_di = (x - b.cell_x * cell)
        .abs()
        .max((y - b.cell_y * cell).abs())
        .clamp(0, cell);
    let mua = i64::from(climate.precipitation_mm_yr).max(0);
    let accumulation = ((da_di * mua) / 1_000).clamp(0, i64::from(u32::MAX)) as u32;

    // Là sông khi nước tích đủ, địa hình không quá dốc, **và** ô này thật sự
    // nằm dưới đáy một rãnh.
    //
    // Điều kiện thứ ba là điều kiện đã thiếu, và cái thiếu đó không hề im lặng:
    // nó làm `is_river` đúng ở **mọi** ô. `accumulation` ở đây xấp xỉ bằng
    // quãng đường đã đi trong ô lưu vực nhân lượng mưa, nên mọi ô cách góc ô
    // lưu vực đủ xa đều vượt ngưỡng — tức là gần hết bản đồ. Tầng vẽ trung
    // thành tô lam mọi ô "sông", và cả thế giới hiện ra xanh lét như chìm dưới
    // nước. Không một bài test nào bắt được, vì `true` ở mọi nơi vẫn là một giá
    // trị hợp lệ.
    //
    // Đáy rãnh kiểm bằng hai ô **vuông góc** với hướng chảy: nước khoét thành
    // lòng thì hai bờ phải cao hơn. Lấy mẫu cách 2 ô chứ không phải 1, vì ở
    // khoảng cách 1 ô nhiễu địa hình lấn át chênh lệch thật và mọi ô đều có lúc
    // trông như một cái rãnh.
    let is_river = accumulation > 400 && elev.slope < 60 && in_channel(seed, p, x, y, dx, dy);

    Flow {
        dx,
        dy,
        accumulation,
        is_river,
        is_water_body: false,
    }
}
