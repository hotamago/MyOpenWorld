//! Bộ test của `mow-settle`.
//!
//! Chia làm ba nhóm, theo ba cách bản quy hoạch có thể hỏng:
//!
//! * **Xác định** — cùng đầu vào phải cho cùng từng byte, khác hạt giống phải
//!   cho khác làng. Đây là thứ mà mọi thứ khác trong thế giới dựa vào.
//! * **Nhất quán** — không ô nào hai vật liệu, không công trình nào chồng nhau,
//!   không chỉ số nào trỏ ra ngoài, không ô nào rơi ra khỏi `buildable`.
//! * **Đọc được** — mái chia hai sắc, mỗi nhà đúng một cửa, đường nối được về
//!   quảng trường, ruộng có luống. Đây là những tính chất mà nếu mất thì bản
//!   quy hoạch vẫn "đúng" nhưng ngôi làng không còn ra ngôi làng.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use mow_settle::material::{
    CROP_GREEN, FARMLAND, IGNEOUS, PATH_GRAVEL, ROOF_DARK, ROOF_LIGHT, SEDIMENTARY, WATER,
};
use mow_settle::{plan, Building, BuildingKind, Plan, Role, SettleRequest};

/// Bãi đất trống vô tận: dùng cho mọi test về hình dáng của làng.
fn plain(_x: i64, _y: i64) -> bool {
    true
}

/// Toàn biển.
fn sea(_x: i64, _y: i64) -> bool {
    false
}

/// Một hòn đảo tròn quanh gốc: dùng để kiểm rằng quy hoạch tôn trọng bờ biển.
fn island(x: i64, y: i64) -> bool {
    x * x + y * y <= 26 * 26
}

fn village(seed: u64, radius: i64) -> Plan {
    plan(
        &SettleRequest {
            seed,
            center: (0, 0),
            radius,
        },
        &plain,
    )
}

/// Tập ô mà một công trình chiếm.
fn footprint(b: &Building) -> BTreeSet<(i64, i64)> {
    (0..b.h)
        .flat_map(|dy| (0..b.w).map(move |dx| (b.origin.0 + dx, b.origin.1 + dy)))
        .collect()
}

fn cells_map(p: &Plan) -> BTreeMap<(i64, i64), &'static str> {
    p.cells.iter().map(|&(x, y, m)| ((x, y), m)).collect()
}

fn of_kind(p: &Plan, kind: BuildingKind) -> Vec<&Building> {
    p.buildings.iter().filter(|b| b.kind == kind).collect()
}

#[test]
fn same_input_gives_the_same_plan_fifty_times() {
    let first = village(0xdead_beef, 40);
    for _ in 0..50 {
        assert_eq!(first, village(0xdead_beef, 40));
    }
}

#[test]
fn cells_come_out_sorted_and_unique() {
    // Thứ tự cũng là một phần của "byte-for-byte": hai kế hoạch cùng nội dung
    // nhưng khác thứ tự sẽ băm ra hai giá trị khác nhau ở phía lưu trữ.
    let p = village(11, 40);
    let mut sorted = p.cells.clone();
    sorted.sort_by_key(|&(x, y, _)| (x, y));
    assert_eq!(p.cells, sorted);

    let coords: BTreeSet<(i64, i64)> = p.cells.iter().map(|&(x, y, _)| (x, y)).collect();
    assert_eq!(coords.len(), p.cells.len(), "một ô bị gán hai vật liệu");
}

#[test]
fn different_seeds_give_different_villages() {
    let a = village(1, 40);
    let b = village(2, 40);
    assert_ne!(a, b);
    assert_ne!(a.cells, b.cells);
}

#[test]
fn every_cell_is_buildable() {
    let p = plan(
        &SettleRequest {
            seed: 5,
            center: (0, 0),
            radius: 30,
        },
        &island,
    );
    assert!(!p.cells.is_empty());
    for &(x, y, _) in &p.cells {
        assert!(island(x, y), "ô ({x}, {y}) nằm ngoài vùng xây được");
    }
}

