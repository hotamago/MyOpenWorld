//! Màu: chuỗi trong file, số nguyên trong bộ nhớ.
//!
//! ## Vì sao chuyển đổi xảy ra một lần, lúc nạp
//!
//! File dùng `"#6b5a3e"` vì người ta đọc và sửa được nó. Mô phỏng dùng
//! `0x006b5a3e` vì so sánh, băm và trộn màu đều là phép trên số nguyên. Nếu để
//! chuỗi sống tới lúc vẽ thì mỗi frame phải phân tích lại nó, và một chuỗi sai
//! định dạng sẽ nổ ở giữa vòng vẽ thay vì lúc mở pack.
//!
//! ## Vì sao không có số thực ở đây
//!
//! Màu đi vào content hash của pack, và hash đó nằm trong save. Số thực làm
//! tròn khác nhau giữa các nền tảng, nên một `f32` trong đường này sẽ làm cùng
//! một pack cho hai hash trên hai máy. Toàn bộ mô-đun này là `u32`.

/// Đọc màu dạng `#RRGGBB` thành `0x00RRGGBB`.
///
/// Trả về lý do dưới dạng chuỗi thay vì một kiểu lỗi riêng: người gọi biết mình
/// đang đọc file nào và trường nào, còn hàm này thì không. Ghép hai nửa lại là
/// việc của [`crate::error::ContentError::BadField`].
///
/// Chỉ nhận đúng sáu chữ số hex. Không nhận dạng rút gọn ba ký tự và không nhận
/// kênh alpha: cả hai đều làm một bảng màu có hai cách viết cho cùng một màu, và
/// hai cách viết nghĩa là hai byte khác nhau đi vào content hash.
pub fn parse_hex_color(text: &str) -> Result<u32, String> {
    let Some(digits) = text.strip_prefix('#') else {
        return Err("thiếu dấu `#` ở đầu, màu phải viết dạng `#RRGGBB`".to_owned());
    };

    // Kiểm ký tự trước, độ dài sau. Ngược lại thì một chuỗi có ký tự nhiều byte
    // sẽ bị báo sai độ dài, và người sửa sẽ đi tìm nhầm chỗ.
    if let Some(bad) = digits.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(format!("`{bad}` không phải chữ số hex"));
    }
    if digits.len() != 6 {
        return Err(format!(
            "cần đúng 6 chữ số hex sau `#`, nhận được {}",
            digits.len()
        ));
    }

    u32::from_str_radix(digits, 16).map_err(|e| e.to_string())
}

/// Viết `0x00RRGGBB` trở lại dạng `#RRGGBB`.
///
/// Có mặt để bảng màu chỉ còn **một** nguồn: tầng vẽ sinh được bảng của mình từ
/// dữ liệu đã nạp thay vì giữ một bản chép tay song song. Ba byte cao bị bỏ, nên
/// hàm này luôn ra đúng bảy ký tự.
pub fn format_hex_color(value: u32) -> String {
    format!("#{:06x}", value & 0x00ff_ffff)
}

#[cfg(test)]
mod tests {
    use super::{format_hex_color, parse_hex_color};

    #[test]
    fn hex_hop_le_ra_so_nguyen() {
        assert_eq!(parse_hex_color("#6b5a3e"), Ok(0x006b_5a3e));
        assert_eq!(parse_hex_color("#000000"), Ok(0));
        assert_eq!(parse_hex_color("#ffffff"), Ok(0x00ff_ffff));
        // Chữ hoa cũng đọc được: người ta chép màu từ trình chọn màu, và trình
        // chọn màu nào cũng in chữ hoa.
        assert_eq!(parse_hex_color("#D4562A"), Ok(0x00d4_562a));
    }

    #[test]
    fn hex_thieu_dau_thang_bao_loi() {
        let e = parse_hex_color("6b5a3e").expect_err("phải lỗi");
        assert!(e.contains('#'), "lỗi phải nói ra thứ đang thiếu: {e}");
    }

    #[test]
    fn hex_sai_do_dai_bao_loi_kem_do_dai_that() {
        let e = parse_hex_color("#abc").expect_err("phải lỗi");
        assert!(e.contains('6') && e.contains('3'), "{e}");
    }

    #[test]
    fn hex_co_ky_tu_khong_phai_hex_bao_loi_kem_ky_tu() {
        let e = parse_hex_color("#12345g").expect_err("phải lỗi");
        assert!(e.contains('g'), "lỗi phải chỉ đúng ký tự sai: {e}");
    }

    #[test]
    fn doc_roi_viet_lai_ra_dung_chuoi_cu() {
        for s in ["#0d1014", "#2c5c8a", "#cfe6f0", "#d4562a"] {
            let v = parse_hex_color(s).expect("màu hợp lệ");
            assert_eq!(format_hex_color(v), s, "đi về phải khép kín");
        }
    }
}
