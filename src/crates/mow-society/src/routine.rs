//! Bộ lập lịch sinh hoạt hàng ngày cho cư dân.
//!
//! ## Vì sao cần nó
//!
//! Một NPC đi lang thang ngẫu nhiên không phải là cư dân. Người xem nhìn vào
//! và thấy **nhiễu**: hôm nay nó ở đây, mai nó ở kia, không vì lý do gì. Không
//! có gì để đoán, nên cũng không có gì để **bất ngờ** khi bị phá vỡ.
//!
//! Một cư dân thì khác: sáng ra giếng, ngày ra đồng, chiều muộn ra quảng
//! trường, tối về nhà. Cái nhịp đó làm được hai việc mà ngẫu nhiên không làm
//! được:
//!
//! - Nó **tạo ra kỳ vọng**. Vì có kỳ vọng nên "hôm nay bác thợ rèn không ra
//!   giếng" mới là một dữ kiện, chứ không phải một lần tung xúc xắc.
//! - Nó **tạo ra giao lộ**. Cả làng đi qua cùng một cái giếng vào cùng một
//!   khung giờ, nên đồ thị tiếp xúc trong [`crate::household`] nổi lên từ hành
//!   vi thay vì được khai báo.
//!
//! ## Vì sao lịch không được cứng
//!
//! Nhưng một lịch cứng cũng hỏng, chỉ theo hướng ngược lại: nó cho ra những
//! con rối diễu hành. Một người đói lả vẫn cắm mặt cày ruộng vì "đang giờ làm"
//! thì không phải người. Cái làm cho nhịp sống có sức sống là **những lúc nó
//! bị phá** — và nó phải bị phá bởi thứ có lý do: nhu cầu của chính cơ thể.
//!
//! Nên module này là một lịch **có thể bị nhu cầu ghi đè**, và thứ tự ghi đè
//! được nói rõ ở [`decide`].
//!
//! ## Bảng pha trong ngày
//!
//! Mọi mốc là **phần trăm của một ngày** (`0..100`), không phải giờ đồng hồ:
//! một ngày dài bao nhiêu tick là chuyện của người gọi, module này chỉ biết tỉ
//! lệ. Đêm là khoảng bọc qua nửa đêm, nên nó viết dạng `bắt đầu → kết thúc`.
//!
//! | Vai      | Đêm      | Ăn sáng | Ra giếng | Làm việc | Quảng trường | Ăn tối | Nghỉ  |
//! |----------|----------|---------|----------|----------|--------------|--------|-------|
//! | `Farmer` | 92 → 25  | 25–29   | 29–33    | 33–70    | 70–80        | 80–88  | 88–92 |
//! | `Smith`  | 92 → 27  | 27–31   | 31–35    | 35–72    | 72–82        | 82–88  | 88–92 |
//! | `Hunter` | 96 → 20  | 20–23   | 23–26    | 26–80    | 80–88        | 88–94  | 94–96 |
//! | `Elder`  | 90 → 30  | 30–35   | 35–40    | 40–52    | 52–84        | 84–88  | 88–90 |
//! | `Child`  | 88 → 30  | 30–35   | 35–40    | 40–68\*  | 68–82        | 82–86  | 86–88 |
//!
//! \* `Child` **không làm việc**: khung đó là giờ chơi, xem [`Role::Child`].
//!
//! Ba nhịp khác nhau đọc thẳng ra được từ bảng:
//!
//! - `Hunter` dậy sớm nhất (20) và về nhà muộn nhất (88) — đi xa thì mất cả
//!   quãng đường, và con mồi không đợi theo giờ hành chính.
//! - `Elder` chỉ làm 12% ngày (40–52) nhưng ở quảng trường 32% (52–84) — sức
//!   đã hết, nhưng chuyện thì còn, và quảng trường là nơi chuyện được kể.
//! - `Farmer`/`Smith` là nhịp chuẩn, `Smith` lệch muộn hơn một chút vì lò rèn
//!   không cần ánh sáng ban ngày như thửa ruộng.
//!
//! ## Số nguyên
//!
//! Không có số thực ở đây (`§P10.2.1`). Mốc pha là phần trăm nguyên, ngưỡng
//! nhu cầu là [`i64`] nguyên. Cùng một [`Situation`] luôn cho cùng một
//! [`Intent`] trên mọi máy, nên bộ lập lịch này an toàn để chạy trong vòng lặp
//! tất định của mô phỏng.

