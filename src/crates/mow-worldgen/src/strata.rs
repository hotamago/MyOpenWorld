//! Buoc 6 — dia tang (`§7.3`).

use crate::elevation::Elevation;
use crate::noise::value;
use crate::profile::GenerationProfile;
use mow_math::{CanonicalHash, Fx, StateHasher};

/// Vat lieu cua mot lop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Material {
    /// Khong khi.
    Air,
    /// Nuoc.
    Water,
    /// Bang.
    Ice,
    /// Cat.
    Sand,
    /// Dat mat, co chat huu co.
    Topsoil,
    /// Set.
    Clay,
    /// Da tram tich.
    Sedimentary,
    /// Da bien chat.
    Metamorphic,
    /// Da macma.
    Igneous,
    /// Quang.
    Ore,
    /// Magma.
    Magma,
}

impl Material {
    /// Ten on dinh, dung trong content pack va tren duong truyen.
    pub fn as_str(self) -> &'static str {
        match self {
            Material::Air => "air",
            Material::Water => "water",
            Material::Ice => "ice",
            Material::Sand => "sand",
            Material::Topsoil => "topsoil",
            Material::Clay => "clay",
            Material::Sedimentary => "sedimentary",
            Material::Metamorphic => "metamorphic",
            Material::Igneous => "igneous",
            Material::Ore => "ore",
            Material::Magma => "magma",
        }
    }
}

impl CanonicalHash for Material {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

/// Cac lop theo chieu sau.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strata {
    /// Vat lieu ngay tren be mat.
    pub surface: Material,
    /// Do day lop dat mat, met.
    pub soil_depth_m: i32,
    /// Do sau toi lop da goc, met.
    pub bedrock_depth_m: i32,
    /// Co quang trong pham vi dao duoc khong.
    pub ore_present: bool,
    /// Co hang dong khong.
    pub cave: bool,
}

impl CanonicalHash for Strata {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.surface.canonical_hash(h);
        h.write_i64(i64::from(self.soil_depth_m));
        h.write_i64(i64::from(self.bedrock_depth_m));
        h.write_bool(self.ore_present);
        h.write_bool(self.cave);
    }
}

/// Lay mau dia tang.
pub fn sample(seed: u64, p: &GenerationProfile, x: i64, y: i64, elev: &Elevation) -> Strata {
    // Dat mat mong dan tren suon doc: no bi rua troi. Day la ly do dinh nui
    // tro da con thung lung thi mau mo, va no ra tu mot dong chu khong can mot
    // mo phong xoi mon.
    let day_dat = (12 - elev.slope / 8).clamp(0, 12) as i32;

    let surface = if elev.submerged {
        Material::Water
    } else if elev.slope > 90 {
        Material::Igneous
    } else if day_dat == 0 {
        Material::Sedimentary
    } else {
        Material::Topsoil
    };

    // Quang: hiem, va tap trung theo dai. Nguong cao nen `ore_present` dung la
    // mot phat hien dang gia chu khong phai chuyen thuong ngay.
    let q = value(seed, "strata.ore", x, y, 256);
    let ore_present = q > Fx::from_frac(72, 100).unwrap_or(Fx::ONE);

    // Hang: chi trong da, khong o duoi lop bun day bien.
    let hang = value(seed, "strata.cave", x, y, 96);
    let cave = !elev.submerged && hang > Fx::from_frac(60, 100).unwrap_or(Fx::ONE);

    let _ = p;
    Strata {
        surface,
        soil_depth_m: day_dat,
        bedrock_depth_m: day_dat + 8,
        ore_present,
        cave,
    }
}

