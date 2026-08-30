//! Test sổ đăng ký.

use mow_plugin::manifest::{content_hash, PackRef};
use mow_plugin::{PackManifest, Registry, RegistryError};
use std::collections::BTreeMap;

fn mf(id: &str, requires: &[&str], overrides: &[&str]) -> PackManifest {
    PackManifest {
        id: id.to_owned(),
        version: "1.0.0".to_owned(),
        name: String::new(),
        description: String::new(),
        requires: requires
            .iter()
            .map(|r| PackRef {
                id: (*r).to_owned(),
                version: String::new(),
            })
            .collect(),
        overrides: overrides.iter().map(|s| (*s).to_owned()).collect(),
        tests: Vec::new(),
    }
}

fn khong_co_file() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// content/core nạp qua đúng cơ chế của cộng đồng — §P10.7
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn content_core_nap_duoc_qua_duong_binh_thuong() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content/core");
    let mut r = Registry::new();
    r.add_from_dir(&dir).expect("content/core phải nạp được");
    assert_eq!(r.len(), 1);

    let m = r.manifest("core").expect("có manifest");
    assert_eq!(m.id, "core");
    assert!(
        r.hash_of("core").is_some(),
        "pack chính thức cũng phải có content hash như mọi pack khác"
    );

    let order = r.resolve_order().unwrap();
    assert_eq!(order.0, vec!["core"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.29 — namespace bắt buộc, xung đột là lỗi
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn id_khong_co_namespace_bi_tu_choi() {
    let mut r = Registry::new();
    r.add(mf("core", &[], &[]), &khong_co_file()).unwrap();
    let e = r.define("core", "apple").expect_err("phải lỗi");
    assert!(matches!(e, RegistryError::MissingNamespace { .. }), "{e}");
    // Có namespace thì được.
    r.define("core", "core.apple").unwrap();
}

#[test]
fn muon_namespace_cua_pack_khac_bi_tu_choi() {
    let mut r = Registry::new();
    r.add(mf("core", &[], &[]), &khong_co_file()).unwrap();
    r.add(mf("mypack", &[], &[]), &khong_co_file()).unwrap();

    let e = r
        .define("mypack", "core.apple")
        .expect_err("mượn namespace phải bị chặn");
    assert!(matches!(e, RegistryError::ForeignNamespace { .. }), "{e}");
}

#[test]
fn ghi_de_khai_bao_tuong_minh_thi_duoc() {
    let mut r = Registry::new();
    r.add(mf("core", &[], &[]), &khong_co_file()).unwrap();
    r.add(mf("mypack", &["core"], &["core.apple"]), &khong_co_file())
        .unwrap();

    r.define("core", "core.apple").unwrap();
    r.define("mypack", "core.apple")
        .expect("đã khai báo trong overrides");
    assert_eq!(
        r.owner_of("core.apple"),
        Some("mypack"),
        "ghi đè khai báo tường minh phải chuyển quyền sở hữu"
    );
}

#[test]
fn xung_dot_khong_khai_bao_la_loi_khong_phai_ai_sau_thi_thang() {
    // Đây là bất biến quan trọng nhất của §22.29. "Ai load sau thì thắng" biến
    // thứ tự nạp thành một phần vô hình của luật chơi.
    let mut r = Registry::new();
    r.add(mf("a", &[], &[]), &khong_co_file()).unwrap();
    r.add(mf("b", &[], &["a.thing"]), &khong_co_file()).unwrap();
    r.define("a", "a.thing").unwrap();
    // b khai báo ghi đè nên được.
    r.define("b", "a.thing").unwrap();

    // Nhưng c thì không khai báo.
    r.add(mf("c", &[], &[]), &khong_co_file()).unwrap();
    let e = r.define("c", "a.thing").expect_err("phải là xung đột");
    assert!(
        matches!(
            e,
            RegistryError::ForeignNamespace { .. } | RegistryError::Conflict { .. }
        ),
        "{e}"
    );
}

#[test]
fn manifest_ghi_de_chinh_minh_bi_tu_choi() {
    let mut r = Registry::new();
    let e = r
        .add(mf("core", &[], &["core.apple"]), &khong_co_file())
        .expect_err("phải lỗi");
    assert!(matches!(e, RegistryError::BadManifest { .. }), "{e}");
}

#[test]
fn manifest_id_co_dau_cham_bi_tu_choi() {
    let mut r = Registry::new();
    let e = r
        .add(mf("my.pack", &[], &[]), &khong_co_file())
        .expect_err("phải lỗi");
    assert!(matches!(e, RegistryError::BadManifest { .. }), "{e}");
}

// ─────────────────────────────────────────────────────────────────────────────
// Thứ tự nạp xác định
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn thu_tu_nap_ton_trong_phu_thuoc() {
    let mut r = Registry::new();
    // Thêm theo thứ tự ngược để chắc chắn kết quả không phải là thứ tự thêm.
    r.add(mf("zz", &["aa"], &[]), &khong_co_file()).unwrap();
    r.add(mf("aa", &[], &[]), &khong_co_file()).unwrap();

    let o = r.resolve_order().unwrap();
    assert_eq!(o.0, vec!["aa", "zz"], "phụ thuộc phải nạp trước");
}

#[test]
fn thu_tu_nap_khong_phu_thuoc_thu_tu_them() {
    // Hai pack độc lập: thứ tự giữa chúng không ảnh hưởng ngữ nghĩa, nhưng nó
    // ảnh hưởng content hash của bộ pack, và hash đó nằm trong save.
    let mk = |dao: bool| {
        let mut r = Registry::new();
        let ds = if dao {
            vec!["m", "a", "z"]
        } else {
            vec!["z", "m", "a"]
        };
        for id in ds {
            r.add(mf(id, &[], &[]), &khong_co_file()).unwrap();
        }
        r.resolve_order().unwrap().0
    };
    assert_eq!(mk(false), mk(true));
    assert_eq!(mk(false), vec!["a", "m", "z"], "phá hòa bằng id tăng dần");
}

#[test]
fn thieu_phu_thuoc_bi_bat() {
    let mut r = Registry::new();
    r.add(mf("b", &["khong_co"], &[]), &khong_co_file())
        .unwrap();
    let e = r.resolve_order().expect_err("phải lỗi");
    assert!(matches!(e, RegistryError::MissingDependency { .. }), "{e}");
}

#[test]
fn phu_thuoc_vong_bi_bat_chu_khong_treo() {
    let mut r = Registry::new();
    r.add(mf("a", &["b"], &[]), &khong_co_file()).unwrap();
    r.add(mf("b", &["a"], &[]), &khong_co_file()).unwrap();
    let e = r.resolve_order().expect_err("phải lỗi");
    assert!(matches!(e, RegistryError::CyclicDependency(_)), "{e}");
}

#[test]
fn pack_trung_id_bi_tu_choi() {
    let mut r = Registry::new();
    r.add(mf("core", &[], &[]), &khong_co_file()).unwrap();
    let e = r
        .add(mf("core", &[], &[]), &khong_co_file())
        .expect_err("phải lỗi");
    assert!(matches!(e, RegistryError::DuplicatePack(_)), "{e}");
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.30 — content hash và save
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn content_hash_doi_khi_noi_dung_doi() {
    let m = mf("core", &[], &[]);
    let mut a = BTreeMap::new();
    a.insert("x.yaml".to_owned(), b"mot".to_vec());
    let mut b = BTreeMap::new();
    b.insert("x.yaml".to_owned(), b"hai".to_vec());
    assert_ne!(content_hash(&m, &a), content_hash(&m, &b));
}

#[test]
fn content_hash_khong_phu_thuoc_dau_gach_cua_he_dieu_hanh() {
    // Cùng một pack không được cho hai hash khác nhau trên Windows và Linux.
    let m = mf("core", &[], &[]);
    let mut win = BTreeMap::new();
    win.insert("items\\apple.yaml".to_owned(), b"x".to_vec());
    let mut nix = BTreeMap::new();
    nix.insert("items/apple.yaml".to_owned(), b"x".to_vec());
    assert_eq!(
        content_hash(&m, &win),
        content_hash(&m, &nix),
        "dấu gạch của hệ điều hành không được lọt vào content hash"
    );
}

#[test]
fn save_lech_hash_thi_tu_choi_load_thay_vi_load_mot_phan() {
    let mut cu = Registry::new();
    let mut files = BTreeMap::new();
    files.insert("a.yaml".to_owned(), b"ban cu".to_vec());
    cu.add(mf("core", &[], &[]), &files).unwrap();
    cu.resolve_order().unwrap();
    let da_luu = cu.pack_set();

    // Người chơi sửa pack rồi mở lại save.
    let mut moi = Registry::new();
    let mut files2 = BTreeMap::new();
    files2.insert("a.yaml".to_owned(), b"ban da sua".to_vec());
    moi.add(mf("core", &[], &[]), &files2).unwrap();
    moi.resolve_order().unwrap();

    let e = moi.verify_against(&da_luu).expect_err("phải từ chối");
    assert!(matches!(e, RegistryError::HashMismatch { .. }), "{e}");
}

#[test]
fn save_thieu_pack_thi_tu_choi() {
    let mut cu = Registry::new();
    cu.add(mf("core", &[], &[]), &khong_co_file()).unwrap();
    cu.add(mf("mypack", &[], &[]), &khong_co_file()).unwrap();
    cu.resolve_order().unwrap();
    let da_luu = cu.pack_set();

    let mut moi = Registry::new();
    moi.add(mf("core", &[], &[]), &khong_co_file()).unwrap();
    moi.resolve_order().unwrap();

    let e = moi.verify_against(&da_luu).expect_err("phải từ chối");
    assert!(matches!(e, RegistryError::PackAbsent(_)), "{e}");
}

#[test]
fn pack_set_theo_thu_tu_nap() {
    let mut r = Registry::new();
    r.add(mf("zz", &["aa"], &[]), &khong_co_file()).unwrap();
    r.add(mf("aa", &[], &[]), &khong_co_file()).unwrap();
    r.resolve_order().unwrap();
    let ps = r.pack_set();
    assert_eq!(
        ps.entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
        vec!["aa", "zz"]
    );
}
