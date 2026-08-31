//! Bốn vòng kinh tế của một ngôi làng — thứ biến khu định cư từ mô hình trưng
//! bày thành một hệ **có thể hỏng**.
//!
//! ## Vì sao module này tồn tại
//!
//! Một khu định cư "trưng bày" là khu định cư mà bạn có thể xóa một nửa dân số
//! và không có gì xảy ra: các ngôi nhà vẫn đứng, khói vẫn bốc, thanh tài nguyên
//! vẫn nhích lên. Nó *trông* như một nền kinh tế nhưng không có mắt xích nào
//! thật, nên không có gì để người chơi phá và không có gì để người chơi cứu.
//!
//! Ở đây mỗi công trình phải trả lời được ba câu, không trả lời được thì không
//! đưa vào mô hình:
//!
//! 1. **Ai đến đây?**
//! 2. **Mang gì vào, lấy gì ra?**
//! 3. **Ngừng 48 giờ thì cái gì hỏng?**
//!
//! | Công trình | Ai đến | Vào → Ra | Ngừng 48h thì hỏng gì |
//! |---|---|---|---|
//! | Giếng | Farmer, Child | công → nước | dân khát, bếp tắt, ruộng khô, vụ mùa tụt |
//! | Ruộng | Farmer | nước + giống → lương thực | kho vơi 20 khẩu phần/ngày, không gì bù |
//! | Rừng | Hunter | công → củi + đạm | bếp hết củi (nấu không được), lò rèn tắt |
//! | Lò rèn | Smith | củi → độ bền công cụ | công cụ tụt dưới ngưỡng, mọi năng suất **giảm 33%** |
//!
//! Lò rèn là mắt xích dễ làm giả nhất: ở phần lớn game, thợ rèn là một hoạt
//! cảnh — có animation, có tiếng búa, xóa đi thì không ai chết. Ở đây xóa thợ
//! rèn thì sau **hai ngày** [`Stock::tools`] tụt dưới số thợ, [`run_day`] nhận
//! `tools_ok == false`, và nước, củi, mùa màng đồng loạt mất một phần ba.
//!
//! ## Bốn vòng, và thứ cạn kiệt ở cuối mỗi vòng
//!
//! Mỗi vòng phải có một cái đáy. Một vòng không có đáy là một máy in tài nguyên,
//! và một máy in tài nguyên xóa mọi quyết định của người chơi sau ngày thứ ba.
//!
//! | Vòng | Sản xuất → tiêu thụ | Mức/ngày (làng 10 người) | Thứ cạn kiệt |
//! |---|---|---|---|
//! | Nước | Farmer + Child lấy → dân, bếp, ruộng | 60 (30 sinh hoạt, 10 nấu, 20 tưới) | công suất giếng, sức chứa chum |
//! | Lương thực | Farmer trồng → dân ăn | 22 (20 ăn, 2 giữ giống) | hạt giống, độ phì đất |
//! | Gỗ và thịt | Hunter lấy → bếp, sửa chữa | 5 bó củi (+1 cho lò), 4 phần đạm | thú và cây quanh làng |
//! | Bảo trì | Smith sửa → công trình, công cụ | 1 công cụ / 2 ngày | độ bền |
//!
//! ## Đây là một mô hình thuần
//!
//! Không biết gì về `Sim`, HTTP hay ECS. Nhận [`Stock`] và [`Workforce`], trả
//! [`DayReport`]. Không RNG, không đồng hồ, không I/O, không `HashMap` (thứ tự
//! duyệt của nó không xác định). Cùng đầu vào **luôn** cho cùng đầu ra, nên nó
//! chạy được trong một bản replay, trong một bản dự đoán "nếu tôi làm thế này
//! thì mười ngày nữa ra sao", và trong một test.
//!
//! ## Không có số thực (`§P10.2.1`)
//!
//! Mọi tỉ lệ ở đây là phân số nguyên: hình phạt thiếu công cụ là `× 2 / 3`, hao
//! hụt kho là `stock / 50`, độ phì là thang nguyên `0..=1000`. Lý do không phải
//! là sự khắt khe: một `f32` cho kết quả khác nhau giữa hai bản build, và một
//! nền kinh tế lệch một `ulp` ở ngày thứ nhất là một nền kinh tế lệch một mùa
//! gặt ở ngày thứ chín mươi.
//!
//! ## Kho không bao giờ âm
//!
//! Mọi khoản trừ đi qua [`take`], hàm chỉ lấy được phần đang có và **báo lại
//! phần thiếu**. Một kho âm là một lỗi im lặng: nó không panic, không log, nó
//! chỉ lặng lẽ biến "làng đã chết đói" thành "làng đang nợ 40 khẩu phần" và lan
//! con số đó ra khắp mô hình cho tới khi một biểu đồ ở đâu đó trông vô lý.

use serde::{Deserialize, Serialize};

// ───────────────────────── Vòng nước ─────────────────────────

/// Nước sinh hoạt mỗi đầu người mỗi ngày: uống, rửa, giặt.
pub const WATER_DOMESTIC_PER_HEAD: i64 = 3;
/// Nước cho bếp mỗi đầu người mỗi ngày.
pub const WATER_COOKING_PER_HEAD: i64 = 1;
/// Nước tưới mỗi Farmer mỗi ngày — một Farmer trông được chừng này thửa.
pub const WATER_IRRIGATION_PER_FARMER: i64 = 5;
/// Một Farmer gánh được chừng này nước một ngày, ngoài việc đồng áng.
pub const WATER_PER_FARMER: i64 = 12;
/// Một Child gánh được chừng này — ít hơn, nhưng **không phải không có**.
///
/// Đây là chỗ trẻ con thôi làm vật trang trí. Một đứa trẻ gánh 6 và uống 4, nên
/// nó là **lãi ròng 2 đơn vị/ngày**. Bỏ hai đứa khỏi làng 10 người là mất 12 đơn
/// vị cung và chỉ bớt 8 đơn vị cầu: làng chuyển từ hòa vốn sang âm 4/ngày, và
/// hết đúng 15 ngày đệm thì bắt đầu khát.
pub const WATER_PER_CHILD: i64 = 6;
/// Công suất giếng: mạch nước hồi chừng này một ngày, thêm người gánh cũng vô ích.
///
/// Sức gánh của làng chuẩn (4 Farmer + 2 Child) là 60, đúng bằng nhu cầu 60 —
/// **không có một giọt dư nào**. Đó là chủ ý: một làng hòa vốn tuyệt đối là một
/// làng mà mọi cú sốc đều đọc được ngay, chứ không bị một cái đệm nuốt mất.
///
/// Trần giếng 72 nằm cao hơn một chút, nên nó chưa cắn ở làng 10 người nhưng
/// cắn rất rõ từ khoảng 12 người trở lên: **giếng là trần dân số**. Người thứ
/// mười ba không mang về thêm giọt nào, dù có thêm bao nhiêu người gánh — muốn
/// lớn hơn thì phải đào giếng thứ hai, và đó là một quyết định chứ không phải
/// một thanh tiến trình.
pub const WELL_YIELD_CAP: i64 = 72;
/// Sức chứa chum vại của làng — hai ngày dùng của làng 10 người.
///
/// Nước không tích trữ vô hạn được: chum có đáy, và nước để lâu thành nước tù.
pub const WATER_STORAGE_CAP: i64 = 120;

// ───────────────────── Vòng lương thực ─────────────────────

