//! Harness determinism (`plan.md §P7.5`).
//!
//! > Công cụ giá trị nhất của dự án.
//!
//! Nó không kiểm tra một tính năng nào cả. Nó kiểm tra rằng **thế giới là một
//! hàm của seed**, và đó là giả định mà mọi thứ khác đứng trên: replay, repro
//! bundle, branch, so sánh hai nhánh, và cả việc gỡ lỗi bằng cách chạy lại.
//!
//! Khi hai lần chạy lệch nhau, câu hỏi khó không phải "có lệch không" mà là
//! **"lệch từ đâu"**. Một thế giới chạy 90 ngày có hàng triệu tick; biết rằng
//! hash cuối khác nhau thì gần như vô dụng. Nên phần đắt giá của module này là
//! [`bisect`]: tìm **tick đầu tiên** mà hai lần chạy khác nhau, bằng tìm kiếm
//! nhị phân trên trục thời gian.
//!
//! Tìm kiếm nhị phân hợp lệ ở đây vì tính lệch là **đơn điệu**: một khi hai
//! thế giới đã khác nhau thì chúng không bao giờ giống lại. Điều đó đúng vì
//! state của tick sau là hàm của state tick trước.

use mow_math::StateHash;
use std::collections::BTreeMap;

/// Một lần chạy để so sánh.
///
/// Trait thay vì một hàm cụ thể, vì cùng harness này phải chạy được trên nhiều
/// thứ: một `Sim` trong bộ nhớ, một kịch bản, hay một repro bundle.
pub trait Runnable {
    /// Nhãn của lần chạy này, hiện trong báo cáo. Ví dụ `threads=8`.
    fn label(&self) -> String;

    /// Chạy từ đầu tới `tick` rồi trả state hash tại đúng đó.
    ///
    /// **Phải chạy lại từ đầu mỗi lần**, không được tiếp tục từ lần trước.
    /// Nếu tiếp tục, ta đang so hai điểm trên **cùng một** dòng thời gian và
    /// bisect sẽ luôn nói "không lệch".
    fn hash_at(&mut self, tick: u64) -> StateHash;
}

/// Kết quả so sánh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Mọi lần chạy khớp nhau ở mọi mốc kiểm.
    Identical {
        /// Hash cuối.
        hash: StateHash,
        /// Số mốc đã so.
        checkpoints: usize,
    },
    /// Có lệch.
    Diverged(Divergence),
}

/// Mô tả một lần lệch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    /// Tick **đầu tiên** mà hai lần chạy khác nhau.
    pub first_bad_tick: u64,
    /// Tick cuối cùng còn giống nhau. Đây là điểm để chụp repro bundle.
    pub last_good_tick: u64,
    /// Hash của từng lần chạy tại `first_bad_tick`.
    pub hashes: BTreeMap<String, StateHash>,
}

impl core::fmt::Display for Divergence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "lệch tại tick {} (còn khớp tới tick {})",
            self.first_bad_tick, self.last_good_tick
        )?;
        for (nhan, h) in &self.hashes {
            writeln!(f, "  {nhan:<20} {}", h.short())?;
        }
        writeln!(
            f,
            "  → chụp repro bundle từ tick {} rồi chạy một tick để xem event nào khác nhau",
            self.last_good_tick
        )
    }
}

/// So nhiều lần chạy tại các mốc, rồi bisect nếu lệch.
///
/// `checkpoints` là các tick sẽ so. Chọn thưa cũng được — bisect sẽ thu hẹp
/// tiếp. Nhưng phải có ít nhất một mốc, và mốc cuối nên là điểm cuối của bài.
pub fn compare(runs: &mut [Box<dyn Runnable>], checkpoints: &[u64]) -> Verdict {
    assert!(runs.len() >= 2, "cần ít nhất hai lần chạy để so");
    assert!(!checkpoints.is_empty(), "cần ít nhất một mốc kiểm");

    let mut moc_cuoi_khop = 0u64;

    for &t in checkpoints {
        let hashes: Vec<(String, StateHash)> =
            runs.iter_mut().map(|r| (r.label(), r.hash_at(t))).collect();

        let dau = hashes[0].1;
        if hashes.iter().all(|(_, h)| *h == dau) {
            moc_cuoi_khop = t;
            continue;
        }

        // Đã tìm thấy một mốc lệch. Thu hẹp giữa `moc_cuoi_khop` và `t`.
        let d = bisect(runs, moc_cuoi_khop, t);
        return Verdict::Diverged(d);
    }

    let hash = runs[0].hash_at(*checkpoints.last().expect("đã kiểm không rỗng"));
    Verdict::Identical {
        hash,
        checkpoints: checkpoints.len(),
    }
}

/// Tìm tick đầu tiên mà các lần chạy khác nhau, trong khoảng `(good, bad]`.
///
/// Điều kiện tiên quyết: mọi lần chạy khớp nhau tại `good` và khác nhau tại
/// `bad`. Nếu điều kiện này không đúng thì kết quả vô nghĩa — nên [`compare`]
/// là đường vào chính, còn hàm này để lộ ra chủ yếu cho `mow-cli`.
pub fn bisect(runs: &mut [Box<dyn Runnable>], good: u64, bad: u64) -> Divergence {
    let mut lo = good;
    let mut hi = bad;

    while hi - lo > 1 {
        let giua = lo + (hi - lo) / 2;
        let dau = runs[0].hash_at(giua);
        let khop = runs.iter_mut().all(|r| r.hash_at(giua) == dau);
        if khop {
            lo = giua;
        } else {
            hi = giua;
        }
    }

    let hashes = runs
        .iter_mut()
        .map(|r| (r.label(), r.hash_at(hi)))
        .collect();

    Divergence {
        first_bad_tick: hi,
        last_good_tick: lo,
        hashes,
    }
}

/// Sinh dãy mốc kiểm theo cấp số nhân rồi thêm điểm cuối.
///
/// Dày ở đầu, thưa ở cuối. Lý do thực dụng: phần lớn lỗi determinism xuất hiện
/// **sớm** — một `HashMap` bị duyệt, một `f64` lọt vào — và chúng lộ ra trong
/// vài trăm tick đầu. Lỗi xuất hiện muộn thì hiếm hơn nhiều, và bisect vẫn
/// tìm được chúng.
pub fn checkpoints_upto(last: u64) -> Vec<u64> {
    let mut ra = Vec::new();
    let mut t = 1u64;
    while t < last {
        ra.push(t);
        t = t.saturating_mul(4);
    }
    ra.push(last);
    ra.dedup();
    ra
}
