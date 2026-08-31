//! Giam sat vong doi sidecar Python (`plan.md §P3.4`, `PF-12`).
//!
//! Ba viec, va bo viec nao cung de lai mot tien trinh Python mo coi tren may
//! nguoi choi:
//!
//! 1. Khoi dong **truoc** khi WebView goi toi.
//! 2. Cho no **tu bao san sang** — khong phai cho mot khoang thoi gian co dinh.
//! 3. **Tat** khi ung dung dong, ke ca khi dong bat thuong.
//!
//! Diem 3 duoc bao dam bang `Drop`, khong bang mot ham `shutdown()` ma cho goi
//! phai nho: mot panic o giua `setup()` se bo qua moi loi goi tuong minh, con
//! `Drop` thi van chay.
//!
//! ## Thieu sidecar khong phai loi
//!
//! `§P3.4`: khong co tang nhan thuc thi the gioi **van chay day du** o ba tang
//! dau cua thap hanh vi `§10.3`. Nen [`Supervisor::start`] tra `Ok` ca khi
//! khong tim thay binary — no ghi lai va di tiep. Mot ung dung tu choi khoi
//! dong vi thieu phan tuy chon la mot ung dung hong hon phan no thieu.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::sidecar::{parse_ready, TrangThai, TEN_SIDECAR};

/// Cho toi da bao lau de sidecar tu bao san sang.
///
/// Khong phai `sleep` — day la tran cho mot su kien. Mot tran 30 giay du cho
/// may cham; het tran thi ung dung di tiep khong co tang nhan thuc, chu khong
/// treo.
pub const TRAN_CHO: Duration = Duration::from_secs(30);

/// Giam sat mot tien trinh sidecar.
#[derive(Debug)]
pub struct Supervisor {
    child: Option<Child>,
    state: TrangThai,
}

impl Supervisor {
    /// Chua chay gi.
    pub fn new() -> Supervisor {
        Supervisor {
            child: None,
            state: TrangThai::ChuaChay,
        }
    }

    /// Trang thai hien tai, khong hoi lai tien trinh.
    ///
    /// Dung `poll()` neu can biet sidecar con song khong.
    #[allow(dead_code)]
    pub fn state(&self) -> TrangThai {
        self.state
    }

    /// Duong dan sidecar trong ban dong goi: canh binary, khong o thu muc lam viec.
    pub fn binary_path(resource_dir: &Path) -> PathBuf {
        let ten = if cfg!(windows) {
            format!("{TEN_SIDECAR}.exe")
        } else {
            TEN_SIDECAR.to_owned()
        };
        resource_dir.join(ten)
    }

    /// Khoi dong sidecar va **cho no tu bao san sang**.
    ///
    /// Khong tim thay binary thi tra `Ok` voi trang thai `ChuaChay`: xem ghi
    /// chu dau module.
    pub fn start(&mut self, resource_dir: &Path) -> std::io::Result<TrangThai> {
        let bin = Supervisor::binary_path(resource_dir);
        if !bin.exists() {
            self.state = TrangThai::ChuaChay;
            return Ok(self.state);
        }

        let mut child = Command::new(&bin)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        self.state = TrangThai::DangKhoiDong;

        // Doc stdout tren mot luong rieng va bao ve qua kenh. Doc thang o day
        // se treo vinh vien neu sidecar khong bao gio in dong san sang.
        let Some(out) = child.stdout.take() else {
            self.child = Some(child);
            return Ok(self.state);
        };
        let (tx, rx) = mpsc::channel::<u16>();
        std::thread::spawn(move || {
            for dong in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(p) = parse_ready(&dong) {
                    let _ = tx.send(p);
                    return;
                }
            }
        });