/// Khẩu phần mỗi đầu người mỗi ngày. Làng 10 người ăn 20.
pub const FOOD_PER_HEAD: i64 = 2;
/// Một Farmer thu được chừng này khẩu phần/ngày trên đất còn nguyên độ phì.
pub const FOOD_PER_FARMER: i64 = 6;
/// Lượng giống gieo xuống mỗi ngày có canh tác. Ruộng có kích thước cố định, nên
/// con số này **không** nhân theo số Farmer — thêm người là chăm kỹ hơn, không
/// phải gieo dày hơn.
pub const SEED_SOWN_PER_DAY: i64 = 2;
/// Lượng giống giữ lại từ vụ gặt mỗi ngày, khi làng còn Elder.
pub const SEED_KEPT_PER_DAY: i64 = 2;
/// Lượng giống giữ lại khi làng **không** còn Elder — mất một nửa.
///
/// Giữ giống là nghề: phơi đúng nắng, ủ đúng độ ẩm, biết bông nào để lại. Không
/// có người già thì kho giống hụt dần cho tới ngày không còn gì để gieo.
pub const SEED_KEPT_WITHOUT_ELDER: i64 = 1;
/// Sức chứa kho giống. Giống để quá lâu mất sức nảy mầm, nên không tích vô hạn.
pub const SEED_STORE_CAP: i64 = 40;
/// Còn dưới chừng này ngày gieo thì báo [`Shortage::SeedGrain`].
pub const SEED_WARN_DAYS: i64 = 3;
/// Kho lương thực còn dưới chừng này ngày ăn thì báo [`Shortage::Food`].
///
/// Cảnh báo phải đến **trước** lúc chạm 0, nếu không nó chỉ là một cáo phó.
pub const FOOD_WARN_DAYS: i64 = 3;
/// Mỗi ngày `kho / 50` khẩu phần hỏng: mốc, mọt, chuột.
///
/// Đây là thứ giữ cho kho không phình vô hạn — một làng dư ăn sẽ ổn định quanh
/// mức mà hao hụt bằng thặng dư, chứ không leo tới mười nghìn khẩu phần.
pub const FOOD_SPOIL_DIVISOR: i64 = 50;
/// Hao hụt khi không có Elder trông kho: hỏng gấp đôi.
pub const FOOD_SPOIL_DIVISOR_WITHOUT_ELDER: i64 = 25;
/// Không nấu được thì ăn sống, và ăn sống thì tốn thêm `1 / 4`.
///
/// Đây là dây nối bếp với kho: củi hết không chỉ khó chịu, nó làm lương thực
/// cạn nhanh hơn 25%.
pub const RAW_FOOD_PENALTY_DIVISOR: i64 = 4;

// ────────────────── Vòng gỗ và thịt ──────────────────

/// Củi một Hunter mang về mỗi ngày.
pub const WOOD_PER_HUNTER: i64 = 3;
/// Phần đạm (thịt, cá) một Hunter mang về mỗi ngày — cộng thẳng vào lương thực.
pub const MEAT_PER_HUNTER: i64 = 2;
/// Trần củi mỗi ngày: cây quanh làng chỉ mọc lại chừng này.
///
/// Hunter thứ tư không mang thêm bó nào — vòng gỗ có đáy, và đáy của nó là khu
/// rừng chứ không phải số người.
pub const FORAGE_WOOD_CAP: i64 = 9;
/// Trần đạm mỗi ngày: thú quanh làng chỉ đẻ lại chừng này.
pub const FORAGE_MEAT_CAP: i64 = 6;
/// Bếp đốt một bó cho mỗi hai đầu người (làm tròn lên). Làng 10 người: 5 bó.
pub const HEADS_PER_COOKING_WOOD: u64 = 2;
/// Củi lò rèn mỗi Smith mỗi ngày. Không có củi thì không có lửa, không có lửa
/// thì không sửa được gì — đây là chỗ Hunter nối vào Smith.
pub const WOOD_FORGE_PER_SMITH: i64 = 1;
/// Còn dưới chừng này ngày đốt thì báo [`Shortage::Wood`].
pub const WOOD_WARN_DAYS: i64 = 2;

// ─────────────────── Vòng bảo trì ───────────────────

/// Một công cụ lành đáng chừng này điểm bền.
///
/// [`Stock::tools`] đếm **điểm bền**, không đếm cái. Lý do là `§P10.2.1`: đề bài
/// là "1 công cụ / 2 ngày", và không có cách nào viết một nửa cái rìu bằng số
/// nguyên. Đổi đơn vị thì tỉ lệ đó thành `1 điểm / ngày` — cùng một tốc độ, và
/// không có `f32` nào lọt vào state.
pub const TOOL_POINTS_PER_TOOL: i64 = 2;
/// Một Smith phục hồi 1 điểm bền/ngày, tức **1 công cụ mỗi 2 ngày**.
pub const TOOL_POINTS_PER_SMITH: i64 = 1;
/// Bao nhiêu người làm thì mòn hết 1 điểm bền một ngày (làm tròn lên).
pub const WORKERS_PER_TOOL_WEAR: u64 = 7;
/// Tử số hình phạt thiếu công cụ.
pub const TOOL_PENALTY_NUM: i64 = 2;
/// Mẫu số hình phạt thiếu công cụ: `× 2 / 3`, tức **giảm 33%**.
///
/// Nằm giữa khoảng 25–40% đã khai. Đây là con số chứng minh Smith là mắt xích
/// thật: nó đủ lớn để một ngày mất mùa hiện ra trên biểu đồ, và đủ nhỏ để làng
/// không chết ngay trong tuần đầu thiếu thợ.
pub const TOOL_PENALTY_DEN: i64 = 3;

// ──────────────────── Độ phì đất ────────────────────

/// Trần độ phì. Thang nguyên `0..=1000`, không phải `0.0..=1.0` (`§P10.2.1`).
pub const FERTILITY_MAX: i64 = 1000;
/// Mỗi `6` khẩu phần gặt được rút `1` điểm độ phì. Đất trả tiền cho vụ mùa.
pub const FERTILITY_DRAIN_DIVISOR: i64 = 6;
/// Độ phì hồi mỗi ngày **có** canh tác: phân chuồng, tro bếp, xác rơm.
///
/// Với làng chuẩn (4 Farmer, gặt 24) thì rút `24 / 6 = 4` và hồi `4` — hòa đúng
/// bằng không. Đất nuôi được **bốn** Farmer vô thời hạn. Người thứ năm bắt đầu
/// ăn vào vốn, và đó là lúc mô hình bắt đầu có ý kiến về quyết định của người chơi.
///
/// Điểm cân bằng này là **tự chỉnh**: đất bạc thì gặt ít, gặt ít thì rút ít, nên
/// độ phì tụt về một mức rồi đứng lại chứ không lao về 0. Một làng thâm canh 6
/// Farmer sẽ dừng ở khoảng 667 — vẫn sống, nhưng vĩnh viễn nghèo hơn.
pub const FERTILITY_WORK_REGEN: i64 = 4;
/// Độ phì hồi mỗi ngày **bỏ hoang**: cỏ dại, đạm tự do, đất nghỉ.
pub const FERTILITY_FALLOW_REGEN: i64 = 12;
/// Dưới mức này thì báo [`Shortage::Fertility`]: đất đang bị khai thác lẹm vốn.
pub const FERTILITY_WARN: i64 = 600;

// ──────────────────── Kiểu dữ liệu ────────────────────

/// Kho của làng. Mọi trường là số nguyên và **không bao giờ âm**.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stock {
    /// Khẩu phần trong kho.
    pub food: i64,
    /// Nước trong chum, trần là [`WATER_STORAGE_CAP`].
    pub water: i64,
    /// Bó củi.
    pub wood: i64,
    /// Điểm bền công cụ. Xem [`TOOL_POINTS_PER_TOOL`] để biết vì sao là điểm.
    pub tools: i64,
    /// Hạt giống. **Không phải** lương thực dự phòng — xem [`run_day`].
    pub seed_grain: i64,
    /// Độ phì đất, thang `0..=`[`FERTILITY_MAX`].
    pub soil_fertility: i64,
}

