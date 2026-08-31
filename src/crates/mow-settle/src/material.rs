//! Định danh vật liệu và thứ tự đè khi hai bút vẽ chạm nhau.
//!
//! Đây là các `id` có thật trong `content/core/blocks/`. Chúng là chuỗi chứ
//! không phải `enum` vì crate này không được biết bảng vật liệu của thế giới —
//! nó chỉ phát ra tên, còn việc tra tên ra khối là việc của `mow-content`.

/// Ngói mái phía đón nắng — nửa trên của mái.
pub const ROOF_LIGHT: &str = "roof_light";
/// Ngói mái phía khuất nắng — nửa dưới của mái.
pub const ROOF_DARK: &str = "roof_dark";
/// Đường làng, quảng trường và ô cửa.
pub const PATH_GRAVEL: &str = "path_gravel";
/// Luống đã cày.
pub const FARMLAND: &str = "farmland";
/// Luống đang có cây.
pub const CROP_GREEN: &str = "crop_green";
/// Viền giếng.
pub const SEDIMENTARY: &str = "sedimentary";
/// Ống khói.
pub const IGNEOUS: &str = "igneous";
/// Lòng giếng.
pub const WATER: &str = "water";
/// Đất chưa động tới — dùng làm nửa còn lại của răng cưa mép đường.
pub const TOPSOIL: &str = "topsoil";

/// Thứ tự đè: số lớn hơn thắng.
///
/// Cần đến nó vì các con đường được phép nhập vào nhau. Khi mép răng cưa của
/// đường này rơi trúng tim của đường kia, ô đó phải là mặt đường chứ không phải
/// vệt đất — nếu để "ai vẽ trước thắng" thì con đường sẽ thủng lỗ chỗ ở đúng
/// những chỗ nó đông đúc nhất.
pub(crate) const PRIO_DITHER: u8 = 1;
/// Tim đường và mặt quảng trường.
pub(crate) const PRIO_PATH: u8 = 2;
/// Luống ruộng.
pub(crate) const PRIO_FIELD: u8 = 3;
/// Viền giếng.
pub(crate) const PRIO_WELL_RIM: u8 = 4;
/// Lòng giếng.
pub(crate) const PRIO_WELL: u8 = 5;
/// Mái, tường, ống khói, ô cửa.
pub(crate) const PRIO_BUILT: u8 = 6;

/// Ô có đứng lên được không.
///
/// Dùng để chọn chỗ xuất hiện cho cư dân: một cư dân sinh ra giữa lòng giếng
/// hoặc trên nóc nhà là thứ người chơi sẽ thấy trong ba giây đầu tiên.
pub(crate) fn stand_on(material: &str) -> bool {
    matches!(material, PATH_GRAVEL | FARMLAND | CROP_GREEN | TOPSOIL)
}