use serde::{Deserialize, Serialize};

/// Vai trò của cư dân trong làng.
///
/// Vai quyết định **hai** thứ chứ không phải một: nơi làm việc, và **nhịp** —
/// giờ dậy, giờ về, tỉ lệ thời gian dành cho việc và cho người.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Nông dân — làm ngoài đồng, nhịp chuẩn của làng.
    Farmer,
    /// Thợ rèn — làm ở xưởng, dậy và nghỉ muộn hơn nông dân một chút.
    Smith,
    /// Thợ săn — đi xa, dậy sớm nhất và về nhà muộn nhất.
    Hunter,
    /// Người già — làm rất ít, phần lớn thời gian ở quảng trường.
    Elder,
    /// Trẻ con — **không bao giờ** làm việc; giờ "làm" của nó là giờ chơi, và
    /// khi không có ai bên cạnh thì nó đi tìm người lớn.
    Child,
}

/// Địa điểm trong làng mà một ý định có thể trỏ tới.
///
/// Cố tình **thô**: đây là danh mục nơi chốn theo *chức năng sinh hoạt*, không
/// phải tọa độ. Việc biến `Field` thành một điểm cụ thể trên bản đồ là việc
/// của lớp không gian, không phải của bộ lập lịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// Nhà — nơi duy nhất ăn và ngủ được.
    Home,
    /// Xưởng/nơi làm việc có mái che (thợ rèn, người già).
    Workplace,
    /// Giếng làng — việc vặt buổi sáng, và là giao lộ tiếp xúc dày nhất.
    Well,
    /// Quảng trường — nơi gặp gỡ buổi chiều muộn.
    Square,
    /// Ngoài làng: đồng ruộng và cả rừng săn. Gộp làm một vì với bộ lập lịch
    /// thì cả hai đều là "ra khỏi làng để kiếm ăn".
    Field,
}

/// Pha sinh hoạt trong ngày, sau khi đã quy chiếu theo vai.
///
/// Công khai vì lớp gọi thường cần hiển thị *vì sao* một cư dân đang làm việc
/// nó đang làm ("đang giờ ăn tối") — và vì test cần khẳng định về nhịp mà
/// không phải suy ngược từ [`Intent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Đêm — ngủ.
    Night,
    /// Sáng sớm, ăn ở nhà.
    Breakfast,
    /// Việc vặt buổi sáng ở giếng.
    Errand,
    /// Giờ làm việc theo vai (với `Child` là giờ chơi).
    Work,
    /// Chiều muộn ở quảng trường.
    Social,
    /// Về nhà, ăn tối.
    Dinner,
    /// Nghỉ ở nhà trước khi ngủ.
    Rest,
}

/// Ý định mà bộ lập lịch trả về.
///
/// Đây là **ý định**, không phải hành động: nó không biết đường đi, không biết
/// có đủ thức ăn không, không biết giường có trống không. Lớp mô phỏng nhận ý
/// định này rồi tự quyết định có thực hiện được hay không — và chính chỗ đó là
/// nơi thế giới được phép nói "không".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// Di chuyển tới một địa điểm.
    GoTo {
        /// Nơi cần tới.
        place: Place,
    },
    /// Ngủ (chỉ hợp lệ khi đang ở [`Place::Home`]).
    Sleep,
    /// Ăn (chỉ hợp lệ khi đang ở [`Place::Home`]).
    Eat,
    /// Làm việc tại nơi làm việc của vai.
    Work,
    /// Giao tiếp với người bên cạnh.
    Socialize {
        /// Người cụ thể nếu biết. `None` khi chỉ biết "có người quanh đây" mà
        /// chưa xác định được ai — lớp mô phỏng tự chọn.
        with: Option<u64>,
    },
    /// Không có việc gì đáng làm ngay lúc này.
    ///
    /// `Idle` **không** phải là thất bại của bộ lập lịch. Một cư dân đứng
    /// không ở quảng trường lúc chiều muộn vì chưa có ai tới là một cảnh đúng.
    Idle,
}