impl Stock {
    /// Kho khởi điểm của làng chuẩn: **7 ngày** lương thực, **3 ngày** củi.
    ///
    /// Bảy ngày là khoảng đủ để một người chơi kịp nhận ra có chuyện và kịp làm
    /// một việc gì đó; ba ngày củi là khoảng vừa đủ ngắn để mất Hunter là một sự
    /// kiện chứ không phải một dòng log.
    ///
    /// `tools` khởi điểm là `7` điểm — ba cái lành và một cái đang sửa dở — bằng
    /// đúng số người làm, nên làng bắt đầu ở đúng mép của [`tools_sufficient`].
    pub fn starting_village() -> Self {
        Self {
            food: 140,
            water: 60,
            wood: 18,
            tools: 7,
            seed_grain: 20,
            soil_fertility: FERTILITY_MAX,
        }
    }

    /// Kẹp mọi trường về miền hợp lệ.
    ///
    /// Gọi ở **cửa** của [`run_day`] chứ không phải ở lối ra: nếu một giá trị bẩn
    /// lọt vào từ bên ngoài (một bản save cũ, một endpoint gõ tay), ta muốn nó
    /// chết ở đây chứ không muốn nó nhân với độ phì rồi lan thành sáu con số sai.
    pub fn clamped(&self) -> Self {
        Self {
            food: self.food.max(0),
            water: self.water.max(0),
            wood: self.wood.max(0),
            tools: self.tools.max(0),
            seed_grain: self.seed_grain.max(0),
            soil_fertility: self.soil_fertility.clamp(0, FERTILITY_MAX),
        }
    }

    /// Có trường nào âm không. Dùng cho assert; đúng ra là không bao giờ.
    pub fn has_deficit(&self) -> bool {
        self.food < 0
            || self.water < 0
            || self.wood < 0
            || self.tools < 0
            || self.seed_grain < 0
            || self.soil_fertility < 0
    }
}

/// Số người theo vai.
///
/// Không có `population` tổng: tổng là hàm của các vai, và một trường tổng lưu
/// riêng là một trường sẽ lệch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workforce {
    /// Người trồng trọt, và là người gánh nước chính.
    pub farmers: u32,
    /// Người săn bắt và kiếm củi.
    pub hunters: u32,
    /// Thợ rèn, người giữ [`Stock::tools`] trên ngưỡng.
    pub smiths: u32,
    /// Người già: giữ giống và trông kho. Không ra đồng, nhưng không phải vô dụng.
    pub elders: u32,
    /// Trẻ con: gánh nước. Ăn như người lớn, làm được một việc thật.
    pub children: u32,
}

impl Workforce {
    /// Làng chuẩn 10 người: 4 Farmer, 2 Hunter, 1 Smith, 1 Elder, 2 Child.
    pub fn starting_village() -> Self {
        Self {
            farmers: 4,
            hunters: 2,
            smiths: 1,
            elders: 1,
            children: 2,
        }
    }

    /// Tổng số miệng ăn. Mọi vai đều ăn, kể cả vai không sản xuất gì.
    pub fn population(&self) -> i64 {
        i64::from(self.farmers)
            + i64::from(self.hunters)
            + i64::from(self.smiths)
            + i64::from(self.elders)
            + i64::from(self.children)
    }

    /// Số người làm việc nặng: Farmer, Hunter, Smith. Đây là số làm mòn công cụ.
    pub fn workers(&self) -> i64 {
        i64::from(self.farmers) + i64::from(self.hunters) + i64::from(self.smiths)
    }
}

/// Một thứ đang thiếu. Thiếu hụt là **tín hiệu**, không phải sự kiện chết người:
/// nó xuất hiện lúc còn kịp làm gì đó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Shortage {
    /// Kho dưới [`FOOD_WARN_DAYS`] ngày ăn, hoặc hôm nay có người không được ăn đủ.
    Food,
    /// Hôm nay có phần nước không được đáp ứng: dân, bếp, hoặc ruộng.
    Water,
    /// Kho dưới [`WOOD_WARN_DAYS`] ngày đốt, hoặc bếp/lò rèn không đủ củi.
    Wood,
    /// Điểm bền dưới số người làm — mọi năng suất sắp mất một phần ba.
    Tools,
    /// Không đủ giống để gieo, hoặc kho giống dưới [`SEED_WARN_DAYS`] ngày gieo.
    SeedGrain,
    /// Độ phì dưới [`FERTILITY_WARN`]: đang canh tác lẹm vào vốn của đất.
    Fertility,
}

/// Kết quả một ngày.
///
/// `stock` là kho **sau** ngày; `produced` và `consumed` là hai vế của cùng một
/// phương trình. Bất biến mà mô hình này luôn giữ:
///
/// ```text
/// stock == clamped(đầu vào) + produced - consumed   (từng trường một)
/// ```
///
/// Có bất biến đó nghĩa là không có tài nguyên nào xuất hiện hay biến mất mà
/// không đi qua một dòng có tên. Không có nó thì "kho giảm" và "kho bị ăn" là
/// hai chuyện không phân biệt được, và mọi bảng cân đối đều là phỏng đoán.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayReport {
    /// Kho sau ngày.
    pub stock: Stock,
    /// Những thứ đang thiếu, thứ tự cố định và không trùng lặp.
    pub shortages: Vec<Shortage>,
    /// Vào kho hôm nay.
    pub produced: Stock,
    /// Ra khỏi kho hôm nay (ăn, đốt, gieo, mòn, hỏng, đất bạc).
    pub consumed: Stock,
}

// ───────────────────── Hàm phụ trợ ─────────────────────

/// Lấy `want` từ `pool`, trả về **phần thực sự lấy được**, và không bao giờ để
/// `pool` âm.
///
/// Đây là hàm duy nhất được phép trừ vào kho trong module này. Lý do là một quy
/// tắc bằng cấu trúc thay vì bằng kỷ luật: nếu chỗ nào cũng được `-=` thì sớm
/// muộn có một chỗ quên kiểm tra, và kho âm là một lỗi **im lặng** — không panic,
/// không log, chỉ lan ra khắp mô hình.
pub fn take(pool: &mut i64, want: i64) -> i64 {
    let given = want.clamp(0, (*pool).max(0));
    *pool -= given;
    given
}

/// Áp hình phạt thiếu công cụ: `× 2 / 3` khi `tools_ok == false`.
fn with_tools(amount: i64, tools_ok: bool) -> i64 {
    if tools_ok {
        amount
    } else {
        amount * TOOL_PENALTY_NUM / TOOL_PENALTY_DEN
    }
}

/// Chia làm tròn lên trên số nguyên không dấu.
///
/// Đi vòng qua `u64` vì `i64::div_ceil` còn unstable ở Rust 1.90, và vì mọi chỗ
/// dùng nó (đầu người, người làm) đều không âm theo định nghĩa.
fn ceil_div(value: i64, divisor: u64) -> i64 {
    (value.max(0) as u64).div_ceil(divisor) as i64
}

/// Công cụ còn đủ cho số người đang làm không.
///
/// Ngưỡng là **1 điểm bền / người làm**, tức một công cụ (2 điểm) dùng chung cho
/// hai người. Người gọi dùng hàm này để tính tham số `tools_ok` của [`run_day`];
/// [`run_day`] nhận nó như tham số chứ không tự tính, để người gọi còn ép được
/// tình huống "công cụ hỏng hết" mà không phải giả mạo kho.
pub fn tools_sufficient(stock: &Stock, work: &Workforce) -> bool {
    stock.tools >= work.workers()
}

// ────────────────────── Bốn vòng ──────────────────────

/// Sổ ghi một ngày: hai vế của phương trình cân đối.
#[derive(Default)]
struct DayLedger {
    produced: Stock,
    consumed: Stock,
}

/// Trạng thái nước sau khi phân phối, để các vòng sau dùng.
struct WaterOutcome {
    /// Ruộng xin bao nhiêu.
    irrigation_want: i64,
    /// Ruộng nhận được bao nhiêu.
    irrigation_got: i64,
    /// Bếp có đủ nước không.
    kitchen_water_ok: bool,
    /// Có phần nào không được đáp ứng không.
    short: bool,
}

