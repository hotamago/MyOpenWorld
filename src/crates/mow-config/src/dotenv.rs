//! Nạp `.env` (`plan.md §P10.6`).
//!
//! Tài liệu của `config/` viết từ đầu rằng "bí mật nằm ở `.env`". Cho tới file
//! này, **không có gì đọc `.env`** — [`crate::load`] chỉ đọc biến môi trường
//! của tiến trình, nên câu đó đúng về ý định và sai về thực tế: chép khóa vào
//! `.env` rồi chạy thì khóa không tới đâu cả.
//!
//! ## Ba quy tắc, và cả ba đều là phản ứng với một lỗi cụ thể
//!
//! **1. Môi trường thật luôn thắng `.env`.** Một biến đã có trong tiến trình
//! không bị ghi đè. Đây là quy tắc quan trọng nhất: CI và container đặt biến
//! qua môi trường, còn `.env` là tiện nghi của máy cá nhân. Đảo chiều ưu tiên
//! nghĩa là một file `.env` cũ trên máy ai đó lặng lẽ thắng cấu hình của CI.
//!
//! **2. Nạp là một lời gọi tường minh.** [`crate::load`] không tự tìm `.env`.
//! Test chạy với `tempdir` và không được bất ngờ thừa hưởng bí mật của máy
//! đang chạy chúng.
//!
//! **3. Không có `.env` không phải lỗi.** Trên máy chủ thì đó là tình trạng
//! bình thường và đúng.
//!
//! ## Vì sao tự viết bộ đọc thay vì lấy `dotenvy`
//!
//! Vì thứ cần ở đây nhỏ hơn thứ một thư viện `.env` cung cấp, và phần chênh
//! lệch toàn là thứ ta **không** muốn: nội suy biến, `export`, đa dòng. Mỗi
//! tính năng đó là một cách để giá trị đọc lên khác giá trị nhìn thấy trong
//! file — và đây là file chứa bí mật, nơi "khác một chút" là điều tệ nhất.
//!
//! Bộ đọc dưới đây làm đúng bốn việc: bỏ dòng trống và dòng chú thích, cắt ở
//! dấu `=` **đầu tiên**, bóc một lớp nháy nếu có, và giữ nguyên mọi thứ khác.

use std::collections::BTreeMap;
use std::path::Path;

/// Kết quả nạp một file `.env`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KetQua {
    /// Số biến đã đặt vào môi trường tiến trình.
    pub da_dat: Vec<String>,
    /// Biến có trong file nhưng **bỏ qua** vì môi trường đã có sẵn.
    pub da_co_san: Vec<String>,
    /// Số dòng không phân tích được, kèm số thứ tự dòng.
    pub dong_hong: Vec<usize>,
}

impl KetQua {
    /// Có gì được đặt không.
    #[must_use]
    pub fn co_thay_doi(&self) -> bool {
        !self.da_dat.is_empty()
    }
}

/// Phân tích nội dung một file `.env` thành các cặp, theo thứ tự xuất hiện.
///
/// Tách riêng khỏi phần đặt biến để kiểm được mà không đụng vào môi trường.
#[must_use]
pub fn phan_tich(noi_dung: &str) -> (Vec<(String, String)>, Vec<usize>) {
    let mut cap = Vec::new();
    let mut hong = Vec::new();

    for (i, dong) in noi_dung.lines().enumerate() {
        let t = dong.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some(vi_tri) = t.find('=') else {
            hong.push(i + 1);
            continue;
        };
        let ten = t[..vi_tri].trim();
        // `export FOO=bar` là cú pháp shell, không phải cú pháp `.env`. Chấp
        // nhận nó thì file này có hai cú pháp; từ chối nó thì có một.
        if ten.is_empty() || ten.contains(char::is_whitespace) {
            hong.push(i + 1);
            continue;
        }
        let gia_tri = t[vi_tri + 1..].trim();
        // Bóc **một** lớp nháy. Nháy trong `.env` chỉ để bảo vệ khoảng trắng ở
        // hai đầu; không có nội suy bên trong, kể cả với nháy kép.
        let gia_tri = if gia_tri.len() >= 2
            && ((gia_tri.starts_with('"') && gia_tri.ends_with('"'))
                || (gia_tri.starts_with('\'') && gia_tri.ends_with('\'')))
        {
            &gia_tri[1..gia_tri.len() - 1]
        } else {
            gia_tri
        };
        cap.push((ten.to_owned(), gia_tri.to_owned()));
    }
    (cap, hong)
}

/// Nạp `.env` tại `duong_dan` vào môi trường tiến trình.
///
/// Không có file thì trả về [`KetQua`] rỗng — đó là tình trạng bình thường trên
/// máy chủ, không phải lỗi.
///
/// # Errors
/// Khi file tồn tại nhưng không đọc được (quyền, hỏng mã hóa).
pub fn nap(duong_dan: impl AsRef<Path>) -> std::io::Result<KetQua> {
    let duong_dan = duong_dan.as_ref();
    if !duong_dan.exists() {
        return Ok(KetQua::default());
    }
    let noi_dung = std::fs::read_to_string(duong_dan)?;
    let (cap, hong) = phan_tich(&noi_dung);

    let mut kq = KetQua {
        dong_hong: hong,
        ..KetQua::default()
    };
    // Trùng tên trong cùng file: **dòng cuối thắng**, giống mọi bộ đọc `.env`
    // khác. Dựng map trước rồi mới đặt, để không đặt hai lần.
    let mut cuoi: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in cap {
        cuoi.insert(k, v);
    }
    for (k, v) in cuoi {
        if std::env::var_os(&k).is_some() {
            kq.da_co_san.push(k);
            continue;
        }
        std::env::set_var(&k, v);
        kq.da_dat.push(k);
    }
    Ok(kq)
}

