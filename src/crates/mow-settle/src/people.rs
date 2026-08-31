//! Cư dân: tên, vai trò, nhà ở, chỗ làm, chỗ đứng lúc làng vừa sinh ra.
//!
//! Một ngôi làng trống là một mô hình kiến trúc. Thứ biến nó thành nơi có người
//! ở là mỗi cư dân có **hai địa chỉ** — nhà và chỗ làm — nằm ở hai chỗ khác
//! nhau trên bản đồ. Chỉ cần thế là mọi lịch sinh hoạt sau này đều có sẵn một
//! đoạn đường để đi, và cả làng bắt đầu chuyển động.

use std::collections::BTreeSet;

use crate::canvas::Canvas;
use crate::hash::{hash_i, pick, salt};
use crate::material::stand_on;
use crate::{Building, BuildingKind, Resident, Role, Site};

/// Bảng tên cố định.
///
/// Cố định trong mã chứ không sinh theo âm tiết: một bộ sinh tên sẽ đẻ ra
/// "Grxth" ở lần thứ ba mươi, và một cái tên đọc không được làm hỏng ảo giác
/// nhanh hơn mọi lỗi hình học. Bốn mươi tám cái tên đủ cho mọi ngôi làng khởi
/// đầu mà vẫn chọn bằng băm được.
const NAMES: [&str; 48] = [
    "Aldric", "Bryn", "Corin", "Dara", "Edda", "Fenn", "Gilda", "Haro", "Ilse", "Joran", "Kestrel",
    "Lund", "Mira", "Nell", "Orin", "Peta", "Quill", "Rowan", "Sable", "Tove", "Ulric", "Vesna",
    "Wren", "Yarrow", "Alva", "Bodil", "Cade", "Dorn", "Elsi", "Fara", "Gorm", "Hilda", "Ivar",
    "Jonna", "Kalle", "Linnea", "Mads", "Nora", "Osk", "Pell", "Runa", "Sten", "Thora", "Ulf",
    "Vidar", "Wanda", "Ylva", "Zoran",
];

/// Năm vai trò đầu tiên của mọi ngôi làng, theo đúng thứ tự này.
///
/// Cố định chứ không rút thăm, vì một ngôi làng thiếu thợ rèn hay thiếu trẻ con
/// là một ngôi làng *hỏng* chứ không phải một biến thể thú vị. Chỉ phần dân cư
/// vượt quá bộ khung mới được rút theo băm.
const CORE_ROLES: [Role; 5] = [
    Role::Elder,
    Role::Smith,
    Role::Farmer,
    Role::Hunter,
    Role::Child,
];

/// Bể vai trò cho phần dân cư còn lại. Lặp `Farmer` là một cách đặt trọng số:
/// làng nào cũng chủ yếu là người làm đồng.
const EXTRA_ROLES: [Role; 7] = [
    Role::Farmer,
    Role::Child,
    Role::Farmer,
    Role::Hunter,
    Role::Keeper,
    Role::Farmer,
    Role::Child,
];

/// Số cư dân tối đa một nóc nhà chứa được.
///
/// Chặn này chỉ cắn khi làng bị teo vì hết đất; nó tồn tại để một ngôi làng có
/// đúng một cái nhà không sinh ra mười hai người cùng đứng trên một ô cửa.
const FOLK_PER_HOUSE: usize = 3;

