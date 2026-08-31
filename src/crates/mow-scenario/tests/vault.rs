//! Test Seed Vault (`PF-04`, `§7.6.5`).

use mow_scenario::vault::{diff, preview, same_world, Impact, SeedVault, VaultEntry, VaultError};
use mow_scenario::worldseed::{GenesisStep, Worldseed};
use std::collections::{BTreeMap, BTreeSet};

fn hat(id: &str) -> Worldseed {
    Worldseed {
        id: id.to_owned(),
        version: 1,
        description: "một thung lũng ôn hòa".to_owned(),
        generation_profile: "gaia.temperate".to_owned(),
        seed: Some(4_242),
        packs: vec!["core".to_owned()],
        genesis: vec![GenesisStep {
            command: "spawn_settlement".to_owned(),
            args: BTreeMap::new(),
            name: Some("veskar".to_owned()),
        }],
        named_entities: BTreeMap::new(),
    }
}

fn muc(id: &str) -> VaultEntry {
    VaultEntry {
        seed: hat(id),
        tags: BTreeSet::from(["ôn hòa".to_owned()]),
        forked_from: None,
        origin: "official".to_owned(),
    }
}

fn kho() -> SeedVault {
    let mut v = SeedVault::new();
    v.add(muc("gaia:temperate_valley")).unwrap();
    let mut khac = muc("gaia:frozen_north");
    khac.seed.description = "băng giá và ít người".to_owned();
    khac.tags = BTreeSet::from(["khắc nghiệt".to_owned()]);
    khac.origin = "community".to_owned();
    v.add(khac).unwrap();
    v
}

// ───────────────────── duyệt, tìm, gắn thẻ ─────────────────────

/// Tìm theo chữ trong id lẫn mô tả.
#[test]
fn tim_theo_chu_trong_id_lan_mo_ta() {
    let v = kho();
    assert_eq!(v.search("frozen", &BTreeSet::new()).len(), 1);
    assert_eq!(v.search("băng giá", &BTreeSet::new()).len(), 1);
    assert_eq!(v.search("", &BTreeSet::new()).len(), 2);
}

/// **Thứ tự tìm ổn định** — đảo thứ tự giữa hai lần tìm làm người dùng mất chỗ.
#[test]
fn thu_tu_tim_on_dinh() {
    let v = kho();
    let a: Vec<&str> = v
        .search("", &BTreeSet::new())
        .iter()
        .map(|e| e.seed.id.as_str())
        .collect();
    let b: Vec<&str> = v
        .search("", &BTreeSet::new())
        .iter()
        .map(|e| e.seed.id.as_str())
        .collect();
    assert_eq!(a, b);
    assert_eq!(a, vec!["gaia:frozen_north", "gaia:temperate_valley"]);
}

/// Lọc theo thẻ, và nhiều thẻ là **giao**, không phải hợp.
#[test]
fn loc_theo_the_la_giao_khong_phai_hop() {
    let mut v = kho();
    v.tag("gaia:frozen_north", "cộng đồng").unwrap();

    let mot_the = BTreeSet::from(["khắc nghiệt".to_owned()]);
    assert_eq!(v.search("", &mot_the).len(), 1);

    let hai_the = BTreeSet::from(["khắc nghiệt".to_owned(), "cộng đồng".to_owned()]);
    assert_eq!(v.search("", &hai_the).len(), 1);

    let khong_khop = BTreeSet::from(["khắc nghiệt".to_owned(), "ôn hòa".to_owned()]);
    assert!(v.search("", &khong_khop).is_empty());
}

/// Gắn thẻ cho mục không có là lỗi, không im lặng.
#[test]
fn gan_the_cho_muc_khong_co_la_loi() {
    assert!(matches!(
        kho().tag("khong:ton_tai", "x").unwrap_err(),
        VaultError::NotFound(_)
    ));
}

// ───────────────────── fork giữ quan hệ cha–con ─────────────────────

/// **Fork giữ nguyên quan hệ cha–con.**
#[test]
fn fork_giu_quan_he_cha_con() {
    let mut v = kho();
    v.fork("gaia:temperate_valley", "gaia:my_valley").unwrap();
    let con = v.get("gaia:my_valley").unwrap();
    assert_eq!(con.forked_from.as_deref(), Some("gaia:temperate_valley"));
    assert_eq!(
        con.origin, "local",
        "bản sửa của mình không còn là official"
    );
}