/// Nạp `.env` ở gốc repo, tính từ thư mục `config/`.
///
/// `config/` nằm ở `src/config`, còn `.env` ở gốc repo — nên đi lên hai bậc.
/// Cả `src/.env` cũng được thử, vì đó là chỗ người ta hay đặt nó trước.
///
/// # Errors
/// Khi một file tồn tại nhưng không đọc được.
pub fn nap_canh_config(config_root: impl AsRef<Path>) -> std::io::Result<KetQua> {
    let root = config_root.as_ref();
    let mut kq = KetQua::default();
    // Thứ tự: gần trước, xa sau. Vì bước đặt biến không ghi đè, file gần
    // `config/` hơn sẽ thắng — đúng với trực giác "file gần hơn thì cụ thể hơn".
    for ung_vien in [
        root.join(".env"),
        root.join("../.env"),
        root.join("../../.env"),
    ] {
        let mot = nap(&ung_vien)?;
        kq.da_dat.extend(mot.da_dat);
        kq.da_co_san.extend(mot.da_co_san);
        kq.dong_hong.extend(mot.dong_hong);
    }
    Ok(kq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phan_tich_cap_don_gian() {
        let (c, h) = phan_tich("A=1\nB=hai\n");
        assert_eq!(
            c,
            vec![("A".into(), "1".into()), ("B".into(), "hai".into())]
        );
        assert!(h.is_empty());
    }

    #[test]
    fn bo_qua_dong_trong_va_chu_thich() {
        let (c, h) = phan_tich("# chú thích\n\n  \nA=1\n");
        assert_eq!(c.len(), 1);
        assert!(h.is_empty());
    }

    #[test]
    fn khoa_openrouter_co_dau_gach_va_bang_van_nguyen_ven() {
        // Khóa thật có dạng `sk-or-v1-<hex>`; không có `=` nhưng nếu có thì
        // cắt ở dấu `=` **đầu tiên** vẫn giữ nguyên phần còn lại.
        let (c, _) = phan_tich("OPENROUTER_API_KEY=sk-or-v1-abc=def==\n");
        assert_eq!(c[0].1, "sk-or-v1-abc=def==");
    }

    #[test]
    fn boc_mot_lop_nhay() {
        let (c, _) = phan_tich("A=\"có khoảng trắng \"\nB='đơn'\n");
        assert_eq!(c[0].1, "có khoảng trắng ");
        assert_eq!(c[1].1, "đơn");
    }

    #[test]
    fn khong_noi_suy_bien() {
        // `$HOME` phải đi vào môi trường **nguyên văn**. Nội suy ở đây nghĩa là
        // giá trị đọc lên khác giá trị nhìn thấy trong file.
        let (c, _) = phan_tich("A=$HOME/x\n");
        assert_eq!(c[0].1, "$HOME/x");
    }

    #[test]
    fn export_bi_tu_choi_chu_khong_bi_hieu_nham() {
        let (c, h) = phan_tich("export A=1\n");
        assert!(c.is_empty(), "{c:?}");
        assert_eq!(h, vec![1]);
    }

    #[test]
    fn dong_khong_co_dau_bang_duoc_bao_lai() {
        let (_, h) = phan_tich("A=1\nrác\nB=2\n");
        assert_eq!(h, vec![2]);
    }

    #[test]
    fn gia_tri_rong_van_la_mot_cap() {
        // `HF_TOKEN=` là cách nói "biến này có, cố ý để trống".
        let (c, _) = phan_tich("HF_TOKEN=\n");
        assert_eq!(c, vec![("HF_TOKEN".into(), String::new())]);
    }

    #[test]
    fn khong_co_file_thi_khong_phai_loi() {
        let d = tempfile::tempdir().unwrap();
        let kq = nap(d.path().join("khong-ton-tai.env")).unwrap();
        assert_eq!(kq, KetQua::default());
    }

    #[test]
    fn moi_truong_that_thang_file() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join(".env");
        // Tên biến riêng cho bài này để không đụng bài khác chạy song song.
        let ten = "MOW_TEST_DOTENV_UU_TIEN";
        std::env::set_var(ten, "tu-moi-truong");
        std::fs::write(&f, format!("{ten}=tu-file\n")).unwrap();

        let kq = nap(&f).unwrap();
        assert_eq!(std::env::var(ten).unwrap(), "tu-moi-truong");
        assert!(kq.da_co_san.iter().any(|k| k == ten), "{kq:?}");
        assert!(!kq.da_dat.iter().any(|k| k == ten), "{kq:?}");
        std::env::remove_var(ten);
    }

    #[test]
    fn dat_bien_chua_co() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join(".env");
        let ten = "MOW_TEST_DOTENV_MOI";
        std::env::remove_var(ten);
        std::fs::write(&f, format!("{ten}=gia-tri\n")).unwrap();

        let kq = nap(&f).unwrap();
        assert_eq!(std::env::var(ten).unwrap(), "gia-tri");
        assert!(kq.co_thay_doi());
        std::env::remove_var(ten);
    }

    #[test]
    fn trung_ten_thi_dong_cuoi_thang() {
        let (c, _) = phan_tich("A=1\nA=2\n");
        assert_eq!(c.len(), 2, "phân tích giữ cả hai dòng");
        // Việc chọn dòng cuối nằm ở bước đặt biến, không ở bước phân tích.
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join(".env");
        let ten = "MOW_TEST_DOTENV_TRUNG";
        std::env::remove_var(ten);
        std::fs::write(&f, format!("{ten}=mot\n{ten}=hai\n")).unwrap();
        nap(&f).unwrap();
        assert_eq!(std::env::var(ten).unwrap(), "hai");
        std::env::remove_var(ten);
    }
}
