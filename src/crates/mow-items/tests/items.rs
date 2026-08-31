//! Test vật phẩm.

use mow_items::{
    Capacity, CraftQuality, EquipError, Equipment, Inventory, InventoryError, ItemDef,
    ItemInstance, ItemLocation, ItemStack, LocationError,
};
use mow_math::{Mass, Unit, Volume, WorldPos};
use std::collections::BTreeMap;

fn riu() -> ItemDef {
    ItemDef {
        id: "core.axe".into(),
        mass: Mass::new(2_500),
        volume: Volume::new(3_000),
        max_stack: 1,
        parts: vec!["blade".into(), "haft".into()],
        equip_slots: vec![],
        layer: None,
        coverage: Unit::ZERO,
        tags: vec!["weapon".into(), "tool".into()],
    }
}

fn mui_ten() -> ItemDef {
    ItemDef {
        id: "core.arrow".into(),
        mass: Mass::new(50),
        volume: Volume::new(40),
        max_stack: 50,
        parts: vec![],
        equip_slots: vec![],
        layer: None,
        coverage: Unit::ZERO,
        tags: vec![],
    }
}

fn ao_giap() -> ItemDef {
    ItemDef {
        id: "core.mail".into(),
        mass: Mass::new(12_000),
        volume: Volume::new(8_000),
        max_stack: 1,
        parts: vec!["rings".into()],
        equip_slots: vec!["core.torso".into()],
        layer: Some(2),
        coverage: Unit::from_frac(7, 10).unwrap(),
        tags: vec!["armor".into()],
    }
}

fn bang() -> BTreeMap<String, ItemDef> {
    [riu(), mui_ten(), ao_giap()]
        .into_iter()
        .map(|d| (d.id.clone(), d))
        .collect()
}

