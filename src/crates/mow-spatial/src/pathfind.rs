//! Tìm đường A* trên lưới 8 hướng, **toàn số nguyên** (`§P10.2.1`).
//!
//! Người chơi bấm chuột vào một ô; nhân vật phải tự đi tới. Nghe như một bài
//! tập, nhưng ba ràng buộc của dự án biến nó thành một bài khó:
//!
//! ## 1. Không có số thực (`§P10.2.1`)
//!
//! Cách quen thuộc để tính chi phí đường chéo là `sqrt(2)`. Ở đây thì không
//! được: một `f64` trong đường commit là một giá trị có thể khác nhau giữa hai
//! máy, và đường đi *là* state — nó đi vào replay và vào state hash. Nên chi
//! phí được cân theo thang mười: một bước thẳng là [`STEP_STRAIGHT`] = 10, một
//! bước chéo là [`STEP_DIAGONAL`] = 14. Tỉ số `14/10` lệch khỏi `sqrt(2)`
//! khoảng 1%, và 1% sai trong việc chọn *đường* là không đáng kể; 100% sai
//! trong việc *hai máy chọn khác đường nhau* thì là lỗi desync.
//!
//! Heuristic cũng theo thang đó — octile distance, cũng bằng số nguyên. Nó
//! không bao giờ ước lượng vượt chi phí thật, nên A* vẫn trả đường tối ưu.
//!
//! ## 2. Thế giới vô hạn (`§7.1`)
//!
//! Toạ độ là `i64` và không có biên. Một A* không giới hạn nhắm vào một ô
//! không tới được sẽ không kết thúc — nó quét ra ngoài mãi cho tới khi hết RAM.
//! Đó không phải giả thuyết: người chơi bấm ra giữa biển là chuyện xảy ra mỗi
//! phiên chơi. Vì vậy [`PathRequest::max_nodes`] là **bắt buộc**, không phải
//! tuỳ chọn, và khi cạn trần ta trả [`PathOutcome::BudgetExhausted`] kèm đường
//! tới ô gần đích nhất đã chạm được. Nhân vật đi tới mép bờ rồi dừng — điều đó
//! đọc được như một hành vi; đứng im không nói gì thì đọc như một bug.
//!
//! ## 3. Xác định, kể cả khi hoà
//!
//! Trong một cánh đồng trống có hàng nghìn đường cùng chi phí. Nếu chọn đường
//! nào phụ thuộc thứ tự duyệt của [`std::collections::HashMap`], hai máy sẽ
//! chọn khác nhau dù cùng đầu vào — và `§P10.3` cấm duyệt `HashMap` trên đường
//! commit đúng vì lý do này. Ở đây:
//!
//! - hàng đợi ưu tiên phá hoà theo `(f, h, x, y)` — một thứ tự **toàn phần**,
//!   nên không có hai phần tử nào "bằng nhau nhưng khác nhau";
//! - `g` và cây cha dùng [`std::collections::BTreeMap`];
//! - tám hướng đi được duyệt theo thứ tự `(dx, dy)` tăng dần, cố định trong mã.
//!
//! ## Quy ước đường đi
//!
//! Đường trả về **gồm cả ô xuất phát và ô đích**: `[from, …, to]`. Do đó đường
//! ngắn nhất khác rỗng có đúng hai phần tử.
//!
//! Đường **rỗng luôn có nghĩa là "không cần di chuyển"**. Nó xuất hiện khi
//! `from == to`, và khi `best_effort` không tiến được bước nào. Quy ước này
//! quan trọng với phía client: nó không phải phân biệt "rỗng" với "đứng yên
//! tại chỗ" bằng cách so sánh phần tử đầu với vị trí hiện tại.
//!
//! ## Điều module này cố ý *không* làm
//!
//! - **Không dùng `WorldPos`.** Di chuyển đi bộ nằm trong mặt phẳng `z` hiện
//!   tại; đổi tầng (cầu thang, cổng) là một luật khác với chi phí khác, và trộn
//!   nó vào đây sẽ khiến heuristic mất tính chấp nhận được.
//! - **Không tự đọc [`crate::ChunkStore`].** Tính đi được hay không được truyền
//!   vào dưới dạng hàm, nên module này không kéo theo việc materialize chunk —
//!   một A* tự nạp chunk sẽ vi phạm `§22.12` bằng cách ghé qua hàng nghìn chunk
//!   cho mỗi cú bấm chuột.
//!
//! ## Hạn chế đã biết
//!
//! [`PathOutcome::Unreachable`] không mang theo `best_effort`. Nếu người chơi
//! bấm vào giữa một cái hồ **nhỏ** nằm trong một hòn đảo **nhỏ**, vùng tới được
//! sẽ bị quét cạn trước khi cạn trần, và kết quả là `Unreachable` — nhân vật
//! đứng im, dù về mặt trải nghiệm đi tới mép hồ mới đúng. Đây là đánh đổi có ý
//! thức: `Unreachable` mang thông tin mạnh hơn hẳn (ta đã **chứng minh** không
//! có đường, chứ không phải hết ngân sách), và client biết điều đó thì có thể
//! phát lại yêu cầu với một đích khác nếu muốn.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// Toạ độ một ô trên mặt phẳng di chuyển: `(x, y)`.
pub type Coord = (i64, i64);

