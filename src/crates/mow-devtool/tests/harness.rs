//! Test harness. Harness phải tự kiểm được chính nó trước khi ta tin nó.

use mow_devtool::determinism::{bisect, checkpoints_upto, compare, Runnable, Verdict};
use mow_devtool::repro::{Manifest, ReproBundle};
use mow_math::{StateHash, StateHasher};
use std::collections::BTreeMap;

/// Lần chạy giả: hash là hàm của `(tick, salt)`, và `salt` đổi từ `bad_from`.
///
/// Mô phỏng đúng hình dạng của một lệch thật: giống hệt nhau cho tới một tick
/// nào đó, rồi khác mãi mãi.
struct GiaLap {
    label: String,
    bad_from: Option<u64>,
    /// Đếm số lần `hash_at` được gọi, để kiểm bisect thật sự là `O(log n)`.
    calls: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Runnable for GiaLap {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn hash_at(&mut self, tick: u64) -> StateHash {
        self.calls.set(self.calls.get() + 1);
        let mut h = StateHasher::with_domain("gia_lap");
        h.write_u64(tick);
        if self.bad_from.is_some_and(|b| tick >= b) {
            h.write_str(&self.label);
        }
        h.finish()
    }
}

fn chay(bad_from: Option<u64>) -> (Vec<Box<dyn Runnable>>, std::rc::Rc<std::cell::Cell<usize>>) {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let runs: Vec<Box<dyn Runnable>> = vec![
        Box::new(GiaLap {
            label: "threads=1".into(),
            bad_from,
            calls: calls.clone(),
        }),
        Box::new(GiaLap {
            label: "threads=8".into(),
            bad_from,
            calls: calls.clone(),
        }),
    ];
    (runs, calls)
}

#[test]
fn khong_lech_thi_bao_giong_nhau() {
    let (mut runs, _) = chay(None);
    let v = compare(&mut runs, &checkpoints_upto(10_000));
    assert!(matches!(v, Verdict::Identical { .. }), "{v:?}");
}

#[test]
fn lech_thi_tim_dung_tick_dau_tien() {
    // Đây là tính chất khiến harness có ích: không phải "có lệch" mà là
    // "lệch từ đâu".
    let (mut runs, _) = chay(Some(1_337));
    let Verdict::Diverged(d) = compare(&mut runs, &checkpoints_upto(10_000)) else {
        panic!("phải phát hiện lệch");
    };
    assert_eq!(d.first_bad_tick, 1_337, "{d}");
    assert_eq!(d.last_good_tick, 1_336, "{d}");
    assert_eq!(d.hashes.len(), 2);
    assert_ne!(
        d.hashes["threads=1"], d.hashes["threads=8"],
        "hai lần chạy phải thật sự khác nhau tại tick đó"
    );
}

#[test]
fn bisect_la_logarit_khong_phai_tuyen_tinh() {
    // Một thế giới 90 ngày có hàng triệu tick. Nếu bisect quét tuyến tính thì
    // nó không dùng được trên đúng những ca mà nó cần thiết nhất.
    let (mut runs, calls) = chay(Some(999_983));
    let Verdict::Diverged(d) = compare(&mut runs, &checkpoints_upto(4_000_000)) else {
        panic!("phải phát hiện lệch");
    };
    assert_eq!(d.first_bad_tick, 999_983);
    // log2(4e6) ≈ 22, nhân 2 lần chạy, cộng các mốc. Vài trăm là quá dư dả;
    // quét tuyến tính sẽ là hàng triệu.
    assert!(
        calls.get() < 400,
        "bisect gọi {} lần — có vẻ đang quét tuyến tính",
        calls.get()
    );
}

#[test]
fn lech_ngay_tu_tick_1() {
    let (mut runs, _) = chay(Some(1));
    let Verdict::Diverged(d) = compare(&mut runs, &checkpoints_upto(1_000)) else {
        panic!("phải phát hiện lệch");
    };
    assert_eq!(d.first_bad_tick, 1);
    assert_eq!(d.last_good_tick, 0);
}

#[test]
fn bisect_truc_tiep_tren_khoang_da_biet() {
    let (mut runs, _) = chay(Some(77));
    let d = bisect(&mut runs, 0, 128);
    assert_eq!(d.first_bad_tick, 77);
}

#[test]
fn moc_kiem_day_o_dau_thua_o_cuoi() {
    // Phần lớn lỗi determinism lộ ra sớm, nên mốc phải dày ở đầu.
    let c = checkpoints_upto(10_000);
    assert_eq!(c[0], 1);
    assert_eq!(*c.last().unwrap(), 10_000);
    assert!(
        c.windows(2).all(|w| w[0] < w[1]),
        "mốc phải tăng dần: {c:?}"
    );
    assert!(c.len() < 20, "quá nhiều mốc thì bisect mất tác dụng");
}

// ─────────────────────────────────────────────────────────────────────────────
// Repro bundle
// ─────────────────────────────────────────────────────────────────────────────

fn hash(n: u8) -> StateHash {
    StateHash([n; 32])
}

fn manifest_mau() -> Manifest {
    let mut packs = BTreeMap::new();
    packs.insert("core".to_owned(), ("0.1.0".to_owned(), hash(1)));
    Manifest {
        id: "repro-2026-08-31-a3f1".to_owned(),
        git_sha: "abc123".to_owned(),
        engine_version: "0.1.0".to_owned(),
        captured_at: "2026-08-31T00:00:00Z".to_owned(),
        worldseed: "test:tiny_village".to_owned(),
        config_hash: hash(9),
        packs,
        from_tick: 1_000,
        to_tick: 1_050,
        expected_hash: hash(7),
        symptom: "vật phẩm nhân đôi khi hai người cùng nhặt".to_owned(),
    }
}

#[test]
fn chup_roi_mo_lai_duoc_nguyen_ven() {
    let d = tempfile::tempdir().unwrap();
    let m = manifest_mau();
    let b = ReproBundle::capture(d.path(), m.clone(), b"snapshot-bytes", b"events-bytes").unwrap();

    let mo_lai = ReproBundle::open(&b.root).unwrap();
    assert_eq!(mo_lai.manifest, m);
    assert_eq!(mo_lai.snapshot().unwrap(), b"snapshot-bytes");
    assert_eq!(mo_lai.events().unwrap(), b"events-bytes");
}

#[test]
fn pack_doi_thi_tu_choi_chay_chu_khong_chay_ra_ket_qua_sai() {
    // Bài quan trọng nhất của repro bundle. Nếu không kiểm, một bundle chạy
    // sau khi content pack đã đổi sẽ cho kết quả khác, và ta kết luận sai rằng
    // bug đã tự hết.
    let d = tempfile::tempdir().unwrap();
    let b = ReproBundle::capture(d.path(), manifest_mau(), b"s", b"e").unwrap();

    let mut hien_tai = BTreeMap::new();
    hien_tai.insert("core".to_owned(), ("0.2.0".to_owned(), hash(2)));

    let e = b.verify(&hien_tai, hash(9)).expect_err("phải từ chối");
    let s = e.to_string();
    assert!(s.contains("core"), "{s}");
    assert!(
        s.contains("không phải bằng chứng đã sửa"),
        "thông báo phải nói rõ đây là môi trường đổi, không phải bug hết: {s}"
    );
}

#[test]
fn pack_thua_cung_la_lech() {
    // Một pack mới nạp có thể đăng ký luật mới và đổi kết quả, kể cả khi mọi
    // pack cũ vẫn nguyên.
    let d = tempfile::tempdir().unwrap();
    let b = ReproBundle::capture(d.path(), manifest_mau(), b"s", b"e").unwrap();

    let mut hien_tai = BTreeMap::new();
    hien_tai.insert("core".to_owned(), ("0.1.0".to_owned(), hash(1)));
    hien_tai.insert("mypack".to_owned(), ("1.0.0".to_owned(), hash(5)));

    let e = b.verify(&hien_tai, hash(9)).expect_err("phải từ chối");
    assert!(e.to_string().contains("mypack"), "{e}");
}

#[test]
fn thieu_pack_thi_bao_ro() {
    let d = tempfile::tempdir().unwrap();
    let b = ReproBundle::capture(d.path(), manifest_mau(), b"s", b"e").unwrap();
    let e = b
        .verify(&BTreeMap::new(), hash(9))
        .expect_err("phải từ chối");
    assert!(e.to_string().contains("thiếu pack `core`"), "{e}");
}

#[test]
fn cau_hinh_doi_cung_la_lech() {
    let d = tempfile::tempdir().unwrap();
    let b = ReproBundle::capture(d.path(), manifest_mau(), b"s", b"e").unwrap();
    let mut hien_tai = BTreeMap::new();
    hien_tai.insert("core".to_owned(), ("0.1.0".to_owned(), hash(1)));
    let e = b.verify(&hien_tai, hash(8)).expect_err("phải từ chối");
    assert!(e.to_string().contains("cấu hình đã đổi"), "{e}");
}

#[test]
fn moi_truong_khop_thi_cho_chay() {
    let d = tempfile::tempdir().unwrap();
    let b = ReproBundle::capture(d.path(), manifest_mau(), b"s", b"e").unwrap();
    let mut hien_tai = BTreeMap::new();
    hien_tai.insert("core".to_owned(), ("0.1.0".to_owned(), hash(1)));
    b.verify(&hien_tai, hash(9)).expect("môi trường khớp");
}

#[test]
fn chay_lai_dung_hash_thi_tai_hien_thanh_cong() {
    let d = tempfile::tempdir().unwrap();
    let b = ReproBundle::capture(d.path(), manifest_mau(), b"s", b"e").unwrap();
    b.check_result(hash(7)).expect("đúng hash");
    let e = b.check_result(hash(6)).expect_err("sai hash");
    assert!(e.to_string().contains("chạy lại cho hash"), "{e}");
}
