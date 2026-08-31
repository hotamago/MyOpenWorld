//! Năm rào cản giữa các loài (`idea.md §9.11`, `PE-15`).
//!
//! > Có **năm loại, độc lập với nhau**, và một cặp loài có thể vượt được rào
//! > này mà không vượt được rào kia.
//!
//! Chữ *"độc lập"* là toàn bộ thiết kế, và nó là một chỉ dẫn tiêu cực: **không
//! được gộp thành một chỉ số quan hệ chủng tộc**. Lý do không phải là thẩm mỹ.
//!
//! Một chỉ số duy nhất không phân biệt nổi hai tình huống hoàn toàn khác nhau:
//!
//! | | Sinh sản | Môi trường | Tri giác | Thời gian | Xã hội |
//! |---|---|---|---|---|---|
//! | Người ↔ Elf | lai được, con hiếm muộn | chung dải | chung giác quan | **chênh 40 lần** | cùng lưỡng bội |
//! | Người ↔ Kiến-nhân | **không lai được** | chung dải | chung giác quan | tương đương | **đơn-lưỡng bội** |
//!
//! Gộp lại thì cả hai ra "quan hệ khó". Nhưng người và elf **cưới nhau được và
//! bi kịch nằm ở chỗ một bên chết trước**, còn người và kiến-nhân thì cả khái
//! niệm hôn nhân cũng không ánh xạ được. Hai cốt truyện khác nhau hoàn toàn, và
//! một con số làm mất đúng cái phần khác nhau đó.
//!
//! Nên [`Barriers`] là **năm trường**, và [`Barriers::summary`] cố tình **không**
//! trả về một con số: nó trả về danh sách rào nào chắn, rào nào không.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Rào cản sinh sản (`§9.11.1`, `§9.5.4`).
///
/// Rào cản **duy nhất đo được bằng thí nghiệm** — nên cũng là rào cản duy nhất
/// mà một nền văn minh đủ tò mò sẽ lập được bản đồ, và bản đồ đó lập tức thành
/// tài liệu chính trị.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reproductive {
    /// Lai được, con khỏe và sinh sản tiếp được.
    FullyCompatible,
    /// Lai được nhưng con giảm sức sống.
    ReducedViability,
    /// Lai được, con sống nhưng **vô sinh**.
    ///
    /// Đây là trạng thái khó nhất về mặt xã hội: nó cho phép tình yêu và hôn
    /// nhân nhưng chặn dòng dõi, nên nó tạo ra luật thừa kế chứ không tạo ra
    /// cấm đoán.
    SterileHybrid,
    /// Không thụ thai được.
    Incompatible,
}

impl Reproductive {
    /// Có con chung được không (dù con có sinh sản tiếp được hay không).
    pub fn can_bear_offspring(self) -> bool {
        !matches!(self, Reproductive::Incompatible)
    }

    /// Dòng dõi lai có tiếp tục được không.
    pub fn lineage_continues(self) -> bool {
        matches!(
            self,
            Reproductive::FullyCompatible | Reproductive::ReducedViability
        )
    }
}

/// Một dải điều kiện sống (`§9.7.5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    /// Cận dưới.
    pub lo: i32,
    /// Cận trên.
    pub hi: i32,
}

impl Range {
    /// Dựng.
    pub const fn new(lo: i32, hi: i32) -> Range {
        Range { lo, hi }
    }

    /// Phần chồng lấn với dải khác, nếu có.
    pub fn overlap(self, other: Range) -> Option<Range> {
        let lo = self.lo.max(other.lo);
        let hi = self.hi.min(other.hi);
        (lo <= hi).then_some(Range { lo, hi })
    }

    /// Độ rộng.
    pub fn width(self) -> i64 {
        i64::from(self.hi) - i64::from(self.lo)
    }
}

/// Hồ sơ môi trường của một loài (`§9.11.2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Habitat {
    /// Nhiệt độ chịu được, phần trăm độ C.
    pub temperature: Range,
    /// Mật độ mana chịu được.
    pub mana: Range,
    /// Nồng độ khí quyển chịu được.
    pub atmosphere: Range,
}

/// Chồng lấn dưới mức này trên một trục bất kỳ thì gọi là tranh chấp, phần nghìn.
pub const NGUONG_CHAT: i64 = 250;

