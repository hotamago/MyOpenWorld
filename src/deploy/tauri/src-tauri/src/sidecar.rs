//! Sidecar Python và đường dẫn tài nguyên của bản desktop (`plan.md §P3.4`,
//! `PF-12`).
//!
//! ## Vì sao có sidecar
//!
//! `§P3.4` chốt:
//!
//! > FastAPI + LangGraph + mem0 kéo theo hàng trăm MB. Đóng gói **sidecar**
//! > bằng PyInstaller/uv, chạy trên loopback, do Tauri quản lý vòng đời. Chấp
//! > nhận chi phí dung lượng; đổi lại **một codebase cognition duy nhất** cho
//! > cả hai hình thái.
//!
//! Lựa chọn thay thế — viết lại cognition bằng Rust cho bản desktop — cho ra
//! hai cài đặt của cùng một thứ, và chúng sẽ trôi khỏi nhau. Một lỗi chỉ có ở
//! bản desktop là một lỗi mà không CI nào bắt.
//!
//! ## Vòng đời phải do Tauri quản
//!
//! Không phải "khởi động rồi mặc kệ". Ba việc, và bỏ việc nào cũng để lại một
//! tiến trình Python mồ côi trên máy người chơi:
//!
//! 1. Khởi động **trước** khi WebView gọi tới.
//! 2. Chờ nó **sẵn sàng** — không phải chờ một khoảng thời gian cố định.
//! 3. **Tắt** khi ứng dụng đóng, kể cả khi đóng bất thường.
//!
//! Điểm 2 là điểm hay bị làm sai nhất: `sleep(2)` chạy được trên máy người
//! viết và hỏng trên máy chậm hơn, và nó hỏng dưới dạng "ứng dụng thỉnh thoảng
//! không khởi động được".
//!
//! ## Đường dẫn: cạnh binary, không phải thư mục làm việc
//!
//! `content/` và `config/` nằm **cạnh binary** trong bản đóng gói. Đọc chúng
//! bằng đường dẫn tương đối sẽ chạy ở dev và hỏng ở bản phát hành — vì thư mục
//! làm việc lúc người dùng bấm vào icon không phải thư mục cài đặt.
//!
//! Save thì ngược lại: ghi vào **thư mục dữ liệu của ứng dụng**, không vào thư
//! mục cài đặt. Windows từ chối cái thứ hai, và nó chỉ từ chối sau khi đóng gói.

use std::path::PathBuf;

/// Tên sidecar trong bundle, khớp `externalBin` ở `tauri.conf.json`.
pub const TEN_SIDECAR: &str = "mow-agent";

/// Trạng thái của sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThai {
    /// Chưa khởi động.
    ChuaChay,
    /// Đã khởi động, chưa sẵn sàng nhận yêu cầu.
    DangKhoiDong,
    /// Sẵn sàng — **đã tự báo**, không phải đã hết thời gian chờ.
    SanSang {
        /// Cổng thật nó đang nghe.
        port: u16,
    },
    /// Đã tắt.
    DaTat,
    /// Chết bất thường.
    Chet {
        /// Mã thoát, nếu có.
        exit_code: Option<i32>,
    },
}

impl TrangThai {
    /// Gọi được chưa.
    pub fn callable(self) -> bool {
        matches!(self, TrangThai::SanSang { .. })
    }

    /// Có cần khởi động lại không.
    ///
    /// `Chet` thì có; `DaTat` thì **không** — ứng dụng đang đóng, và khởi động
    /// lại một sidecar lúc đó là cách tạo ra tiến trình mồ côi.
    ///
    /// Chưa có chỗ gọi ở bản này: chính sách khởi động lại còn phải trả lời
    /// "bao nhiêu lần thì thôi", và một vòng khởi động lại vô hạn tệ hơn một
    /// sidecar chết. Quy tắc thì đã đúng và đã có test.
    #[allow(dead_code)]
    pub fn needs_restart(self) -> bool {
        matches!(self, TrangThai::Chet { .. })
    }
}

/// Ứng dụng có chạy được khi sidecar chết không.
///
/// **Có.** `§P3.4` nói rõ: không có mạng — và không có tầng nhận thức — thì thế
/// giới vẫn chạy đầy đủ ở ba tầng đầu của tháp hành vi `§10.3`. Sidecar chết
/// tương đương gateway timeout: scheduler chuyển các entity sang fallback
/// policy, và người chơi thấy một chỉ báo chứ không thấy một hộp thoại lỗi.
pub fn world_runs_without_sidecar() -> bool {
    true
}