fn suc_chua() -> Capacity {
    Capacity {
        volume: Volume::new(40_000),
        comfortable_mass: Mass::new(30_000),
        max_mass: Mass::new(60_000),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.34 — CraftQuality bất biến
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sua_chua_khong_lam_do_re_thanh_kiet_tac() {
    // Đây là điều mà việc gộp hai khái niệm sẽ phá hỏng.
    let mut re = ItemInstance::craft(&riu(), CraftQuality::Crude, Some(1), 0);
    let mut tot = ItemInstance::craft(&riu(), CraftQuality::Masterwork, Some(2), 0);

    for it in [&mut re, &mut tot] {
        it.wear("blade", Unit::from_frac(9, 10).unwrap());
        it.repair("blade", Unit::ONE, 3, 100);
    }

    assert_eq!(
        re.quality(),
        CraftQuality::Crude,
        "sửa chữa đã đổi chất lượng chế tác"
    );
    assert_eq!(tot.quality(), CraftQuality::Masterwork);
    assert!(
        tot.effectiveness_percent() > re.effectiveness_percent(),
        "sau khi sửa, đồ rẻ và kiệt tác trở nên như nhau"
    );
}

#[test]
fn danh_tieng_tho_ren_ton_tai_duoc() {
    // Không có ai mài mòn được tay nghề.
    let it = ItemInstance::craft(&riu(), CraftQuality::Superior, Some(42), 500);
    assert_eq!(it.crafted_by(), Some(42));
    assert_eq!(it.crafted_at(), 500);
}

#[test]
fn tinh_trang_theo_bo_phan_khong_phai_mot_con_so() {
    // "Cán còn tốt, lưỡi mẻ" phải nói được.
    let mut it = ItemInstance::craft(&riu(), CraftQuality::Fine, None, 0);
    it.wear("blade", Unit::from_frac(8, 10).unwrap());
    let luoi = it.conditions.iter().find(|c| c.part == "blade").unwrap();
    let can = it.conditions.iter().find(|c| c.part == "haft").unwrap();
    assert!(luoi.condition < can.condition);
    assert_eq!(can.condition, Unit::ONE);
}

#[test]
fn bo_phan_yeu_nhat_quyet_dinh_chu_khong_phai_trung_binh() {
    // Một cây rìu cán gãy thì không dùng được, dù lưỡi hoàn hảo.
    let mut it = ItemInstance::craft(&riu(), CraftQuality::Plain, None, 0);
    it.wear("haft", Unit::ONE);
    assert_eq!(it.worst_condition(), Unit::ZERO);
    assert!(it.is_broken());
    assert_eq!(it.effectiveness_percent(), 0);
}

#[test]
fn lich_su_sua_chua_ghi_lai_ai_da_sua() {
    let mut it = ItemInstance::craft(&riu(), CraftQuality::Plain, None, 0);
    it.wear("blade", Unit::from_frac(5, 10).unwrap());
    it.repair("blade", Unit::from_frac(3, 10).unwrap(), 77, 900);

    assert_eq!(it.repairs.len(), 1);
    assert_eq!(it.repairs[0].by, 77);
    assert_eq!(it.repairs[0].at_tick, 900);
    assert_eq!(it.repairs[0].part, "blade");
}

#[test]
fn sua_bo_phan_khong_ton_tai_thi_bao_that_bai() {
    let mut it = ItemInstance::craft(&riu(), CraftQuality::Plain, None, 0);
    assert!(!it.repair("khong_co", Unit::ONE, 1, 0));
    assert!(it.repairs.is_empty());
}

#[test]
fn chat_luong_nhan_len_hieu_qua() {
    let re = ItemInstance::craft(&riu(), CraftQuality::Crude, None, 0);
    let tot = ItemInstance::craft(&riu(), CraftQuality::Masterwork, None, 0);
    assert_eq!(re.effectiveness_percent(), 70);
    assert_eq!(tot.effectiveness_percent(), 180);
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.32 — chồng là dữ liệu, không phải entity
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn gop_chong_cung_loai_cung_chat_luong() {
    let mut a = ItemStack {
        def: "core.arrow".into(),
        count: 30,
        quality: CraftQuality::Plain,
    };
    let b = ItemStack {
        def: "core.arrow".into(),
        count: 30,
        quality: CraftQuality::Plain,
    };
    let du = a.merge(&b, 50);
    assert_eq!(a.count, 50);
    assert_eq!(
        du.map(|d| d.count),
        Some(10),
        "phần vượt trần phải thành chồng mới"
    );
}

#[test]
fn khong_gop_chong_khac_chat_luong() {
    // Đồ trong một chồng phải giống hệt nhau, kể cả chất lượng — nếu không,
    // gộp rồi tách sẽ biến đồ tốt thành đồ thường.
    let mut a = ItemStack {
        def: "core.arrow".into(),
        count: 10,
        quality: CraftQuality::Fine,
    };
    let b = ItemStack {
        def: "core.arrow".into(),
        count: 10,
        quality: CraftQuality::Crude,
    };
    let du = a.merge(&b, 50);
    assert_eq!(a.count, 10);
    assert_eq!(du.map(|d| d.count), Some(10));
}

#[test]
fn tach_chong() {
    let mut a = ItemStack {
        def: "core.arrow".into(),
        count: 30,
        quality: CraftQuality::Plain,
    };
    let m = a.split(10).unwrap();
    assert_eq!(a.count, 20);
    assert_eq!(m.count, 10);
    assert_eq!(a.split(20), None, "không tách được nhiều hơn số có");
    assert_eq!(a.split(0), None);
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.33 — đúng một nơi
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn chuyen_cho_la_thay_the_nguyen_tu() {
    let tren_dat = ItemLocation::Cell {
        at: WorldPos::new(1, 2, 0),
    };
    let vao_tui = tren_dat
        .move_to(ItemLocation::Inventory { owner: 5 }, 99)
        .unwrap();
    assert_eq!(vao_tui.holder(), Some(5));
    assert_eq!(vao_tui.kind(), "inventory");
    // Không có khoảnh khắc nào nó ở cả hai nơi: `move_to` trả giá trị mới.
    assert_ne!(tren_dat, vao_tui);
}

#[test]
fn vat_chua_khong_chua_duoc_chinh_no() {
    let l = ItemLocation::Cell {
        at: WorldPos::ORIGIN,
    };
    assert_eq!(
        l.move_to(ItemLocation::Container { entity: 7 }, 7),
        Err(LocationError::SelfContainment)
    );
}

#[test]
fn chuyen_toi_noi_dang_o_la_loi() {
    let l = ItemLocation::Inventory { owner: 1 };
    assert_eq!(l.move_to(l, 9), Err(LocationError::AlreadyThere));
}

// ─────────────────────────────────────────────────────────────────────────────
// PB-19 — thể tích và khối lượng là hai ràng buộc
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn the_tich_va_khoi_luong_chan_hai_thu_khac_nhau() {
    let defs = bang();
    let cap = suc_chua();

    // Bó rơm: nhẹ nhưng cồng kềnh.
    let mut rom_def = mui_ten();
    rom_def.id = "core.straw".into();
    rom_def.mass = Mass::new(5);
    rom_def.volume = Volume::new(2_000);
    rom_def.max_stack = 100;
    let mut d2 = defs.clone();
    d2.insert(rom_def.id.clone(), rom_def.clone());

    let mut inv = Inventory::new();
    let e = inv.insert(
        ItemStack {
            def: rom_def.id.clone(),
            count: 100,
            quality: CraftQuality::Plain,
        },
        &d2,
        cap,
    );
    assert!(
        matches!(e, Err(InventoryError::NoVolume { .. })),
        "cồng kềnh phải bị chặn bởi thể tích"
    );

    // Vàng: gọn nhưng nặng — không bị chặn, chỉ làm chậm.
    let mut vang = mui_ten();
    vang.id = "core.gold".into();
    vang.mass = Mass::new(1_000);
    vang.volume = Volume::new(10);
    vang.max_stack = 1_000;
    let mut d3 = defs.clone();
    d3.insert(vang.id.clone(), vang.clone());

    let mut inv2 = Inventory::new();
    inv2.insert(
        ItemStack {
            def: vang.id.clone(),
            count: 50,
            quality: CraftQuality::Plain,
        },
        &d3,
        cap,
    )
    .expect("nặng thì vẫn nhặt được");
    let l = inv2.load(&d3, cap, Mass::ZERO, Volume::ZERO);
    assert!(l.is_overloaded(), "50 kg vàng phải là quá tải");
}

#[test]
fn qua_tai_lam_cham_chu_khong_chan() {
    // `§18.15.2`. Chặn cứng biến một khoảnh khắc kịch tính thành hộp thoại lỗi.
    let cap = suc_chua();
    let l = mow_items::Load {
        volume_used: Volume::new(1_000),
        volume_max: cap.volume,
        mass_carried: Mass::new(45_000),
        mass_comfortable: cap.comfortable_mass,
        mass_max: cap.max_mass,
    };
    assert!(l.is_overloaded());
    assert!(!l.is_immobilized());
    assert_eq!(
        l.speed_percent(),
        50,
        "giữa mức thoải mái và tối đa thì đi nửa tốc"
    );
}

#[test]
fn toc_do_giam_lien_tuc_khong_co_bac_thang() {
    // Người chơi phải cảm nhận được cái giá của từng món nhặt thêm.
    let cap = suc_chua();
    let toc = |m: i64| {
        mow_items::Load {
            volume_used: Volume::ZERO,
            volume_max: cap.volume,
            mass_carried: Mass::new(m),
            mass_comfortable: cap.comfortable_mass,
            mass_max: cap.max_mass,
        }
        .speed_percent()
    };
    assert_eq!(toc(30_000), 100);
    assert!(toc(35_000) < toc(30_000));
    assert!(toc(40_000) < toc(35_000));
    assert_eq!(toc(60_000), 0);
    assert_eq!(toc(100_000), 0, "vượt tối đa thì vẫn là 0, không âm");
}

#[test]
fn hai_thanh_rieng_biet_cho_ui() {
    // `§18.15.1`: người chơi phải thấy mình bị chặn bởi cái nào.
    let cap = suc_chua();
    let l = mow_items::Load {
        volume_used: Volume::new(20_000),
        volume_max: cap.volume,
        mass_carried: Mass::new(45_000),
        mass_comfortable: cap.comfortable_mass,
        mass_max: cap.max_mass,
    };
    assert_eq!(l.volume_fraction(), Unit::from_frac(1, 2).unwrap());
    assert!(
        l.mass_ratio() > mow_math::Fx::ONE,
        "tỉ lệ khối lượng phải vượt được 1"
    );
}

#[test]
fn lay_ra_dung_so_luong() {
    let defs = bang();
    let mut inv = Inventory::new();
    inv.insert(
        ItemStack {
            def: "core.arrow".into(),
            count: 40,
            quality: CraftQuality::Plain,
        },
        &defs,
        suc_chua(),
    )
    .unwrap();
    assert_eq!(inv.count("core.arrow"), 40);
    assert_eq!(inv.take("core.arrow", 15), 15);
    assert_eq!(inv.count("core.arrow"), 25);
    assert_eq!(
        inv.take("core.arrow", 100),
        25,
        "lấy nhiều hơn số có thì lấy hết"
    );
    assert!(inv.is_empty());
}

#[test]
fn thu_tu_chong_on_dinh_khong_theo_thu_tu_nhat() {
    let defs = bang();
    let mk = |dao: bool| {
        let mut inv = Inventory::new();
        let ds = ["core.arrow", "core.axe"];
        let ds: Vec<_> = if dao {
            ds.iter().rev().copied().collect()
        } else {
            ds.to_vec()
        };
        for d in ds {
            inv.insert(
                ItemStack {
                    def: d.into(),
                    count: 1,
                    quality: CraftQuality::Plain,
                },
                &defs,
                suc_chua(),
            )
            .unwrap();
        }
        inv.stacks()
            .iter()
            .map(|s| s.def.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(mk(false), mk(true));
}

// ─────────────────────────────────────────────────────────────────────────────
// PB-21 — trang bị theo bộ phận cơ thể
// ─────────────────────────────────────────────────────────────────────────────

fn co_the_nguoi() -> Vec<String> {
    ["core.head", "core.torso", "core.arm_left", "core.arm_right"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

#[test]
fn giai_phau_khac_thi_cho_mac_khac() {
    // Đây là toàn bộ lý do chỗ mặc suy ra từ sơ đồ cơ thể.
    let mut e = Equipment::new();
    let ran = vec!["core.head".to_owned()]; // không có thân theo nghĩa mặc giáp

    assert!(matches!(
        e.equip(1, &ao_giap(), "core.torso", &ran),
        Err(EquipError::NoSuchSlot { .. })
    ));
    e.equip(1, &ao_giap(), "core.torso", &co_the_nguoi())
        .expect("người thì mặc được");
}

#[test]
fn loai_bon_tay_co_bon_cho_deo_gang() {
    let mut gang = ao_giap();
    gang.id = "core.glove".into();
    gang.equip_slots = (1..=4).map(|i| format!("mypack.arm_{i}")).collect();
    gang.layer = Some(1);

    let bon_tay: Vec<String> = (1..=4).map(|i| format!("mypack.arm_{i}")).collect();
    let cho = Equipment::available_slots(&gang, &bon_tay);
    assert_eq!(cho.len(), 4, "không cần sửa engine để có bốn chỗ đeo găng");
}

#[test]
fn cung_bo_phan_cung_lop_thi_khong_mac_chong() {
    let mut e = Equipment::new();
    e.equip(1, &ao_giap(), "core.torso", &co_the_nguoi())
        .unwrap();
    assert!(matches!(
        e.equip(2, &ao_giap(), "core.torso", &co_the_nguoi()),
        Err(EquipError::LayerOccupied { .. })
    ));
}

#[test]
fn lop_khac_nhau_thi_mac_chong_duoc_va_xuyen_tu_ngoai_vao() {
    let mut ao_lot = ao_giap();
    ao_lot.id = "core.gambeson".into();
    ao_lot.layer = Some(0);
    let mut ao_choang = ao_giap();
    ao_choang.id = "core.cloak".into();
    ao_choang.layer = Some(3);

    let mut e = Equipment::new();
    e.equip(1, &ao_lot, "core.torso", &co_the_nguoi()).unwrap();
    e.equip(2, &ao_giap(), "core.torso", &co_the_nguoi())
        .unwrap();
    e.equip(3, &ao_choang, "core.torso", &co_the_nguoi())
        .unwrap();

    let lop: Vec<&str> = e
        .layers_over("core.torso")
        .iter()
        .map(|x| x.def.as_str())
        .collect();
    assert_eq!(
        lop,
        vec!["core.cloak", "core.mail", "core.gambeson"],
        "đòn đánh phải xuyên từ ngoài vào trong"
    );
}

#[test]
fn hai_mon_che_60_phan_tram_khong_thanh_kin_tuyet_doi() {
    // Cộng thẳng sẽ cho 120% và một bộ giáp tầm thường thành bất khả xâm phạm.
    let mut a = ao_giap();
    a.coverage = Unit::from_frac(6, 10).unwrap();
    a.layer = Some(1);
    let mut b = a.clone();
    b.id = "core.mail2".into();
    b.layer = Some(2);

    let mut e = Equipment::new();
    e.equip(1, &a, "core.torso", &co_the_nguoi()).unwrap();
    e.equip(2, &b, "core.torso", &co_the_nguoi()).unwrap();

    let che = e.coverage_of("core.torso");
    assert!(che < Unit::ONE, "che phủ đạt 100% từ hai món 60%");
    // 1 − 0.4×0.4 = 0.84
    assert!(che > Unit::from_frac(8, 10).unwrap());
}

#[test]
fn che_phu_quyet_dinh_thuong_tich_roi_vao_dau() {
    let mut e = Equipment::new();
    e.equip(1, &ao_giap(), "core.torso", &co_the_nguoi())
        .unwrap();

    // Giáp che 70%: một đòn với roll 0.9 rơi vào chỗ hở.
    assert!(e.hits_gap("core.torso", Unit::from_frac(9, 10).unwrap()));
    assert!(!e.hits_gap("core.torso", Unit::from_frac(5, 10).unwrap()));
    // Bộ phận không có giáp thì mọi đòn đều vào chỗ hở.
    assert!(e.hits_gap("core.head", Unit::from_frac(1, 100).unwrap()));
}

#[test]
fn coi_do_thi_mat_che_phu() {
    let mut e = Equipment::new();
    e.equip(1, &ao_giap(), "core.torso", &co_the_nguoi())
        .unwrap();
    assert!(e.coverage_of("core.torso") > Unit::ZERO);
    assert!(e.unequip(1));
    assert_eq!(e.coverage_of("core.torso"), Unit::ZERO);
    assert!(!e.unequip(1), "cởi hai lần thì lần hai không làm gì");
}

#[test]
fn do_khong_phai_trang_bi_thi_khong_mac_duoc() {
    let mut e = Equipment::new();
    assert!(matches!(
        e.equip(1, &riu(), "core.torso", &co_the_nguoi()),
        Err(EquipError::NotEquipment(_))
    ));
}
