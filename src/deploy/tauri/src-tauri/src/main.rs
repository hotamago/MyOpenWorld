//! Ban desktop.
//!
//! Muc dich cua no o Giai doan A la mot smoke build (`PA-09`): chung minh rang
//! frontend dong goi duoc, duong dan file dung, va WebView noi duoc toi backend
//! nhung. Chuc nang day du toi o `PF-12`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            // Duong dan `content/` va `config/` nam canh binary trong ban dong
            // goi, khong o thu muc lam viec. Doc chung bang duong dan tuong doi
            // se chay o dev va hong o ban phat hanh — day la mot trong ba lop
            // loi ma `PA-09` ton tai de bat som.
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("khong khoi dong duoc ung dung");
}
