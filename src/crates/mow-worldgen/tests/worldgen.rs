//! Test worldgen.
//!
//! Bài quan trọng nhất là [`song_va_nui_khong_dut_o_bien_chunk`] — `PA-02` tồn
//! tại vì lỗi đó, và nó là loại lỗi chỉ lộ ra khi người chơi đi dọc một con
//! sông, tức là rất muộn.

use mow_math::{CanonicalHash, Fx, WorldSeed};
use mow_worldgen::profile::{EdgePolicy, Topology};
use mow_worldgen::{Biome, GenError, GenerationProfile, Worldgen};
use proptest::prelude::*;

fn wg(seed: u64) -> Worldgen {
    Worldgen::new(WorldSeed(seed), GenerationProfile::default())
}

// ─────────────────────────────────────────────────────────────────────────────
// §7.2 — sinh lười và xác định
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cung_toa_do_luon_cho_cung_o() {
    let w = wg(42);
    for (x, y) in [(0, 0), (1_000, -2_000), (i64::MAX / 2, i64::MIN / 2)] {
        let a = w.base_cell(x, y).unwrap();
        let b = w.base_cell(x, y).unwrap();
        assert_eq!(a, b, "tại ({x}, {y})");
    }
}

#[test]
fn thu_tu_mo_chunk_khong_anh_huong_ket_qua() {
    // Đây là bất biến nền của `§7.2`: mở chunk ở tick 10 hay tick 10 triệu đều
    // cho cùng địa hình. Kiểm bằng cách hỏi cùng một tập ô theo hai thứ tự.
    let w = wg(7);
    let toa_do: Vec<(i64, i64)> = (0..40).map(|i| (i * 37, i * -53)).collect();

    let xuoi: Vec<_> = toa_do
        .iter()
        .map(|(x, y)| w.base_cell(*x, *y).unwrap())
        .collect();
    let mut nguoc: Vec<_> = toa_do
        .iter()
        .rev()
        .map(|(x, y)| w.base_cell(*x, *y).unwrap())
        .collect();
    nguoc.reverse();

    assert_eq!(xuoi, nguoc);
}

#[test]
fn seed_khac_thi_the_gioi_khac() {
    let a = wg(1).base_cell(500, 500).unwrap();
    let b = wg(2).base_cell(500, 500).unwrap();
    assert_ne!(a.state_hash(), b.state_hash());
}

