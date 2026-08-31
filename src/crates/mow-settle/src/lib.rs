//! # `mow-settle` — quy hoạch khu định cư khởi đầu
//!
//! Crate này biến "ba chấm trên bãi đất trống" thành một ngôi làng **đọc được
//! trong ba mươi giây**: quảng trường có giếng, mấy nóc nhà mái dốc quây quanh,
//! đường mòn nối từng ô cửa về giữa làng, ruộng kẻ luống ở vòng ngoài, và
//! khoảng chục con người có tên, có nhà, có chỗ làm.
//!
//! ## Nó không biết gì cả, và đó là điểm mạnh
//!
//! `plan()` nhận đúng hai thứ: một [`SettleRequest`] và một vị từ
//! `buildable(x, y)`. Nó không biết `Sim`, không biết sinh thế giới, không biết
//! HTTP. Hệ quả:
//!
//! * Test chạy trên một `|_, _| true` hoặc một hình dạng bờ biển bịa ra, không
//!   cần dựng thế giới thật — nên bộ test này chạy trong mili giây và không bao
//!   giờ hỏng vì ai đó sửa địa hình.
//! * Người gọi tự quyết định "ở được" nghĩa là gì: trên cạn, ngoài vùng cấm,
//!   chưa ai xây, đủ bằng phẳng — quy hoạch không cần biết.
//!
//! ## Xác định tuyệt đối
//!
//! Cùng `(seed, center, radius)` và cùng một vị từ thì cho ra cùng một [`Plan`],
//! byte-for-byte, trên mọi máy. Không có `rand`, không có `HashMap` để duyệt,
//! không có số thực. Mọi quyết định là một hàm băm thuần của hạt giống và tọa
//! độ (xem `hash`), nên thêm một quyết định mới ở giữa thuật toán không làm xê
//! dịch những quyết định đã có.
//!
//! ## Thoái lui chứ không hoảng
//!
//! Không đủ đất thì làng nhỏ lại: ít nhà hơn, ít ruộng hơn, và nếu đến cái
//! giếng cũng không có chỗ thì [`Plan`] rỗng. Không có nhánh nào panic, và
//! không có ô nào rơi xuống chỗ `buildable` nói không.
//!
//! ```
//! use mow_settle::{plan, SettleRequest};
//!
//! let req = SettleRequest { seed: 7, center: (0, 0), radius: 40 };
//! let village = plan(&req, &|_x, _y| true);
//! assert!(!village.cells.is_empty());
//! assert!(village.residents.iter().all(|r| r.home < village.buildings.len()));
//! ```

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::similar_names)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

mod canvas;
mod farm;
mod geom;
mod hash;
pub mod material;
mod people;
mod plaza;
mod road;
mod structure;

use serde::Serialize;

use crate::canvas::Canvas;
use crate::hash::{hash_i, pick, salt};

/// Yêu cầu quy hoạch một khu định cư.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SettleRequest {
    /// Hạt giống. Đổi hạt giống là đổi làng; giữ nguyên là giữ nguyên từng ô.
    pub seed: u64,
    /// Tâm mong muốn. Giếng sẽ nằm ở ô gần đây nhất mà xây được, nên tâm thật
    /// của làng có thể lệch đi vài ô khi tâm mong muốn rơi xuống nước.
    pub center: (i64, i64),
    /// Nửa cạnh vùng quy hoạch, theo khoảng cách ô vuông (Chebyshev).
    ///
    /// Dưới `12` thì chỉ đủ chỗ cho quảng trường và vài nóc nhà; `24` trở lên
    /// mới ra một ngôi làng đủ nhà, đủ kho, đủ ruộng.
    pub radius: i64,
}

/// Bản kế hoạch: đủ để người gọi ghi thẳng vào thế giới.
///
/// `cells` sắp xếp theo `(x, y)` tăng dần và không có ô nào lặp lại — nó sinh ra
/// từ một cây tra, nên "một ô một vật liệu" là tính chất cấu trúc chứ không
/// phải một quy ước phải nhớ.
///
/// Khi kế hoạch không rỗng, `buildings[0]` luôn là cái giếng: nó là công trình
/// được đặt đầu tiên và mọi thứ khác quây quanh nó.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Plan {
    /// Ô cần ghi: `(x, y, id vật liệu)`.
    pub cells: Vec<(i64, i64, &'static str)>,
    /// Người sẽ sống trong đó.
    pub residents: Vec<Resident>,
    /// Công trình đã dựng, kể cả giếng và ruộng.
    pub buildings: Vec<Building>,
}

