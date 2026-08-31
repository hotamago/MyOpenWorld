//! Content pack của **bên thứ ba** nạp được và không đổi hash của world không
//! dùng nó (`PF-13`, `§19.7`, `§22.30`).
//!
//! Đây là bài kiểm mà tài liệu modder không tự chứng minh được, và là bài kiểm
//! quyết định việc có ai dám cài mod hay không:
//!
//! > Nếu chỉ cần **có mặt** trong thư mục mod là hash đổi, thì mọi người chơi
//! > cài thêm một mod sẽ không mở lại được save cũ.
//!
//! Bài chạy trên `content/example-thirdparty` thật trên đĩa, không trên một
//! manifest dựng trong test. Một pack mẫu mà chỉ tồn tại trong test thì không
//! chứng minh được cây thư mục thật nạp được.

use mow_plugin::{PackSet, Registry};
use std::path::{Path, PathBuf};

fn goc() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content")
}

fn chi_core() -> Registry {
    let mut r = Registry::new();
    r.add_from_dir(goc().join("core")).expect("core nạp được");
    r.resolve_order().expect("core không có phụ thuộc vòng");
    r
}

fn ca_hai() -> Registry {
    let mut r = Registry::new();
    r.add_from_dir(goc().join("core")).expect("core nạp được");
    r.add_from_dir(goc().join("example-thirdparty"))
        .expect("pack bên thứ ba nạp được");
    r.resolve_order().expect("giải được thứ tự");
    r
}

// ───────────────── pack bên thứ ba nạp được ─────────────────

/// **Pack của cộng đồng đi qua đúng cơ chế mà `content/core` đi qua.**
#[test]
fn pack_ben_thu_ba_nap_duoc_qua_dung_co_che() {
    let r = ca_hai();
    assert_eq!(r.len(), 2);
    assert!(r.manifest("example_thirdparty").is_some());
}

/// Thứ tự nạp giải đúng: phụ thuộc trước.
#[test]
fn phu_thuoc_nap_truoc() {
    let thu_tu = ca_hai().resolve_order().unwrap();
    assert_eq!(thu_tu.0, vec!["core", "example_thirdparty"]);
}

/// **Pack có `laws/` phải xin `define_law`** — và pack mẫu xin đúng.
#[test]
fn pack_mau_xin_dung_quyen_no_can() {
    let r = ca_hai();
    let q = r.grants_of("example_thirdparty").expect("đã nạp");
    assert!(q.has(mow_plugin::Capability::DefineLaw));
    assert!(
        !q.has(mow_plugin::Capability::OverrideForeign),
        "pack mẫu không ghi đè của ai, nên không xin quyền đó"
    );
}

/// Pack mẫu **hiện lên trong danh sách rủi ro** — nó có chạm luật.
#[test]
fn pack_mau_hien_len_trong_danh_sach_rui_ro() {
    let r = ca_hai();
    let rui_ro = r.packs_with_risky_capabilities();
    assert!(
        rui_ro.iter().any(|(id, _)| *id == "example_thirdparty"),
        "một pack viết luật phải hiện lên trước khi người dùng bấm cài"
    );
}

/// Mọi id của pack mẫu **có namespace của chính nó** (`§22.29`).
#[test]
fn moi_id_cua_pack_mau_co_namespace_cua_chinh_no() {
    let mut r = ca_hai();
    r.define("example_thirdparty", "example_thirdparty.rye_loaf")
        .expect("id có namespace đúng thì đăng ký được");
    assert_eq!(
        r.owner_of("example_thirdparty.rye_loaf"),
        Some("example_thirdparty")
    );

    // Và nó **không** đăng ký được một id thuộc core.
    assert!(
        r.define("example_thirdparty", "core.apple").is_err(),
        "pack không khai overrides mà đăng ký id của core thì phải bị chặn"
    );
}

// ───────── không đổi hash của world không dùng nó ─────────

/// **Bài kiểm trung tâm của `PF-13`.**
///
/// Một world tạo bằng `core` thôi, rồi người chơi cài thêm pack bên thứ ba.
/// Save cũ phải mở lại được — nếu không thì cài mod là một quyết định không
/// hoàn tác được, và không ai dám cài.
#[test]
fn cai_them_pack_khong_lam_hong_save_cu() {
    // Trước khi cài mod: world lưu pack set của nó.
    let luu: PackSet = chi_core().pack_set();
    assert_eq!(luu.entries.len(), 1);

    // Sau khi cài mod: cùng save, môi trường có thêm một pack.
    ca_hai()
        .verify_against(&luu)
        .expect("save chỉ dùng core phải mở được khi máy có thêm pack khác");
}

/// **Hash của `core` không đổi vì có pack khác nằm cạnh.**
///
/// Vế mạnh hơn: không chỉ "save mở được" mà là "con số y hệt". Nếu hash của
/// core phụ thuộc vào những gì nằm cạnh nó thì mọi bảo đảm khác đều lung lay.
#[test]
fn hash_cua_core_khong_doi_vi_co_pack_khac_nam_canh() {
    assert_eq!(
        chi_core().hash_of("core"),
        ca_hai().hash_of("core"),
        "content hash của một pack chỉ được phụ thuộc vào nội dung của chính nó"
    );
}

/// Thứ tự nạp của `core` cũng không đổi.
#[test]
fn thu_tu_nap_cua_core_khong_doi() {
    let a = chi_core().pack_set();
    let b = ca_hai().pack_set();
    assert_eq!(a.entries[0], b.entries[0]);
}

/// **Ngược lại: một world CÓ dùng pack mẫu thì phát hiện được khi nó biến mất.**
///
/// Vế đối chứng. Nếu bỏ nó thì một cài đặt luôn trả `Ok` cũng qua được bài
/// trên, và cả cơ chế `§22.30` thành trang trí.
#[test]
fn world_co_dung_pack_mau_thi_phat_hien_duoc_khi_no_bien_mat() {
    let luu = ca_hai().pack_set();
    assert_eq!(luu.entries.len(), 2);
    assert!(
        chi_core().verify_against(&luu).is_err(),
        "save dùng pack mẫu mà máy không có nó thì phải từ chối, không nạp một nửa"
    );
}

/// Sửa nội dung pack mẫu mà giữ version thì save cũ **không** mở được.
///
/// Dùng registry dựng tay chứ không sửa file trên đĩa: test không được để lại
/// dấu vết, và một test sửa `content/` sẽ làm test kế tiếp chạy trên dữ liệu
/// khác.
#[test]
fn sua_noi_dung_pack_mau_ma_giu_version_thi_bi_bat() {
    let luu = ca_hai().pack_set();

    let mut da_sua = Registry::new();
    da_sua.add_from_dir(goc().join("core")).unwrap();
    let mut m = ca_hai()
        .manifest("example_thirdparty")
        .expect("đã nạp")
        .clone();
    m.description.push_str(" (ai đó sửa)");
    da_sua.add(m, &std::collections::BTreeMap::new()).unwrap();

    assert!(da_sua.verify_against(&luu).is_err());
}
