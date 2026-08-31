//! Băm xác định từ hạt giống và tọa độ.
//!
//! Vì sao không dùng `rand`: một `Rng` có trạng thái nội tại, nên kết quả phụ
//! thuộc vào *thứ tự* các lần rút. Chỉ cần một lần thêm một phép thử vị trí ở
//! giữa thuật toán là cả ngôi làng đổi hình, kể cả khi phần còn lại không sửa
//! gì. Băm thuần từ `(seed, salt, x, y)` thì mỗi quyết định độc lập với mọi
//! quyết định khác: sửa cách chọn cửa không làm xê dịch luống ruộng.

/// Bộ trộn cuối của `splitmix64`.
///
/// Chọn nó vì nó là hàm nguyên thuần, không bảng tra, không phụ thuộc kiến
/// trúc — nên cùng một `seed` cho cùng một byte trên mọi máy chạy server.
const fn mix64(x: u64) -> u64 {
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Băm một tọa độ: `(seed, salt, x, y)` cho một `u64`.
///
/// `x` và `y` vào theo bit (`as u64`) chứ không qua giá trị tuyệt đối, để hai
/// ô đối xứng qua gốc không nhận cùng một số.
pub(crate) fn hash_xy(seed: u64, salt: u64, x: i64, y: i64) -> u64 {
    let mut h = mix64(seed ^ 0x9e37_79b9_7f4a_7c15);
    h = mix64(h ^ salt.wrapping_mul(0xbf58_476d_1ce4_e5b9));
    h = mix64(h ^ (x as u64).wrapping_mul(0x94d0_49bb_1331_11eb));
    mix64(h ^ (y as u64).wrapping_mul(0xd6e8_feb8_6659_fd93))
}

/// Băm một chỉ số: dùng cho những quyết định không gắn với ô nào cả (số nhà,
/// số cư dân, tên).
pub(crate) fn hash_i(seed: u64, salt: u64, i: u64) -> u64 {
    hash_xy(seed, salt, i as i64, -1)
}

/// Rút một số trong `[0, n)`.
///
/// Có lệch modulo, nhưng `n` ở đây luôn là hằng nhỏ (2..48) còn `h` là 64 bit,
/// nên độ lệch nằm dưới `2^-58` — không thứ gì trong một ngôi làng nhìn thấy
/// được sai số đó.
pub(crate) fn pick(h: u64, n: u64) -> u64 {
    debug_assert!(n > 0, "pick khong nhan n = 0");
    h % n.max(1)
}

/// Muối băm: mỗi quyết định một hằng riêng.
///
/// Tách muối là thứ giữ cho các quyết định độc lập — nếu cửa nhà và ống khói
/// cùng đọc `hash_xy(seed, x, y)` thì chúng sẽ tương quan với nhau và mọi ngôi
/// nhà trong làng sẽ có cửa nằm cùng phía với ống khói.
pub(crate) mod salt {
    /// Chọn số nhà của làng.
    pub(crate) const HOUSE_COUNT: u64 = 0x5011;
    /// Chọn số thửa ruộng.
    pub(crate) const FIELD_COUNT: u64 = 0x5012;
    /// Chọn số cư dân.
    pub(crate) const FOLK_COUNT: u64 = 0x5013;
    /// Chọn tên cư dân.
    pub(crate) const NAME: u64 = 0x5014;
    /// Chọn vai trò cho phần cư dân ngoài bộ khung bắt buộc.
    pub(crate) const ROLE: u64 = 0x5015;
    /// Xoay vòng gán nhà ở cho cư dân.
    pub(crate) const HOME: u64 = 0x5016;
    /// Lệch điểm bắt đầu khi đi vòng một vành đai để lấy chỗ đặt công trình.
    pub(crate) const SLOT: u64 = 0x5017;
    /// Chọn cột cửa ở mép dưới công trình.
    pub(crate) const DOOR: u64 = 0x5018;
    /// Chọn cột ống khói.
    pub(crate) const CHIMNEY: u64 = 0x5019;
    /// Chọn kích thước và hướng luống của một thửa ruộng.
    pub(crate) const FIELD_SHAPE: u64 = 0x501a;
    /// Răng cưa mép đường và mép quảng trường.
    pub(crate) const DITHER: u64 = 0x501b;
}