/// **Fork bắt đầu lại version từ 1** — không tiếp số của cha.
#[test]
fn fork_bat_dau_lai_version_tu_1() {
    let mut v = SeedVault::new();
    let mut cha = muc("a");
    cha.seed.version = 7;
    v.add(cha).unwrap();
    v.fork("a", "b").unwrap();
    assert_eq!(
        v.get("b").unwrap().seed.version,
        1,
        "tiếp số của cha thì hai dòng sẽ đụng số nhau sau vài lần sửa"
    );
}

/// Chuỗi tổ tiên qua nhiều đời.
#[test]
fn chuoi_to_tien_qua_nhieu_doi() {
    let mut v = kho();
    v.fork("gaia:temperate_valley", "b").unwrap();
    v.fork("b", "c").unwrap();
    v.fork("c", "d").unwrap();
    assert_eq!(v.ancestry("d"), vec!["c", "b", "gaia:temperate_valley"]);
    assert!(v.ancestry("gaia:temperate_valley").is_empty());
}

/// Fork trùng id bị từ chối.
#[test]
fn fork_trung_id_bi_tu_choi() {
    let mut v = kho();
    assert!(matches!(
        v.fork("gaia:temperate_valley", "gaia:frozen_north")
            .unwrap_err(),
        VaultError::Duplicate(_)
    ));
}

// ───────────────────── diff ở mức dữ liệu ─────────────────────

/// **Đổi mô tả không đổi thế giới.**
///
/// `diff` văn bản sẽ báo một dòng đổi; câu trả lời đúng là *"cùng một thế
/// giới"*.
#[test]
fn doi_mo_ta_khong_doi_the_gioi() {
    let a = hat("x");
    let mut b = hat("x");
    b.description = "viết lại cho hay hơn".to_owned();

    let d = diff(&a, &b);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].impact, Impact::Cosmetic);
    assert!(same_world(&a, &b));
}

/// **Đổi seed là đổi cả thế giới**, dù chỉ một dòng.
#[test]
fn doi_seed_la_doi_ca_the_gioi() {
    let a = hat("x");
    let mut b = hat("x");
    b.seed = Some(9_999);

    let d = diff(&a, &b);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].impact, Impact::WholeWorld);
    assert!(!same_world(&a, &b));
}

/// Đổi generation profile hoặc pack cũng là đổi cả thế giới.
#[test]
fn doi_profile_hoac_pack_la_doi_ca_the_gioi() {
    for sua in [
        |w: &mut Worldseed| w.generation_profile = "gaia.volcanic".to_owned(),
        |w: &mut Worldseed| w.packs.push("bakery".to_owned()),
    ] {
        let a = hat("x");
        let mut b = hat("x");
        sua(&mut b);
        assert!(!same_world(&a, &b));
    }
}

/// Đổi bước genesis là đổi điều kiện ban đầu, **không** đổi địa hình nền.
#[test]
fn doi_genesis_la_doi_dieu_kien_ban_dau() {
    let a = hat("x");
    let mut b = hat("x");
    b.genesis.push(GenesisStep {
        command: "spawn_settlement".to_owned(),
        args: BTreeMap::new(),
        name: Some("thứ hai".to_owned()),
    });
    let d = diff(&a, &b);
    assert_eq!(d[0].impact, Impact::InitialConditions);
    assert!(!same_world(&a, &b));
}

/// Hai worldseed giống hệt thì không có khác biệt nào.
#[test]
fn hai_worldseed_giong_het_thi_khong_co_khac_biet() {
    assert!(diff(&hat("x"), &hat("x")).is_empty());
    assert!(same_world(&hat("x"), &hat("x")));
}

// ───────────────────── preview và báo cáo rủi ro ─────────────────────

fn co_core() -> BTreeSet<String> {
    BTreeSet::from(["core".to_owned()])
}

/// Preview nói rõ seed đã giải và các pack cần.
#[test]
fn preview_noi_ro_seed_da_giai_va_pack_can() {
    let p = preview(&hat("x"), &co_core());
    assert_eq!(p.resolved_seed, 4_242);
    assert_eq!(p.required_packs, vec![("core".to_owned(), true)]);
    assert_eq!(p.genesis_steps, 1);
    assert!(p.creatable());
}

/// **Thiếu pack thì báo trước khi tạo, không lỗi giữa chừng.**
#[test]
fn thieu_pack_thi_bao_truoc_khi_tao() {
    let mut w = hat("x");
    w.packs.push("bakery".to_owned());
    let p = preview(&w, &co_core());

    assert!(!p.creatable());
    let chan = p.blockers();
    assert_eq!(chan.len(), 1);
    assert_eq!(chan[0].code, "pack.missing");
    assert!(chan[0].detail.contains("bakery"));
}