/// Chi phí một bước theo trục.
///
/// Thang mười chứ không phải một: nó chừa chỗ để [`STEP_DIAGONAL`] xấp xỉ
/// `sqrt(2)` bằng số nguyên.
pub const STEP_STRAIGHT: i64 = 10;

/// Chi phí một bước chéo.
///
/// `14/10 = 1.4`, còn `sqrt(2) ≈ 1.41421`. Sai số dưới 1%, và nó là **cùng một
/// sai số trên mọi máy** — đó mới là thứ đáng giá ở đây.
pub const STEP_DIAGONAL: i64 = 14;

/// Tám hướng đi được, theo thứ tự `(dx, dy)` tăng dần.
///
/// Thứ tự này cố định trong mã và không được sắp lại: nó là một trong những thứ
/// quyết định đường nào thắng khi hai đường cùng chi phí.
const DIRECTIONS: [Coord; 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

/// Một yêu cầu tìm đường.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathRequest {
    /// Ô nhân vật đang đứng.
    pub from: Coord,
    /// Ô người chơi vừa bấm vào.
    pub to: Coord,
    /// Trần số node được **mở rộng** (pop khỏi hàng đợi rồi duyệt hàng xóm).
    ///
    /// Đây là thứ duy nhất ngăn một cú bấm ra giữa biển làm treo server. Không
    /// có giá trị "vô hạn", và đó là cố ý.
    pub max_nodes: usize,
}

/// Kết quả tìm đường.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathOutcome {
    /// Có đường tới đích, và nó **tối ưu** theo thang chi phí ở trên.
    ///
    /// Gồm cả ô xuất phát và ô đích. Rỗng khi `from == to`.
    Found(Vec<Coord>),
    /// Đã quét cạn vùng tới được mà không chạm đích: **chắc chắn** không có
    /// đường. Khác hẳn [`PathOutcome::BudgetExhausted`], vốn chỉ nói "chưa tìm
    /// thấy trong ngân sách cho phép".
    Unreachable,
    /// Cạn trần [`PathRequest::max_nodes`] trước khi tới đích.
    BudgetExhausted {
        /// Đường tới ô **gần đích nhất** từng chạm được, theo cùng quy ước với
        /// [`PathOutcome::Found`].
        ///
        /// Rỗng khi không tiến được bước nào — ví dụ `max_nodes` bằng 0, hoặc
        /// nhân vật bị vây kín ngay tại chỗ.
        best_effort: Vec<Coord>,
    },
}

/// Một ứng viên trong hàng đợi ưu tiên.
///
/// Cố tình **không** mang `g`: `f` và `h` đã xác định `g`, nên `PartialEq` dẫn
/// xuất khớp đúng với [`Ord`] viết tay bên dưới. Hai bản ghi cùng một ô mà khác
/// `g` thì cũng khác `f`, nên không bao giờ "bằng nhau" theo nhầm nghĩa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Candidate {
    f: i64,
    h: i64,
    at: Coord,
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // `BinaryHeap` là max-heap nên mọi so sánh đảo chiều: "lớn nhất" ở đây
        // nghĩa là "đáng mở rộng nhất".
        //
        // Phá hoà theo `h` trước `(x, y)` không chỉ để xác định — nó còn khiến
        // A* bám sát hướng đích khi nhiều node cùng `f`, và đó chính là thứ làm
        // `best_effort` có ý nghĩa khi ngân sách cạn giữa chừng.
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.h.cmp(&self.h))
            .then_with(|| other.at.cmp(&self.at))
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Khoảng cách theo một trục, không bao giờ tràn.
///
/// `to.x - from.x` trên `i64` tràn ngay khi hai đầu nằm ở hai cực của thế giới,
/// và một hiệu tràn thành số âm sẽ biến heuristic thành vô nghĩa — A* sẽ đi
/// ngược hướng đích. Tính trong `i128` rồi kẹp lại là cách rẻ nhất để chuyện đó
/// không xảy ra được.
fn axis_delta(a: i64, b: i64) -> i64 {
    let d = i128::from(a).abs_diff(i128::from(b));
    i64::try_from(d).unwrap_or(i64::MAX)
}