/// Loại công trình.
///
/// Giếng và ruộng nằm chung một danh sách với nhà là có chủ ý: `workplace` của
/// cư dân là chỉ số vào `buildings`, mà người làm đồng thì làm ở ruộng còn cụ
/// già thì ngồi ở giếng. Tách ruộng ra một danh sách riêng sẽ buộc `workplace`
/// phải mang thêm một cái nhãn cho biết nó trỏ vào danh sách nào.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum BuildingKind {
    /// Nhà ở, 5x4, mái hai sắc, một ống khói.
    House,
    /// Xưởng, cùng khuôn với nhà nhưng hai ống khói.
    Workshop,
    /// Kho, rộng hơn và không ống khói.
    Granary,
    /// Thửa ruộng kẻ luống.
    Field,
    /// Giếng giữa quảng trường.
    Well,
}

/// Một công trình đã có chỗ đứng.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Building {
    /// Loại công trình.
    pub kind: BuildingKind,
    /// Góc trên trái của móng.
    pub origin: (i64, i64),
    /// Bề rộng theo ô.
    pub w: i64,
    /// Bề cao theo ô.
    pub h: i64,
    /// Ô mà người ta đứng lên để dùng công trình này.
    ///
    /// Với nhà, xưởng và kho, đó là ô cửa ở mép dưới và nó luôn là
    /// [`material::PATH_GRAVEL`]. Với ruộng, đó là ô mép quay về phía làng. Với
    /// giếng, đó là ô quảng trường ngay dưới thành giếng. Trong mọi trường hợp
    /// nó là một ô có mặt trong `cells` và đứng lên được.
    pub door: (i64, i64),
}

/// Vai trò của một cư dân.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Role {
    /// Làm đồng: chỗ làm là một thửa ruộng.
    Farmer,
    /// Thợ rèn: chỗ làm là xưởng.
    Smith,
    /// Đi săn: mang thú về kho.
    Hunter,
    /// Người già: ngồi ở giếng, chỗ tin tức của làng đi qua.
    Elder,
    /// Trẻ con: chỗ làm chính là nhà mình.
    Child,
    /// Coi kho.
    Keeper,
}

/// Một cư dân.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Resident {
    /// Tên, không trùng với bất kỳ ai trong cùng một làng.
    pub name: String,
    /// Vai trò.
    pub role: Role,
    /// Chỉ số nhà ở trong `Plan::buildings`.
    pub home: usize,
    /// Chỉ số chỗ làm trong `Plan::buildings`.
    pub workplace: usize,
    /// Ô đứng lúc làng vừa sinh ra; luôn là một ô có trong `Plan::cells`.
    pub start: (i64, i64),
}

/// Bối cảnh dùng chung của một lần quy hoạch.
///
/// Gom lại thành một chỗ vì gần như hàm nào cũng cần cả ba: hạt giống để băm,
/// tâm làng để hướng đường về, bán kính quảng trường để biết khi nào con đường
/// đã tới nơi.
pub(crate) struct Site {
    /// Tâm thật của làng — ô chính giữa giếng.
    pub(crate) hub: (i64, i64),
    /// Nửa cạnh quảng trường.
    pub(crate) plaza_half: i64,
    /// Hạt giống.
    pub(crate) seed: u64,
}

/// Bán kính lớn nhất được chấp nhận.
///
/// Không phải giới hạn thẩm mỹ mà là chặn chi phí: dò tâm làng quét cả vùng, và
/// vị từ `buildable` của người gọi có thể là một lần tra địa hình thật. Một
/// `radius` gõ nhầm thành một tỉ sẽ treo server chứ không báo lỗi.
const MAX_RADIUS: i64 = 512;

/// Khoảng cách tối thiểu từ tâm làng tới biên kiểu `i64`.
///
/// Bên trong, tọa độ chỉ đi xa tâm chừng `radius` cộng vài chục ô lề. Giữ cả
/// vùng cách biên chừng này thì mọi phép cộng tọa độ trong crate là an toàn mà
/// không phải rắc `saturating_add` lên từng dòng — và một `center` sát biên là
/// lỗi của người gọi, đáng được trả lời bằng một kế hoạch rỗng chứ không phải
/// bằng một lần tràn số im lặng.
const EDGE_GUARD: i64 = MAX_RADIUS * 4;

/// Số nhà ít nhất của một làng đủ đất.
const MIN_HOUSES: u64 = 5;
/// Số nhà nhiều nhất.
const MAX_HOUSES: u64 = 7;
/// Số thửa ruộng ít nhất.
const MIN_FIELDS: u64 = 2;
/// Số thửa ruộng nhiều nhất.
const MAX_FIELDS: u64 = 3;