/// Vòng 1 — nước: giếng và chum.
fn cycle_water(
    next: &mut Stock,
    led: &mut DayLedger,
    work: &Workforce,
    tools_ok: bool,
) -> WaterOutcome {
    let heads = work.population();
    let farmers = i64::from(work.farmers);

    // Gánh nước: Farmer và Child. Thiếu thùng, thiếu đòn gánh thì gánh được ít hơn.
    let haul = with_tools(
        farmers * WATER_PER_FARMER + i64::from(work.children) * WATER_PER_CHILD,
        tools_ok,
    );
    // Hai cái trần, và cả hai đều thật: mạch giếng, và đáy chum.
    let room = (WATER_STORAGE_CAP - next.water).max(0);
    let drawn = haul.min(WELL_YIELD_CAP).min(room);
    next.water += drawn;
    led.produced.water = drawn;

    // Thứ tự ưu tiên là một quyết định thiết kế, không phải chi tiết cài đặt:
    // người uống trước, bếp thứ hai, ruộng cuối. Nên khi giếng yếu, thứ hỏng đầu
    // tiên là **vụ mùa** — một hậu quả chậm, tới sau vài ngày, khi đã quá muộn.
    let domestic_want = heads * WATER_DOMESTIC_PER_HEAD;
    let domestic_got = take(&mut next.water, domestic_want);
    let cooking_want = heads * WATER_COOKING_PER_HEAD;
    let cooking_got = take(&mut next.water, cooking_want);
    let irrigation_want = farmers * WATER_IRRIGATION_PER_FARMER;
    let irrigation_got = take(&mut next.water, irrigation_want);
    led.consumed.water = domestic_got + cooking_got + irrigation_got;

    WaterOutcome {
        irrigation_want,
        irrigation_got,
        kitchen_water_ok: cooking_got >= cooking_want,
        short: domestic_got < domestic_want
            || cooking_got < cooking_want
            || irrigation_got < irrigation_want,
    }
}

/// Trạng thái gỗ và bếp sau vòng 3.
struct ForestOutcome {
    /// Phần đạm mang về, cộng thẳng vào lương thực.
    meat: i64,
    /// Bếp có đủ củi không.
    kitchen_wood_ok: bool,
    /// Lò rèn có đủ củi để nhóm lửa không.
    forge_lit: bool,
    /// Có phần củi nào không được đáp ứng, hoặc kho sắp cạn.
    short: bool,
}

/// Vòng 3 — gỗ và thịt: rừng, bếp, lò rèn.
fn cycle_forest(
    next: &mut Stock,
    led: &mut DayLedger,
    work: &Workforce,
    tools_ok: bool,
) -> ForestOutcome {
    let heads = work.population();
    let hunters = i64::from(work.hunters);
    let smiths = i64::from(work.smiths);

    // Trần rừng áp **trước** hình phạt công cụ: khu rừng chỉ có bấy nhiêu, và
    // rìu cùn thì lấy được ít hơn cả bấy nhiêu.
    let wood = with_tools((hunters * WOOD_PER_HUNTER).min(FORAGE_WOOD_CAP), tools_ok);
    let meat = with_tools((hunters * MEAT_PER_HUNTER).min(FORAGE_MEAT_CAP), tools_ok);
    next.wood += wood;
    led.produced.wood = wood;

    let cooking_want = ceil_div(heads, HEADS_PER_COOKING_WOOD);
    let cooking_got = take(&mut next.wood, cooking_want);
    let forge_want = smiths * WOOD_FORGE_PER_SMITH;
    let forge_got = take(&mut next.wood, forge_want);
    led.consumed.wood = cooking_got + forge_got;

    ForestOutcome {
        meat,
        kitchen_wood_ok: cooking_got >= cooking_want,
        forge_lit: smiths > 0 && forge_got >= forge_want,
        short: cooking_got < cooking_want
            || forge_got < forge_want
            || next.wood < cooking_want * WOOD_WARN_DAYS,
    }
}

/// Trạng thái ruộng sau vòng 2.
struct FieldOutcome {
    /// Gặt được bao nhiêu, sau độ phì, sau nước, sau công cụ.
    harvest: i64,
    /// Có Farmer mà không gieo được vì hết giống.
    seed_short: bool,
}

/// Vòng 2 — lương thực: ruộng, giống, độ phì.
fn cycle_field(
    next: &mut Stock,
    led: &mut DayLedger,
    work: &Workforce,
    tools_ok: bool,
    water: &WaterOutcome,
    fertility_at_dawn: i64,
) -> FieldOutcome {
    let farmers = i64::from(work.farmers);
    if farmers == 0 {
        // Không có ai ra đồng: không gieo, không gặt, không rút giống. Ruộng bỏ
        // hoang là một trạng thái hợp lệ, không phải một lỗi.
        return FieldOutcome {
            harvest: 0,
            seed_short: false,
        };
    }

    let sown = take(&mut next.seed_grain, SEED_SOWN_PER_DAY);
    led.consumed.seed_grain = sown;
    if sown < SEED_SOWN_PER_DAY {
        // Không đủ giống thì ruộng nằm không, dù có bao nhiêu người và bao nhiêu
        // nước. Đây là cái đáy của vòng lương thực.
        return FieldOutcome {
            harvest: 0,
            seed_short: true,
        };
    }

    let mut harvest = farmers * FOOD_PER_FARMER;
    // Ba hệ số nhân, mỗi cái là một vòng khác đang nói chuyện với vòng này.
    harvest = harvest * fertility_at_dawn / FERTILITY_MAX;
    harvest = with_tools(harvest, tools_ok);
    if water.irrigation_want > 0 {
        harvest = harvest * water.irrigation_got / water.irrigation_want;
    }

    FieldOutcome {
        harvest,
        seed_short: false,
    }
}

/// Vòng 4 — bảo trì: lò rèn giữ công cụ trên ngưỡng.
fn cycle_upkeep(next: &mut Stock, led: &mut DayLedger, work: &Workforce, forge_lit: bool) {
    let repaired = if forge_lit {
        i64::from(work.smiths) * TOOL_POINTS_PER_SMITH
    } else {
        0
    };
    next.tools += repaired;
    led.produced.tools = repaired;

    let wear_want = ceil_div(work.workers(), WORKERS_PER_TOOL_WEAR);
    led.consumed.tools = take(&mut next.tools, wear_want);
}

/// Đất trả tiền cho vụ mùa, và đất nghỉ thì hồi lại.
fn settle_fertility(
    next: &mut Stock,
    led: &mut DayLedger,
    work: &Workforce,
    harvest: i64,
    fertility_at_dawn: i64,
) {
    let drained = (harvest / FERTILITY_DRAIN_DIVISOR).min(fertility_at_dawn);
    let after_drain = fertility_at_dawn - drained;
    let regen_want = if work.farmers > 0 {
        FERTILITY_WORK_REGEN
    } else {
        FERTILITY_FALLOW_REGEN
    };
    // Hồi tối đa tới trần, không hơn — nhờ vậy bất biến cân đối vẫn đúng mà không
    // cần một bước "kẹp" nào phá vỡ nó.
    let regen = regen_want.min(FERTILITY_MAX - after_drain);
    next.soil_fertility = after_drain + regen;
    led.produced.soil_fertility = regen;
    led.consumed.soil_fertility = drained;
}

// ──────────────────────── Một ngày ────────────────────────

