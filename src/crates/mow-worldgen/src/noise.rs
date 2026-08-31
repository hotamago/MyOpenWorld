//! Nhiễu **số nguyên**, xác định trên mọi nền tảng.
//!
//! Mọi thư viện nhiễu thông dụng đều dùng `f32`/`f64`, và điều đó loại chúng
//! khỏi đây: `plan.md §P10.2` cấm số thực trên đường commit, và địa hình *là*
//! đường commit — nó quyết định thế giới trông thế nào và do đó mọi thứ sau đó.
//!
//! Một `f64` trong worldgen sẽ không hỏng ngay. Nó hỏng khi bạn build trên một
//! máy khác, hoặc khi trình biên dịch quyết định dùng FMA cho một biểu thức,
//! và bỗng nhiên một con sông chảy lệch đi ba ô ở nửa bên kia bản đồ. Không ai
//! truy ra nguyên nhân, vì không có gì trong log nói tới nó.
//!
//! Cách làm ở đây: giá trị nhiễu ở mỗi nút lưới là **hàm băm của tọa độ**, và
//! nội suy giữa các nút là số học Q16.16. Không có trạng thái, không có bảng
//! hoán vị, không có thứ tự khởi tạo — nên không có gì để lệch.

use mow_math::{Fx, StateHasher};

/// Miền băm, để nhiễu của hai hệ thống không tương quan với nhau.
const DOMAIN: &str = "mow.noise.v1";

/// Giá trị nhiễu tại một nút lưới nguyên, trong `[-1, 1]`.
///
/// Băm `(seed, kênh, x, y)` rồi lấy 32 bit đầu. Vì nó là hàm thuần của tọa độ,
/// hai chunk kề nhau lấy mẫu cùng một nút sẽ luôn ra cùng giá trị — đó là toàn
/// bộ lý do biên chunk không có đường nối.
pub fn lattice(seed: u64, channel: &str, x: i64, y: i64) -> Fx {
    let mut h = StateHasher::with_domain(DOMAIN);
    h.write_u64(seed);
    h.write_str(channel);
    h.write_i64(x);
    h.write_i64(y);
    let b = h.finish().0;
    let raw = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    // [0, 2^32) → [-1, 1] ở thang Q16.16, tức [-65536, 65536].
    Fx::from_raw((i64::from(raw) >> 15) - 65_536)
}

/// Đường cong làm mượt `3t² − 2t³`, tính trên Q16.16.
///
/// Nội suy tuyến tính thuần để lại nếp gấp nhìn thấy được ở mỗi nút lưới — địa
/// hình sẽ trông như một tấm vải căng trên đinh. Đường cong này có đạo hàm bằng
/// 0 ở hai đầu nên các ô ghép lại trơn.
fn smooth(t: Fx) -> Fx {
    // 3t² − 2t³ = t²(3 − 2t)
    let t2 = t.mul(t).unwrap_or(Fx::ZERO);
    let ba = Fx::from_int(3).expect("3 biểu diễn được");
    let hai_t = t.scale_int(2).unwrap_or(Fx::ZERO);
    let trong = ba.sub(hai_t).unwrap_or(Fx::ZERO);
    t2.mul(trong).unwrap_or(Fx::ZERO)
}

/// Nội suy tuyến tính giữa hai giá trị Q16.16.
fn lerp(a: Fx, b: Fx, t: Fx) -> Fx {
    let d = b.sub(a).unwrap_or(Fx::ZERO);
    a.add(d.mul(t).unwrap_or(Fx::ZERO)).unwrap_or(a)
}

/// Nhiễu giá trị nội suy, ô lưới cạnh `cell`.
///
/// `cell` là bước lưới tính bằng ô thế giới. Lớn hơn cho đặc trưng lớn hơn:
/// `cell = 4096` là hình dạng lục địa, `cell = 32` là gồ ghề cục bộ.
pub fn value(seed: u64, channel: &str, x: i64, y: i64, cell: i64) -> Fx {
    debug_assert!(cell > 0, "bước lưới phải dương");

    // `div_euclid`/`rem_euclid` chứ không phải `/` và `%`: với `x = -1`, phép
    // chia cắt-về-0 cho ô 0, nên ô -1 và ô 0 rơi vào cùng ô lưới còn ô -cell
    // thì không. Lưới sẽ lệch đúng một ô quanh gốc, và mọi bài test seam sẽ
    // chạy qua mà không thấy gì.
    let gx = x.div_euclid(cell);
    let gy = y.div_euclid(cell);
    let fx_ = x.rem_euclid(cell);
    let fy = y.rem_euclid(cell);

    let tx = smooth(Fx::from_frac(fx_, cell).unwrap_or(Fx::ZERO));
    let ty = smooth(Fx::from_frac(fy, cell).unwrap_or(Fx::ZERO));

    let n00 = lattice(seed, channel, gx, gy);
    let n10 = lattice(seed, channel, gx + 1, gy);
    let n01 = lattice(seed, channel, gx, gy + 1);
    let n11 = lattice(seed, channel, gx + 1, gy + 1);

    lerp(lerp(n00, n10, tx), lerp(n01, n11, tx), ty)
}

/// Một tầng nhiễu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Octave {
    /// Bước lưới, tính bằng ô thế giới.
    pub cell: i64,
    /// Biên độ, Q16.16.
    pub amplitude: Fx,
}

/// Tổng nhiều tầng nhiễu (`§7.3` bước 3: continental → mountain → hill → detail).
///
/// Trả về **tổng chưa chuẩn hóa**; chỗ gọi tự quyết định thang. Chuẩn hóa ngầm
/// ở đây sẽ làm việc thêm một tầng đổi kết quả của mọi tầng khác, và một thay
/// đổi tưởng như cục bộ sẽ viết lại toàn bộ địa hình.
pub fn fbm(seed: u64, channel: &str, x: i64, y: i64, octaves: &[Octave]) -> Fx {
    let mut tong = Fx::ZERO;
    for (i, o) in octaves.iter().enumerate() {
        // Mỗi tầng dùng một kênh riêng: nếu dùng chung, hai tầng có bước lưới
        // bội số của nhau sẽ lấy mẫu **cùng những nút** và tạo ra hoa văn lưới
        // rõ rệt thay vì nhiễu.
        let kenh = format!("{channel}.{i}");
        let n = value(seed, &kenh, x, y, o.cell.max(1));
        tong = tong
            .add(n.mul(o.amplitude).unwrap_or(Fx::ZERO))
            .unwrap_or(tong);
    }
    tong
}

/// Nhiễu dạng gờ, cho dãy núi (`§7.3` bước 2–3).
///
/// `1 − |n|` biến các đường không của nhiễu thành các sống núi liên tục, thay
/// vì những đỉnh tròn rời rạc. Đây là cách rẻ nhất để có dãy núi *có hướng*
/// thay vì một đám gò.
pub fn ridged(seed: u64, channel: &str, x: i64, y: i64, cell: i64) -> Fx {
    let n = value(seed, channel, x, y, cell);
    let abs = n.abs().unwrap_or(Fx::ZERO);
    Fx::ONE.sub(abs).unwrap_or(Fx::ZERO)
}