/// Sinh cư dân cho một danh sách công trình đã dựng xong.
pub(crate) fn populate(canvas: &Canvas, site: &Site, buildings: &[Building]) -> Vec<Resident> {
    let houses = index_of(buildings, BuildingKind::House);
    if houses.is_empty() {
        return Vec::new();
    }
    let fields = index_of(buildings, BuildingKind::Field);
    let workshop = index_of(buildings, BuildingKind::Workshop).first().copied();
    let granary = index_of(buildings, BuildingKind::Granary).first().copied();
    let well = index_of(buildings, BuildingKind::Well).first().copied();

    let wanted = 8 + pick(hash_i(site.seed, salt::FOLK_COUNT, 0), 5) as usize;
    let count = wanted.min(houses.len() * FOLK_PER_HOUSE);
    let rotate = pick(hash_i(site.seed, salt::HOME, 0), houses.len() as u64) as usize;

    let mut taken_names = BTreeSet::new();
    let mut taken_cells = BTreeSet::new();
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let role = role_for(site.seed, i);
        let home = houses[(i + rotate) % houses.len()];
        let workplace = match role {
            // Người làm đồng chia đều ra các thửa; hết ruộng thì về kho, hết
            // kho thì ở nhà — bậc lùi nào cũng phải là một chỉ số có thật.
            Role::Farmer if !fields.is_empty() => fields[i % fields.len()],
            Role::Farmer => granary.unwrap_or(home),
            Role::Smith => workshop.unwrap_or(home),
            Role::Hunter | Role::Keeper => granary.or(workshop).unwrap_or(home),
            Role::Elder => well.unwrap_or(home),
            Role::Child => home,
        };
        out.push(Resident {
            name: name_for(site.seed, i, &mut taken_names),
            role,
            home,
            workplace,
            start: start_for(canvas, buildings, home, well, &mut taken_cells),
        });
    }
    out
}

/// Chỉ số của mọi công trình thuộc một loại, giữ nguyên thứ tự dựng.
fn index_of(buildings: &[Building], kind: BuildingKind) -> Vec<usize> {
    buildings
        .iter()
        .enumerate()
        .filter(|(_, b)| b.kind == kind)
        .map(|(i, _)| i)
        .collect()
}

/// Vai trò của cư dân thứ `i`.
fn role_for(seed: u64, i: usize) -> Role {
    if i < CORE_ROLES.len() {
        return CORE_ROLES[i];
    }
    let h = hash_i(seed, salt::ROLE, i as u64);
    EXTRA_ROLES[pick(h, EXTRA_ROLES.len() as u64) as usize]
}

/// Một cái tên chưa ai dùng.
///
/// Băm rồi dò tuyến tính: băm cho tên phụ thuộc hạt giống, còn dò tuyến tính
/// bảo đảm không trùng mà vẫn xác định. Rút lại cho tới khi khác nhau thì sẽ
/// không dừng khi bảng cạn, còn dò thì luôn dừng.
fn name_for(seed: u64, i: usize, taken: &mut BTreeSet<usize>) -> String {
    let start = pick(hash_i(seed, salt::NAME, i as u64), NAMES.len() as u64) as usize;
    for step in 0..NAMES.len() {
        let idx = (start + step) % NAMES.len();
        if taken.insert(idx) {
            return NAMES[idx].to_string();
        }
    }
    // Không tới được: `FOLK_PER_HOUSE` chặn dân số dưới xa 48.
    NAMES[start].to_string()
}

/// Chỗ đứng lúc làng vừa sinh ra.
///
/// Chọn trong đúng những ô đã tô và đứng được, nên chỗ đứng nào cũng thỏa
/// `buildable` mà không phải hỏi lại vị từ. Tránh trùng ô vì hai người chồng
/// lên nhau ở khung hình đầu tiên là thứ đập vào mắt ngay.
fn start_for(
    canvas: &Canvas,
    buildings: &[Building],
    home: usize,
    well: Option<usize>,
    taken: &mut BTreeSet<(i64, i64)>,
) -> (i64, i64) {
    let door = buildings[home].door;
    let front = (door.0, door.1 + 1);
    let mut options = vec![
        front,
        door,
        (front.0 - 1, front.1),
        (front.0 + 1, front.1),
        (front.0, front.1 + 1),
    ];
    if let Some(w) = well {
        options.push(buildings[w].door);
    }
    for p in options {
        if taken.contains(&p) {
            continue;
        }
        if canvas.material_at(p).is_some_and(stand_on) {
            taken.insert(p);
            return p;
        }
    }
    door
}