/// Chạy **một ngày** của làng.
///
/// Thuần và xác định: không RNG, không đồng hồ, không I/O. Cùng `stock`, `work`,
/// `tools_ok` thì luôn cùng [`DayReport`].
///
/// `tools_ok` là tham số chứ không phải thứ hàm tự suy ra từ `stock`, để người
/// gọi ép được tình huống "công cụ hỏng hết" mà không phải giả mạo kho. Muốn
/// nối nhiều ngày một cách trung thực thì dùng [`run_days`], hoặc tự tính bằng
/// [`tools_sufficient`].
///
/// # Thứ tự trong ngày
///
/// Nước → rừng và bếp → ruộng → giữ giống → ăn → hao hụt → công cụ → độ phì.
/// Thứ tự này không tùy tiện: nước phải có trước để bếp nấu và ruộng tưới, bếp
/// phải xong trước bữa ăn để biết ăn chín hay ăn sống, và độ phì chốt cuối cùng
/// vì nó là hóa đơn của vụ vừa gặt.
///
/// # Vì sao ăn hết lương thực **không** ăn vào giống
///
/// Đây là ràng buộc riêng, không phải một trường hợp của "kho không âm".
///
/// [`Stock::food`] và [`Stock::seed_grain`] là hai kho tách rời, và bữa ăn chỉ
/// chạm vào kho thứ nhất. Khi `food` về 0, người đói **không** được đụng vào
/// `seed_grain` — dù trong đời thật họ sẽ đụng, và dù về mặt dinh dưỡng hai thứ
/// là một.
///
/// Lý do là hai loại thất bại này khác nhau về chất, và gộp lại thì loại thứ hai
/// biến mất khỏi mô hình:
///
/// - Ăn hết kho là một cú đói **một mùa**. Vụ sau vẫn gieo được, làng vẫn gượng dậy.
/// - Ăn hết giống là mất **cả vụ sau**. Không có gì để gieo nghĩa là không có gì
///   để gặt, và cú đói thành vĩnh viễn.
///
/// Nếu để bữa ăn tự động rút sang `seed_grain` thì làng luôn "sống thêm được vài
/// ngày" và ta không bao giờ quan sát được điểm không quay lại. Bằng cách làm mô
/// hình **không thể diễn đạt** việc ăn giống, ta biến nó thành một quyết định
/// nằm ở lớp trên — nơi người chơi (hoặc một luật ở `mow-law`) phải chủ động
/// chuyển giống thành lương thực và chịu trách nhiệm cho việc đó.
pub fn run_day(stock: &Stock, work: &Workforce, tools_ok: bool) -> DayReport {
    let mut next = stock.clamped();
    let fertility_at_dawn = next.soil_fertility;
    let mut led = DayLedger::default();

    let water = cycle_water(&mut next, &mut led, work, tools_ok);
    let forest = cycle_forest(&mut next, &mut led, work, tools_ok);
    let field = cycle_field(
        &mut next,
        &mut led,
        work,
        tools_ok,
        &water,
        fertility_at_dawn,
    );

    // Giữ giống: cắt ra **trước** khi phần còn lại vào kho ăn. Người già biết
    // chọn bông nào để lại; không có người già thì giữ được một nửa.
    let keep_want = if field.harvest > 0 {
        if work.elders > 0 {
            SEED_KEPT_PER_DAY
        } else {
            SEED_KEPT_WITHOUT_ELDER
        }
    } else {
        0
    };
    let seed_room = (SEED_STORE_CAP - next.seed_grain).max(0);
    let kept = keep_want.min(field.harvest).min(seed_room);
    next.seed_grain += kept;
    led.produced.seed_grain = kept;

    let to_granary = field.harvest - kept + forest.meat;
    next.food += to_granary;
    led.produced.food = to_granary;

    // Bữa ăn. Nấu được thì đủ; không nấu được thì ăn sống và tốn thêm 1/4.
    let kitchen_ok = forest.kitchen_wood_ok && water.kitchen_water_ok;
    let base_eat = work.population() * FOOD_PER_HEAD;
    let eat_want = if kitchen_ok {
        base_eat
    } else {
        base_eat + base_eat / RAW_FOOD_PENALTY_DIVISOR
    };
    let ate = take(&mut next.food, eat_want);

    // Hao hụt kho: mốc, mọt, chuột. Không có Elder trông kho thì hỏng gấp đôi.
    let spoil_divisor = if work.elders > 0 {
        FOOD_SPOIL_DIVISOR
    } else {
        FOOD_SPOIL_DIVISOR_WITHOUT_ELDER
    };
    let spoil_want = next.food / spoil_divisor;
    let spoiled = take(&mut next.food, spoil_want);
    led.consumed.food = ate + spoiled;

    cycle_upkeep(&mut next, &mut led, work, forest.forge_lit);
    settle_fertility(&mut next, &mut led, work, field.harvest, fertility_at_dawn);

    // Thứ tự cố định, mỗi loại nhiều nhất một lần — để hai lần chạy giống nhau
    // cho hai `Vec` bằng nhau, kể cả khi so sánh bằng `assert_eq!`.
    let mut shortages = Vec::new();
    if ate < eat_want || (eat_want > 0 && next.food < eat_want * FOOD_WARN_DAYS) {
        shortages.push(Shortage::Food);
    }
    if water.short {
        shortages.push(Shortage::Water);
    }
    if forest.short {
        shortages.push(Shortage::Wood);
    }
    if !tools_ok || next.tools < work.workers() {
        shortages.push(Shortage::Tools);
    }
    if field.seed_short
        || (work.farmers > 0 && next.seed_grain < SEED_SOWN_PER_DAY * SEED_WARN_DAYS)
    {
        shortages.push(Shortage::SeedGrain);
    }
    if next.soil_fertility < FERTILITY_WARN {
        shortages.push(Shortage::Fertility);
    }

    DayReport {
        stock: next,
        shortages,
        produced: led.produced,
        consumed: led.consumed,
    }
}

/// Chạy `days` ngày liên tiếp, tự tính `tools_ok` mỗi ngày bằng [`tools_sufficient`].
///
/// Đây là cách trung thực để mô phỏng nhiều ngày: nó để vòng bảo trì phản hồi
/// ngược vào ba vòng kia, nên mất Smith là một chuỗi hậu quả chứ không phải một
/// con số bị chỉnh tay.
pub fn run_days(stock: &Stock, work: &Workforce, days: u32) -> Vec<DayReport> {
    let mut current = stock.clamped();
    let mut out = Vec::with_capacity(days as usize);
    for _ in 0..days {
        let report = run_day(&current, work, tools_sufficient(&current, work));
        current = report.stock;
        out.push(report);
    }
    out
}