/// Quan hệ lãnh thổ giữa hai loài, suy ra từ hai [`Habitat`].
///
/// **Không phải một thang từ hòa bình tới chiến tranh.** Hai cực đối lập của
/// bảng này — không chồng lấn gì và chồng lấn hoàn toàn — đều cho ra ít xung
/// đột lãnh thổ, vì lý do trái ngược nhau. Xung đột nằm ở **giữa**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Territorial {
    /// Không sống chung được ⇒ **không bao giờ tranh chấp lãnh thổ**.
    ///
    /// Thăm nhau cần trang bị, thuốc, hoặc phép duy trì — một chuyến thăm ngoại
    /// giao trở thành hoạt động có chi phí và có rủi ro.
    Disjoint,
    /// Chồng lấn hẹp ⇒ **tranh chấp gay gắt**: cả hai cần đúng một dải.
    NarrowContested,
    /// Chồng lấn rộng ⇒ chia sẻ được, đủ chỗ cho cả hai.
    BroadShared,
}

/// Bộ giác quan của một loài (`§9.11.3`).
///
/// `§9.11.3` gọi đây là *"rào cản bị bỏ quên nhiều nhất và thú vị nhất"*: hai
/// loài khác giác quan **sống trong những thế giới cảm nhận khác nhau**, chứ
/// không chỉ nói ngôn ngữ khác nhau.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Senses {
    /// Các kênh: `sight.visible`, `mana.gradient`, `pheromone`, `echolocation`…
    pub channels: BTreeSet<String>,
}

/// Vì sao một node tri thức không dạy được qua rào tri giác.
///
/// Không `Deserialize`: `bridges` là bảng tra tĩnh của engine. Đây là **kết quả
/// tính toán**, không phải state — nó không đi vào save, và ba cách xây khái
/// niệm cầu nối là chuyện engine biết chứ không phải chuyện content pack khai.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unteachable {
    /// Node nào.
    pub node: String,
    /// Kênh giác quan mà bên học không có.
    pub missing_channel: String,
    /// Ba cách xây khái niệm cầu nối (`§9.11.3`).
    ///
    /// Có sẵn ở đây vì *"không dạy được"* mà không kèm lối đi thì chỉ là một
    /// bức tường — và `§9.11.3` nói rõ rào này **vượt được**, chỉ là tốn công.
    pub bridges: &'static [&'static str],
}

/// Ba cầu nối khả dĩ.
pub const BRIDGES: &[&str] = &[
    "ẩn dụ mượn từ một kênh cả hai bên đều có",
    "dụng cụ đo biến đại lượng đó thành thứ nhìn/nghe được",
    "phép chia sẻ giác quan tạm thời",
];

impl Senses {
    /// Kênh mà bên kia có còn mình thì không.
    pub fn missing_from(&self, other: &Senses) -> BTreeSet<String> {
        other.channels.difference(&self.channels).cloned().collect()
    }

    /// Những node **không dạy trực tiếp được** cho một loài thiếu giác quan.
    ///
    /// `requires` là bảng: node nào cần kênh nào. Một node cần `mana.gradient`
    /// thì không dạy thẳng được cho loài không cảm được mana — dịch ngôn ngữ
    /// không đủ, vì từ đó **không có vật quy chiếu** ở phía bên kia.
    pub fn unteachable_to(
        &self,
        learner: &Senses,
        requires: &[(String, String)],
    ) -> Vec<Unteachable> {
        requires
            .iter()
            .filter(|(_, kenh)| self.channels.contains(kenh) && !learner.channels.contains(kenh))
            .map(|(node, kenh)| Unteachable {
                node: node.clone(),
                missing_channel: kenh.clone(),
                bridges: BRIDGES,
            })
            .collect()
    }

    /// Đội hỗn hợp nhìn được bao nhiêu kênh — `§13.5`, `§12.15.3`.
    ///
    /// Mạnh hơn vì nhìn được nhiều mặt của hiện tượng, nhưng trả giá bằng chi
    /// phí phối hợp: xem [`coordination_cost`].
    pub fn union_with(&self, other: &Senses) -> BTreeSet<String> {
        self.channels.union(&other.channels).cloned().collect()
    }
}

/// Chi phí phối hợp của một đội hỗn hợp, phần nghìn.
///
/// Tỉ lệ với **số kênh không chung**: mỗi kênh một bên có mà bên kia không là
/// một nguồn hiểu lầm có thật, không phải một khoản phạt tùy tiện.
pub fn coordination_cost(a: &Senses, b: &Senses) -> u32 {
    let chung = a.channels.intersection(&b.channels).count();
    let tong = a.channels.union(&b.channels).count();
    if tong == 0 {
        return 0;
    }
    let khong_chung = tong - chung;
    u32::try_from(khong_chung * 1_000 / tong).unwrap_or(1_000)
}

