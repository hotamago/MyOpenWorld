//! Thửa ruộng.
//!
//! Cỏ dại cũng xanh, đất trống cũng nâu. Thứ phân biệt "có người cày ở đây" với
//! "chỗ này cây mọc" không phải là màu mà là **hình học**: một chữ nhật, những
//! luống thẳng song song, bước luống đều. Thiên nhiên không kẻ đường thẳng.
//!
//! Vì thế `farmland` và `crop_green` ở đây không phải hai vật liệu khác nhau
//! cạnh nhau, mà là hai pha của cùng một luật xen kẽ — đổi bảng màu đi thì
//! ruộng vẫn còn là ruộng.

use crate::canvas::{Canvas, Claim};
use crate::geom::Rect;
use crate::hash::{hash_xy, pick, salt};
use crate::material::{CROP_GREEN, FARMLAND, PRIO_FIELD};
use crate::road;
use crate::{Building, BuildingKind, Site};

/// Thử mở một thửa ruộng có tâm rơi vào `slot`.
///
/// Ruộng không đòi phải có đường về làng: một lối mòn ra ruộng là thứ tốt khi
/// có, nhưng bắt buộc nó thì những thửa nằm sau lưng một dãy nhà sẽ bị loại,
/// và làng mất ruộng vì một lý do không ai nhìn thấy.
pub(crate) fn place(canvas: &mut Canvas, site: &Site, slot: (i64, i64)) -> Option<Building> {
    let shape = hash_xy(site.seed, salt::FIELD_SHAPE, slot.0, slot.1);
    let w = 8 + pick(shape, 3) as i64;
    let h = 5 + pick(shape >> 8, 2) as i64;
    let rect = Rect::centered(slot, w, h);
    if !canvas.rect_free(rect, 1) {
        return None;
    }

    // Hướng luống đổi theo thửa. Cả làng cùng một hướng luống trông như hoa
    // văn in sẵn; đổi hướng làm mỗi thửa ra một mảnh đất có chủ riêng.
    let along_x = shape >> 16 & 1 == 0;
    canvas.claim_rect(rect, Claim::Solid);
    for p in rect.cells() {
        let furrow = if along_x { p.0 - rect.x } else { p.1 - rect.y };
        let material = if furrow % 2 == 0 {
            CROP_GREEN
        } else {
            FARMLAND
        };
        canvas.paint(p, material, PRIO_FIELD);
    }

    let door = gate(rect, site.hub);
    let outside = (
        door.0 + (door.0 - slot.0).signum(),
        door.1 + (door.1 - slot.1).signum(),
    );
    if canvas.free_path(outside) {
        road::carve(canvas, site, outside, Some(rect));
    }

    Some(Building {
        kind: BuildingKind::Field,
        origin: (rect.x, rect.y),
        w,
        h,
        door,
    })
}

/// Ô mép ruộng quay về phía làng — chỗ người làm đồng bước vào.
///
/// Chọn theo phía chứ không theo góc: một cái cổng ở giữa cạnh hướng về quảng
/// trường là thứ con đường bấu vào tự nhiên nhất.
fn gate(rect: Rect, hub: (i64, i64)) -> (i64, i64) {
    let cx = hub.0.clamp(rect.x, rect.x + rect.w - 1);
    let cy = hub.1.clamp(rect.y, rect.y + rect.h - 1);
    if hub.1 < rect.y {
        (cx, rect.y)
    } else if hub.1 >= rect.y + rect.h {
        (cx, rect.y + rect.h - 1)
    } else if hub.0 < rect.x {
        (rect.x, cy)
    } else {
        (rect.x + rect.w - 1, cy)
    }
}
