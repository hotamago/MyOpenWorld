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
