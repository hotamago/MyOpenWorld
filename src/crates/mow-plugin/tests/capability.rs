//! Test quyền của content pack và đối chiếu pack set (`PF-01`, `PF-02`).

use mow_plugin::capability::{Capability, ContentKind, Grants};
use mow_plugin::manifest::PackRef;
use mow_plugin::{PackManifest, PackSet, Registry, RegistryError};
use std::collections::BTreeMap;

fn mf(id: &str, caps: &[Capability]) -> PackManifest {
    PackManifest {
        id: id.to_owned(),
        version: "1.0.0".to_owned(),
        name: String::new(),
        description: String::new(),
        requires: Vec::new(),
        overrides: Vec::new(),
        tests: Vec::new(),
        capabilities: caps.to_vec(),
    }
}

fn files(paths: &[&str]) -> BTreeMap<String, Vec<u8>> {
    paths
        .iter()
        .map(|p| ((*p).to_owned(), b"noi dung".to_vec()))
        .collect()
}

// ───────────────────── PF-01 · quyền theo capability ─────────────────────

/// **Mặc định là không có quyền gì** ngoài dữ liệu tĩnh.
#[test]
fn mac_dinh_chi_khai_duoc_du_lieu_tinh() {
    let g = Grants::default();
    assert!(g.has(Capability::DefineContent));
    for c in [
        Capability::DefineLaw,
        Capability::DefineModule,
        Capability::DefinePrompt,
        Capability::DefineGenerator,
        Capability::OverrideForeign,
    ] {
        assert!(!g.has(c), "mặc định không được có {c:?}");
    }
}

/// Pack chỉ thêm vật phẩm thì nạp được mà không khai gì.
#[test]
fn pack_chi_them_vat_pham_nap_duoc_khong_can_khai_gi() {
    let mut r = Registry::new();
    assert!(r
        .add(
            mf("bakery", &[]),
            &files(&["content/bread.yaml", "content/oven.yaml"]),
        )
        .is_ok());
    assert!(r.grants_of("bakery").unwrap().risky().is_empty());
}

/// **Quyền kiểm bằng nội dung thật, không bằng lời khai.**
///
/// Một pack khai `capabilities: []` mà thư mục có `laws/` thì bị từ chối — nếu
/// tin lời khai thì trường này chỉ là trang trí.
#[test]
fn pack_khai_khong_xin_gi_ma_co_luat_thi_bi_tu_choi() {
    let mut r = Registry::new();
    let loi = r
        .add(
            mf("sneaky", &[]),
            &files(&["content/sword.yaml", "laws/gravity.yaml"]),
        )
        .unwrap_err();
    match loi {
        RegistryError::MissingCapability { violations, .. } => {
            assert_eq!(violations.len(), 1);
            assert_eq!(violations[0].path, "laws/gravity.yaml");
            assert_eq!(violations[0].needs, Capability::DefineLaw);
        }
        khac => panic!("{khac}"),
    }
    assert!(
        r.is_empty(),
        "bị từ chối mà vẫn nằm trong sổ thì đã nạp một phần"
    );
}

/// **Báo mọi vi phạm cùng lúc**, không dừng ở cái đầu tiên.
#[test]
fn bao_moi_vi_pham_cung_luc() {
    let mut r = Registry::new();
    let loi = r
        .add(
            mf("everything", &[]),
            &files(&[
                "laws/a.yaml",
                "modules/b.wasm",
                "prompts/c.yaml",
                "generators/d.yaml",
                "content/e.yaml",
            ]),
        )
        .unwrap_err();
    match loi {
        RegistryError::MissingCapability { violations, .. } => assert_eq!(violations.len(), 4),
        khac => panic!("{khac}"),
    }
}

/// Xin đúng quyền thì nạp được.
#[test]
fn xin_dung_quyen_thi_nap_duoc() {
    let mut r = Registry::new();
    r.add(
        mf("physics_overhaul", &[Capability::DefineLaw]),
        &files(&["laws/gravity.yaml", "content/anvil.yaml"]),
    )
    .unwrap();
    assert_eq!(
        r.grants_of("physics_overhaul").unwrap().risky(),
        vec![Capability::DefineLaw]
    );
}