/// Octile distance: chi phí đi từ `a` tới `b` trên một lưới **trống**.
///
/// Đọc thẳng từ công thức: đi chéo `min(dx, dy)` bước, rồi đi thẳng phần dư.
/// Vì lưới thật chỉ có thể *đắt hơn* lưới trống, đây là cận dưới — điều kiện để
/// A* trả đường tối ưu.
fn octile(a: Coord, b: Coord) -> i64 {
    let dx = axis_delta(a.0, b.0);
    let dy = axis_delta(a.1, b.1);
    let (lo, hi) = if dx < dy { (dx, dy) } else { (dy, dx) };
    // Bão hoà thay vì tràn: ở khoảng cách cỡ `i64::MAX` giá trị chính xác không
    // còn ý nghĩa, nhưng một số âm do tràn thì phá cả thuật toán.
    lo.saturating_mul(STEP_DIAGONAL)
        .saturating_add((hi - lo).saturating_mul(STEP_STRAIGHT))
}

/// Chi phí một bước theo `step`.
fn step_cost(step: Coord) -> i64 {
    if step.0 != 0 && step.1 != 0 {
        STEP_DIAGONAL
    } else {
        STEP_STRAIGHT
    }
}

/// Ô đích của một bước, hoặc `None` nếu bước đó không hợp lệ.
///
/// Ba lý do từ chối, và lý do thứ ba là lý do hàm này tồn tại:
///
/// 1. **Tràn `i64`.** Thế giới vô hạn nhưng `i64` thì không. Không có
///    `checked_add`, một bước ở rìa miền sẽ quấn vòng và dịch chuyển nhân vật
///    sang đầu kia thế giới.
/// 2. **Ô đích đặc.**
/// 3. **Cắt góc.** Đi chéo khi *một trong hai* ô kề trục bị chặn nghĩa là lách
///    qua đúng cái góc giữa hai bức tường — nhân vật xuyên qua chỗ hẹp hơn cả
///    thân mình. Trường hợp cả hai ô đều đặc thì còn tệ hơn: đó là đi xuyên
///    tường. Cả hai đều bị chặn ở đây, nên luật là "cả hai ô kề trục phải đi
///    được", không phải "ít nhất một".
fn step_target(at: Coord, step: Coord, passable: &impl Fn(i64, i64) -> bool) -> Option<Coord> {
    let nx = at.0.checked_add(step.0)?;
    let ny = at.1.checked_add(step.1)?;
    if !passable(nx, ny) {
        return None;
    }
    if step.0 != 0 && step.1 != 0 && (!passable(nx, at.1) || !passable(at.0, ny)) {
        return None;
    }
    Some((nx, ny))
}

/// Dựng lại đường từ cây cha.
///
/// Trần `guard` là phòng thủ chiều sâu. Cây cha **không thể** có chu trình —
/// một node chỉ được nhận cha mới khi `g` giảm, và cha luôn có `g` nhỏ hơn con
/// — nhưng nếu bất biến đó vỡ vì một lần sửa sau này, vòng lặp không trần sẽ
/// treo cứng luồng server thay vì trả kết quả sai. Một phép so sánh mỗi bước là
/// cái giá rẻ để đổi lấy điều đó.
fn rebuild(parent: &BTreeMap<Coord, Coord>, from: Coord, target: Coord) -> Vec<Coord> {
    if target == from {
        return Vec::new();
    }
    let mut out = vec![target];
    let mut cur = target;
    let mut guard = parent.len() + 1;
    while cur != from {
        let Some(&prev) = parent.get(&cur) else {
            // Chuỗi đứt: trả "không di chuyển" thay vì một đường cụt.
            return Vec::new();
        };
        out.push(prev);
        cur = prev;
        guard -= 1;
        if guard == 0 {
            return Vec::new();
        }
    }
    out.reverse();
    out
}