#[test]
fn buildings_never_overlap() {
    let p = village(7, 40);
    let mut claimed: BTreeSet<(i64, i64)> = BTreeSet::new();
    for b in &p.buildings {
        for c in footprint(b) {
            assert!(claimed.insert(c), "hai công trình cùng chiếm ô {c:?}");
        }
    }
}

#[test]
fn roofed_buildings_hold_only_their_own_materials() {
    // Cách chặt chẽ nhất để nói "nhà không bị đường hay ruộng ăn vào": dưới mái
    // chỉ được có mái, ống khói và đúng một ô cửa.
    let p = village(3, 40);
    let map = cells_map(&p);
    for b in &p.buildings {
        match b.kind {
            BuildingKind::House | BuildingKind::Workshop | BuildingKind::Granary => {
                for c in footprint(b) {
                    let m = map.get(&c).copied().expect("ô của công trình chưa được tô");
                    assert!(
                        matches!(m, ROOF_LIGHT | ROOF_DARK | IGNEOUS | PATH_GRAVEL),
                        "ô {c:?} trong công trình lại là {m}"
                    );
                }
            }
            BuildingKind::Field => {
                for c in footprint(b) {
                    let m = map.get(&c).copied().expect("ô của ruộng chưa được tô");
                    assert!(
                        matches!(m, FARMLAND | CROP_GREEN),
                        "ô {c:?} của ruộng là {m}"
                    );
                }
            }
            BuildingKind::Well => {}
        }
    }
}

#[test]
fn each_house_has_exactly_one_gravel_door_on_its_lower_edge() {
    let p = village(9, 40);
    let map = cells_map(&p);
    for b in of_kind(&p, BuildingKind::House) {
        let doors: Vec<(i64, i64)> = footprint(b)
            .into_iter()
            .filter(|c| map.get(c) == Some(&PATH_GRAVEL))
            .collect();
        assert_eq!(doors, vec![b.door], "nhà phải có đúng một ô cửa");
        assert_eq!(map.get(&b.door), Some(&PATH_GRAVEL));
        assert_eq!(b.door.1, b.origin.1 + b.h - 1, "cửa phải ở mép dưới");
        assert!(b.door.0 > b.origin.0 && b.door.0 < b.origin.0 + b.w - 1);
    }
}

#[test]
fn roofs_read_as_a_slope_with_a_chimney() {
    let p = village(21, 40);
    let map = cells_map(&p);
    for b in of_kind(&p, BuildingKind::House) {
        let lit = (b.h + 1) / 2;
        let mut chimneys = 0;
        for dy in 0..b.h {
            for dx in 0..b.w {
                let c = (b.origin.0 + dx, b.origin.1 + dy);
                let m = map[&c];
                if m == IGNEOUS {
                    chimneys += 1;
                    assert!(dy < lit, "ống khói phải nằm trên sườn sáng");
                    continue;
                }
                if c == b.door {
                    continue;
                }
                let want = if dy < lit { ROOF_LIGHT } else { ROOF_DARK };
                assert_eq!(m, want, "sắc mái sai ở {c:?}");
            }
        }
        assert_eq!(chimneys, 1, "nhà ở có đúng một ống khói");
    }
    for b in of_kind(&p, BuildingKind::Workshop) {
        let count = footprint(b).iter().filter(|c| map[c] == IGNEOUS).count();
        assert_eq!(count, 2, "xưởng có hai ống khói");
    }
    for b in of_kind(&p, BuildingKind::Granary) {
        let count = footprint(b).iter().filter(|c| map[c] == IGNEOUS).count();
        assert_eq!(count, 0, "kho không có lửa nên không có ống khói");
    }
}