/// Quy hoạch một khu định cư.
///
/// `buildable(x, y)` trả `true` cho ô mà kế hoạch được phép động vào. Không ô
/// nào trong [`Plan::cells`] nằm ngoài tập ấy, kể cả khi điều đó có nghĩa là
/// làng chỉ còn hai nóc nhà — hoặc không còn gì cả.
///
/// Vị từ được hỏi nhiều lần cho cùng một ô, nhưng câu trả lời đầu tiên được nhớ
/// lại và dùng cho tới hết; nên nó không cần rẻ, chỉ cần thuần.
#[must_use]
pub fn plan(req: &SettleRequest, buildable: &impl Fn(i64, i64) -> bool) -> Plan {
    let radius = req.radius.clamp(0, MAX_RADIUS);
    if req.center.0.saturating_abs() > i64::MAX - EDGE_GUARD
        || req.center.1.saturating_abs() > i64::MAX - EDGE_GUARD
    {
        return Plan::default();
    }
    let probe: &dyn Fn(i64, i64) -> bool = buildable;
    let mut canvas = Canvas::new(probe, req.center, radius);

    // Không có chỗ cho cái giếng thì không có làng. Trả kế hoạch rỗng chứ không
    // hạ một ngôi nhà xuống biển để "có cái gì đó".
    let Some(hub) = plaza::find_hub(&canvas, req.center, radius) else {
        return Plan::default();
    };
    let site = Site {
        hub,
        plaza_half: plaza::plaza_half(radius),
        seed: req.seed,
    };

    let mut buildings = vec![plaza::carve(&mut canvas, &site)];
    let slots = geom::slots(hub, site.plaza_half, radius, req.seed);
    raise_buildings(&mut canvas, &site, &slots, &mut buildings);
    open_fields(&mut canvas, &site, &slots, &mut buildings);

    let residents = people::populate(&canvas, &site, &buildings);
    Plan {
        cells: canvas.into_cells(),
        residents,
        buildings,
    }
}

/// Thứ tự dựng công trình.
///
/// Xưởng và kho chen vào ngay sau hai nóc nhà đầu chứ không xếp cuối: chỗ đặt
/// cạn dần từ trong ra ngoài, và một ngôi làng thà thiếu nóc nhà thứ bảy còn
/// hơn thiếu hẳn cái kho.
fn build_order(seed: u64) -> Vec<BuildingKind> {
    let houses = MIN_HOUSES
        + pick(
            hash_i(seed, salt::HOUSE_COUNT, 0),
            MAX_HOUSES - MIN_HOUSES + 1,
        );
    let mut order = vec![
        BuildingKind::House,
        BuildingKind::House,
        BuildingKind::Workshop,
        BuildingKind::Granary,
    ];
    order.extend(std::iter::repeat_n(
        BuildingKind::House,
        (houses - 2) as usize,
    ));
    order
}

/// Dựng nhà, xưởng, kho vào các chỗ đặt, từ trong ra ngoài.
///
/// Chỗ đặt nào từ chối thì thử chỗ kế tiếp *cho cùng loại đó*, chứ không nhảy
/// sang loại sau — nếu không, một dải bờ biển ăn mất mấy chỗ liền nhau sẽ đẩy
/// cái kho ra tận rìa làng.
fn raise_buildings(
    canvas: &mut Canvas,
    site: &Site,
    slots: &[(i64, i64)],
    buildings: &mut Vec<Building>,
) {
    let order = build_order(site.seed);
    let mut next = 0;
    for &slot in slots {
        let Some(&kind) = order.get(next) else { break };
        if let Some(b) = structure::place(canvas, site, kind, slot) {
            buildings.push(b);
            next += 1;
        }
    }
}

/// Mở ruộng ở những chỗ đặt còn lại, từ ngoài vào trong.
///
/// Duyệt ngược vì ruộng phải nằm ngoài rìa: đi xuôi thì thửa đầu tiên chiếm mất
/// khoảnh đất ngay cạnh quảng trường, và làng biến thành một cái nông trại có
/// vài cái nhà đứng nhờ.
fn open_fields(
    canvas: &mut Canvas,
    site: &Site,
    slots: &[(i64, i64)],
    buildings: &mut Vec<Building>,
) {
    let wanted = MIN_FIELDS
        + pick(
            hash_i(site.seed, salt::FIELD_COUNT, 0),
            MAX_FIELDS - MIN_FIELDS + 1,
        );
    let mut made = 0;
    for &slot in slots.iter().rev() {
        if made >= wanted {
            break;
        }
        if let Some(b) = farm::place(canvas, site, slot) {
            buildings.push(b);
            made += 1;
        }
    }
}