        self.state = match rx.recv_timeout(TRAN_CHO) {
            Ok(port) => TrangThai::SanSang { port },
            // Het tran: sidecar co the van dang khoi dong. Khong giet no —
            // no co the san sang muon va cac lan goi sau se dung duoc.
            Err(_) => TrangThai::DangKhoiDong,
        };
        self.child = Some(child);
        Ok(self.state)
    }

    /// Kiem xem sidecar con song khong.
    ///
    /// Goi truoc moi lan dung. `Child::try_wait` khong chan, nen goi no o duong
    /// nong duoc — con `wait` thi khong.
    ///
    /// Day la cho `TrangThai::Chet` duoc dung: mot sidecar chet lang le trong
    /// khi ung dung van chay la truong hop hay xay ra nhat (Python het bo nho,
    /// mot phu thuoc thieu), va khong phat hien duoc no nghia la moi lan goi
    /// sau deu timeout thay vi chuyen sang fallback ngay.
    pub fn poll(&mut self) -> TrangThai {
        if let Some(c) = self.child.as_mut() {
            match c.try_wait() {
                Ok(Some(status)) => {
                    self.state = TrangThai::Chet {
                        exit_code: status.code(),
                    };
                    self.child = None;
                }
                // Con song, hoac khong hoi duoc — giu nguyen trang thai.
                Ok(None) | Err(_) => {}
            }
        }
        self.state
    }

    /// Tang nhan thuc co dung duoc khong ngay bay gio.
    ///
    /// Sai thi the gioi **van chay** o ba tang dau cua thap hanh vi (`§10.3`).
    pub fn cognition_available(&mut self) -> bool {
        self.poll().callable()
    }

    /// Tat sidecar.
    ///
    /// Goi duoc nhieu lan. Mot ham tat chi goi duoc mot lan la mot ham ma cho
    /// goi phai nho da goi chua.
    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.state = TrangThai::DaTat;
    }
}

impl Default for Supervisor {
    fn default() -> Supervisor {
        Supervisor::new()
    }
}

impl Drop for Supervisor {
    /// **Tat ke ca khi dong bat thuong.**
    ///
    /// Day la ly do `Supervisor` ton tai thay vi mot cap ham `spawn`/`kill`:
    /// mot panic giua `setup()` bo qua moi loi goi tuong minh, con `Drop` van
    /// chay — va tien trinh Python khong con song sot qua lan dong ung dung.
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thieu_binary_khong_phai_loi() {
        let mut s = Supervisor::new();
        let ra = s
            .start(Path::new("/khong/co/thu/muc/nay"))
            .expect("thieu sidecar khong duoc lam ung dung khong khoi dong");
        assert_eq!(ra, TrangThai::ChuaChay);
        assert!(!ra.callable());
    }

    #[test]
    fn duong_dan_sidecar_nam_canh_tai_nguyen() {
        let p = Supervisor::binary_path(Path::new("/opt/mow/resources"));
        assert!(p.starts_with("/opt/mow/resources"));
        assert!(p.to_string_lossy().contains(TEN_SIDECAR));
    }

    #[test]
    fn tat_goi_duoc_nhieu_lan() {
        let mut s = Supervisor::new();
        s.stop();
        s.stop();
        assert_eq!(s.state(), TrangThai::DaTat);
    }

    #[test]
    fn thieu_sidecar_thi_khong_co_tang_nhan_thuc_nhung_the_gioi_van_chay() {
        let mut s = Supervisor::new();
        let _ = s.start(Path::new("/khong/co/thu/muc/nay"));
        assert!(!s.cognition_available());
        assert!(
            crate::sidecar::world_runs_without_sidecar(),
            "§P3.4: khong co tang nhan thuc thi ba tang dau van chay"
        );
    }

    #[test]
    fn poll_khong_doi_trang_thai_khi_chua_chay_gi() {
        let mut s = Supervisor::new();
        assert_eq!(s.poll(), TrangThai::ChuaChay);
    }

    #[test]
    fn tran_cho_la_tran_khong_phai_sleep() {
        // Mot `sleep` co dinh chay duoc tren may nguoi viet va hong tren may
        // cham hon. Tran o day chi la gioi han tren: sidecar bao som thi di
        // som.
        assert!(TRAN_CHO >= Duration::from_secs(10));
    }
}
