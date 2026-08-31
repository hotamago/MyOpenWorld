//! Tấm giấy can: nơi duy nhất ghi "ô này đã có chủ" và "ô này màu gì".
//!
//! Hai bảng tách rời nhau có chủ ý. `claims` trả lời câu hỏi *đặt được không*,
//! `paint` trả lời câu hỏi *trông ra sao*. Gộp làm một thì con đường — thứ được
//! phép nhập vào con đường khác nhưng không được phép bị nhà đè lên — sẽ không
//! diễn đạt được.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::geom::Rect;

/// Kiểu đặt chỗ của một ô.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Claim {
    /// Công trình đặc: không nét nào khác được chạm vào.
    Solid,
    /// Mặt đi lại: đường mới được phép chạy đè lên đường cũ và nhập vào nó.
    Path,
}

/// Bản vẽ đang dở của một khu định cư.
pub(crate) struct Canvas<'a> {
    buildable: &'a dyn Fn(i64, i64) -> bool,
    center: (i64, i64),
    radius: i64,
    /// Nhớ lại câu trả lời của `buildable`.
    ///
    /// Vị từ này có thể là một lần tra địa hình thật; một lần quy hoạch hỏi
    /// cùng một ô hàng chục lần (dò tâm, thử móng, thử lề, dò đường). Nhớ lại
    /// vừa nhanh hơn vừa *khóa* câu trả lời: nếu vị từ của người gọi lỡ không
    /// thuần thì kế hoạch vẫn tự nhất quán chứ không mâu thuẫn giữa chừng.
    known: RefCell<BTreeMap<(i64, i64), bool>>,
    claims: BTreeMap<(i64, i64), Claim>,
    paint: BTreeMap<(i64, i64), (&'static str, u8)>,
}

impl<'a> Canvas<'a> {
    /// Mở một tấm giấy can trên vùng vuông tâm `center`, bán kính `radius`.
    pub(crate) fn new(
        buildable: &'a dyn Fn(i64, i64) -> bool,
        center: (i64, i64),
        radius: i64,
    ) -> Self {
        Self {
            buildable,
            center,
            radius,
            known: RefCell::new(BTreeMap::new()),
            claims: BTreeMap::new(),
            paint: BTreeMap::new(),
        }
    }

    /// Ô có dùng được không: vừa trong vùng quy hoạch, vừa được phép xây.
    pub(crate) fn usable(&self, p: (i64, i64)) -> bool {
        if crate::geom::cheb(p, self.center) > self.radius {
            return false;
        }
        if let Some(known) = self.known.borrow().get(&p) {
            return *known;
        }
        let ok = (self.buildable)(p.0, p.1);
        self.known.borrow_mut().insert(p, ok);
        ok
    }

    /// Ô còn trống hoàn toàn.
    pub(crate) fn free(&self, p: (i64, i64)) -> bool {
        self.usable(p) && !self.claims.contains_key(&p)
    }

    /// Ô đi lại được: hoặc còn trống, hoặc đã là mặt đường.
    pub(crate) fn free_path(&self, p: (i64, i64)) -> bool {
        self.usable(p) && !matches!(self.claims.get(&p), Some(Claim::Solid))
    }

    /// Móng `rect` cùng lề `margin` quanh nó có nhận được công trình không.
    ///
    /// Lề là thứ giữ hai ngôi nhà không dính lưng vào nhau. Nhìn từ trên xuống,
    /// hai mái chạm nhau đọc ra là *một* khối lớn dị dạng chứ không phải hai
    /// ngôi nhà, nên lề ở đây là yêu cầu thị giác chứ không phải sự cẩn thận.
    pub(crate) fn rect_free(&self, rect: Rect, margin: i64) -> bool {
        for p in rect.expand(margin).cells() {
            if !self.usable(p) {
                return false;
            }
            match self.claims.get(&p) {
                // Công trình khác: cấm cả ở phần lề.
                Some(Claim::Solid) => return false,
                // Mặt đường: chỉ cấm dưới móng, còn chạy sát lề thì tốt.
                Some(Claim::Path) if rect.contains(p) => return false,
                _ => {}
            }
        }
        true
    }

    /// Đặt chỗ một ô. `Solid` được phép nâng cấp từ `Path` (giếng nằm giữa
    /// quảng trường), chiều ngược lại thì không.
    pub(crate) fn claim(&mut self, p: (i64, i64), kind: Claim) {
        match (self.claims.get(&p), kind) {
            (Some(Claim::Solid), Claim::Path) => {}
            _ => {
                self.claims.insert(p, kind);
            }
        }
    }

    /// Đặt chỗ cả một chữ nhật.
    pub(crate) fn claim_rect(&mut self, rect: Rect, kind: Claim) {
        for p in rect.cells() {
            self.claim(p, kind);
        }
    }

    /// Tô một ô. Nét sau thắng khi cùng độ ưu tiên, nét ưu tiên cao luôn thắng.
    pub(crate) fn paint(&mut self, p: (i64, i64), material: &'static str, prio: u8) {
        if !self.usable(p) {
            return;
        }
        match self.paint.get(&p) {
            Some((_, old)) if *old > prio => {}
            _ => {
                self.paint.insert(p, (material, prio));
            }
        }
    }

    /// Vật liệu đang nằm ở một ô.
    pub(crate) fn material_at(&self, p: (i64, i64)) -> Option<&'static str> {
        self.paint.get(&p).map(|(m, _)| *m)
    }

    /// Kết tinh thành danh sách ô của kế hoạch.
    ///
    /// `BTreeMap` là lý do bất biến "một ô một vật liệu" không cần ai kiểm tra:
    /// nó không có chỗ để ghi hai giá trị. Test chỉ còn phải xác nhận rằng danh
    /// sách phẳng ra vẫn giữ được điều đó.
    pub(crate) fn into_cells(self) -> Vec<(i64, i64, &'static str)> {
        self.paint
            .into_iter()
            .map(|((x, y), (m, _))| (x, y, m))
            .collect()
    }
}