/// Trạng thái đầu vào của một cư dân tại một tick.
///
/// Cố tình **không** chứa `Sim`, tọa độ hay đồ thị đường đi: bộ lập lịch phải
/// kiểm chứng được bằng một struct dựng tay trong test, và phải dùng lại được
/// cho bất kỳ biểu diễn thế giới nào.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Situation {
    /// Tick tuyệt đối của thế giới.
    pub tick: u64,
    /// Số tick trong một ngày. `0` được xử lý an toàn, xem [`day_percent`].
    pub ticks_per_day: u64,
    /// Vai trò của cư dân.
    pub role: Role,
    /// Mức đói: `0` là no, càng cao càng đói.
    pub hunger: i64,
    /// Mức mệt: `0` là khỏe, càng cao càng kiệt sức.
    pub fatigue: i64,
    /// Đang đứng ở đâu.
    pub at: Place,
    /// Số người ở ngay bên cạnh.
    pub nearby: u32,
    /// Người gần nhất, nếu lớp gọi biết.
    ///
    /// Thêm ngoài API đề xuất ban đầu vì nếu không có nó thì
    /// [`Intent::Socialize::with`] vĩnh viễn là `None` — một trường chết. Để
    /// `None` là hợp lệ: khi đó ý định vẫn là "nói chuyện với ai đó quanh
    /// đây".
    pub nearest: Option<u64>,
}

/// Ngưỡng đói **cực đại**: thắng tất cả, kể cả kiệt sức.
///
/// Vì đói tới mức này thì ngủ không cứu được, mà ăn thì cứu được.
pub const HUNGER_STARVING: i64 = 85;

/// Ngưỡng **kiệt sức**: thắng mọi thứ trừ đói cực đại.
pub const FATIGUE_COLLAPSE: i64 = 80;

/// Ngưỡng đói **cấp bách**: thắng lịch, kể cả đang giữa giờ làm.
pub const HUNGER_URGENT: i64 = 55;

/// Ngưỡng **mệt vừa**: thắng những pha nhàn (giếng, quảng trường, nghỉ) nhưng
/// **không** thắng giờ làm.
///
/// Vì mệt vừa là lúc người ta bỏ buổi trà chiều để về ngủ sớm, chứ không phải
/// lúc người ta bỏ ruộng.
pub const FATIGUE_TIRED: i64 = 60;

/// Bảng mốc pha của một vai, tính theo phần trăm của ngày.
///
/// Bất biến: các mốc **tăng nghiêm ngặt** theo thứ tự khai báo, và tất cả nằm
/// trong `1..100`. Có test giữ bất biến này.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DayPlan {
    /// Mốc dậy — kết thúc đêm.
    pub wake: u64,
    /// Kết thúc bữa sáng ở nhà.
    pub breakfast_end: u64,
    /// Bắt đầu giờ làm — kết thúc việc vặt ở giếng.
    pub work_start: u64,
    /// Kết thúc giờ làm.
    pub work_end: u64,
    /// Kết thúc giờ quảng trường.
    pub social_end: u64,
    /// Kết thúc bữa tối.
    pub dinner_end: u64,
    /// Bắt đầu đêm.
    pub night_start: u64,
}

/// Bảng mốc pha theo vai. Xem bảng đầy đủ ở doc của module.
#[must_use]
pub const fn plan_of(role: Role) -> DayPlan {
    match role {
        Role::Farmer => DayPlan {
            wake: 25,
            breakfast_end: 29,
            work_start: 33,
            work_end: 70,
            social_end: 80,
            dinner_end: 88,
            night_start: 92,
        },
        Role::Smith => DayPlan {
            wake: 27,
            breakfast_end: 31,
            work_start: 35,
            work_end: 72,
            social_end: 82,
            dinner_end: 88,
            night_start: 92,
        },
        // Thợ săn: dậy sớm nhất, giờ làm dài nhất, về muộn nhất.
        Role::Hunter => DayPlan {
            wake: 20,
            breakfast_end: 23,
            work_start: 26,
            work_end: 80,
            social_end: 88,
            dinner_end: 94,
            night_start: 96,
        },
        // Người già: giờ làm ngắn nhất (12%), giờ quảng trường dài nhất (32%).
        Role::Elder => DayPlan {
            wake: 30,
            breakfast_end: 35,
            work_start: 40,
            work_end: 52,
            social_end: 84,
            dinner_end: 88,
            night_start: 90,
        },
        // Trẻ con: khung "làm" là khung chơi, và đi ngủ sớm nhất.
        Role::Child => DayPlan {
            wake: 30,
            breakfast_end: 35,
            work_start: 40,
            work_end: 68,
            social_end: 82,
            dinner_end: 86,
            night_start: 88,
        },
    }
}