#[test]
fn profile_khac_thi_the_gioi_khac_va_hash_noi_ra_ngay() {
    // Hai world cùng seed nhưng khác profile là hai world khác nhau, và điều đó
    // phải nhìn thấy ở tầng hash chứ không phải sau khi đi bộ 4000 ô.
    let p2 = GenerationProfile {
        sea_level_m: 100,
        ..Default::default()
    };
    assert_ne!(
        GenerationProfile::default().snapshot_hash(),
        p2.snapshot_hash()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §7.4 — không đứt ở biên chunk
// ─────────────────────────────────────────────────────────────────────────────

/// **Bài quan trọng nhất của `PA-02`.**
///
/// Đi ngang qua nhiều biên chunk và khẳng định độ cao biến thiên liên tục.
/// Một bậc thang đột ngột ở đúng bội số của kích thước chunk là dấu hiệu kinh
/// điển của việc sinh từng chunk rồi khâu mép.
#[test]
fn song_va_nui_khong_dut_o_bien_chunk() {
    let w = wg(1234);
    const CHUNK: i64 = 32;

    let mut buoc_lon_nhat = 0i64;
    let mut o_dau = 0i64;
    let mut truoc = w.base_cell(-3 * CHUNK, 0).unwrap().elevation.height_m;

    for x in (-3 * CHUNK + 1)..=(3 * CHUNK) {
        let h = w.base_cell(x, 0).unwrap().elevation.height_m;
        let buoc = (h - truoc).abs();
        if buoc > buoc_lon_nhat {
            buoc_lon_nhat = buoc;
            o_dau = x;
        }
        truoc = h;
    }

    // Địa hình tự nhiên có vách đá, nên không đòi bước nhỏ tuyệt đối. Điều
    // đáng đòi là: bước lớn nhất **không rơi vào đúng biên chunk**.
    assert!(
        o_dau.rem_euclid(CHUNK) != 0 || buoc_lon_nhat < 40,
        "bước độ cao lớn nhất ({buoc_lon_nhat} m) rơi đúng vào biên chunk tại x={o_dau} \
         — dấu hiệu của việc sinh theo chunk rồi khâu mép"
    );
}

#[test]
fn luu_vuc_giong_nhau_o_hai_ben_bien_chunk() {
    // Hai ô kề nhau nằm hai bên biên chunk phải thuộc **cùng lưu vực** và có
    // **cùng outlet**. Đó mới là bất biến; hướng chảy thì không, vì hai ô nằm
    // hai phía của outlet sẽ chảy ngược nhau một cách hoàn toàn đúng đắn.
    use mow_worldgen::hydrology::basin_of;

    let p = GenerationProfile::default();
    const CHUNK: i64 = 32;
    let mut kiem = 0;

    for k in -8..8i64 {
        let x = k * CHUNK;
        let a = basin_of(99, &p, x - 1, 100);
        let b = basin_of(99, &p, x, 100);
        if a.cell_x == b.cell_x && a.cell_y == b.cell_y {
            assert_eq!(
                (a.outlet_x, a.outlet_y),
                (b.outlet_x, b.outlet_y),
                "cùng lưu vực nhưng outlet khác nhau ở biên chunk x={x}"
            );
            kiem += 1;
        }
    }
    assert!(kiem > 0, "không có cặp nào trong cùng lưu vực để kiểm");
}

#[test]
fn luu_vuc_la_ham_thuan_cua_toa_do() {
    // Tính chất khiến biên chunk vô hình: outlet của một lưu vực không phụ
    // thuộc vào việc ai hỏi hay hỏi từ đâu.
    use mow_worldgen::hydrology::basin_of;
    let p = GenerationProfile::default();
    let a = basin_of(7, &p, 5_000, -3_000);
    let b = basin_of(7, &p, 5_001, -3_000);
    if a.cell_x == b.cell_x && a.cell_y == b.cell_y {
        assert_eq!((a.outlet_x, a.outlet_y), (b.outlet_x, b.outlet_y));
    }
}

#[test]
fn nhieu_lay_mau_cung_nut_thi_cho_cung_gia_tri() {
    // Đây là tính chất khiến biên chunk vô hình: giá trị ở một nút lưới là hàm
    // thuần của tọa độ nút, không phụ thuộc ai hỏi.
    use mow_worldgen::noise::lattice;
    let a = lattice(1, "test", 100, 200);
    let b = lattice(1, "test", 100, 200);
    assert_eq!(a, b);
    assert_ne!(a, lattice(1, "test", 101, 200));
    assert_ne!(a, lattice(2, "test", 100, 200));
    assert_ne!(a, lattice(1, "khac", 100, 200));
}

// ─────────────────────────────────────────────────────────────────────────────
// §7.4 — topology
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn the_gioi_co_bien_tu_choi_toa_do_ngoai_bien() {
    let p = GenerationProfile {
        topology: Topology::BoundedBox {
            half_x: 1_000,
            half_y: 1_000,
        },
        ..Default::default()
    };
    let w = Worldgen::new(WorldSeed(1), p);

    w.base_cell(1_000, 1_000).expect("trên biên là hợp lệ");
    assert_eq!(
        w.base_cell(1_001, 0),
        Err(GenError::OutOfBounds { x: 1_001, y: 0 })
    );
}

#[test]
fn chinh_sach_clamp_thi_kep_thay_vi_tu_choi() {
    let p = GenerationProfile {
        topology: Topology::BoundedBox {
            half_x: 100,
            half_y: 100,
        },
        edge_policy: EdgePolicy::Clamp,
        ..Default::default()
    };
    let w = Worldgen::new(WorldSeed(1), p);

    let ngoai = w.base_cell(500, 0).unwrap();
    let tren_bien = w.base_cell(100, 0).unwrap();
    assert_eq!(ngoai, tren_bien);
}

#[test]
fn mat_xuyen_quan_lai_dung_cho() {
    let p = GenerationProfile {
        topology: Topology::ToroidalXy {
            period_x: 4_096,
            period_y: 4_096,
        },
        ..Default::default()
    };
    let w = Worldgen::new(WorldSeed(5), p);

    // Đi hết một vòng phải quay về đúng chỗ cũ — không có đường nối.
    assert_eq!(
        w.base_cell(10, 10).unwrap(),
        w.base_cell(4_106, 10).unwrap()
    );
    assert_eq!(
        w.base_cell(10, 10).unwrap(),
        w.base_cell(10, 4_106).unwrap()
    );
    assert_eq!(w.base_cell(0, 0).unwrap(), w.base_cell(-4_096, 0).unwrap());
}

// ─────────────────────────────────────────────────────────────────────────────
// Chất lượng thế giới sinh ra
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn co_ca_dat_lien_lan_bien() {
    // Một thế giới toàn biển hoặc toàn đất là dấu hiệu thang bị hỏng.
    let w = wg(2026);
    let mut dat = 0;
    let mut nuoc = 0;
    for y in (-6_000..6_000).step_by(400) {
        for x in (-6_000..6_000).step_by(400) {
            if w.base_cell(x, y).unwrap().elevation.submerged {
                nuoc += 1;
            } else {
                dat += 1;
            }
        }
    }
    let tong = dat + nuoc;
    assert!(
        dat * 10 > tong && nuoc * 10 > tong,
        "tỉ lệ đất/nước lệch quá: {dat} đất, {nuoc} nước"
    );
}

#[test]
fn bien_da_dang_khong_don_dieu() {
    // Nếu cả thế giới chỉ có hai biome thì phép phân loại đang bỏ qua một trục
    // đầu vào nào đó.
    let w = wg(555);
    let mut thay = std::collections::BTreeSet::new();
    for y in (-20_000..20_000).step_by(700) {
        for x in (-20_000..20_000).step_by(700) {
            thay.insert(w.base_cell(x, y).unwrap().biome);
        }
    }
    assert!(
        thay.len() >= 6,
        "chỉ sinh ra {} loại biome: {:?}",
        thay.len(),
        thay.iter().map(|b| b.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn len_cao_thi_lanh_di() {
    // Kiểm quan hệ lapse rate **trực tiếp**: cùng một chỗ, hai độ cao khác nhau.
    //
    // Cách khác — đi dọc bản đồ tìm hai ô chênh cao rồi so nhiệt — nghe tự
    // nhiên hơn nhưng kiểm sai thứ: hai ô ở hai chỗ khác nhau còn khác cả tọa
    // độ khí hậu, nên bài test sẽ đo tổng của hai hiệu ứng và trở nên chập chờn.
    use mow_worldgen::climate;
    use mow_worldgen::elevation::Elevation;

    let p = GenerationProfile::default();
    let thap = Elevation {
        height_m: 0,
        slope: 2,
        submerged: false,
    };
    let cao = Elevation {
        height_m: 2_000,
        slope: 2,
        submerged: false,
    };

    let a = climate::sample(11, &p, 1_234, 5_678, &thap);
    let b = climate::sample(11, &p, 1_234, 5_678, &cao);

    assert!(b.temp_mk < a.temp_mk, "lên 2000 m mà không lạnh đi");
    // 2000 m × 6.5 K/km = 13 K = 13 000 mK.
    assert_eq!(a.temp_mk - b.temp_mk, 13_000);
}

#[test]
fn sau_trong_luc_dia_thi_bien_do_mua_lon_hon() {
    // Khí hậu lục địa và khí hậu hải dương phải khác nhau, và khác vì một lý do
    // đọc được chứ không phải vì một bảng tra cứu.
    let w = wg(2_024);
    let mut ven_bien = i64::MAX;
    let mut sau_dat = 0i64;
    for x in (-40_000..40_000).step_by(503) {
        let c = w.base_cell(x, 0).unwrap();
        if c.elevation.submerged {
            continue;
        }
        ven_bien = ven_bien.min(c.climate.seasonal_range_mk);
        sau_dat = sau_dat.max(c.climate.seasonal_range_mk);
    }
    assert!(
        sau_dat > ven_bien,
        "biên độ mùa không đổi theo độ lục địa: {ven_bien}..{sau_dat}"
    );
}

#[test]
fn duoi_nuoc_thi_khong_phai_rung() {
    let w = wg(31);
    for y in (-5_000..5_000).step_by(313) {
        for x in (-5_000..5_000).step_by(317) {
            let c = w.base_cell(x, y).unwrap();
            if c.elevation.submerged {
                assert!(
                    matches!(c.biome, Biome::Ocean | Biome::ShallowSea | Biome::Lake),
                    "ô dưới nước tại ({x},{y}) lại là {}",
                    c.biome.as_str()
                );
            }
        }
    }
}

#[test]
fn dat_mat_mong_dan_tren_suon_doc() {
    // Đất bị rửa trôi ở sườn dốc — đây là lý do đỉnh núi trơ đá còn thung lũng
    // thì màu mỡ, và nó ra từ một dòng chứ không cần mô phỏng xói mòn.
    let w = wg(77);
    let mut doc_nhat = (0i64, 0i32);
    let mut phang_nhat = (i64::MAX, 0i32);
    for x in (0..30_000).step_by(97) {
        let c = w.base_cell(x, 0).unwrap();
        if c.elevation.submerged {
            continue;
        }
        if c.elevation.slope > doc_nhat.0 {
            doc_nhat = (c.elevation.slope, c.strata.soil_depth_m);
        }
        if c.elevation.slope < phang_nhat.0 {
            phang_nhat = (c.elevation.slope, c.strata.soil_depth_m);
        }
    }
    assert!(
        doc_nhat.1 <= phang_nhat.1,
        "sườn dốc (slope {}) có đất dày {} m, chỗ phẳng (slope {}) chỉ {} m",
        doc_nhat.0,
        doc_nhat.1,
        phang_nhat.0,
        phang_nhat.1
    );
}

#[test]
fn khong_co_so_thuc_trong_worldgen() {
    // Cùng kiểm tra như `mow-math`: worldgen là đường commit.
    let goc = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut vi_pham = Vec::new();
    quet(&goc, &mut vi_pham);
    assert!(vi_pham.is_empty(), "{}", vi_pham.join("\n"));
}

fn quet(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(e) = std::fs::read_dir(dir) else {
        return;
    };
    let mut ps: Vec<_> = e.flatten().map(|x| x.path()).collect();
    ps.sort();
    for p in ps {
        if p.is_dir() {
            quet(&p, out);
            continue;
        }
        let Ok(s) = std::fs::read_to_string(&p) else {
            continue;
        };
        for (i, d) in s.lines().enumerate() {
            let t = d.trim_start();
            if t.starts_with("//") || t.starts_with('*') {
                continue;
            }
            for m in ["f32", "f64"] {
                if d.contains(m) {
                    out.push(format!("{}:{}: {}", p.display(), i + 1, d.trim()));
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    /// Không tọa độ nào làm worldgen panic, kể cả ở biên `i64`.
    #[test]
    fn khong_bao_gio_panic(x in any::<i64>(), y in any::<i64>()) {
        let w = wg(3);
        let _ = w.base_cell(x, y);
    }

    /// Các trường suy ra luôn nằm trong miền khai báo.
    #[test]
    fn truong_luon_trong_mien(x in -1_000_000i64..1_000_000, y in -1_000_000i64..1_000_000) {
        let c = wg(4).base_cell(x, y).unwrap();
        prop_assert!(c.elevation.slope >= 0);
        prop_assert!(c.climate.humidity >= Fx::ZERO && c.climate.humidity <= Fx::ONE);
        prop_assert!(c.climate.precipitation_mm_yr >= 0);
        prop_assert!(c.strata.soil_depth_m >= 0);
        prop_assert!((-1..=1).contains(&c.flow.dx));
        prop_assert!((-1..=1).contains(&c.flow.dy));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §7.4 — sông là lòng sông, không phải cả lưu vực
// ─────────────────────────────────────────────────────────────────────────────

/// Cờ đúng ở **mọi** ô là một cờ không mang thông tin nào.
///
/// Bản đầu tính `is_river` chỉ từ nước tích lũy, mà nước tích lũy lại xấp xỉ
/// bằng quãng đường đã đi trong ô lưu vực — nên gần như ô nào cũng vượt ngưỡng.
/// Tầng vẽ trung thành tô lam mọi ô "sông", và cả thế giới hiện ra xanh lét như
/// chìm dưới nước. Không bài test nào bắt được, vì `true` ở mọi nơi vẫn là một
/// giá trị hợp lệ — chỉ có màn hình nói ra.
#[test]
fn song_khong_phu_kin_ban_do() {
    let mut xet = 0_usize;
    for seed in [42_u64, 7, 648_238, 999] {
        let g = wg(seed);
        let mut song = 0_usize;
        let mut tong = 0_usize;
        for y in -60..60 {
            for x in -60..60 {
                let Ok(c) = g.base_cell(x, y) else { continue };
                if c.elevation.submerged {
                    continue;
                }
                tong += 1;
                if c.flow.is_river {
                    song += 1;
                }
            }
        }
        // Một hành tinh đại dương là một thế giới hợp lệ: quanh gốc tọa độ của
        // seed đó không có ô cạn nào, và bài này không có gì để nói về nó.
        if tong == 0 {
            continue;
        }
        xet += 1;
        let ti_le = song * 100 / tong;
        assert!(
            ti_le < 25,
            "seed {seed}: {ti_le}% ô cạn là sông — cờ này không còn nói lên điều gì"
        );
    }
    assert!(xet > 0, "không seed nào có đất cạn để xét — bài này đã thành vô nghĩa");
}

/// Nhưng cũng không được **không có** con sông nào ở đâu cả.
///
/// Hai bài này là một cặp có chủ ý: sửa một cờ luôn đúng bằng cách làm nó luôn
/// sai là đổi một lỗi im lặng lấy một lỗi im lặng khác.
#[test]
fn van_con_song_o_dau_do() {
    let g = wg(42);
    let mut song = 0_usize;
    for y in -300..300 {
        for x in -300..300 {
            if g.base_cell(x, y).is_ok_and(|c| c.flow.is_river) {
                song += 1;
            }
        }
    }
    assert!(song > 0, "cả một vùng 600×600 ô mà không có một khúc sông nào");
}