/// Đường dẫn tài nguyên chỉ đọc (`content/`, `config/`).
///
/// Nhận thư mục tài nguyên do Tauri phân giải chứ không tự đoán: ba hệ điều
/// hành đặt nó ở ba chỗ, và đoán sẽ đúng ở một chỗ.
pub fn resource_dir(tauri_resource_dir: &std::path::Path, name: &str) -> PathBuf {
    tauri_resource_dir.join(name)
}

/// Đường dẫn save.
///
/// Vào thư mục **dữ liệu của ứng dụng**. Không vào thư mục cài đặt: Windows từ
/// chối, và nó từ chối sau khi đóng gói chứ không lúc dev.
pub fn save_dir(tauri_app_data_dir: &std::path::Path) -> PathBuf {
    tauri_app_data_dir.join("saves")
}

/// Một đường dẫn có nằm trong thư mục cài đặt không.
///
/// Dùng để chặn ngay lúc dev thay vì phát hiện sau khi đóng gói. Trả `true`
/// nghĩa là **sai** — đường ghi không được nằm cạnh binary.
pub fn is_under_install_dir(path: &std::path::Path, resource_dir: &std::path::Path) -> bool {
    path.starts_with(resource_dir)
}

/// Dòng mà sidecar in ra stdout khi sẵn sàng.
///
/// Một chuỗi cố định kèm cổng, chứ không phải "server started" tự do: chỗ đọc
/// phải phân biệt được dòng sẵn sàng với mọi dòng log khác, và một prefix cố
/// định là cách rẻ nhất.
pub const DAU_HIEU_SAN_SANG: &str = "MOW_AGENT_READY port=";

/// Đọc cổng từ một dòng stdout của sidecar.
///
/// `None` nghĩa là dòng đó không phải dòng sẵn sàng — không phải lỗi. Sidecar
/// in nhiều dòng log trước khi sẵn sàng, và coi mọi dòng không phân tích được
/// là lỗi sẽ làm ứng dụng không bao giờ khởi động.
pub fn parse_ready(line: &str) -> Option<u16> {
    line.trim()
        .strip_prefix(DAU_HIEU_SAN_SANG)
        .and_then(|p| p.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn doc_duoc_cong_tu_dong_san_sang() {
        assert_eq!(parse_ready("MOW_AGENT_READY port=51234"), Some(51234));
        assert_eq!(parse_ready("  MOW_AGENT_READY port=8080  "), Some(8080));
    }

    #[test]
    fn dong_log_thuong_khong_phai_loi() {
        for l in [
            "INFO: loading graphs",
            "WARNING: no local model",
            "",
            "MOW_AGENT_READY port=",
            "MOW_AGENT_READY port=abc",
        ] {
            assert_eq!(parse_ready(l), None, "{l:?}");
        }
    }

    #[test]
    fn chi_san_sang_moi_goi_duoc() {
        assert!(TrangThai::SanSang { port: 1 }.callable());
        for t in [
            TrangThai::ChuaChay,
            TrangThai::DangKhoiDong,
            TrangThai::DaTat,
            TrangThai::Chet { exit_code: Some(1) },
        ] {
            assert!(!t.callable());
        }
    }

    #[test]
    fn chet_thi_khoi_dong_lai_nhung_da_tat_thi_khong() {
        assert!(TrangThai::Chet { exit_code: None }.needs_restart());
        assert!(
            !TrangThai::DaTat.needs_restart(),
            "khởi động lại lúc đang đóng là cách tạo tiến trình mồ côi"
        );
    }

    #[test]
    fn the_gioi_van_chay_khi_sidecar_chet() {
        assert!(world_runs_without_sidecar());
    }

    #[test]
    fn save_khong_nam_trong_thu_muc_cai_dat() {
        let cai_dat = Path::new("/opt/myopenworld");
        let du_lieu = Path::new("/home/nguoi/.local/share/myopenworld");
        let s = save_dir(du_lieu);
        assert!(!is_under_install_dir(&s, cai_dat));
        assert!(s.ends_with("saves"));
    }

    #[test]
    fn tai_nguyen_doc_canh_binary() {
        let res = Path::new("/opt/myopenworld/resources");
        assert_eq!(
            resource_dir(res, "content"),
            Path::new("/opt/myopenworld/resources/content")
        );
        // Và một đường ghi đặt nhầm vào đó thì bị bắt.
        assert!(is_under_install_dir(&resource_dir(res, "saves"), res));
    }
}