/// Vị trí trong ngày tính theo phần trăm, luôn trong `0..100`.
///
/// `ticks_per_day == 0` trả về `0` thay vì chia cho không. `0%` rơi vào đêm
/// với mọi vai, nên một cấu hình hỏng cho ra cư dân đi ngủ — hành vi vô hại và
/// nhìn ra ngay, thay vì một cú panic giữa vòng lặp mô phỏng.
#[must_use]
pub fn day_percent(tick: u64, ticks_per_day: u64) -> u64 {
    if ticks_per_day == 0 {
        return 0;
    }
    let into_day = tick % ticks_per_day;
    // Nhân qua u128: `into_day * 100` tràn u64 khi ngày dài hơn u64::MAX/100.
    // Kết quả chắc chắn < 100 nên hạ về u64 không mất mát.
    (u128::from(into_day) * 100 / u128::from(ticks_per_day)) as u64
}

/// Pha sinh hoạt hiện tại của một cư dân.
#[must_use]
pub fn phase_of(s: &Situation) -> Phase {
    let now = day_percent(s.tick, s.ticks_per_day);
    let plan = plan_of(s.role);
    if now < plan.wake || now >= plan.night_start {
        Phase::Night
    } else if now < plan.breakfast_end {
        Phase::Breakfast
    } else if now < plan.work_start {
        Phase::Errand
    } else if now < plan.work_end {
        Phase::Work
    } else if now < plan.social_end {
        Phase::Social
    } else if now < plan.dinner_end {
        Phase::Dinner
    } else {
        Phase::Rest
    }
}

/// Nơi làm việc của một vai.
///
/// `Child` trỏ về quảng trường vì nó không có nơi làm việc — chỗ "làm" của nó
/// là chỗ có người lớn.
#[must_use]
pub const fn workplace_of(role: Role) -> Place {
    match role {
        Role::Farmer | Role::Hunter => Place::Field,
        Role::Smith | Role::Elder => Place::Workplace,
        Role::Child => Place::Square,
    }
}

/// Quyết định việc cần làm ngay bây giờ.
///
/// # Thứ tự ưu tiên
///
/// Đây là phần quan trọng nhất của module. Lịch **luôn ở đáy**:
///
/// 1. `hunger >= `[`HUNGER_STARVING`] — ăn. Thắng tất cả, kể cả kiệt sức.
/// 2. `fatigue >= `[`FATIGUE_COLLAPSE`] — ngủ. Thắng mọi thứ trừ mục 1.
/// 3. `hunger >= `[`HUNGER_URGENT`] — ăn. **Thắng giờ làm việc.**
/// 4. `fatigue >= `[`FATIGUE_TIRED`] và đang **không** trong pha
///    [`Phase::Work`] — về ngủ sớm.
/// 5. Lịch theo pha và theo vai.
///
/// Vì sao đói xếp trên kiệt sức ở mục 1: ngủ khi đói lả không giải quyết được
/// gì, còn ăn thì giải quyết được — cơ thể ưu tiên cái cứu được mình.
///
/// Ăn và ngủ chỉ xảy ra ở [`Place::Home`]; ở nơi khác thì ý định là
/// [`Intent::GoTo`] về nhà. Bộ lập lịch không giả vờ rằng cư dân ăn được giữa
/// đồng.
///
/// # Tính thuần
///
/// Hàm thuần và tất định: cùng [`Situation`] cho cùng [`Intent`], không RNG,
/// không số thực, không đọc trạng thái ngoài.
#[must_use]
pub fn decide(s: &Situation) -> Intent {
    // 1. Đói cực đại thắng tất cả.
    if s.hunger >= HUNGER_STARVING {
        return go_eat(s.at);
    }
    // 2. Kiệt sức thắng mọi thứ còn lại.
    if s.fatigue >= FATIGUE_COLLAPSE {
        return go_sleep(s.at);
    }
    // 3. Đói cấp bách thắng lịch — kể cả đang giữa giờ làm. Đây là ranh giới
    //    giữa một cư dân và một con rối.
    if s.hunger >= HUNGER_URGENT {
        return go_eat(s.at);
    }
    let phase = phase_of(s);
    // 4. Mệt vừa chỉ cắt được những pha nhàn, không cắt được công việc.
    if s.fatigue >= FATIGUE_TIRED && phase != Phase::Work {
        return go_sleep(s.at);
    }
    // 5. Không có nhu cầu nào gào lên: sống theo nhịp của làng.
    follow_schedule(s, phase)
}