/// **Khai `overrides` là cần, chưa đủ.**
///
/// Nếu chỉ cần khai `overrides` thì quyền ghi đè tự cấp được bằng một dòng
/// YAML, và nó không còn là quyền nữa.
#[test]
fn khai_overrides_ma_khong_xin_quyen_thi_van_khong_ghi_de_duoc() {
    let mut r = Registry::new();
    r.add(mf("core", &[]), &files(&["content/apple.yaml"]))
        .unwrap();
    r.define("core", "core.apple").unwrap();

    let mut khai_suong = mf("mod", &[]);
    khai_suong.overrides = vec!["core.apple".to_owned()];
    r.add(khai_suong, &files(&["content/apple.yaml"])).unwrap();

    assert!(matches!(
        r.define("mod", "core.apple").unwrap_err(),
        RegistryError::ForeignNamespace { .. }
    ));
    assert_eq!(r.owner_of("core.apple"), Some("core"));
}

/// Khai `overrides` **và** xin quyền thì ghi đè được.
#[test]
fn khai_overrides_va_xin_quyen_thi_ghi_de_duoc() {
    let mut r = Registry::new();
    r.add(mf("core", &[]), &files(&["content/apple.yaml"]))
        .unwrap();
    r.define("core", "core.apple").unwrap();

    let mut dung_luat = mf("mod", &[Capability::OverrideForeign]);
    dung_luat.overrides = vec!["core.apple".to_owned()];
    r.add(dung_luat, &files(&["content/apple.yaml"])).unwrap();

    r.define("mod", "core.apple").unwrap();
    assert_eq!(r.owner_of("core.apple"), Some("mod"));
}

/// Có quyền ghi đè nhưng **không khai id nào** thì vẫn không ghi đè được.
#[test]
fn co_quyen_ghi_de_ma_khong_khai_id_thi_van_khong_ghi_de_duoc() {
    let mut r = Registry::new();
    r.add(mf("core", &[]), &files(&["content/apple.yaml"]))
        .unwrap();
    r.define("core", "core.apple").unwrap();
    r.add(
        mf("mod", &[Capability::OverrideForeign]),
        &files(&["content/x.yaml"]),
    )
    .unwrap();
    assert!(r.define("mod", "core.apple").is_err());
}

/// **UI phải phân biệt được** pack thêm đồ và pack viết lại luật.
#[test]
fn ui_phan_biet_duoc_pack_them_do_va_pack_viet_lai_luat() {
    let mut r = Registry::new();
    r.add(mf("bakery", &[]), &files(&["content/bread.yaml"]))
        .unwrap();
    r.add(
        mf("physics", &[Capability::DefineLaw]),
        &files(&["laws/g.yaml"]),
    )
    .unwrap();
    r.add(
        mf("minds", &[Capability::DefinePrompt]),
        &files(&["prompts/p.yaml"]),
    )
    .unwrap();

    let rui_ro = r.packs_with_risky_capabilities();
    let ten: Vec<&str> = rui_ro.iter().map(|(id, _)| *id).collect();
    assert_eq!(ten, vec!["minds", "physics"]);
    assert!(!Capability::DefineContent.affects_simulation());
}

/// Mỗi quyền có một câu cảnh báo riêng — không dùng lại câu nào.
#[test]
fn moi_quyen_co_mot_cau_canh_bao_rieng() {
    let cac = [
        Capability::DefineContent,
        Capability::DefineLaw,
        Capability::DefineModule,
        Capability::DefinePrompt,
        Capability::DefineGenerator,
        Capability::OverrideForeign,
    ];
    let canh: std::collections::BTreeSet<&str> = cac.iter().map(|c| c.warning()).collect();
    assert_eq!(canh.len(), cac.len());
}

/// **Thư mục lạ được coi là dữ liệu tĩnh** — mặc định nghiêng về ít quyền.
#[test]
fn thu_muc_la_duoc_coi_la_du_lieu_tinh() {
    assert_eq!(ContentKind::from_path("laws/a.yaml"), ContentKind::Law);
    assert_eq!(
        ContentKind::from_path("modules/a.wasm"),
        ContentKind::Module
    );
    assert_eq!(
        ContentKind::from_path("gi_do_la/a.yaml"),
        ContentKind::Content
    );
    // Windows dùng dấu chéo ngược; phân loại không được đổi theo hệ điều hành.
    assert_eq!(ContentKind::from_path("laws\\a.yaml"), ContentKind::Law);
}

