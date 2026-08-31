//! Quảng trường và cái giếng ở giữa nó.
//!
//! Giếng là mỏ neo của cả bản quy hoạch: nó được chọn *trước*, mọi thứ khác
//! quây quanh nó. Đảo thứ tự lại — đặt nhà trước rồi tìm chỗ cho giếng — thì
//! làng mất tâm, và cái làm cho ba chục nóc nhà đọc ra là một khu định cư chứ
//! không phải một vụ tai nạn chính là cái tâm đó.

use crate::canvas::{Canvas, Claim};
use crate::geom::{cheb, ring, Rect};
use crate::material::{
    stand_on, PATH_GRAVEL, PRIO_DITHER, PRIO_PATH, PRIO_WELL, PRIO_WELL_RIM, SEDIMENTARY, WATER,
};
use crate::road::dither;
use crate::{Building, BuildingKind, Site};

/// Bán kính quảng trường theo bán kính vùng quy hoạch.
///
/// Tối thiểu là 2 chứ không phải 1: với 1 thì lòng giếng ăn hết quảng trường,
/// không còn một ô mặt đường nào để đứng mà múc nước, và các con đường không có
/// gì để nhập vào.
pub(crate) fn plaza_half(radius: i64) -> i64 {
    (radius / 8).clamp(2, 3)
}

/// Tìm tâm làng: ô gần `center` nhất mà cả khoảnh 3x3 quanh nó xây được.
///
/// Dò theo vành đai từ trong ra ngoài nên "gần nhất" là gần thật, và thứ tự dò
/// cố định nên hai lần gọi cho cùng một tâm. Đòi cả 3x3 vì cái giếng chiếm đúng
/// chừng ấy: một tâm mà chỉ có một ô khô ráo sẽ cho ra một cái giếng thò ra
/// biển.
pub(crate) fn find_hub(canvas: &Canvas, center: (i64, i64), radius: i64) -> Option<(i64, i64)> {
    for r in 0..=radius {
        for p in ring(center, r) {
            if !canvas.usable(p) {
                continue;
            }
            if Rect::centered(p, 3, 3).cells().all(|c| canvas.free(c)) {
                return Some(p);
            }
        }
    }
    None
}

/// Lát quảng trường, đào giếng, và trả về cái giếng dưới dạng công trình.
///
/// Giếng là một `Building` để cư dân có chỗ mà chỉ tới: `Elder` làm việc ở
/// giếng, và `workplace` là chỉ số vào `buildings` nên giếng buộc phải nằm
/// trong danh sách đó.
pub(crate) fn carve(canvas: &mut Canvas, site: &Site) -> Building {
    let hub = site.hub;
    let half = site.plaza_half;

    // Mặt quảng trường trước, để cái giếng có nền mà đè lên.
    for p in Rect::centered(hub, half * 2 + 1, half * 2 + 1).cells() {
        if !canvas.free_path(p) {
            continue;
        }
        canvas.claim(p, Claim::Path);
        canvas.paint(p, PATH_GRAVEL, PRIO_PATH);
    }
    // Vành ngoài răng cưa, cùng luật với mép đường: quảng trường phải tan dần
    // vào đất chứ không được cắt một hình vuông sắc lẹm giữa đồng cỏ.
    for p in ring(hub, half + 1) {
        if !canvas.free_path(p) {
            continue;
        }
        canvas.claim(p, Claim::Path);
        canvas.paint(p, dither(site.seed, p), PRIO_DITHER);
    }

    let well = Rect::centered(hub, 3, 3);
    canvas.claim_rect(well, Claim::Solid);
    for p in well.cells() {
        canvas.paint(p, SEDIMENTARY, PRIO_WELL_RIM);
    }
    canvas.paint(hub, WATER, PRIO_WELL);

    Building {
        kind: BuildingKind::Well,
        origin: (well.x, well.y),
        w: 3,
        h: 3,
        door: well_door(canvas, hub, half),
    }
}

/// Ô mà người ta đứng để múc nước.
///
/// Ưu tiên mặt quảng trường ngay dưới thành giếng; nếu vùng quá chật đến mức ô
/// đó không tồn tại thì lùi vào chính thành giếng — vẫn là một ô có thật trong
/// `cells`, và đó là điều kiện duy nhất mà người gọi trông cậy được.
fn well_door(canvas: &Canvas, hub: (i64, i64), half: i64) -> (i64, i64) {
    let apron = (hub.0, hub.1.saturating_add(half));
    debug_assert!(cheb(apron, hub) <= half);
    match canvas.material_at(apron) {
        Some(m) if stand_on(m) => apron,
        _ => (hub.0, hub.1 + 1),
    }
}