#[test]
fn a_road_reaches_every_door_from_the_plaza() {
    let p = village(4, 40);
    let map = cells_map(&p);
    let well = of_kind(&p, BuildingKind::Well)[0];
    assert_eq!(map.get(&well.door), Some(&PATH_GRAVEL));

    let mut seen = BTreeSet::from([well.door]);
    let mut queue = VecDeque::from([well.door]);
    while let Some((x, y)) = queue.pop_front() {
        for n in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if map.get(&n) == Some(&PATH_GRAVEL) && seen.insert(n) {
                queue.push_back(n);
            }
        }
    }
    for b in &p.buildings {
        if matches!(
            b.kind,
            BuildingKind::House | BuildingKind::Workshop | BuildingKind::Granary
        ) {
            assert!(seen.contains(&b.door), "cửa {:?} không nối về làng", b.door);
        }
    }
}

#[test]
fn road_edges_are_dithered_rather_than_ruler_straight() {
    // Có `topsoil` trong bản vẽ nghĩa là mép đường có chỗ lồi chỗ lõm: vật liệu
    // ấy không được dùng ở bất cứ đâu khác.
    let p = village(6, 40);
    let ragged = p
        .cells
        .iter()
        .filter(|&&(_, _, m)| m == mow_settle::material::TOPSOIL)
        .count();
    assert!(ragged > 20, "mép đường thẳng quá: chỉ {ragged} ô răng cưa");
}

#[test]
fn the_well_is_one_water_cell_ringed_with_stone() {
    let p = village(8, 40);
    let map = cells_map(&p);
    let well = of_kind(&p, BuildingKind::Well)[0];
    let eye = (well.origin.0 + 1, well.origin.1 + 1);
    assert_eq!(map.get(&eye), Some(&WATER));
    for c in footprint(well) {
        if c != eye {
            assert_eq!(map.get(&c), Some(&SEDIMENTARY), "thành giếng hở ở {c:?}");
        }
    }
    let water = p.cells.iter().filter(|&&(_, _, m)| m == WATER).count();
    assert_eq!(water, 1, "làng chỉ có một lòng giếng");
}

#[test]
fn fields_are_striped_not_just_coloured() {
    let p = village(12, 40);
    let map = cells_map(&p);
    for b in of_kind(&p, BuildingKind::Field) {
        // Luống song song: đọc theo một trục thì màu đổi mỗi ô, đọc theo trục
        // kia thì màu không đổi. Đúng một trong hai trục thỏa điều đó.
        let along_x = (0..b.h).all(|dy| {
            (0..b.w).all(|dx| {
                let m = map[&(b.origin.0 + dx, b.origin.1 + dy)];
                m == if dx % 2 == 0 { CROP_GREEN } else { FARMLAND }
            })
        });
        let along_y = (0..b.h).all(|dy| {
            (0..b.w).all(|dx| {
                let m = map[&(b.origin.0 + dx, b.origin.1 + dy)];
                m == if dy % 2 == 0 { CROP_GREEN } else { FARMLAND }
            })
        });
        assert!(along_x ^ along_y, "thửa ruộng không có luống");
        assert!(b.w >= 8 && b.h >= 5, "thửa ruộng quá bé để đọc ra là ruộng");
    }
}

#[test]
fn residents_point_at_real_buildings_and_stand_on_real_cells() {
    let p = village(13, 40);
    let map = cells_map(&p);
    assert!(!p.residents.is_empty());
    for r in &p.residents {
        assert!(r.home < p.buildings.len(), "chỉ số nhà ở trỏ ra ngoài");
        assert!(
            r.workplace < p.buildings.len(),
            "chỉ số chỗ làm trỏ ra ngoài"
        );
        assert_eq!(p.buildings[r.home].kind, BuildingKind::House);
        assert!(map.contains_key(&r.start), "cư dân đứng ngoài bản vẽ");
        assert!(plain(r.start.0, r.start.1));
    }
}

#[test]
fn resident_names_never_repeat() {
    for seed in 0..16 {
        let p = village(seed, 40);
        let names: BTreeSet<&str> = p.residents.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names.len(),
            p.residents.len(),
            "hạt giống {seed} có tên trùng"
        );
    }
}