/// Lịch thuần túy, khi không nhu cầu nào ghi đè.
fn follow_schedule(s: &Situation, phase: Phase) -> Intent {
    match phase {
        Phase::Night => go_sleep(s.at),
        // Bữa sáng và bữa tối cùng một hành vi: về nhà và ăn.
        Phase::Breakfast | Phase::Dinner => go_eat(s.at),
        Phase::Errand => do_errand(s),
        Phase::Work => do_work(s),
        Phase::Social => do_social(s),
        Phase::Rest => do_rest(s),
    }
}

/// Ăn: chỉ ăn được ở nhà.
const fn go_eat(at: Place) -> Intent {
    if matches!(at, Place::Home) {
        Intent::Eat
    } else {
        Intent::GoTo { place: Place::Home }
    }
}

/// Ngủ: chỉ ngủ được ở nhà.
const fn go_sleep(at: Place) -> Intent {
    if matches!(at, Place::Home) {
        Intent::Sleep
    } else {
        Intent::GoTo { place: Place::Home }
    }
}

/// Việc vặt buổi sáng ở giếng.
///
/// Cả làng dồn về một chỗ trong một khung hẹp — đó là chủ đích, vì đó là chỗ
/// đồ thị tiếp xúc được sinh ra.
fn do_errand(s: &Situation) -> Intent {
    if s.at == Place::Well {
        chat_or(s, Intent::Idle)
    } else {
        Intent::GoTo { place: Place::Well }
    }
}

/// Giờ làm việc theo vai.
fn do_work(s: &Situation) -> Intent {
    if s.role == Role::Child {
        return play(s);
    }
    let workplace = workplace_of(s.role);
    if s.at == workplace {
        Intent::Work
    } else {
        Intent::GoTo { place: workplace }
    }
}

/// Giờ chơi của trẻ con.
///
/// Không bao giờ trả [`Intent::Work`]. Có người bên cạnh thì chơi với người
/// đó; không có ai thì đi ra quảng trường tìm người lớn, vì trẻ con bám theo
/// người lớn chứ không tự chọn chỗ đứng.
fn play(s: &Situation) -> Intent {
    if s.nearby > 0 {
        return Intent::Socialize { with: s.nearest };
    }
    if s.at == Place::Square {
        Intent::Idle
    } else {
        Intent::GoTo {
            place: Place::Square,
        }
    }
}

/// Chiều muộn ở quảng trường.
fn do_social(s: &Situation) -> Intent {
    if s.at == Place::Square {
        chat_or(s, Intent::Idle)
    } else {
        Intent::GoTo {
            place: Place::Square,
        }
    }
}

/// Nghỉ ở nhà trước khi ngủ.
fn do_rest(s: &Situation) -> Intent {
    if s.at == Place::Home {
        chat_or(s, Intent::Idle)
    } else {
        Intent::GoTo { place: Place::Home }
    }
}