/// Vật liệu tại một ô 3D `(x, y, z)`, với `z` là độ cao tính bằng mét.
///
/// Đây là hàm mà **màn hình** cần: `§18.1` vẽ theo lát `z`, nên câu hỏi
/// "ô này là gì" phải trả lời được cho mọi `z`, không chỉ cho bề mặt.
///
/// Nó ở đây chứ không ở phía renderer vì nó là một luật của thế giới, không
/// phải một lựa chọn hiển thị. Hai chỗ tự suy ra tầng đất sẽ lệch nhau, và khi
/// lệch thì người chơi đào xuyên qua một lớp đá mà bản đồ vẽ là đất.
///
/// Quy ước: `z == height_m` là **ô bề mặt** — ô cuối cùng còn là chất rắn.
/// `z > height_m` là không khí, hoặc nước nếu ô nằm dưới mực biển.
#[must_use]
pub fn material_at(elev: &Elevation, s: &Strata, sea_level_m: i64, z: i64) -> Material {
    if z > elev.height_m {
        // Trên mặt đất nhưng dưới mực biển: cột nước.
        return if z <= sea_level_m {
            Material::Water
        } else {
            Material::Air
        };
    }

    // `depth` 0 nghĩa là chính ô bề mặt.
    let depth = elev.height_m - z;

    // Hang nằm trong đá, dưới lớp đất. Nó là **khoảng rỗng**, nên nó phải trả
    // về `Air` chứ không phải một loại đá "rỗng" — người chơi rơi xuống hang,
    // và một cái hang bằng đá thì không phải hang.
    if s.cave
        && depth > i64::from(s.bedrock_depth_m) + 4
        && depth < i64::from(s.bedrock_depth_m) + 9
    {
        return Material::Air;
    }

    if depth < i64::from(s.soil_depth_m) {
        return s.surface;
    }
    if depth < i64::from(s.bedrock_depth_m) {
        // Dưới lớp đất mặt là trầm tích, trừ nơi đất đã bị rửa trôi hết —
        // ở đó `surface` vốn đã là đá.
        return Material::Sedimentary;
    }

    // Quặng nằm thành dải trong đá gốc, không rải đều: chỉ một khoảng độ sâu
    // hẹp mới có. Không có ràng buộc đó thì "tìm thấy quặng" mất hết ý nghĩa.
    if s.ore_present
        && depth >= i64::from(s.bedrock_depth_m) + 12
        && depth < i64::from(s.bedrock_depth_m) + 20
    {
        return Material::Ore;
    }
    if depth > i64::from(s.bedrock_depth_m) + 900 {
        return Material::Magma;
    }
    Material::Igneous
}

#[cfg(test)]
mod tests_material_at {
    use super::*;

    fn nen() -> (Elevation, Strata) {
        let e = Elevation {
            height_m: 100,
            slope: 4,
            submerged: false,
        };
        let s = Strata {
            surface: Material::Topsoil,
            soil_depth_m: 3,
            bedrock_depth_m: 11,
            ore_present: false,
            cave: false,
        };
        (e, s)
    }

    #[test]
    fn tren_be_mat_la_khong_khi() {
        let (e, s) = nen();
        assert_eq!(material_at(&e, &s, 0, 101), Material::Air);
        assert_eq!(material_at(&e, &s, 0, 500), Material::Air);
    }

    #[test]
    fn o_be_mat_la_vat_lieu_be_mat() {
        // `z == height_m` phải là ô rắn cuối cùng. Lệch một ô ở đây nghĩa là
        // nhân vật đứng lơ lửng, hoặc lún nửa người xuống đất.
        let (e, s) = nen();
        assert_eq!(material_at(&e, &s, 0, 100), Material::Topsoil);
    }

    #[test]
    fn duoi_lop_dat_la_tram_tich_roi_toi_da_goc() {
        let (e, s) = nen();
        assert_eq!(material_at(&e, &s, 0, 98), Material::Topsoil);
        assert_eq!(material_at(&e, &s, 0, 96), Material::Sedimentary);
        assert_eq!(material_at(&e, &s, 0, 80), Material::Igneous);
    }

    #[test]
    fn duoi_muc_bien_thi_cot_tren_la_nuoc() {
        let e = Elevation {
            height_m: -20,
            slope: 1,
            submerged: true,
        };
        let s = Strata {
            surface: Material::Water,
            soil_depth_m: 2,
            bedrock_depth_m: 10,
            ore_present: false,
            cave: false,
        };
        assert_eq!(material_at(&e, &s, 0, -5), Material::Water);
        assert_eq!(material_at(&e, &s, 0, 0), Material::Water);
        // Trên mực biển vẫn là không khí.
        assert_eq!(material_at(&e, &s, 0, 1), Material::Air);
    }

    #[test]
    fn hang_la_khoang_rong_chu_khong_phai_mot_loai_da() {
        let (e, mut s) = nen();
        s.cave = true;
        let z = e.height_m - (i64::from(s.bedrock_depth_m) + 6);
        assert_eq!(material_at(&e, &s, 0, z), Material::Air);
    }

    #[test]
    fn quang_chi_o_mot_dai_do_sau() {
        let (e, mut s) = nen();
        s.ore_present = true;
        let trong_dai = e.height_m - (i64::from(s.bedrock_depth_m) + 15);
        let ngoai_dai = e.height_m - (i64::from(s.bedrock_depth_m) + 40);
        assert_eq!(material_at(&e, &s, 0, trong_dai), Material::Ore);
        assert_eq!(material_at(&e, &s, 0, ngoai_dai), Material::Igneous);
    }

    #[test]
    fn xac_dinh_tuyet_doi() {
        let (e, s) = nen();
        for z in -50..150 {
            assert_eq!(material_at(&e, &s, 0, z), material_at(&e, &s, 0, z));
        }
    }
}
