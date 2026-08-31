//! Buoc 7 — bien (`§7.3`).
//!
//! Phan loai tu nhiet, am, do cao, nuoc va dat. **Khong** tu mot bang tra cuu
//! ngau nhien: bien phai la HE QUA cua khi hau, vi nguoi choi se doc no nhu mot
//! manh moi. Mot khu rung mua nam canh sa mac ma khong co ly do se pha long tin
//! rang the gioi nay co quy luat.

#![allow(clippy::many_single_char_names)]
use crate::climate::Climate;
use crate::elevation::Elevation;
use crate::hydrology::Flow;
use crate::strata::{Material, Strata};
use mow_math::{CanonicalHash, StateHasher};

/// Quan xa sinh vat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Biome {
    /// Bien sau.
    Ocean,
    /// Bien nong ven bo.
    ShallowSea,
    /// Ho.
    Lake,
    /// Song.
    River,
    /// Bo bien cat.
    Beach,
    /// Bang gia vinh cuu.
    Glacier,
    /// Dai nguyen.
    Tundra,
    /// Rung la kim.
    BorealForest,
    /// Rung on doi.
    TemperateForest,
    /// Dong co on doi.
    Grassland,
    /// Thao nguyen kho.
    Steppe,
    /// Sa mac.
    Desert,
    /// Xa van.
    Savanna,
    /// Rung mua nhiet doi.
    Rainforest,
    /// Dam lay.
    Wetland,
    /// Nui da tro.
    Alpine,
}

impl Biome {
    /// Ten on dinh.
    pub fn as_str(self) -> &'static str {
        match self {
            Biome::Ocean => "ocean",
            Biome::ShallowSea => "shallow_sea",
            Biome::Lake => "lake",
            Biome::River => "river",
            Biome::Beach => "beach",
            Biome::Glacier => "glacier",
            Biome::Tundra => "tundra",
            Biome::BorealForest => "boreal_forest",
            Biome::TemperateForest => "temperate_forest",
            Biome::Grassland => "grassland",
            Biome::Steppe => "steppe",
            Biome::Desert => "desert",
            Biome::Savanna => "savanna",
            Biome::Rainforest => "rainforest",
            Biome::Wetland => "wetland",
            Biome::Alpine => "alpine",
        }
    }

    /// Toan bo bien, theo thu tu on dinh. Dung de sinh bang mau va kiem CI.
    pub const ALL: &'static [Biome] = &[
        Biome::Ocean,
        Biome::ShallowSea,
        Biome::Lake,
        Biome::River,
        Biome::Beach,
        Biome::Glacier,
        Biome::Tundra,
        Biome::BorealForest,
        Biome::TemperateForest,
        Biome::Grassland,
        Biome::Steppe,
        Biome::Desert,
        Biome::Savanna,
        Biome::Rainforest,
        Biome::Wetland,
        Biome::Alpine,
    ];
}

impl CanonicalHash for Biome {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

/// Nguong nhiet, mK.
const BANG: i64 = 271_000;
const LANH: i64 = 278_000;
const MAT: i64 = 285_000;
const AM: i64 = 295_000;

/// Phan loai.
pub fn classify(e: &Elevation, c: &Climate, f: &Flow, s: &Strata) -> Biome {
    // Nuoc truoc: mot o duoi nuoc thi khong con la rung hay sa mac nua.
    if f.is_water_body {
        // Nong hay sau tinh theo do cao so voi day, khong theo mot hang so
        // tuyet doi — mot cai ho tren nui van la ho.
        return if e.height_m > -80 {
            Biome::ShallowSea
        } else {
            Biome::Ocean
        };
    }
    if f.is_river {
        return Biome::River;
    }

    // Bang: lanh hon diem dong quanh nam.
    if c.temp_mk < BANG {
        return Biome::Glacier;
    }

    // Nui cao tro da: khong phai vi lanh ma vi khong con dat.
    if e.slope > 90 && s.surface == Material::Igneous {
        return Biome::Alpine;
    }

    // Bo bien cat.
    if s.surface == Material::Sand && e.height_m < 12 {
        return Biome::Beach;
    }

    let mua = c.precipitation_mm_yr;

    // Dam lay: nhieu nuoc ma dia hinh bang phang nen nuoc khong thoat.
    if mua > 1_400 && e.slope < 4 {
        return Biome::Wetland;
    }

    match (c.temp_mk, mua) {
        (t, _) if t < LANH => Biome::Tundra,
        (t, m) if t < MAT && m > 500 => Biome::BorealForest,
        (t, _) if t < MAT => Biome::Steppe,
        (t, m) if t < AM && m > 900 => Biome::TemperateForest,
        (t, m) if t < AM && m > 350 => Biome::Grassland,
        (t, _) if t < AM => Biome::Steppe,
        (_, m) if m > 1_800 => Biome::Rainforest,
        (_, m) if m > 700 => Biome::Savanna,
        (_, m) if m > 250 => Biome::Grassland,
        _ => Biome::Desert,
    }
}
