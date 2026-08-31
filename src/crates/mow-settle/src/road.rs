//! Đường làng: một nét tim đường rộng một ô, hai mép răng cưa.
//!
//! Vì sao phải răng cưa: một con đường đúng ba ô rộng, hai mép thẳng tắp, đọc
//! ra là *công trình đo đạc* — thứ mà một ngôi làng khởi đầu không có. Xen kẽ
//! `topsoil` ở mép làm cho bề rộng dao động giữa một và ba ô, và mắt đọc ra
//! "lối mòn bị dẫm thành đường".
//!
//! Răng cưa là hàm thuần của `(seed, x, y)` chứ không phải của thứ tự vẽ. Nhờ
//! thế hai con đường chồng lên nhau vẫn răng cưa giống hệt nhau ở chỗ chúng
//! chung, và không có đường nối nào lộ ra vết ghép.

use std::collections::BTreeSet;

use crate::canvas::{Canvas, Claim};
use crate::geom::{cheb, Rect};
use crate::hash::{hash_xy, salt};
use crate::material::{PATH_GRAVEL, PRIO_DITHER, PRIO_PATH, TOPSOIL};
use crate::Site;

/// Chặn trên số ô một con đường được phép chiếm.
///
/// Không phải để tối ưu: đây là dây bảo hiểm. Một lỗi hình học biến vòng lặp
/// men theo đường thành vòng lặp vô hạn sẽ treo cả server; chặn trên biến nó
/// thành "làng thiếu một con đường", thứ nhìn thấy được và không giết ai.
const MAX_ROAD_CELLS: usize = 512;

/// Bước một ô về phía `to`: đi hết trục hoành trước rồi mới tới trục tung.
fn step_toward(from: (i64, i64), to: (i64, i64)) -> (i64, i64) {
    if from.0 == to.0 {
        (from.0, from.1 + (to.1 - from.1).signum())
    } else {
        (from.0 + (to.0 - from.0).signum(), from.1)
    }
}

/// Trải một đường gấp khúc thành danh sách ô, dừng ngay khi chạm quảng trường.
///
/// Dừng sớm là điều làm cho các con đường *nhập* vào quảng trường thay vì đâm
/// xuyên qua nó tới tận miệng giếng.
fn walk(poly: &[(i64, i64)], site: &Site) -> Option<Vec<(i64, i64)>> {
    let in_plaza = |p: (i64, i64)| cheb(p, site.hub) <= site.plaza_half;
    let mut out = vec![poly[0]];
    if in_plaza(poly[0]) {
        return Some(out);
    }
    let mut cur = poly[0];
    for &next in &poly[1..] {
        while cur != next {
            cur = step_toward(cur, next);
            out.push(cur);
            if out.len() > MAX_ROAD_CELLS {
                return None;
            }
            if in_plaza(cur) {
                return Some(out);
            }
        }
    }
    // Chặng cuối luôn là quảng trường nên tới đây là hình học đã sai.
    None
}

/// Các lối đi thử, theo thứ tự: ngang trước, dọc trước, rồi vòng qua sườn nhà.
///
/// Hai lối vòng tồn tại cho những ngôi nhà nằm *dưới* quảng trường. Cửa luôn ở
/// mép dưới, nên với chúng, bước ra khỏi cửa là bước ngược hướng làng, và cả
/// hai lối chữ L đều đâm trở lại vào chính cái mái vừa dựng. Thử cả hai sườn
/// chứ không chỉ sườn quay về làng: nửa số ngôi nhà bị loại oan chỉ vì sườn gần
/// hơn tình cờ đã có một con đường khác chạy qua.
fn routes(from: (i64, i64), site: &Site, avoid: Option<Rect>) -> Vec<Vec<(i64, i64)>> {
    let hub = site.hub;
    let mut out = vec![
        vec![from, (hub.0, from.1), hub],
        vec![from, (from.0, hub.1), hub],
    ];
    if let Some(rect) = avoid {
        let right = rect.x.saturating_add(rect.w).saturating_add(1);
        let left = rect.x.saturating_sub(2);
        let (near, far) = if hub.0 >= from.0 {
            (right, left)
        } else {
            (left, right)
        };
        out.push(vec![from, (near, from.1), (near, hub.1), hub]);
        out.push(vec![from, (far, from.1), (far, hub.1), hub]);
    }
    out
}

/// Vẽ một con đường từ `from` về quảng trường. Trả `false` nếu không lối nào
/// đi lọt — người gọi sẽ tự quyết định bỏ chỗ đặt đó.
///
/// Kiểm hết rồi mới tô: nếu tô dần rồi gặp chướng ngại giữa chừng, làng sẽ có
/// những mẩu đường cụt không dẫn tới đâu, và không cách nào xóa chúng đi nữa.
pub(crate) fn carve(
    canvas: &mut Canvas,
    site: &Site,
    from: (i64, i64),
    avoid: Option<Rect>,
) -> bool {
    for poly in routes(from, site, avoid) {
        let Some(cells) = walk(&poly, site) else {
            continue;
        };
        let blocked = cells
            .iter()
            .any(|&p| !canvas.free_path(p) || avoid.is_some_and(|r| r.contains(p)));
        if blocked {
            continue;
        }
        paint(canvas, site, &cells, avoid);
        return true;
    }
    false
}

/// Tô tim đường rồi rắc răng cưa ra bốn phía.
fn paint(canvas: &mut Canvas, site: &Site, cells: &[(i64, i64)], avoid: Option<Rect>) {
    let core: BTreeSet<(i64, i64)> = cells.iter().copied().collect();
    for &p in cells {
        canvas.claim(p, Claim::Path);
        canvas.paint(p, PATH_GRAVEL, PRIO_PATH);
    }
    for &(x, y) in cells {
        for edge in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if core.contains(&edge) || avoid.is_some_and(|r| r.contains(edge)) {
                continue;
            }
            if !canvas.free_path(edge) {
                continue;
            }
            canvas.claim(edge, Claim::Path);
            canvas.paint(edge, dither(site.seed, edge), PRIO_DITHER);
        }
    }
}

/// Mép đường: một nửa số ô là mặt đường, nửa còn lại là đất chưa dẫm tới.
pub(crate) fn dither(seed: u64, p: (i64, i64)) -> &'static str {
    if hash_xy(seed, salt::DITHER, p.0, p.1) & 1 == 0 {
        PATH_GRAVEL
    } else {
        TOPSOIL
    }
}