/// Có người bên cạnh thì trò chuyện, không thì làm việc mặc định.
fn chat_or(s: &Situation, fallback: Intent) -> Intent {
    if s.nearby > 0 {
        Intent::Socialize { with: s.nearest }
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::{
        day_percent, decide, phase_of, plan_of, workplace_of, DayPlan, Intent, Phase, Place, Role,
        Situation, FATIGUE_COLLAPSE, FATIGUE_TIRED, HUNGER_STARVING, HUNGER_URGENT,
    };
    use std::collections::BTreeSet;

    /// Độ dài ngày dùng chung: 1000 tick cho 1 tick = 0.1% ngày.
    const DAY: u64 = 1000;

    const ROLES: [Role; 5] = [
        Role::Farmer,
        Role::Smith,
        Role::Hunter,
        Role::Elder,
        Role::Child,
    ];

    const PLACES: [Place; 5] = [
        Place::Home,
        Place::Workplace,
        Place::Well,
        Place::Square,
        Place::Field,
    ];

    /// Một cư dân no và khỏe, để lịch tự nói.
    fn calm(role: Role, percent: u64, at: Place) -> Situation {
        Situation {
            tick: percent * DAY / 100,
            ticks_per_day: DAY,
            role,
            hunger: 5,
            fatigue: 5,
            at,
            nearby: 0,
            nearest: None,
        }
    }

    /// Nhãn loại ý định, bỏ qua tham số — để đếm *loại* việc chứ không đếm
    /// biến thể của cùng một việc.
    fn tag(i: Intent) -> &'static str {
        match i {
            Intent::GoTo { .. } => "goto",
            Intent::Sleep => "sleep",
            Intent::Eat => "eat",
            Intent::Work => "work",
            Intent::Socialize { .. } => "socialize",
            Intent::Idle => "idle",
        }
    }

    /// Mốc trong ngày mà vai này chuyển sang đi về nhà buổi tối.
    ///
    /// Quét từ giữa ngày để không nhặt nhầm lần `GoTo{Home}` của đêm hôm
    /// trước.
    fn home_bound_percent(role: Role) -> u64 {
        let midday = plan_of(role).work_start + 10;
        (midday..100)
            .find(|p| {
                decide(&calm(role, *p, workplace_of(role))) == (Intent::GoTo { place: Place::Home })
            })
            .expect("moi vai phai ve nha truoc khi het ngay")
    }

    #[test]
    fn night_puts_every_role_to_sleep() {
        for role in ROLES {
            let plan = plan_of(role);
            // Nửa đêm sâu: điểm giữa của đoạn đêm bọc qua mốc 0.
            for percent in [plan.night_start + 1, 0, plan.wake - 1] {
                let s = calm(role, percent % 100, Place::Home);
                assert_eq!(
                    phase_of(&s),
                    Phase::Night,
                    "{role:?} tai {percent}% phai la dem"
                );
                assert_eq!(decide(&s), Intent::Sleep, "{role:?} tai {percent}%");
            }
        }
    }

    /// Bài quan trọng nhất: đói vượt ngưỡng thắng giờ làm việc.
    #[test]
    fn hunger_beats_the_work_schedule() {
        for role in ROLES {
            let plan = plan_of(role);
            let midwork = u64::midpoint(plan.work_start, plan.work_end);

            // No thì làm việc (trẻ con thì chơi) — cột mốc đối chứng.
            let fed = calm(role, midwork, workplace_of(role));
            assert_eq!(phase_of(&fed), Phase::Work, "{role:?} phai dang gio lam");
            if role != Role::Child {
                assert_eq!(decide(&fed), Intent::Work, "{role:?} no thi phai lam viec");
            }

            // Đói thì bỏ việc mà đi ăn, dù đang đứng ngay nơi làm việc.
            let mut hungry = fed;
            hungry.hunger = HUNGER_URGENT;
            assert_ne!(
                decide(&hungry),
                Intent::Work,
                "{role:?} doi ma van lam viec"
            );
            assert_eq!(
                decide(&hungry),
                Intent::GoTo { place: Place::Home },
                "{role:?} doi thi phai ve nha an"
            );

            // Và nếu đã ở nhà thì ăn luôn.
            let mut hungry_home = hungry;
            hungry_home.at = Place::Home;
            assert_eq!(decide(&hungry_home), Intent::Eat, "{role:?} o nha thi an");
        }
    }

    #[test]
    fn hunger_wakes_a_sleeper_at_night() {
        let mut s = calm(Role::Farmer, 0, Place::Home);
        assert_eq!(decide(&s), Intent::Sleep);
        s.hunger = HUNGER_URGENT;
        assert_eq!(
            decide(&s),
            Intent::Eat,
            "doi cap bach phai thang ca gio ngu"
        );
    }

    /// Thứ tự ưu tiên: đói cực đại > kiệt sức > đói cấp bách > lịch.
    #[test]
    fn need_priority_order_holds() {
        let midwork = 50;
        let base = calm(Role::Farmer, midwork, Place::Home);
        assert_eq!(phase_of(&base), Phase::Work);

        // Kiệt sức thắng lịch làm việc.
        let mut spent = base;
        spent.fatigue = FATIGUE_COLLAPSE;
        assert_eq!(decide(&spent), Intent::Sleep);

        // Nhưng thua đói cực đại.
        let mut spent_and_starving = spent;
        spent_and_starving.hunger = HUNGER_STARVING;
        assert_eq!(decide(&spent_and_starving), Intent::Eat);

        // Kiệt sức thắng đói cấp bách (chưa cực đại).
        let mut spent_and_hungry = spent;
        spent_and_hungry.hunger = HUNGER_URGENT;
        assert_eq!(decide(&spent_and_hungry), Intent::Sleep);

        // Mệt vừa thì không cắt được giờ làm: vẫn cày ở ngoài đồng...
        let mut tired = base;
        tired.fatigue = FATIGUE_TIRED;
        tired.at = workplace_of(Role::Farmer);
        assert_eq!(decide(&tired), Intent::Work);

        // ...nhưng cắt được buổi quảng trường.
        let mut tired_evening = calm(Role::Farmer, 75, Place::Square);
        tired_evening.fatigue = FATIGUE_TIRED;
        assert_eq!(phase_of(&tired_evening), Phase::Social);
        assert_eq!(decide(&tired_evening), Intent::GoTo { place: Place::Home });
    }

    #[test]
    fn child_never_works() {
        for percent in 0..100 {
            for at in PLACES {
                for hunger in [0, HUNGER_URGENT, HUNGER_STARVING] {
                    for fatigue in [0, FATIGUE_TIRED, FATIGUE_COLLAPSE] {
                        for nearby in [0, 3] {
                            let mut s = calm(Role::Child, percent, at);
                            s.hunger = hunger;
                            s.fatigue = fatigue;
                            s.nearby = nearby;
                            assert_ne!(
                                decide(&s),
                                Intent::Work,
                                "tre con lam viec tai {percent}% o {at:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn child_seeks_adults_when_alone() {
        let alone = calm(Role::Child, 50, Place::Home);
        assert_eq!(phase_of(&alone), Phase::Work, "dang la gio choi");
        assert_eq!(
            decide(&alone),
            Intent::GoTo {
                place: Place::Square
            },
            "khong co ai thi di tim nguoi lon"
        );

        let mut with_adults = alone;
        with_adults.nearby = 2;
        with_adults.nearest = Some(7);
        assert_eq!(decide(&with_adults), Intent::Socialize { with: Some(7) });
    }

    #[test]
    fn hunter_comes_home_later_than_farmer() {
        let farmer = home_bound_percent(Role::Farmer);
        let hunter = home_bound_percent(Role::Hunter);
        assert!(
            hunter > farmer,
            "tho san ({hunter}%) phai ve muon hon nong dan ({farmer}%)"
        );
    }

    #[test]
    fn elder_spends_more_of_the_day_in_the_square_than_farmer() {
        let count = |role: Role| {
            (0..100)
                .filter(|p| phase_of(&calm(role, *p, Place::Square)) == Phase::Social)
                .count()
        };
        assert!(
            count(Role::Elder) > count(Role::Farmer),
            "nguoi gia phai o quang truong nhieu hon nong dan"
        );
        // Và làm ít hơn.
        let plan_elder = plan_of(Role::Elder);
        let plan_farmer = plan_of(Role::Farmer);
        assert!(
            plan_elder.work_end - plan_elder.work_start
                < plan_farmer.work_end - plan_farmer.work_start
        );
    }

    #[test]
    fn morning_errand_goes_to_the_well() {
        let plan = plan_of(Role::Farmer);
        let s = calm(Role::Farmer, plan.breakfast_end, Place::Home);
        assert_eq!(phase_of(&s), Phase::Errand);
        assert_eq!(decide(&s), Intent::GoTo { place: Place::Well });

        let mut at_well = s;
        at_well.at = Place::Well;
        at_well.nearby = 1;
        at_well.nearest = Some(42);
        assert_eq!(decide(&at_well), Intent::Socialize { with: Some(42) });
    }

    #[test]
    fn decide_is_pure() {
        let s = Situation {
            tick: 12_345,
            ticks_per_day: DAY,
            role: Role::Smith,
            hunger: 40,
            fatigue: 33,
            at: Place::Workplace,
            nearby: 2,
            nearest: Some(9),
        };
        let first = decide(&s);
        for _ in 0..1000 {
            assert_eq!(decide(&s), first);
        }
    }

    #[test]
    fn zero_ticks_per_day_does_not_divide_by_zero() {
        assert_eq!(day_percent(999, 0), 0);
        for role in ROLES {
            for at in PLACES {
                let s = Situation {
                    tick: u64::MAX,
                    ticks_per_day: 0,
                    role,
                    hunger: 0,
                    fatigue: 0,
                    at,
                    nearby: 0,
                    nearest: None,
                };
                // Không panic, và rơi về đêm — hành vi an toàn nhất.
                assert_eq!(phase_of(&s), Phase::Night);
                let want = if at == Place::Home {
                    Intent::Sleep
                } else {
                    Intent::GoTo { place: Place::Home }
                };
                assert_eq!(decide(&s), want);
            }
        }
    }

    #[test]
    fn day_percent_never_reaches_one_hundred() {
        for tpd in [1_u64, 7, DAY, u64::MAX] {
            for tick in [0_u64, 1, 999, u64::MAX / 2, u64::MAX] {
                assert!(day_percent(tick, tpd) < 100);
            }
        }
    }

    /// Một cư dân chỉ làm một việc cả ngày là một cư dân chết.
    #[test]
    fn a_full_farmer_day_produces_a_varied_life() {
        // Mô phỏng thô: ý định `GoTo` thì coi như tới nơi ở tick sau.
        let mut at = Place::Home;
        let mut kinds: BTreeSet<&'static str> = BTreeSet::new();
        for tick in 0..DAY {
            let s = Situation {
                tick,
                ticks_per_day: DAY,
                role: Role::Farmer,
                hunger: 10,
                fatigue: 10,
                at,
                nearby: 2,
                nearest: Some(1),
            };
            let intent = decide(&s);
            kinds.insert(tag(intent));
            if let Intent::GoTo { place } = intent {
                at = place;
            }
        }
        assert!(
            kinds.len() >= 4,
            "mot ngay cua nong dan chi co {} loai viec: {kinds:?}",
            kinds.len()
        );
        for want in ["sleep", "eat", "work", "goto"] {
            assert!(
                kinds.contains(want),
                "thieu {want} trong mot ngay: {kinds:?}"
            );
        }
    }

    #[test]
    fn every_role_lives_a_varied_day() {
        for role in ROLES {
            let mut at = Place::Home;
            let mut kinds: BTreeSet<&'static str> = BTreeSet::new();
            for tick in 0..DAY {
                let mut s = calm(role, 0, at);
                s.tick = tick;
                s.nearby = 1;
                let intent = decide(&s);
                kinds.insert(tag(intent));
                if let Intent::GoTo { place } = intent {
                    at = place;
                }
            }
            assert!(kinds.len() >= 4, "{role:?} chi co {kinds:?} trong ca ngay");
        }
    }

    #[test]
    fn day_plans_are_monotonic() {
        for role in ROLES {
            let DayPlan {
                wake,
                breakfast_end,
                work_start,
                work_end,
                social_end,
                dinner_end,
                night_start,
            } = plan_of(role);
            let marks = [
                wake,
                breakfast_end,
                work_start,
                work_end,
                social_end,
                dinner_end,
                night_start,
            ];
            for pair in marks.windows(2) {
                assert!(pair[0] < pair[1], "{role:?}: moc pha khong tang: {marks:?}");
            }
            assert!(
                wake >= 1 && night_start < 100,
                "{role:?}: moc ra ngoai 1..100"
            );
        }
    }

    /// Áp lực nhu cầu phải **đơn điệu**: đã đói tới mức bỏ việc thì đói hơn
    /// nữa không được quay lại làm việc.
    ///
    /// Test này cũng giữ luôn thứ tự các ngưỡng: nếu ai đó đổi
    /// `HUNGER_URGENT` lên trên `HUNGER_STARVING` thì mốc dừng việc lệch và
    /// bài này đỏ.
    #[test]
    fn need_pressure_is_monotone_during_work() {
        let at_work = calm(Role::Farmer, 50, workplace_of(Role::Farmer));
        assert_eq!(phase_of(&at_work), Phase::Work);

        let mut hunger_stop = None;
        for hunger in 0..=120 {
            let mut s = at_work;
            s.hunger = hunger;
            let working = decide(&s) == Intent::Work;
            if !working && hunger_stop.is_none() {
                hunger_stop = Some(hunger);
            }
            assert!(
                working != hunger_stop.is_some(),
                "doi hon ma quay lai lam viec: {hunger}"
            );
        }
        assert_eq!(hunger_stop, Some(HUNGER_URGENT), "moc bo viec vi doi");

        let mut fatigue_stop = None;
        for fatigue in 0..=120 {
            let mut s = at_work;
            s.fatigue = fatigue;
            let working = decide(&s) == Intent::Work;
            if !working && fatigue_stop.is_none() {
                fatigue_stop = Some(fatigue);
            }
            assert!(
                working != fatigue_stop.is_some(),
                "met hon ma quay lai lam viec"
            );
        }
        assert_eq!(
            fatigue_stop,
            Some(FATIGUE_COLLAPSE),
            "chi kiet suc moi cat duoc gio lam, met vua thi khong"
        );
        assert!(
            fatigue_stop > Some(FATIGUE_TIRED),
            "met vua phai thap hon moc kiet suc"
        );
    }
}
