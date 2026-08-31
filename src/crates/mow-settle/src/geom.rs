//! Hình học nguyên: chữ nhật, vành đai vuông, và chỗ đặt công trình.
//!
//! Mọi phép cộng tọa độ ở đây đều bão hòa. Không phải vì làng nào cũng nằm ở
//! `i64::MAX`, mà vì `plan()` nhận `center` từ ngoài vào: một `center` sát biên
//! kiểu số phải cho ra một kế hoạch nghèo nàn, chứ không được cho ra một lần
//! tràn số làm sập cả server.

use crate::hash::{hash_i, pick, salt};

/// Chữ nhật nửa mở tính theo ô, gốc ở góc trên trái.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    /// Hoành độ mép trái.
    pub(crate) x: i64,
    /// Tung độ mép trên.
    pub(crate) y: i64,
    /// Bề rộng theo ô.
    pub(crate) w: i64,
    /// Bề cao theo ô.
    pub(crate) h: i64,
}

impl Rect {
    /// Chữ nhật `w * h` đặt sao cho tâm của nó rơi vào `at`.
    pub(crate) fn centered(at: (i64, i64), w: i64, h: i64) -> Self {
        Self {
            x: at.0.saturating_sub(w / 2),
            y: at.1.saturating_sub(h / 2),
            w,
            h,
        }
    }

    /// Nới đều `m` ô ra bốn phía.
    pub(crate) fn expand(self, m: i64) -> Self {
        Self {
            x: self.x.saturating_sub(m),
            y: self.y.saturating_sub(m),
            w: self.w.saturating_add(m.saturating_mul(2)),
            h: self.h.saturating_add(m.saturating_mul(2)),
        }
    }

    /// Ô có nằm trong chữ nhật không.
    pub(crate) fn contains(self, p: (i64, i64)) -> bool {
        p.0 >= self.x
            && p.1 >= self.y
            && p.0 < self.x.saturating_add(self.w)
            && p.1 < self.y.saturating_add(self.h)
    }

    /// Duyệt ô theo hàng, trái sang phải, trên xuống dưới.
    ///
    /// Thứ tự duyệt là một phần của hợp đồng xác định: nó quyết định nét nào vẽ
    /// sau cùng khi hai nét cùng độ ưu tiên.
    pub(crate) fn cells(self) -> impl Iterator<Item = (i64, i64)> {
        (0..self.h).flat_map(move |dy| {
            (0..self.w).map(move |dx| (self.x.saturating_add(dx), self.y.saturating_add(dy)))
        })
    }
}

/// Khoảng cách Chebyshev — khoảng cách "ô vuông", vì vùng quy hoạch là hình
/// vuông chứ không phải hình tròn.
pub(crate) fn cheb(a: (i64, i64), b: (i64, i64)) -> i64 {
    let dx = a.0.saturating_sub(b.0).saturating_abs();
    let dy = a.1.saturating_sub(b.1).saturating_abs();
    dx.max(dy)
}

/// Các ô nằm đúng trên vành đai vuông bán kính `r` quanh `at`, theo chiều kim
/// đồng hồ từ góc trên trái.
pub(crate) fn ring(at: (i64, i64), r: i64) -> Vec<(i64, i64)> {
    if r <= 0 {
        return vec![at];
    }
    let (cx, cy) = at;
    let (l, t) = (cx.saturating_sub(r), cy.saturating_sub(r));
    let (rt, b) = (cx.saturating_add(r), cy.saturating_add(r));
    let mut out = Vec::with_capacity((8 * r) as usize);
    for x in l..=rt {
        out.push((x, t));
    }
    for y in (t + 1)..=b {
        out.push((rt, y));
    }
    for x in (l..rt).rev() {
        out.push((x, b));
    }
    for y in ((t + 1)..b).rev() {
        out.push((l, y));
    }
    out
}

/// Khoảng cách giữa hai chỗ đặt liền nhau trên cùng một vành đai.
///
/// Bằng bề rộng nhà cộng lề: đặt dày hơn thì mọi chỗ sau chỗ đầu đều bị từ chối
/// vì chạm nhà bên cạnh, và ta tốn công thử.
const SLOT_STRIDE: usize = 7;

/// Khoảng cách giữa hai vành đai kế tiếp.
const RING_STEP: i64 = 6;

/// Danh sách chỗ đặt công trình, từ trong ra ngoài.
///
/// Đi theo vành đai chứ không rải ngẫu nhiên vì đó là thứ làm cho làng có
/// *hình*: nhà quây quanh quảng trường, ruộng ở ngoài. Rải ngẫu nhiên trong
/// hình vuông cho ra một đám nhà không tâm, và mắt sẽ không đọc ra "làng".
///
/// Điểm khởi đầu trên mỗi vành đai lệch theo băm, nếu không thì mọi ngôi làng
/// đều có một ngôi nhà ở đúng góc trên trái.
pub(crate) fn slots(hub: (i64, i64), plaza_half: i64, radius: i64, seed: u64) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut r = plaza_half + 4;
    while r.saturating_add(3) <= radius {
        let cells = ring(hub, r);
        let offset = pick(hash_i(seed, salt::SLOT, r as u64), SLOT_STRIDE as u64) as usize;
        let mut i = offset;
        while i < cells.len() {
            out.push(cells[i]);
            i += SLOT_STRIDE;
        }
        r = r.saturating_add(RING_STEP);
    }
    out
}
