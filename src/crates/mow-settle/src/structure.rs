//! Nhà, xưởng, kho — mọi thứ có mái và có cửa.
//!
//! Bài toán thật của module này không phải là đặt được một hình chữ nhật, mà là
//! làm cho hình chữ nhật ấy **đọc ra là nhà khi nhìn từ trên xuống**. Từ trên
//! cao ta không thấy tường, không thấy cửa sổ, không thấy chiều cao; ta chỉ có
//! một mảng ô màu. Ba chi tiết dưới đây là toàn bộ ngân sách để nói "nhà":
//!
//! * **Mái hai sắc.** Một khối màu đều là nền lát. Một khối chia đôi sáng/tối
//!   theo chiều ngang là một mái *dốc* có sống mái ở giữa — bộ não đọc ra khối
//!   ba chiều từ đúng một đường ranh giới ấy.
//! * **Ống khói.** Một ô đá sẫm lẫn trong mái nói "trong này có lửa, có người".
//!   Kho không có ống khói, và đó không phải là tiết kiệm: kho thóc mà đốt lửa
//!   thì cháy làng.
//! * **Ô cửa ở mép dưới.** Nó vừa chỉ hướng nhà quay mặt, vừa là chỗ con đường
//!   bấu vào. Không có nó, ngôi nhà lơ lửng không liên quan gì tới làng.

use crate::canvas::{Canvas, Claim};
use crate::geom::Rect;
use crate::hash::{hash_xy, pick, salt};
use crate::material::{IGNEOUS, PATH_GRAVEL, PRIO_BUILT, ROOF_DARK, ROOF_LIGHT};
use crate::road;
use crate::{Building, BuildingKind, Site};

/// Kích thước móng theo loại công trình.
///
/// Nhà đúng 5x4 theo yêu cầu. Xưởng giữ nguyên khuôn ấy nhưng hai ống khói —
/// một lò rèn thì khói gấp đôi. Kho to hơn hẳn và không ống khói: ba loại phân
/// biệt được bằng hình dáng, không cần thêm một vật liệu nào.
fn footprint(kind: BuildingKind) -> Option<(i64, i64)> {
    match kind {
        BuildingKind::House | BuildingKind::Workshop => Some((5, 4)),
        BuildingKind::Granary => Some((7, 5)),
        BuildingKind::Field | BuildingKind::Well => None,
    }
}

/// Số ống khói.
fn chimneys(kind: BuildingKind) -> usize {
    match kind {
        BuildingKind::House => 1,
        BuildingKind::Workshop => 2,
        _ => 0,
    }
}

/// Thử dựng một công trình có mái với tâm rơi vào `slot`.
///
/// Trả `None` khi chỗ đó không nhận được — hết đất, chạm công trình khác, hoặc
/// **không kéo nổi một con đường về quảng trường**. Điều kiện cuối cùng mới là
/// điều kiện đắt nhất, và nó nằm ở đây có chủ ý: một ngôi nhà không nối vào làng
/// thì thà đừng dựng, vì cái người chơi nhìn thấy là mạng lưới đường chứ không
/// phải từng cái mái rời.
pub(crate) fn place(
    canvas: &mut Canvas,
    site: &Site,
    kind: BuildingKind,
    slot: (i64, i64),
) -> Option<Building> {
    let (w, h) = footprint(kind)?;
    let rect = Rect::centered(slot, w, h);
    if !canvas.rect_free(rect, 1) {
        return None;
    }

    // Cửa không bao giờ đặt ở ô góc: góc là nơi hai mép mái gặp nhau, một lỗ
    // thủng ở đó đọc ra là mái bị mẻ chứ không phải là cửa.
    let columns = w - 2;
    let first = pick(
        hash_xy(site.seed, salt::DOOR, rect.x, rect.y),
        columns as u64,
    ) as i64;
    for k in 0..columns {
        let col = 1 + (first + k) % columns;
        let door = (rect.x + col, rect.y + h - 1);
        let front = (door.0, door.1 + 1);
        if !canvas.free_path(front) {
            continue;
        }
        if road::carve(canvas, site, front, Some(rect)) {
            stamp(canvas, site, kind, rect, door);
            return Some(Building {
                kind,
                origin: (rect.x, rect.y),
                w,
                h,
                door,
            });
        }
    }
    None
}

/// Tô mái, ống khói và ô cửa.
fn stamp(canvas: &mut Canvas, site: &Site, kind: BuildingKind, rect: Rect, door: (i64, i64)) {
    canvas.claim_rect(rect, Claim::Solid);

    // Nửa đón nắng lấy phần lẻ khi số hàng là số lẻ: nguồn sáng nằm phía trên,
    // nên sườn sáng phải là sườn rộng hơn. Đổi chiều là mái lộn ngược.
    let lit = (rect.h + 1) / 2;
    for p in rect.cells() {
        let row = p.1 - rect.y;
        let material = if row < lit { ROOF_LIGHT } else { ROOF_DARK };
        canvas.paint(p, material, PRIO_BUILT);
    }

    // Ống khói nằm ở hàng thứ hai, tức trên sườn sáng và không dính mép —
    // dính mép thì nó đọc ra là một góc mái bị vỡ.
    let row = rect.y + 1;
    match chimneys(kind) {
        1 => {
            let span = rect.w - 2;
            let col = 1 + pick(
                hash_xy(site.seed, salt::CHIMNEY, rect.x, rect.y),
                span as u64,
            ) as i64;
            canvas.paint((rect.x + col, row), IGNEOUS, PRIO_BUILT);
        }
        // Hai ống khói của xưởng đặt cố định ở hai đầu sống mái. Rải chúng theo
        // băm thì có lúc chúng dính nhau thành một vệt hai ô, và một vệt hai ô
        // không đọc ra là hai ống khói.
        2 => {
            canvas.paint((rect.x + 1, row), IGNEOUS, PRIO_BUILT);
            canvas.paint((rect.x + rect.w - 2, row), IGNEOUS, PRIO_BUILT);
        }
        _ => {}
    }

    canvas.paint(door, PATH_GRAVEL, PRIO_BUILT);
}