/// Tìm đường từ `req.from` tới `req.to`, tránh mọi ô mà `passable` trả `false`.
///
/// `passable(x, y)` phải là **hàm thuần và ổn định trong suốt một lần gọi**:
/// A* giả định chi phí không đổi giữa lúc mở và lúc đóng một node, và một vị từ
/// thay đổi giữa chừng sẽ cho ra đường đi xuyên tường mà không báo lỗi gì.
///
/// Ô xuất phát **không bị kiểm tra**. Nhân vật đang đứng ở đó rồi; từ chối tìm
/// đường vì chính ô mình đang đứng sẽ khiến nhân vật kẹt vĩnh viễn khi một luật
/// mới (nước dâng, nhà sập) biến ô đó thành không đi được.
///
/// # Quy ước trả về
///
/// - `Found(path)` — `path` gồm **cả** `from` và `to`, và **rỗng** khi
///   `from == to`.
/// - `Unreachable` — đã quét cạn vùng tới được, chắc chắn không có đường.
/// - `BudgetExhausted { best_effort }` — cạn trần; `best_effort` là đường tới ô
///   gần đích nhất đã chạm, rỗng nếu không tiến được bước nào.
///
/// # Ví dụ
///
/// ```
/// use mow_spatial::pathfind::{find_path, PathOutcome, PathRequest};
///
/// // Một bức tường dọc ở `x == 1`, chừa một khe ở `y == 0`.
/// let passable = |x: i64, y: i64| x != 1 || y == 0;
/// let req = PathRequest { from: (0, 2), to: (2, 2), max_nodes: 10_000 };
///
/// let PathOutcome::Found(path) = find_path(&req, &passable) else {
///     panic!("phải có đường qua khe");
/// };
/// assert_eq!(path.first(), Some(&(0, 2)));
/// assert_eq!(path.last(), Some(&(2, 2)));
/// assert!(path.iter().all(|&(x, y)| passable(x, y)));
/// ```
pub fn find_path(req: &PathRequest, passable: &impl Fn(i64, i64) -> bool) -> PathOutcome {
    if req.from == req.to {
        return PathOutcome::Found(Vec::new());
    }

    let mut open: BinaryHeap<Candidate> = BinaryHeap::new();
    let mut cost: BTreeMap<Coord, i64> = BTreeMap::new();
    let mut parent: BTreeMap<Coord, Coord> = BTreeMap::new();
    let mut closed: BTreeSet<Coord> = BTreeSet::new();

    let start_h = octile(req.from, req.to);
    cost.insert(req.from, 0);
    open.push(Candidate {
        f: start_h,
        h: start_h,
        at: req.from,
    });

    // Ô gần đích nhất từng chạm tới, dưới dạng `(h, g, toạ độ)` — so sánh từ
    // điển trên bộ ba này vừa là tiêu chí "gần đích nhất, rẻ nhất", vừa là một
    // cách phá hoà xác định. Theo dõi tại lúc *nới lỏng* chứ không phải lúc
    // *đóng* node: với ngân sách nhỏ, ô tốt nhất thường vẫn còn nằm trong hàng
    // đợi khi trần cạn, và nó đã có cha hợp lệ nên dựng lại đường được.
    let mut best: (i64, i64, Coord) = (start_h, 0, req.from);

    while let Some(cur) = open.pop() {
        // Một ô có thể nằm nhiều lần trong hàng đợi vì ta không xoá bản ghi cũ
        // khi tìm được đường rẻ hơn. Bản ghi đầu tiên pop ra là bản rẻ nhất;
        // phần còn lại bỏ qua.
        if !closed.insert(cur.at) {
            continue;
        }
        if cur.at == req.to {
            return PathOutcome::Found(rebuild(&parent, req.from, req.to));
        }
        // Kiểm tra trần *sau* khi thử đích: chạm đích ở đúng node cuối cùng thì
        // vẫn là tìm thấy, không phải hết ngân sách.
        if closed.len() > req.max_nodes {
            return PathOutcome::BudgetExhausted {
                best_effort: rebuild(&parent, req.from, best.2),
            };
        }
        let Some(&g_cur) = cost.get(&cur.at) else {
            continue;
        };

        for step in DIRECTIONS {
            let Some(next) = step_target(cur.at, step, passable) else {
                continue;
            };
            if closed.contains(&next) {
                continue;
            }
            let g_next = g_cur + step_cost(step);
            // `i64::MAX` làm mốc "chưa từng thấy": `g` thật bị chặn trên bởi
            // `14 * max_nodes` nên không bao giờ chạm tới mốc này.
            if g_next >= cost.get(&next).copied().unwrap_or(i64::MAX) {
                continue;
            }
            cost.insert(next, g_next);
            parent.insert(next, cur.at);

            let h = octile(next, req.to);
            if (h, g_next, next) < best {
                best = (h, g_next, next);
            }
            open.push(Candidate {
                f: g_next.saturating_add(h),
                h,
                at: next,
            });
        }
    }

    PathOutcome::Unreachable
}

