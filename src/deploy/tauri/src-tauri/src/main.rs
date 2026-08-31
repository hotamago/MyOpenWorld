//! Ban desktop (`PA-09` smoke build → `PF-12` dong goi day du).
//!
//! `PA-09` chung minh frontend dong goi duoc, duong dan file dung, va WebView
//! noi duoc toi backend nhung. `PF-12` them ba thu con lai cua `§P3.4`:
//!
//! - **Sidecar Python** co vong doi do Tauri quan — xem [`sidecar`].
//! - **Duong dan tai nguyen** phan giai qua Tauri, khong doan.
//! - **`tauri-driver`** cho duong rieng cua desktop — xem `tests/`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod sidecar;
mod supervisor;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // `content/` va `config/` nam CANH BINARY trong ban dong goi, khong
            // o thu muc lam viec. Doc chung bang duong dan tuong doi se chay o
            // dev va hong o ban phat hanh — mot trong ba lop loi ma ban desktop
            // ton tai de bat som.
            let res = app.path().resource_dir()?;
            let content = sidecar::resource_dir(&res, "content");
            let config = sidecar::resource_dir(&res, "config");

            // Save di vao THU MUC DU LIEU, khong vao thu muc cai dat. Windows tu
            // choi cai thu hai, va no chi tu choi sau khi dong goi.
            let data = app.path().app_data_dir()?;
            let saves = sidecar::save_dir(&data);
            std::fs::create_dir_all(&saves)?;

            // Kiem ngay luc khoi dong thay vi de phat hien sau khi phat hanh.
            // Mot duong ghi nam trong thu muc cai dat la mot loi bao duoc o day
            // bang mot dong, va bao duoc o cho khac bang mot bao cao loi tu
            // nguoi choi.
            assert!(
                !sidecar::is_under_install_dir(&saves, &res),
                "duong save khong duoc nam trong thu muc cai dat"
            );

            tracing_lite(&format!(
                "content={} config={} saves={}",
                content.display(),
                config.display(),
                saves.display()
            ));

            // Sidecar Python (`§P3.4`). Vong doi do Tauri quan: khoi dong o
            // day, tat khi ung dung dong — bao dam bang `Drop`, khong bang mot
            // loi goi ma cho goi phai nho.
            //
            // Khong co no thi the gioi VAN CHAY o ba tang dau cua thap hanh vi
            // (`§10.3`), nen `start` tra `Ok` ca khi khong tim thay binary.
            let mut sup = supervisor::Supervisor::new();
            match sup.start(&res) {
                Ok(t) => tracing_lite(&format!("sidecar: {t:?}")),
                Err(e) => tracing_lite(&format!("sidecar khong khoi dong duoc: {e}")),
            }
            if !sup.cognition_available() {
                debug_assert!(sidecar::world_runs_without_sidecar());
                tracing_lite(
                    "khong co tang nhan thuc — the gioi chay o ba tang dau cua §10.3, \
                     cac entity dung fallback policy nhu khi gateway timeout",
                );
            }
            // Giu `Supervisor` song bang tuoi tho cua app. Tha no o day se giet
            // sidecar ngay lap tuc qua `Drop`.
            app.manage(std::sync::Mutex::new(sup));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("khong khoi dong duoc ung dung");
}

/// Ghi mot dong ra stderr.
///
/// Khong keo `tracing` vao ban desktop chi de in ba duong dan: moi crate them
/// vao day deu vao ban dong goi ma nguoi choi tai ve.
fn tracing_lite(msg: &str) {
    eprintln!("[mow-desktop] {msg}");
}