#[test]
fn a_village_has_the_shape_of_a_village() {
    for seed in 0..16 {
        let p = village(seed, 40);
        let houses = of_kind(&p, BuildingKind::House).len();
        let fields = of_kind(&p, BuildingKind::Field).len();
        assert!((5..=7).contains(&houses), "hạt giống {seed}: {houses} nhà");
        assert_eq!(of_kind(&p, BuildingKind::Workshop).len(), 1);
        assert_eq!(of_kind(&p, BuildingKind::Granary).len(), 1);
        assert_eq!(of_kind(&p, BuildingKind::Well).len(), 1);
        assert!(
            (2..=3).contains(&fields),
            "hạt giống {seed}: {fields} ruộng"
        );
        assert!(
            (8..=12).contains(&p.residents.len()),
            "hạt giống {seed}: {} cư dân",
            p.residents.len()
        );
        for role in [
            Role::Farmer,
            Role::Smith,
            Role::Hunter,
            Role::Elder,
            Role::Child,
        ] {
            assert!(
                p.residents.iter().any(|r| r.role == role),
                "hạt giống {seed} thiếu vai trò {role:?}"
            );
        }
    }
}

#[test]
fn every_farmer_works_a_field_and_every_smith_a_workshop() {
    let p = village(17, 40);
    for r in &p.residents {
        let at = p.buildings[r.workplace].kind;
        match r.role {
            Role::Farmer => assert_eq!(at, BuildingKind::Field),
            Role::Smith => assert_eq!(at, BuildingKind::Workshop),
            Role::Hunter | Role::Keeper => assert_eq!(at, BuildingKind::Granary),
            Role::Elder => assert_eq!(at, BuildingKind::Well),
            Role::Child => assert_eq!(r.workplace, r.home),
        }
    }
}

#[test]
fn an_ocean_gives_an_empty_plan() {
    let p = plan(
        &SettleRequest {
            seed: 42,
            center: (100, -100),
            radius: 40,
        },
        &sea,
    );
    assert_eq!(p, Plan::default());
    assert!(p.cells.is_empty() && p.buildings.is_empty() && p.residents.is_empty());
}

#[test]
fn a_tiny_region_shrinks_instead_of_panicking() {
    for radius in [0, 1, 2, 3, 6, 9] {
        let p = village(19, radius);
        for &(x, y, _) in &p.cells {
            assert!(x.abs() <= radius && y.abs() <= radius, "tràn ra ngoài vùng");
        }
        for r in &p.residents {
            assert!(r.home < p.buildings.len() && r.workplace < p.buildings.len());
        }
    }
}

#[test]
fn a_center_at_the_edge_of_the_number_line_is_refused_not_overflowed() {
    for center in [(i64::MAX, 0), (0, i64::MIN), (i64::MIN + 1, i64::MAX - 1)] {
        let p = plan(
            &SettleRequest {
                seed: 1,
                center,
                radius: 40,
            },
            &plain,
        );
        assert_eq!(p, Plan::default());
    }
}

#[test]
fn a_coastline_pushes_the_village_inland_without_breaking_it() {
    let p = plan(
        &SettleRequest {
            seed: 23,
            center: (20, 20),
            radius: 24,
        },
        &island,
    );
    for &(x, y, _) in &p.cells {
        assert!(island(x, y));
    }
    let mut claimed: BTreeSet<(i64, i64)> = BTreeSet::new();
    for b in &p.buildings {
        for c in footprint(b) {
            assert!(island(c.0, c.1), "công trình thò ra biển ở {c:?}");
            assert!(claimed.insert(c));
        }
    }
}

#[test]
fn report_the_size_of_a_typical_village() {
    let p = village(1, 40);
    println!(
        "ô: {}  công trình: {}  cư dân: {}",
        p.cells.len(),
        p.buildings.len(),
        p.residents.len()
    );
    assert!(p.cells.len() > 400, "một làng đủ đất phải phủ kín hơn thế");
}