/// **Cảnh báo khác chặn hẳn.**
///
/// Một thế giới trống là lựa chọn hợp lệ; một thế giới thiếu pack thì tạo sẽ
/// hỏng giữa chừng. Gộp hai cái làm một sẽ hoặc chặn thứ nên cho, hoặc cho
/// thứ nên chặn.
#[test]
fn canh_bao_khac_chan_han() {
    let mut trong = hat("x");
    trong.genesis.clear();
    let p = preview(&trong, &co_core());

    assert!(!p.risks.is_empty());
    assert!(p.blockers().is_empty());
    assert!(p.creatable(), "một world trống là lựa chọn hợp lệ");
    assert_eq!(p.risks[0].code, "genesis.empty");
}

/// Worldseed sai hình dạng: **mỗi lỗi một dòng**, không gộp thành "không hợp lệ".
#[test]
fn worldseed_sai_hinh_dang_moi_loi_mot_dong() {
    let mut sai = hat("");
    sai.generation_profile = String::new();
    let p = preview(&sai, &co_core());
    assert!(!p.creatable());
    assert!(p.blockers().len() >= 2, "phải liệt từng lỗi: {:?}", p.risks);
}

// ───────────────────── xuất/nhập có checksum ─────────────────────

/// Xuất rồi nhập lại thì ra đúng cái cũ.
#[test]
fn xuat_roi_nhap_lai_ra_dung_cai_cu() {
    let v = kho();
    let goi = v.export("gaia:temperate_valley").unwrap();

    let mut kho_moi = SeedVault::new();
    kho_moi.import(&goi).unwrap();
    assert_eq!(
        kho_moi.get("gaia:temperate_valley").unwrap(),
        v.get("gaia:temperate_valley").unwrap()
    );
}

/// **Gói hỏng bị bắt, và bị bắt trước khi vào kho.**
#[test]
fn goi_hong_bi_bat_truoc_khi_vao_kho() {
    let v = kho();
    let mut goi = v.export("gaia:temperate_valley").unwrap();
    // Ai đó sửa nội dung trên đường tải mà không sửa checksum.
    goi.entry.seed.seed = Some(1);

    let mut kho_moi = SeedVault::new();
    assert!(matches!(
        kho_moi.import(&goi).unwrap_err(),
        VaultError::ChecksumMismatch { .. }
    ));
    assert!(
        kho_moi.is_empty(),
        "gói hỏng mà vẫn vào kho thì người dùng phải tự xóa nó"
    );
}

/// Checksum bắt được cả thay đổi rất nhỏ.
#[test]
fn checksum_bat_duoc_ca_thay_doi_rat_nho() {
    let v = kho();
    let a = v.export("gaia:temperate_valley").unwrap();
    let mut b = a.clone();
    b.entry.seed.description.push(' ');
    assert_ne!(a.entry.hash(), b.entry.hash());
}

/// **Checksum truyền tải khác hash danh tính thế giới.**
///
/// Trộn hai cái làm một là một lỗi im lặng: hash danh tính cố tình bỏ qua mô
/// tả (mô tả không đổi thế giới), nên dùng nó làm checksum sẽ để lọt một gói
/// hỏng đúng ở phần mô tả, thẻ, hoặc quan hệ cha–con.
#[test]
fn checksum_truyen_tai_khac_hash_danh_tinh_the_gioi() {
    let a = muc("x");
    let mut b = muc("x");
    b.seed.description = "viết lại".to_owned();

    assert_eq!(
        a.world_identity(),
        b.world_identity(),
        "đổi mô tả vẫn là cùng một thế giới"
    );
    assert_ne!(a.hash(), b.hash(), "nhưng không phải cùng một file");
}

/// Checksum bắt cả hư hại ở thẻ và quan hệ cha–con, không chỉ ở worldseed.
#[test]
fn checksum_bat_ca_hu_hai_ngoai_worldseed() {
    let goc = muc("x");
    for hong in [
        {
            let mut e = muc("x");
            e.tags.insert("bịa".to_owned());
            e
        },
        {
            let mut e = muc("x");
            e.forked_from = Some("ai đó".to_owned());
            e
        },
        {
            let mut e = muc("x");
            e.origin = "community".to_owned();
            e
        },
    ] {
        assert_ne!(goc.hash(), hong.hash());
        assert_eq!(
            goc.world_identity(),
            hong.world_identity(),
            "những trường này không đổi thế giới — nên identity phải giữ nguyên"
        );
    }
}