/// Rào cản thời gian (`§9.11.4`).
///
/// **Không phải `§4.5`.** `§9.11.4` ghi chú rõ: đây là *"tốc độ lão hóa khác
/// nhau trên cùng một đồng hồ"*, không phải hai world chạy hai tốc độ. Hai cơ
/// chế chồng lên nhau được, và khi chồng thì rebase vẫn theo clock domain của
/// từng tiến trình.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lifespan {
    /// Tuổi thọ trung bình, năm.
    pub years: u32,
    /// Tuổi trưởng thành, năm.
    pub adult_at: u32,
}

/// Sáu hệ quả của chênh lệch tuổi thọ (`§9.11.4`).
///
/// Đây là những thứ **tính ra được**, không phải cốt truyện viết tay: `§9.11.4`
/// điểm 5 nói thẳng *"Không cần viết cốt truyện cho việc này; nó là số học"*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeGap {
    /// Sống lâu hơn gấp mấy lần, phần nghìn.
    pub ratio_permille: u32,
    /// Bên sống ngắn thay bao nhiêu **thế hệ** trong một đời bên sống lâu.
    ///
    /// Đếm theo khoảng cách thế hệ — tuổi trưởng thành — chứ không theo tuổi
    /// thọ: một thế hệ mới bắt đầu khi lứa trước đến tuổi sinh sản, không khi
    /// lứa trước chết. Đây là nhịp mà biến đổi văn hóa và di truyền chạy theo.
    ///
    /// `§9.11.4` điểm 1: *"Con người vượt lên không phải vì thông minh hơn elf,
    /// mà vì họ **thay thế hệ**"*.
    pub generations_per_long_life: u32,
    /// Một cá nhân sống lâu giữ được tri thức qua bao nhiêu **đời** bên kia —
    /// và một vụ ám sát xóa sạch ngần ấy.
    ///
    /// Đếm theo tuổi thọ, không theo khoảng cách thế hệ: câu chuyện ở đây là
    /// *"không thiết chế nào của loài người giữ nổi qua ngần ấy thời gian"*, và
    /// đơn vị của nó là một đời người.
    pub individual_as_archive: u32,
    /// "Hòa ước một trăm năm" là bao nhiêu phần đời của mỗi bên, phần nghìn.
    pub treaty_meaning: (u32, u32),
    /// Đường thăng tiến có bị tắc không (`§12.10`): người đứng đầu không chết.
    pub gerontocracy_risk: bool,
    /// Quan hệ liên loài có phải bi kịch số học không.
    pub tragedy_is_arithmetic: bool,
}

/// Tính chênh lệch thời gian giữa hai loài.
pub fn time_gap(ngan: Lifespan, dai: Lifespan, treaty_years: u32) -> TimeGap {
    let ratio = u64::from(dai.years) * 1_000 / u64::from(ngan.years.max(1));
    let the_he = dai.years / ngan.adult_at.max(1);
    TimeGap {
        ratio_permille: u32::try_from(ratio).unwrap_or(u32::MAX),
        generations_per_long_life: the_he,
        individual_as_archive: dai.years / ngan.years.max(1),
        treaty_meaning: (
            treaty_years * 1_000 / ngan.years.max(1),
            treaty_years * 1_000 / dai.years.max(1),
        ),
        // Người đứng đầu sống gấp hơn ba lần đời một thế hệ dưới quyền thì
        // đường thăng tiến đóng lại — xã hội đó phải phát minh ra thoái vị,
        // lưu đày hoặc ngủ đông.
        gerontocracy_risk: ratio >= 3_000,
        tragedy_is_arithmetic: dai.years > ngan.years * 2,
    }
}

/// Cấu trúc sinh sản của loài (`§9.11.5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialStructure {
    /// Lưỡng bội, hộ gia đình — mô hình `§12.9` áp được.
    Diploid,
    /// Đơn-lưỡng bội: chúa và thợ vô sinh.
    ///
    /// Lợi ích tiến hóa của một con thợ nằm ở bộ gen chung của tổ, **không** ở
    /// việc sinh sản của chính nó.
    Haplodiploid,
    /// Quần thể hợp nhất, cá thể không phải đơn vị.
    Colonial,
}

/// Những hệ thống đã có mà **không áp được** cho một loài (`§9.11.5`).
///
/// Trả về danh sách chứ không phải một cờ "khác lạ": mỗi mục là một hệ thống
/// cụ thể phải viết bản riêng, và biết đúng cái nào thì mới ước lượng được
/// công sức.
pub fn inapplicable_systems(s: SocialStructure) -> &'static [&'static str] {
    match s {
        SocialStructure::Diploid => &[],
        SocialStructure::Haplodiploid => &[
            "§12.9 mô hình hộ gia đình — thợ vô sinh không lập hộ",
            "§12.5.2 động cơ phạm tội — chi phí đạo đức tính trên tổ, không trên cá nhân",
            "norm_set của loài khác xếp sai loại hành vi của họ",
        ],
        SocialStructure::Colonial => &[
            "§12.9 mô hình hộ gia đình — không có cá thể để lập hộ",
            "§12.5.2 động cơ phạm tội — không có cá nhân chịu trách nhiệm",
            "§12.10 đường thăng tiến — không có vị trí cá nhân để thăng",
            "norm_set của loài khác xếp sai loại hành vi của họ",
        ],
    }
}