/// Gộp các bước cùng hướng thành đoạn: `(hướng, số bước)`.
///
/// Client nhận đường đi rồi phát lệnh di chuyển. Một đường 200 ô là 199 lệnh
/// nếu gửi từng bước, nhưng thường chỉ vài đoạn nếu gộp — và số lệnh là thứ
/// chiếm băng thông trên đường mạng, không phải độ dài đường.
///
/// `hướng` là hiệu toạ độ giữa hai ô liên tiếp. Với đường do [`find_path`] trả
/// về nó luôn nằm trong `{-1, 0, 1}²`; hàm này không *đòi* điều đó, nên một
/// đường thưa hơn vẫn gộp được (mỗi hiệu khác nhau thành một đoạn riêng).
/// Cặp ô trùng nhau bị bỏ qua vì chúng không mã hoá chuyển động nào.
///
/// Tổng số bước luôn bằng số cặp ô liên tiếp *thật sự có dịch chuyển*, nên với
/// một đường hợp lệ nó bằng `path.len() - 1`.
///
/// # Ví dụ
///
/// ```
/// use mow_spatial::pathfind::simplify;
///
/// let path = [(0, 0), (1, 0), (2, 0), (3, 1), (4, 2)];
/// assert_eq!(simplify(&path), vec![((1, 0), 2), ((1, 1), 2)]);
/// assert!(simplify(&[(7, 7)]).is_empty());
/// ```
pub fn simplify(path: &[Coord]) -> Vec<(Coord, usize)> {
    let mut out: Vec<(Coord, usize)> = Vec::new();
    for pair in path.windows(2) {
        // `saturating_sub` chứ không phải `-`: hàm này công khai và có thể nhận
        // một đường bất kỳ, kể cả hai ô ở hai cực của thế giới.
        let step = (
            pair[1].0.saturating_sub(pair[0].0),
            pair[1].1.saturating_sub(pair[0].1),
        );
        if step == (0, 0) {
            continue;
        }
        match out.last_mut() {
            Some((dir, n)) if *dir == step => *n += 1,
            _ => out.push((step, 1)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trần rộng rãi: đủ để mọi thế giới trong test này bị quét cạn.
    const ROOMY: usize = 100_000;

    /// Thế giới trống trơn.
    fn open_world(_x: i64, _y: i64) -> bool {
        true
    }

    /// Kiểm mọi tính chất mà một đường hợp lệ phải có.
    ///
    /// Gom vào một chỗ vì mỗi bài test dưới đây đều cần cả bộ: một bài chỉ kiểm
    /// hai đầu mút sẽ cho qua một đường nhảy xuyên tường.
    fn assert_valid_path(
        path: &[Coord],
        from: Coord,
        to: Coord,
        passable: &impl Fn(i64, i64) -> bool,
    ) {
        assert_eq!(
            path.first(),
            Some(&from),
            "đường phải bắt đầu ở ô xuất phát"
        );
        assert_eq!(path.last(), Some(&to), "đường phải kết thúc ở ô đích");
        for &(x, y) in path {
            assert!(passable(x, y), "đường đi qua ô đặc ({x}, {y})");
        }
        for pair in path.windows(2) {
            let (ax, ay) = pair[0];
            let (bx, by) = pair[1];
            let dx = bx - ax;
            let dy = by - ay;
            assert!(
                dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0),
                "bước ({dx}, {dy}) không phải một ô kề"
            );
            if dx != 0 && dy != 0 {
                assert!(
                    passable(bx, ay) && passable(ax, by),
                    "bước chéo từ ({ax}, {ay}) sang ({bx}, {by}) cắt góc"
                );
            }
        }
    }

    /// Chi phí thật của một đường, theo thang [`STEP_STRAIGHT`]/[`STEP_DIAGONAL`].
    fn path_cost(path: &[Coord]) -> i64 {
        simplify(path)
            .into_iter()
            .map(|(dir, n)| step_cost(dir) * i64::try_from(n).unwrap_or(i64::MAX))
            .sum()
    }

    fn found(req: &PathRequest, passable: &impl Fn(i64, i64) -> bool) -> Vec<Coord> {
        match find_path(req, passable) {
            PathOutcome::Found(p) => p,
            other => panic!("mong đợi Found, nhận {other:?}"),
        }
    }

    // ── Đường thẳng ──────────────────────────────────────────────────────────

    #[test]
    fn duong_thang_khong_vat_can_dung_do_dai() {
        let req = PathRequest {
            from: (0, 0),
            to: (5, 0),
            max_nodes: ROOMY,
        };
        let path = found(&req, &open_world);
        assert_valid_path(&path, (0, 0), (5, 0), &open_world);
        assert_eq!(path.len(), 6, "5 bước thì có 6 ô, kể cả hai đầu");
        assert_eq!(path_cost(&path), 5 * STEP_STRAIGHT);
        assert_eq!(simplify(&path), vec![((1, 0), 5)]);
    }

    #[test]
    fn duong_cheo_hoan_toan_dung_do_dai() {
        let req = PathRequest {
            from: (0, 0),
            to: (5, 5),
            max_nodes: ROOMY,
        };
        let path = found(&req, &open_world);
        assert_valid_path(&path, (0, 0), (5, 5), &open_world);
        assert_eq!(
            path.len(),
            6,
            "đi chéo 5 bước, không phải 10 bước vuông góc"
        );
        assert_eq!(path_cost(&path), 5 * STEP_DIAGONAL);
    }

    #[test]
    fn cheo_roi_thang_re_hon_thang_roi_cheo_thi_bang_nhau_nhung_toi_uu() {
        // `dx = 4`, `dy = 2`: tối ưu là 2 bước chéo + 2 bước thẳng = 48.
        let req = PathRequest {
            from: (0, 0),
            to: (4, 2),
            max_nodes: ROOMY,
        };
        let path = found(&req, &open_world);
        assert_valid_path(&path, (0, 0), (4, 2), &open_world);
        assert_eq!(path.len(), 5);
        assert_eq!(path_cost(&path), 2 * STEP_DIAGONAL + 2 * STEP_STRAIGHT);
        assert_eq!(path_cost(&path), octile((0, 0), (4, 2)));
    }

    // ── Vật cản ──────────────────────────────────────────────────────────────

    /// Bức tường chữ U mở lên trên, miệng ở `y == 5`.
    ///
    /// ```text
    ///   y=5   . . . . . . .   ← miệng
    ///   y=4   . . # . # . .
    ///   y=3   . . # S # . .
    ///   y=2   . . # . # . .
    ///   y=1   . . # # # . .
    ///   y=0   . . . G . . .
    /// ```
    fn u_wall(x: i64, y: i64) -> bool {
        if !(0..=6).contains(&x) || !(0..=6).contains(&y) {
            return false;
        }
        let arm = (x == 2 || x == 4) && (1..=4).contains(&y);
        let bottom = y == 1 && (2..=4).contains(&x);
        !(arm || bottom)
    }

    #[test]
    fn vong_qua_tuong_chu_u() {
        let req = PathRequest {
            from: (3, 3),
            to: (3, 0),
            max_nodes: ROOMY,
        };
        let path = found(&req, &u_wall);
        assert_valid_path(&path, (3, 3), (3, 0), &u_wall);
        // Đường thẳng là 3 bước; phải trèo ra miệng chữ U rồi vòng xuống, nên
        // bất kỳ đường hợp lệ nào cũng dài hơn hẳn.
        assert!(
            path.len() > 4,
            "đường {path:?} ngắn hơn mức có thể — nó đã xuyên tường"
        );
        assert!(
            path.iter().any(|&(_, y)| y >= 5),
            "phải đi qua miệng chữ U ở y = 5"
        );
    }

    #[test]
    fn dich_bi_bao_kin_tra_unreachable() {
        // Hộp 0..=8, bên trong có một căn phòng kín quanh ô (4, 4).
        let sealed = |x: i64, y: i64| {
            if !(0..=8).contains(&x) || !(0..=8).contains(&y) {
                return false;
            }
            let on_ring = (3..=5).contains(&x)
                && (3..=5).contains(&y)
                && (x == 3 || x == 5 || y == 3 || y == 5);
            !on_ring
        };
        let req = PathRequest {
            from: (0, 0),
            to: (4, 4),
            max_nodes: ROOMY,
        };
        // Vùng tới được có chưa tới 100 ô, còn trần là 100 000 — nên nếu kết
        // quả là `BudgetExhausted` thì lỗi nằm ở chỗ khác chứ không phải ngân
        // sách.
        assert_eq!(find_path(&req, &sealed), PathOutcome::Unreachable);
    }

    #[test]
    fn dich_dat_tren_o_dac_van_unreachable() {
        let walled =
            |x: i64, y: i64| (0..=4).contains(&x) && (0..=4).contains(&y) && (x, y) != (2, 2);
        let req = PathRequest {
            from: (0, 0),
            to: (2, 2),
            max_nodes: ROOMY,
        };
        assert_eq!(find_path(&req, &walled), PathOutcome::Unreachable);
    }

    // ── Cấm cắt góc ──────────────────────────────────────────────────────────

    #[test]
    fn khong_cat_goc_giua_hai_o_dac_cheo_nhau() {
        // Thế giới chỉ có 4 ô; (1,0) và (0,1) đặc. Còn lại (0,0) và (1,1) chéo
        // nhau, và khe giữa chúng là khe giữa hai bức tường — không đi được.
        let pinch = |x: i64, y: i64| {
            (0..=1).contains(&x) && (0..=1).contains(&y) && (x, y) != (1, 0) && (x, y) != (0, 1)
        };
        let req = PathRequest {
            from: (0, 0),
            to: (1, 1),
            max_nodes: ROOMY,
        };
        assert_eq!(
            find_path(&req, &pinch),
            PathOutcome::Unreachable,
            "đi chéo giữa hai ô đặc là đi xuyên tường"
        );
    }

    #[test]
    fn mot_o_ke_bi_chan_thi_di_vong_chu_khong_lach_goc() {
        // Chỉ (1,0) đặc. Đường chéo (0,0) → (1,1) vẫn bị cấm (lách góc), nhưng
        // (0,0) → (0,1) → (1,1) thì hợp lệ.
        let corner =
            |x: i64, y: i64| (0..=1).contains(&x) && (0..=1).contains(&y) && (x, y) != (1, 0);
        let req = PathRequest {
            from: (0, 0),
            to: (1, 1),
            max_nodes: ROOMY,
        };
        let path = found(&req, &corner);
        assert_eq!(path, vec![(0, 0), (0, 1), (1, 1)]);
        assert_eq!(path_cost(&path), 2 * STEP_STRAIGHT);
    }

    // ── Ngân sách ────────────────────────────────────────────────────────────

    #[test]
    fn tran_nho_tra_best_effort_tien_ve_phia_dich() {
        let from = (0, 0);
        let to = (1_000, 0);
        let req = PathRequest {
            from,
            to,
            max_nodes: 64,
        };
        let PathOutcome::BudgetExhausted { best_effort } = find_path(&req, &open_world) else {
            panic!("một đích cách 1000 ô không thể xong trong 64 node");
        };
        assert!(!best_effort.is_empty(), "phải đi được ít nhất một bước");
        let Some(&reached) = best_effort.last() else {
            panic!("vừa kiểm là không rỗng");
        };
        assert_valid_path(&best_effort, from, reached, &open_world);
        assert!(
            octile(reached, to) < octile(from, to),
            "best_effort dừng ở {reached:?}, không gần đích hơn điểm xuất phát"
        );
    }

    #[test]
    fn tran_bang_khong_tra_best_effort_rong() {
        let req = PathRequest {
            from: (0, 0),
            to: (10, 10),
            max_nodes: 0,
        };
        assert_eq!(
            find_path(&req, &open_world),
            PathOutcome::BudgetExhausted {
                best_effort: Vec::new()
            },
            "không có ngân sách thì không có bước nào, và đường rỗng nghĩa là đứng yên"
        );
    }

    #[test]
    fn bi_vay_kin_ngay_tai_cho_van_ket_thuc() {
        // Nhân vật đứng trên ô duy nhất đi được. Không có đường, và quan trọng
        // hơn: hàng đợi cạn ngay, không quét ra vô tận.
        let boxed_in = |x: i64, y: i64| (x, y) == (0, 0);
        let req = PathRequest {
            from: (0, 0),
            to: (50, 50),
            max_nodes: ROOMY,
        };
        assert_eq!(find_path(&req, &boxed_in), PathOutcome::Unreachable);
    }

    #[test]
    fn o_xuat_phat_dac_van_di_ra_duoc() {
        // Nước dâng lên đúng ô nhân vật đang đứng. Nếu ô xuất phát bị kiểm tra,
        // nhân vật kẹt ở đó vĩnh viễn.
        let flooded =
            |x: i64, y: i64| (0..=3).contains(&x) && (0..=3).contains(&y) && (x, y) != (0, 0);
        let req = PathRequest {
            from: (0, 0),
            to: (3, 3),
            max_nodes: ROOMY,
        };
        let path = found(&req, &flooded);
        assert_eq!(path.first(), Some(&(0, 0)));
        assert_eq!(path.last(), Some(&(3, 3)));
        assert!(
            path[1..].iter().all(|&(x, y)| flooded(x, y)),
            "chỉ ô xuất phát được miễn kiểm tra"
        );
    }

    // ── Xác định ─────────────────────────────────────────────────────────────

    #[test]
    fn cung_dau_vao_cho_cung_ket_qua_qua_100_lan() {
        // Bức tường dọc `x == 5` với hai khe **đối xứng** ở `y == 0` và
        // `y == 10`: hai đường vòng có chi phí bằng đúng nhau. Đây là chỗ một
        // cài đặt dựa vào thứ tự duyệt `HashMap` sẽ lộ ra.
        let two_gaps = |x: i64, y: i64| {
            if !(0..=10).contains(&x) || !(0..=10).contains(&y) {
                return false;
            }
            x != 5 || y == 0 || y == 10
        };
        let req = PathRequest {
            from: (0, 5),
            to: (10, 5),
            max_nodes: ROOMY,
        };
        let first = find_path(&req, &two_gaps);
        let PathOutcome::Found(ref path) = first else {
            panic!("phải có đường qua một trong hai khe, nhận {first:?}");
        };
        assert_valid_path(path, (0, 5), (10, 5), &two_gaps);

        // So sánh cả dạng in ra: đây là "byte-for-byte" theo nghĩa chặt nhất mà
        // một bài test trong tiến trình có thể kiểm.
        let reference = format!("{first:?}");
        for lan in 0..100 {
            let again = find_path(&req, &two_gaps);
            assert_eq!(again, first, "lần chạy {lan} cho đường khác");
            assert_eq!(format!("{again:?}"), reference, "lần chạy {lan} khác byte");
        }
    }

    #[test]
    fn ngan_sach_can_cung_xac_dinh() {
        let req = PathRequest {
            from: (0, 0),
            to: (5_000, 3_000),
            max_nodes: 128,
        };
        let first = find_path(&req, &open_world);
        assert!(matches!(first, PathOutcome::BudgetExhausted { .. }));
        for lan in 0..100 {
            assert_eq!(find_path(&req, &open_world), first, "lần chạy {lan} khác");
        }
    }

    // ── Miền toạ độ ──────────────────────────────────────────────────────────

    #[test]
    fn toa_do_am() {
        let from = (-10, -10);
        let to = (-5, -7);
        let req = PathRequest {
            from,
            to,
            max_nodes: ROOMY,
        };
        let path = found(&req, &open_world);
        assert_valid_path(&path, from, to, &open_world);
        assert_eq!(path.len(), 6, "max(5, 3) = 5 bước");
        assert_eq!(path_cost(&path), octile(from, to));
    }

    #[test]
    fn toa_do_rat_lon_quanh_2_mu_40() {
        let base: i64 = 1 << 40;
        let from = (base, -base);
        let to = (base + 7, -base + 3);
        let req = PathRequest {
            from,
            to,
            max_nodes: ROOMY,
        };
        let path = found(&req, &open_world);
        assert_valid_path(&path, from, to, &open_world);
        assert_eq!(path.len(), 8, "max(7, 3) = 7 bước");
        assert_eq!(path_cost(&path), octile(from, to));
    }

    #[test]
    fn khoang_cach_khong_lo_khong_tran_thanh_so_am() {
        // Hiệu vượt `i64` — nếu heuristic tràn, nó thành số âm và A* sẽ bò
        // ngược hướng đích thay vì tiến về phía nó.
        let h = octile((i64::MIN, i64::MIN), (i64::MAX, i64::MAX));
        assert!(h > 0, "heuristic tràn thành {h}");
        assert_eq!(h, i64::MAX, "phải bão hoà chứ không quấn vòng");
    }

    #[test]
    fn buoc_o_ria_mien_i64_khong_quan_vong() {
        // Ở đúng góc miền `i64`, ba trong tám hướng đi ra ngoài miền. Chúng
        // phải bị bỏ qua chứ không quấn về đầu kia thế giới.
        let corner = (i64::MAX, i64::MAX);
        for step in DIRECTIONS {
            if let Some((nx, ny)) = step_target(corner, step, &open_world) {
                assert!(
                    (nx - corner.0).abs() <= 1 && (ny - corner.1).abs() <= 1,
                    "bước {step:?} nhảy tới ({nx}, {ny})"
                );
            }
        }
    }

    // ── Trường hợp biên ──────────────────────────────────────────────────────

    #[test]
    fn from_bang_to_tra_duong_rong() {
        let req = PathRequest {
            from: (7, -3),
            to: (7, -3),
            max_nodes: 0,
        };
        assert_eq!(
            find_path(&req, &|_, _| false),
            PathOutcome::Found(Vec::new()),
            "đã đứng ở đích thì không cần đi, và không cần cả ngân sách"
        );
    }

    #[test]
    fn hai_o_ke_nhau_cho_duong_hai_phan_tu() {
        let req = PathRequest {
            from: (0, 0),
            to: (1, 0),
            max_nodes: ROOMY,
        };
        assert_eq!(found(&req, &open_world), vec![(0, 0), (1, 0)]);
    }

    // ── simplify ─────────────────────────────────────────────────────────────

    #[test]
    fn simplify_gop_doan_cung_huong() {
        let path = [(0, 0), (1, 0), (2, 0), (3, 1), (4, 2)];
        assert_eq!(simplify(&path), vec![((1, 0), 2), ((1, 1), 2)]);
    }

    #[test]
    fn simplify_duong_qua_ngan_thi_rong() {
        assert!(simplify(&[]).is_empty());
        assert!(
            simplify(&[(3, 4)]).is_empty(),
            "một ô thì không có bước nào"
        );
    }

    #[test]
    fn simplify_bo_qua_cap_o_trung_nhau() {
        let path = [(0, 0), (0, 0), (1, 0), (1, 0), (2, 0)];
        assert_eq!(simplify(&path), vec![((1, 0), 2)]);
    }

    #[test]
    fn simplify_giu_nguyen_tong_so_buoc() {
        let req = PathRequest {
            from: (-4, 9),
            to: (12, -3),
            max_nodes: ROOMY,
        };
        let path = found(&req, &open_world);
        let total: usize = simplify(&path).into_iter().map(|(_, n)| n).sum();
        assert_eq!(
            total,
            path.len() - 1,
            "gộp đoạn không được làm mất bước nào"
        );
    }

    #[test]
    fn simplify_khong_gop_hai_huong_khac_nhau() {
        let path = [(0, 0), (1, 1), (2, 0), (3, 1)];
        assert_eq!(
            simplify(&path),
            vec![((1, 1), 1), ((1, -1), 1), ((1, 1), 1)]
        );
    }
}