// ──────────────────────────── Test ────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        run_day, run_days, tools_sufficient, DayReport, Shortage, Stock, Workforce,
        FERTILITY_FALLOW_REGEN, FERTILITY_MAX, FOOD_PER_HEAD, SEED_STORE_CAP, WATER_STORAGE_CAP,
    };

    /// Kho cũ + sản xuất − tiêu thụ phải bằng đúng kho mới, từng trường một.
    fn assert_balanced(before: &Stock, report: &DayReport) {
        let p = &report.produced;
        let c = &report.consumed;
        let s = &report.stock;
        assert_eq!(s.food, before.food + p.food - c.food, "cân đối lương thực");
        assert_eq!(s.water, before.water + p.water - c.water, "cân đối nước");
        assert_eq!(s.wood, before.wood + p.wood - c.wood, "cân đối củi");
        assert_eq!(s.tools, before.tools + p.tools - c.tools, "cân đối công cụ");
        assert_eq!(
            s.seed_grain,
            before.seed_grain + p.seed_grain - c.seed_grain,
            "cân đối giống"
        );
        assert_eq!(
            s.soil_fertility,
            before.soil_fertility + p.soil_fertility - c.soil_fertility,
            "cân đối độ phì"
        );
    }

    /// Ngày đầu của làng chuẩn phải ra **đúng** những con số đã khai trong bảng
    /// thiết kế. Test này là chỗ bảng thiết kế và mã nguồn được buộc vào nhau;
    /// không có nó thì bảng ở đầu file trở thành lời quảng cáo.
    #[test]
    fn ngay_dau_khop_dung_bang_thiet_ke() {
        let stock = Stock::starting_village();
        let work = Workforce::starting_village();
        assert_eq!(work.population(), 10, "làng chuẩn 10 người");
        assert!(tools_sufficient(&stock, &work));

        let r = run_day(&stock, &work, true);

        // Nước: 60 = 30 sinh hoạt + 10 nấu + 20 tưới.
        assert_eq!(r.consumed.water, 60);
        // Lương thực: 22 rời ruộng, trong đó 2 giữ giống.
        assert_eq!(r.produced.seed_grain, 2, "2 giữ giống");
        assert_eq!(r.consumed.food, 20 + 2, "20 ăn + 2 hỏng (140+.. /50)");
        // Củi: 5 bó cho bếp + 1 cho lò rèn. Đạm: 4 phần.
        assert_eq!(r.consumed.wood, 5 + 1);
        assert_eq!(r.produced.wood, 6);
        // 24 gặt − 2 giống + 4 đạm = 26 vào kho ăn.
        assert_eq!(r.produced.food, 26);
        // Công cụ: 1 điểm/ngày/Smith = 1 công cụ mỗi 2 ngày.
        assert_eq!(r.produced.tools, 1);
        assert_eq!(r.consumed.tools, 1);
        // Đất hòa vốn ở làng chuẩn: rút 4, hồi 4.
        assert_eq!(r.consumed.soil_fertility, 4);
        assert_eq!(r.produced.soil_fertility, 4);
        assert_eq!(r.stock.soil_fertility, FERTILITY_MAX);

        assert!(r.shortages.is_empty(), "làng chuẩn không thiếu gì: {r:?}");
        assert_balanced(&stock, &r);
    }

    /// Một làng đủ người, đủ kho, chạy 30 ngày **không** đói.
    #[test]
    fn lang_du_nguoi_chay_30_ngay_khong_doi() {
        let work = Workforce::starting_village();
        let days = run_days(&Stock::starting_village(), &work, 30);

        let mut previous = Stock::starting_village();
        for (i, r) in days.iter().enumerate() {
            assert!(r.stock.food > 0, "ngày {} kho lương thực chạm 0", i + 1);
            assert!(
                !r.shortages.contains(&Shortage::Food),
                "ngày {} báo thiếu lương thực mà lẽ ra không nên: {:?}",
                i + 1,
                r.shortages
            );
            assert!(!r.stock.has_deficit(), "ngày {} có kho âm", i + 1);
            assert_balanced(&previous, r);
            previous = r.stock;
        }
        // Không những không đói: làng còn dư ra, và phần dư bị hao hụt chặn lại
        // chứ không leo vô hạn.
        let last = days[29].stock;
        assert!(last.food > 140, "làng chuẩn phải tích được của ăn của để");
        assert!(last.food < 400, "hao hụt phải chặn kho lại: {}", last.food);
        assert_eq!(last.soil_fertility, FERTILITY_MAX, "đất không bị bạc");
        assert_eq!(last.seed_grain, 20, "giữ giống hòa vốn");
    }

    /// Bỏ hết Farmer ⇒ lương thực cạn trong khoảng ngày dự đoán được, và
    /// [`Shortage::Food`] xuất hiện **trước** khi chạm 0.
    ///
    /// Con số chốt: làng chuẩn mất hết Farmer thì **báo đói ngày 9** và **cạn kho
    /// ngày 13**. Kho 7 ngày ăn chỉ chống được 13 ngày dù đã bớt 4 miệng, vì
    /// nạn đói không đến một mình — nước gánh về tụt còn 12/ngày (chỉ còn trẻ
    /// con gánh), bếp thiếu nước, và từ đó cả làng ăn sống, tốn thêm một phần tư.
    ///
    /// Hai con số này được ghim chặt chứ không để khoảng rộng: nếu một hằng số
    /// nào đó đổi và ngày đói trượt đi, ta muốn biết **ngay**, chứ không muốn
    /// một bài test vẫn xanh trong khi cán cân của cả mô hình đã khác.
    #[test]
    fn mat_het_farmer_thi_doi_va_bao_truoc_khi_cham_khong() {
        let mut work = Workforce::starting_village();
        work.farmers = 0;
        let days = run_days(&Stock::starting_village(), &work, 60);

        let empty_at = days
            .iter()
            .position(|r| r.stock.food == 0)
            .expect("mất hết Farmer thì kho phải cạn trong 60 ngày");
        let warned_at = days
            .iter()
            .position(|r| r.shortages.contains(&Shortage::Food))
            .expect("phải có cảnh báo thiếu lương thực");

        assert_eq!(
            empty_at + 1,
            13,
            "ngày cạn kho đã trượt khỏi con số đã chốt"
        );
        assert_eq!(
            warned_at + 1,
            9,
            "ngày báo đói đã trượt khỏi con số đã chốt"
        );
        assert!(
            warned_at < empty_at,
            "cảnh báo (ngày {}) phải đến trước lúc cạn (ngày {}) — một cảnh báo \
             đến đúng lúc chạm 0 chỉ là một cáo phó",
            warned_at + 1,
            empty_at + 1
        );
        assert!(
            empty_at - warned_at >= 2,
            "cảnh báo phải đến sớm đủ để còn kịp làm gì đó, chỉ được {} ngày",
            empty_at - warned_at
        );
    }

    /// Bỏ Smith 2 ngày ⇒ đến ngày thứ ba `tools_ok` tắt và sản lượng tụt.
    #[test]
    fn bo_smith_hai_ngay_thi_cong_cu_tut_duoi_nguong() {
        let mut work = Workforce::starting_village();
        work.smiths = 0;
        let stock = Stock::starting_village();

        // Ngày 1 và 2 vẫn còn đủ điểm bền; ngày 3 thì không.
        let days = run_days(&stock, &work, 3);
        assert!(tools_sufficient(&stock, &work), "ngày 1 còn đủ");
        assert!(tools_sufficient(&days[0].stock, &work), "ngày 2 còn đủ");
        assert!(
            !tools_sufficient(&days[1].stock, &work),
            "sau đúng 2 ngày không có Smith thì công cụ phải tụt dưới ngưỡng"
        );
        assert!(days[2].shortages.contains(&Shortage::Tools));
    }

    /// Thiếu công cụ phải làm năng suất nước/gỗ/ruộng giảm **25–40%**.
    ///
    /// Đây là bằng chứng thợ rèn là mắt xích thật chứ không phải hoạt cảnh.
    #[test]
    fn thieu_cong_cu_lam_nang_suat_tut_trong_khoang_da_khai() {
        let work = Workforce::starting_village();

        // Nước và củi: đo trên kho nước **rỗng**, để thứ quyết định là sức gánh
        // chứ không phải đáy chum.
        let dry = Stock {
            water: 0,
            ..Stock::starting_village()
        };
        let good = run_day(&dry, &work, true);
        let bad = run_day(&dry, &work, false);

        // Ruộng: đo riêng, trên kho nước **đầy** và làng **không có Hunter**.
        // Hai điều kiện đó tách hình phạt công cụ ra khỏi hai thứ khác vốn cũng
        // kéo mùa màng xuống — thiếu nước tưới, và phần đạm trộn lẫn trong
        // `produced.food`. Không tách thì con số đo được là tích của ba hiệu ứng
        // (thực tế 769‰), và ta sẽ tưởng mình đã khai sai khoảng.
        let wet = Stock {
            water: WATER_STORAGE_CAP,
            ..Stock::starting_village()
        };
        let field_work = Workforce { hunters: 0, ..work };
        let field_good = run_day(&wet, &field_work, true);
        let field_bad = run_day(&wet, &field_work, false);

        for (name, ok, broken) in [
            ("nước", good.produced.water, bad.produced.water),
            ("củi", good.produced.wood, bad.produced.wood),
            ("ruộng", field_good.produced.food, field_bad.produced.food),
        ] {
            assert!(broken < ok, "{name}: thiếu công cụ mà không tụt gì");
            let drop_permille = (ok - broken) * 1000 / ok;
            assert!(
                (250..=400).contains(&drop_permille),
                "{name}: tụt {drop_permille}‰, ngoài khoảng đã khai 250–400‰ ({ok} → {broken})"
            );
        }
    }

    /// Kho không bao giờ âm, kể cả khi mọi vai bằng 0 và chạy 100 ngày.
    #[test]
    fn kho_khong_bao_gio_am_du_chay_100_ngay() {
        let cases = [
            (Stock::starting_village(), Workforce::default()),
            (Stock::default(), Workforce::starting_village()),
            (Stock::default(), Workforce::default()),
            (
                Stock::default(),
                Workforce {
                    farmers: 40,
                    hunters: 30,
                    smiths: 10,
                    elders: 10,
                    children: 40,
                },
            ),
            // Đầu vào bẩn: kho âm và độ phì vượt trần phải chết ở cửa.
            (
                Stock {
                    food: -50,
                    water: -1,
                    wood: -9,
                    tools: -3,
                    seed_grain: -7,
                    soil_fertility: 9_999,
                },
                Workforce::starting_village(),
            ),
        ];

        for (start, work) in cases {
            let mut current = start.clamped();
            for day in 1..=100 {
                let r = run_day(&current, &work, tools_sufficient(&current, &work));
                assert!(
                    !r.stock.has_deficit(),
                    "ngày {day} có kho âm: {:?} với {work:?}",
                    r.stock
                );
                assert!(r.stock.soil_fertility <= FERTILITY_MAX, "độ phì vượt trần");
                assert!(
                    r.stock.water <= WATER_STORAGE_CAP,
                    "nước vượt sức chứa chum"
                );
                assert!(
                    r.stock.seed_grain <= SEED_STORE_CAP,
                    "giống vượt sức chứa kho"
                );
                assert_balanced(&current, &r);
                current = r.stock;
            }
        }
    }

    /// Hàm thuần: 1000 lần cùng đầu vào cho cùng kết quả.
    #[test]
    fn ham_thuan_1000_lan_cung_ket_qua() {
        let stock = Stock {
            food: 37,
            water: 11,
            wood: 3,
            tools: 5,
            seed_grain: 4,
            soil_fertility: 512,
        };
        let work = Workforce {
            farmers: 3,
            hunters: 1,
            smiths: 1,
            elders: 0,
            children: 5,
        };
        let first = run_day(&stock, &work, false);
        for i in 0..1000 {
            assert_eq!(
                run_day(&stock, &work, false),
                first,
                "lần chạy thứ {i} lệch"
            );
        }
        // Và đầu vào không bị sửa: hàm không giữ trạng thái nào ở ngoài.
        assert_eq!(stock.food, 37);
    }

    /// `Workforce` toàn số 0 không chia cho 0, và không làm gì cả.
    #[test]
    fn workforce_toan_khong_khong_chia_cho_khong() {
        let stock = Stock::starting_village();
        let r = run_day(&stock, &Workforce::default(), true);

        assert_eq!(r.consumed.water, 0, "không ai thì không ai uống");
        assert_eq!(r.produced.water, 0, "không ai thì không ai gánh");
        assert_eq!(r.consumed.wood, 0);
        assert_eq!(r.consumed.tools, 0, "không ai làm thì không mòn");
        assert!(
            !r.shortages.contains(&Shortage::Food),
            "một cái làng rỗng thì không đói: {:?}",
            r.shortages
        );
        // Ruộng bỏ hoang thì độ phì hồi, không rút.
        assert_eq!(r.consumed.soil_fertility, 0);
        assert_balanced(&stock, &r);
    }

    /// Ăn hết sạch lương thực **không** được ăn vào `seed_grain`.
    ///
    /// Mất giống là mất cả vụ sau, nên đây là một ràng buộc riêng chứ không phải
    /// một trường hợp của "kho không âm": mô hình phải **không diễn đạt được**
    /// việc ăn giống, để quyết định đó buộc phải nổi lên lớp trên.
    #[test]
    fn an_het_luong_thuc_khong_duoc_an_vao_giong() {
        let stock = Stock {
            food: 1,
            water: 0,
            wood: 0,
            tools: 0,
            seed_grain: 12,
            soil_fertility: FERTILITY_MAX,
        };
        // Không Farmer: không gieo, nên giống không được đụng vào vì bất kỳ lý do nào.
        let work = Workforce {
            farmers: 0,
            hunters: 0,
            smiths: 0,
            elders: 2,
            children: 8,
        };

        let mut current = stock;
        for day in 1..=20 {
            let r = run_day(&current, &work, false);
            assert_eq!(
                r.stock.seed_grain, 12,
                "ngày {day}: người đói đã ăn vào kho giống"
            );
            assert!(r.stock.food >= 0);
            current = r.stock;
        }
        assert_eq!(current.food, 0, "kho ăn phải cạn — nạn đói vẫn phải xảy ra");
        assert_eq!(current.seed_grain, 12, "kho giống phải nguyên vẹn");
    }

    /// Độ phì giảm khi trồng liên tục, và hồi khi bỏ hoang.
    #[test]
    fn do_phi_giam_khi_trong_lien_tuc_va_hoi_khi_bo_hoang() {
        // Thâm canh: 6 Farmer trên mảnh đất chỉ nuôi nổi 4.
        //
        // Không thể đẩy cao hơn nữa: thêm người là thêm miệng uống, và trần giếng
        // cắt nước tưới trước khi đất kịp bạc. Đó không phải giới hạn của test mà
        // là một phát biểu của mô hình — **nước chặn trước đất**, nên muốn vắt
        // kiệt đất thì phải vắt trong một cửa sổ dân số rất hẹp.
        let intensive = Workforce {
            farmers: 6,
            hunters: 2,
            smiths: 1,
            elders: 1,
            children: 0,
        };
        // Công cụ dư dả, để hình phạt thiếu công cụ không trộn vào phép đo.
        let stock = Stock {
            tools: 200,
            ..Stock::starting_village()
        };
        let mined = run_days(&stock, &intensive, 60);
        let after = mined[59].stock;
        assert!(
            after.soil_fertility < FERTILITY_MAX,
            "trồng liên tục quá sức đất mà độ phì không giảm: {}",
            after.soil_fertility
        );

        // Và độ phì phải giảm **đơn điệu** trong giai đoạn đầu, không dao động.
        assert!(mined[0].stock.soil_fertility < FERTILITY_MAX);
        assert!(mined[9].stock.soil_fertility < mined[0].stock.soil_fertility);

        // Bỏ hoang: cùng kho đó, không còn ai ra đồng.
        let fallow_work = Workforce {
            farmers: 0,
            ..intensive
        };
        let rested = run_days(&after, &fallow_work, 10);
        assert_eq!(
            rested[0].stock.soil_fertility,
            after.soil_fertility + FERTILITY_FALLOW_REGEN,
            "đất nghỉ phải hồi đúng mức đã khai"
        );
        assert!(
            rested[9].stock.soil_fertility > after.soil_fertility,
            "bỏ hoang mười ngày mà đất không hồi"
        );
    }

    /// Bỏ Child ⇒ thiếu nước. Trẻ con gánh nước là một mắt xích thật.
    #[test]
    fn bo_tre_con_thi_lang_thieu_nuoc() {
        let work = Workforce {
            children: 0,
            ..Workforce::starting_village()
        };
        let days = run_days(&Stock::starting_village(), &work, 30);
        assert!(
            days.iter().any(|r| r.shortages.contains(&Shortage::Water)),
            "bỏ hai đứa trẻ mà không ai khát — trẻ con đang là vật trang trí"
        );
        // Và hậu quả phải chảy tiếp sang vụ mùa, không dừng ở một dòng cảnh báo.
        //
        // So sánh **sản lượng ngày**, không so tổng kho: làng ít trẻ con cũng là
        // làng ít miệng ăn, nên tổng kho của nó có thể cao hơn dù ruộng khô hơn.
        // Đo tổng kho ở đây là đo nhầm biến.
        let full = run_days(
            &Stock::starting_village(),
            &Workforce::starting_village(),
            30,
        );
        assert!(
            days[29].produced.food < full[29].produced.food,
            "thiếu nước tưới mà mùa màng không suy suyển: {} với {}",
            days[29].produced.food,
            full[29].produced.food
        );
    }

    /// Bỏ Hunter ⇒ bếp tắt, lò rèn tắt, và lương thực cạn nhanh hơn vì ăn sống.
    #[test]
    fn bo_hunter_thi_bep_tat_va_lo_ren_ngung() {
        let work = Workforce {
            hunters: 0,
            ..Workforce::starting_village()
        };
        let days = run_days(&Stock::starting_village(), &work, 12);

        let out_of_wood = days
            .iter()
            .position(|r| r.stock.wood == 0)
            .expect("không có Hunter thì củi phải hết");
        assert!(
            (2..=6).contains(&(out_of_wood + 1)),
            "kho 3 ngày củi phải hết trong khoảng dự đoán được, thực tế ngày {}",
            out_of_wood + 1
        );
        // Lò tắt thì Smith không sửa được gì, và công cụ bắt đầu mòn về 0.
        let last = &days[11];
        assert_eq!(last.produced.tools, 0, "không củi thì lò rèn không đỏ lửa");
        assert!(last.shortages.contains(&Shortage::Wood));
        assert!(last.shortages.contains(&Shortage::Tools));
    }

    /// Bỏ Elder ⇒ mất giống dần và kho hỏng gấp đôi. Người già không phải vật trang trí.
    #[test]
    fn bo_elder_thi_kho_giong_hao_dan() {
        let work = Workforce {
            elders: 0,
            ..Workforce::starting_village()
        };
        let days = run_days(&Stock::starting_village(), &work, 40);
        let with_elder = run_days(
            &Stock::starting_village(),
            &Workforce::starting_village(),
            40,
        );

        assert!(
            days[39].stock.seed_grain < with_elder[39].stock.seed_grain,
            "không có người giữ giống mà kho giống không hao"
        );
        assert!(
            days.iter()
                .any(|r| r.shortages.contains(&Shortage::SeedGrain)),
            "kho giống hao dần mà không ai được báo"
        );
        assert!(
            days[39].stock.food < with_elder[39].stock.food,
            "không ai trông kho mà kho không hỏng thêm"
        );
    }

    /// Thiếu hụt không trùng lặp và theo thứ tự cố định.
    #[test]
    fn thieu_hut_khong_trung_lap_va_theo_thu_tu_co_dinh() {
        // Một cái làng hỏng toàn diện: không kho, không công cụ, đất bạc.
        let stock = Stock {
            food: 0,
            water: 0,
            wood: 0,
            tools: 0,
            seed_grain: 0,
            soil_fertility: 100,
        };
        let r = run_day(&stock, &Workforce::starting_village(), false);

        assert_eq!(
            r.shortages,
            vec![
                Shortage::Food,
                Shortage::Water,
                Shortage::Wood,
                Shortage::Tools,
                Shortage::SeedGrain,
                Shortage::Fertility,
            ],
            "cả sáu vòng cùng hỏng thì phải báo cả sáu, đúng thứ tự"
        );

        let mut sorted = r.shortages.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), r.shortages.len(), "có thiếu hụt bị báo trùng");
    }

    /// Trần chum và trần kho giống là thật: dư ra thì **không lấy về**, chứ không
    /// phải lấy về rồi vứt đi — nhờ vậy bất biến cân đối vẫn đúng.
    #[test]
    fn tran_chum_va_tran_kho_giong_la_that() {
        let brimming = Stock {
            water: WATER_STORAGE_CAP,
            seed_grain: SEED_STORE_CAP,
            ..Stock::starting_village()
        };
        let work = Workforce::starting_village();
        let r = run_day(&brimming, &work, true);

        assert_eq!(
            r.produced.water, 0,
            "chum đầy thì không gánh thêm — nước dư phải là nước **không lấy về**, \
             không phải nước lấy về rồi đổ đi, nếu không bất biến cân đối sai"
        );
        assert!(r.stock.water <= WATER_STORAGE_CAP);
        assert_eq!(
            r.stock.seed_grain, SEED_STORE_CAP,
            "gieo 2 giữ 2, kho vẫn đầy"
        );
        assert!(r.stock.seed_grain <= SEED_STORE_CAP);
        assert_balanced(&brimming, &r);

        // Kho giống không bao giờ vượt trần, kể cả khi bị nhồi từ ngoài vào.
        let overfull = Stock {
            seed_grain: SEED_STORE_CAP * 3,
            ..brimming
        };
        let r = run_day(&overfull, &work, true);
        assert_eq!(
            r.produced.seed_grain, 0,
            "đã quá đầy thì không giữ thêm hạt nào"
        );
        assert_balanced(&overfull, &r);
    }

    /// Vượt trần rừng thì Hunter thứ tư không mang thêm gì — vòng gỗ có đáy.
    #[test]
    fn rung_co_day_them_hunter_khong_them_cui() {
        let stock = Stock {
            wood: 0,
            ..Stock::starting_village()
        };
        let three = Workforce {
            hunters: 3,
            ..Workforce::starting_village()
        };
        let twenty = Workforce {
            hunters: 20,
            ..Workforce::starting_village()
        };
        assert_eq!(
            run_day(&stock, &three, true).produced.wood,
            run_day(&stock, &twenty, true).produced.wood,
            "thêm 17 thợ săn mà rừng vẫn phải chỉ mọc lại bấy nhiêu"
        );
    }

    /// Giếng có trần: thêm người gánh không tạo thêm nước.
    #[test]
    fn gieng_co_tran_them_nguoi_ganh_khong_them_nuoc() {
        let dry = Stock {
            water: 0,
            ..Stock::starting_village()
        };
        let crowd = Workforce {
            children: 50,
            ..Workforce::starting_village()
        };
        let r = run_day(&dry, &crowd, true);
        assert_eq!(
            r.produced.water,
            super::WELL_YIELD_CAP,
            "năm mươi đứa trẻ vẫn không moi được nhiều hơn mạch giếng"
        );
        // Và đám đông đó vẫn phải uống: trần giếng biến thành thiếu nước.
        assert!(r.shortages.contains(&Shortage::Water));
    }

    /// Khẩu phần đúng bằng [`FOOD_PER_HEAD`] mỗi đầu người khi bếp còn nấu được.
    #[test]
    fn khau_phan_dung_bang_so_da_khai_khi_con_nau_duoc() {
        let stock = Stock::starting_village();
        let work = Workforce::starting_village();
        let r = run_day(&stock, &work, true);
        let spoiled = r.consumed.food - work.population() * FOOD_PER_HEAD;
        assert!(
            (0..=5).contains(&spoiled),
            "phần ngoài khẩu phần chỉ được là hao hụt kho, thực tế {spoiled}"
        );

        // Bếp tắt (hết củi) thì **cùng một số người** ăn tốn thêm 25%.
        // Phải cùng workforce ở cả hai vế, nếu không ta chỉ đang đo chênh lệch
        // số miệng ăn và tưởng đã đo được cái bếp.
        let starving_kitchen = Workforce { hunters: 0, ..work };
        let warm = run_day(&stock, &starving_kitchen, true);
        let cold = run_day(&Stock { wood: 0, ..stock }, &starving_kitchen, true);
        let heads = starving_kitchen.population();
        assert!(
            cold.consumed.food > warm.consumed.food,
            "ăn sống mà không tốn thêm gì thì bếp là hoạt cảnh: {} với {}",
            cold.consumed.food,
            warm.consumed.food
        );
        assert!(
            cold.consumed.food - warm.consumed.food >= heads * FOOD_PER_HEAD / 4,
            "phần ăn sống phải bằng ít nhất 1/4 khẩu phần"
        );
    }
}