/// Năm rào cản giữa một cặp loài. **Năm trường, không một chỉ số.**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Barriers {
    /// Sinh sản.
    pub reproductive: Reproductive,
    /// Lãnh thổ, suy ra từ hai habitat.
    pub territorial: Territorial,
    /// Kênh giác quan bên A có mà bên B không.
    pub perceptual_gap: BTreeSet<String>,
    /// Chênh lệch tuổi thọ.
    pub temporal: TimeGap,
    /// Cấu trúc xã hội hai bên có khác nhau không.
    pub social_mismatch: bool,
}

/// Một rào cản cụ thể có chắn hay không.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    /// `§9.11.1`.
    Reproductive,
    /// `§9.11.2`.
    Environmental,
    /// `§9.11.3`.
    Perceptual,
    /// `§9.11.4`.
    Temporal,
    /// `§9.11.5`.
    Social,
}

impl Barriers {
    /// Suy ra quan hệ lãnh thổ từ hai habitat.
    ///
    /// Đo **từng trục một**, rồi lấy trục chật nhất. Gộp ba trục lại trước khi
    /// so là một lỗi im lặng: nếu cả hai loài đều cần một dải khí quyển hẹp,
    /// con số gộp sẽ nhỏ ở cả tử lẫn mẫu, và một trục nhiệt độ chồng lấn đúng
    /// 2,5% — chỗ tranh chấp thật sự — bị pha loãng thành "chia sẻ được".
    ///
    /// Ngưỡng [`NGUONG_CHAT`]: chồng lấn dưới một phần tư dải của bên hẹp hơn
    /// **trên một trục bất kỳ** là tranh chấp. Con số này là một hiệu chỉnh,
    /// không phải một chân lý — nhưng nó có tên và nằm một chỗ, nên chỉnh được.
    pub fn territorial_from(a: Habitat, b: Habitat) -> Territorial {
        let truc = [
            (a.temperature, b.temperature),
            (a.mana, b.mana),
            (a.atmosphere, b.atmosphere),
        ];
        let mut chat_nhat = i64::MAX;
        for (x, y) in truc {
            let Some(chong) = x.overlap(y) else {
                return Territorial::Disjoint;
            };
            let can = x.width().min(y.width());
            // Dải rộng 0 mà vẫn chồng lấn nghĩa là hai bên cần đúng một điểm và
            // cùng có nó — chia sẻ được trên trục này, không phải tranh chấp.
            let phan_nghin = if can == 0 {
                1_000
            } else {
                chong.width() * 1_000 / can
            };
            chat_nhat = chat_nhat.min(phan_nghin);
        }
        if chat_nhat < NGUONG_CHAT {
            Territorial::NarrowContested
        } else {
            Territorial::BroadShared
        }
    }

    /// **Rào nào đang chắn** — cố tình không phải một con số.
    ///
    /// Hàm này là chỗ chống lại cám dỗ gộp: nếu nó trả về `u32` thì mọi chỗ gọi
    /// sẽ so sánh hai cặp loài bằng dấu `<`, và cái mất đi là câu hỏi *"chắn ở
    /// đâu"* — câu hỏi duy nhất mà người chơi thật sự hành động theo được.
    pub fn summary(&self) -> BTreeSet<Axis> {
        let mut v = BTreeSet::new();
        if !self.reproductive.lineage_continues() {
            v.insert(Axis::Reproductive);
        }
        if self.territorial != Territorial::BroadShared {
            v.insert(Axis::Environmental);
        }
        if !self.perceptual_gap.is_empty() {
            v.insert(Axis::Perceptual);
        }
        if self.temporal.tragedy_is_arithmetic {
            v.insert(Axis::Temporal);
        }
        if self.social_mismatch {
            v.insert(Axis::Social);
        }
        v
    }

    /// Hai cặp loài này có **chắn ở cùng những chỗ** không.
    ///
    /// Dùng thay cho việc so hai con số: hai cặp cùng "khó" mà chắn ở hai trục
    /// khác nhau thì cần hai cách xử lý khác nhau.
    pub fn same_shape_as(&self, other: &Barriers) -> bool {
        self.summary() == other.summary()
    }
}
