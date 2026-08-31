//! Test kien truc: devtool khong duoc lot vao ban phat hanh (`plan.md §P10.5`).
//!
//! Co ba lop bao ve, va bai test nay giu lop thu ba:
//!
//! 1. Feature `devtool` **tat mac dinh**.
//! 2. `deploy/docker/server.Dockerfile` quet symbol trong binary release va
//!    fail build neu tim thay.
//! 3. Bai test nay: khong crate nao se duoc dong goi lai phu thuoc **vo dieu
//!    kien** vao `mow-devtool`.
//!
//! Lop thu ba can thiet vi feature flag co the bi bat nham qua mot phu thuoc
//! bac cau. Cargo hop nhat feature: neu `mow-server` phu thuoc `mow-devtool`
//! khong dieu kien, thi feature `devtool` cua no se bat o moi build, ke ca
//! `--release`, va khong ai nhan ra cho toi khi quet binary.

use std::path::{Path, PathBuf};

/// Nhung crate se duoc dong goi vao ban phat hanh.
const SE_PHAT_HANH: &[&str] = &["mow-server", "mow-worker"];

fn goc() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Feature `devtool` khong duoc nam trong `default`.
///
/// Kiem tren **manifest** chu khong tren `cfg!(feature = ...)`. Ly do rat cu
/// the: `cargo test --all-features` — chinh la lenh ma `make test-rust` chay —
/// bat moi feature len, nen mot bai test dua vao `cfg!` se do o do. Va no do
/// **dung**: `--all-features` la mot lan bat tuong minh, tuc la khong vi pham gi.
///
/// Mot bai test do trong tinh huong hop le la mot bai test se bi tat, va luc do
/// lop bao ve nay bien mat. Nen no doc thang cai ma no thuc su muon khang dinh:
/// dong `default = [...]` trong `Cargo.toml`.
#[test]
fn feature_devtool_khong_nam_trong_default() {
    let mp = goc().join("crates/mow-devtool/Cargo.toml");
    let text = std::fs::read_to_string(&mp).expect("co Cargo.toml");

    let dong_default = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("default"))
        .unwrap_or("default = []");

    assert!(
        !dong_default.contains("devtool"),
        "`devtool` nam trong feature mac dinh cua mow-devtool: {dong_default}
         No phai duoc bat tuong minh, neu khong ban release se mang theo cong go loi."
    );
}

#[test]
fn khong_crate_phat_hanh_nao_phu_thuoc_devtool_vo_dieu_kien() {
    let crates_dir = goc().join("crates");
    let mut vi_pham = Vec::new();

    for ten in SE_PHAT_HANH {
        let mp = crates_dir.join(ten).join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&mp) else {
            // Crate chua ton tai (Giai doan 0). Bai test van co gia tri: no se
            // bat dau kiem ngay khi crate do duoc tao ra.
            continue;
        };
        for (i, dong) in text.lines().enumerate() {
            let cat = dong.trim();
            if cat.starts_with('#') || !cat.contains("mow-devtool") {
                continue;
            }
            // Phu thuoc co dieu kien phai nam duoi `[target...]` hoac duoc khai
            // bao `optional = true` roi keo vao qua feature.
            if !cat.contains("optional = true") {
                vi_pham.push(format!("{}:{}: {}", mp.display(), i + 1, cat));
            }
        }
    }

    assert!(
        vi_pham.is_empty(),
        "crate se phat hanh phu thuoc `mow-devtool` vo dieu kien:\n{}\n\
         Cargo hop nhat feature, nen phu thuoc nay se bat `devtool` o ca ban release.",
        vi_pham.join("\n")
    );
}

#[test]
fn dockerfile_van_con_buoc_quet_symbol() {
    // Lop bao ve thu hai phai con nguyen. Xoa no di thi hai lop con lai deu la
    // kiem tra o muc nguon, va khong con gi kiem tra chinh cai binary.
    let p = goc().join("deploy/docker/server.Dockerfile");
    let text = std::fs::read_to_string(&p).expect("co server.Dockerfile");
    assert!(
        text.contains("strings") && text.contains("mow_devtool"),
        "server.Dockerfile mat buoc quet symbol devtool trong binary release"
    );
}
