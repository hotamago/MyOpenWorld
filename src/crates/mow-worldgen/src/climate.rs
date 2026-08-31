//! Buoc 5 — khi hau (`§7.3`).

use crate::elevation::Elevation;
use crate::macro_fields::{climate_coord, continentality};
use crate::noise::value;
use crate::profile::GenerationProfile;
use mow_math::{CanonicalHash, Fx, StateHasher};

/// Khi hau tai mot o.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Climate {
    /// Nhiet do trung binh nam, mK.
    pub temp_mk: i64,
    /// Bien do nhiet giua mua nong va mua lanh, mK.
    ///
    /// Sau trong luc dia thi bien do lon; gan bien thi nho. Day la co che tao
    /// ra "khi hau luc dia" va "khi hau hai duong" ma khong can mo phong gi.
    pub seasonal_range_mk: i64,
    /// Luong mua, mm moi nam.
    pub precipitation_mm_yr: i32,
    /// Do am tuong doi, Q16.16 trong `[0,1]`.
    pub humidity: Fx,
}

impl CanonicalHash for Climate {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.temp_mk);
        h.write_i64(self.seasonal_range_mk);
        h.write_i64(i64::from(self.precipitation_mm_yr));
        h.write_i64(self.humidity.raw());
    }
}

/// Lay mau khi hau.
pub fn sample(seed: u64, p: &GenerationProfile, x: i64, y: i64, elev: &Elevation) -> Climate {
    let c = climate_coord(seed, p, x, y);
    let cont = continentality(seed, p, x, y);

    // Nhiet nen theo dai khi hau. |c| chu khong phai c: hai phia cua dai am
    // deu lanh dan khi di xa, giong nhu hai ban cau.
    let xa_dai_am = c.abs().unwrap_or(Fx::ZERO);
    let giam_theo_dai = xa_dai_am.scale_int(45_000).unwrap_or(Fx::ZERO).round_int();

    // Giam theo do cao. Chi tinh phan tren muc bien: duoi nuoc thi nhiet do
    // khong theo lapse rate cua khi quyen.
    let cao_m = (elev.height_m - p.sea_level_m).max(0);
    let giam_theo_cao = (cao_m * p.lapse_rate_mk_per_km) / 1_000;

    let temp_mk = p.base_temp_mk - giam_theo_dai - giam_theo_cao;

    // Bien do mua: sau trong luc dia thi lon.
    let seasonal_range_mk = cont.scale_int(28_000).unwrap_or(Fx::ZERO).round_int() + 4_000;

    // Mua: nhieu o gan bien, it o sau luc dia va o noi lanh (khong khi lanh
    // giu duoc it hoi nuoc — day la ly do sa mac vung cuc ton tai).
    let gan_bien = cont.complement_unit();
    let he_so_nhiet =
        Fx::from_frac((temp_mk - 240_000).clamp(0, 60_000), 60_000).unwrap_or(Fx::ZERO);
    let nhieu_mua =
        Fx::from_raw(value(seed, "climate.precip", x, y, p.climate_cell.max(1) / 4).raw() / 3);

    let mua = gan_bien
        .and(mow_math::Unit::saturating(he_so_nhiet))
        .get()
        .add(nhieu_mua)
        .unwrap_or(Fx::ZERO)
        .clamp(Fx::ZERO, Fx::ONE);

    let precipitation_mm_yr = mua.scale_int(3_200).unwrap_or(Fx::ZERO).round_int() as i32;

    Climate {
        temp_mk,
        seasonal_range_mk,
        precipitation_mm_yr,
        humidity: mua,
    }
}

/// Tien ich: `1 - v` tren mien `[0,1]`.
trait ComplementUnit {
    fn complement_unit(self) -> mow_math::Unit;
}

impl ComplementUnit for Fx {
    fn complement_unit(self) -> mow_math::Unit {
        mow_math::Unit::saturating(self).complement()
    }
}