// ───────────────────── PF-02 · save và pack set ─────────────────────

fn dung_so() -> Registry {
    let mut r = Registry::new();
    r.add(mf("core", &[]), &files(&["content/apple.yaml"]))
        .unwrap();
    let mut phu = mf("bakery", &[]);
    phu.requires = vec![PackRef {
        id: "core".to_owned(),
        version: String::new(),
    }];
    r.add(phu, &files(&["content/bread.yaml"])).unwrap();
    r.resolve_order().unwrap();
    r
}

/// Pack set ghi **cả version lẫn content hash**.
#[test]
fn pack_set_ghi_ca_version_lan_hash() {
    let ps = dung_so().pack_set();
    assert_eq!(ps.entries.len(), 2);
    for (_, ver, _) in &ps.entries {
        assert_eq!(ver, "1.0.0");
    }
    assert_eq!(ps.entries[0].0, "core", "thứ tự nạp, không phải thứ tự chữ");
}

/// Bộ pack không đổi thì mở lại được.
#[test]
fn bo_pack_khong_doi_thi_mo_lai_duoc() {
    let luu = dung_so().pack_set();
    assert!(dung_so().verify_against(&luu).is_ok());
}

/// **Thiếu pack thì từ chối** — không nạp một phần.
#[test]
fn thieu_pack_thi_tu_choi_khong_nap_mot_phan() {
    let luu = dung_so().pack_set();
    let mut it_hon = Registry::new();
    it_hon
        .add(mf("core", &[]), &files(&["content/apple.yaml"]))
        .unwrap();
    assert!(matches!(
        it_hon.verify_against(&luu).unwrap_err(),
        RegistryError::PackAbsent(p) if p == "bakery"
    ));
}

/// **Version lệch thì từ chối**, và báo khác với hash lệch.
///
/// Hai lỗi nói hai chuyện: hash lệch nghĩa là có ai sửa file, version lệch
/// nghĩa là người dùng đã cập nhật pack. Cách xử lý khác hẳn nhau.
#[test]
fn version_lech_thi_tu_choi_va_bao_khac_voi_hash_lech() {
    let luu = dung_so().pack_set();

    let mut moi_hon = Registry::new();
    let mut v2 = mf("core", &[]);
    v2.version = "2.0.0".to_owned();
    moi_hon.add(v2, &files(&["content/apple.yaml"])).unwrap();
    let mut phu = mf("bakery", &[]);
    phu.requires = vec![PackRef {
        id: "core".to_owned(),
        version: String::new(),
    }];
    moi_hon.add(phu, &files(&["content/bread.yaml"])).unwrap();

    assert!(matches!(
        moi_hon.verify_against(&luu).unwrap_err(),
        RegistryError::VersionMismatch { .. }
    ));
}

/// **Nội dung bị sửa mà version giữ nguyên** cũng bị bắt.
///
/// Đây là trường hợp mà chỉ kiểm version sẽ bỏ lọt, và nó là trường hợp phổ
/// biến nhất: một modder sửa file rồi quên tăng version.
#[test]
fn noi_dung_bi_sua_ma_version_giu_nguyen_van_bi_bat() {
    let luu = dung_so().pack_set();

    let mut da_sua = Registry::new();
    da_sua
        .add(mf("core", &[]), &files(&["content/apple.yaml"]))
        .unwrap();
    let mut phu = mf("bakery", &[]);
    phu.requires = vec![PackRef {
        id: "core".to_owned(),
        version: String::new(),
    }];
    // Cùng version, nội dung khác.
    da_sua
        .add(phu, &files(&["content/bread.yaml", "content/cake.yaml"]))
        .unwrap();

    assert!(matches!(
        da_sua.verify_against(&luu).unwrap_err(),
        RegistryError::HashMismatch { pack, .. } if pack == "bakery"
    ));
}

/// **Pack thừa không phải lỗi.**
///
/// Một world không dùng pack nào đó thì việc pack đó có mặt trong máy không
/// ảnh hưởng gì — và cấm nó sẽ khiến người dùng phải gỡ mod mỗi lần đổi save.
#[test]
fn pack_thua_khong_phai_loi() {
    let luu = PackSet {
        entries: dung_so().pack_set().entries[..1].to_vec(),
    };
    assert!(dung_so().verify_against(&luu).is_ok());
}
