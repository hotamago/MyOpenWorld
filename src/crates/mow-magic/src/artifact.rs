//! Vật phẩm mang hành vi, tám cổng sử dụng, thiên phú và khải thị
//! (`idea.md §8.10`, `§13.8`, `PE-05`, `PE-06`, `PE-07`, `PE-08`).
//!
//! ## Vật phẩm **không chứa code**
//!
//! `§8.10.1`: nó chứa một **tham chiếu** tới law/spell đã đăng ký, cộng một bộ
//! tham số **đã đóng băng lúc phù phép**.
//!
//! > Vật phẩm không mở được cửa sau nào mà spell thường không có. Một cái trượng
//! > chỉ là một cách **đóng gói và trao đi** khả năng thi triển, **không phải một
//! > hệ thống luật song song**.
//!
//! Nên [`Behaviour`] có `module: String` và `bound_params`, và không có trường
//! nào chứa biểu thức. Nếu vật phẩm mang được mã nguồn thì mọi bất biến của
//! `§13.9` phải được kiểm lại lần thứ hai ở một đường khác — và đường thứ hai
//! bao giờ cũng lỏng hơn đường thứ nhất.
//!
//! ## "Dễ dùng" không phải một con số độ khó
//!
//! `§8.10.2`: nó là **tập cổng** mà người dùng phải qua, và **mỗi cổng có một
//! đường phá riêng**.
//!
//! > Mọi cổng phải khám phá được và phá được bằng phương tiện có trong world.
//! > Một cổng không có đường vượt là **một cái khóa tùy tiện**, không phải nội
//! > dung chơi được.
//!
//! Nên [`Gate::escape_routes`] tồn tại, và [`Behaviour::arbitrary_locks`] là một
//! bộ kiểm: một vật phẩm có cổng không lối thoát là lỗi dữ liệu.
//!
//! Ba thứ rơi ra từ bảng cổng mà không phải viết riêng:
//!
//! - Trượng mạnh **mất khẩu quyết** thành di vật không ai dùng được — và một
//!   học giả có lý do bỏ cả đời nghiên cứu nó.
//! - Tra khảo chủ nhân để lấy khẩu quyết là **tội** theo `§12.5`, có động cơ và
//!   để lại chứng cứ.
//! - Thử mò khẩu quyết là hợp lệ, xác suất thấp, `risk` cao — đó là lý do các
//!   phòng thí nghiệm phép thuật hay phát nổ.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tám cổng sử dụng (`§8.10.2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    /// Đọc được vật mang chữ.
    Literacy,
    /// Biết node tri thức ở mức tối thiểu.
    Knowledge,
    /// Chỉ số đủ ngưỡng.
    Stat,
    /// Mật khẩu, câu thần chú, trình tự rune.
    CommandWord,
    /// Ràng buộc theo linh hồn, huyết thống, lời thề.
    Attunement,
    /// Ổ khóa, phong ấn, vật chứa cần chìa.
    Physical,
    /// Mana, tuổi thọ, máu, lần dùng còn lại.
    Cost,
    /// Không chặn, nhưng dùng sai thì phản đòn.
    Risk,
}

/// Tám cổng, để lặp.
pub const GATES: [Gate; 8] = [
    Gate::Literacy,
    Gate::Knowledge,
    Gate::Stat,
    Gate::CommandWord,
    Gate::Attunement,
    Gate::Physical,
    Gate::Cost,
    Gate::Risk,
];

impl Gate {
    /// **Đường vượt qua trong world.**
    ///
    /// Mỗi cổng có ít nhất một, và đó là ràng buộc thiết kế chứ không phải một
    /// bảng tra cho vui: một cổng không có đường vượt là một cái khóa tùy tiện.
    pub fn escape_routes(self) -> &'static [&'static str] {
        match self {
            Gate::Literacy => &["học ngôn ngữ", "thuê người dịch", "giải mã"],
            Gate::Knowledge => &["học", "được dạy", "nghiên cứu", "ăn cắp tri thức"],
            Gate::Stat => &["luyện tập", "thuốc", "nghi thức tăng cường"],
            Gate::CommandWord => &[
                "được truyền lại",
                "tra khảo chủ cũ",
                "tìm ghi chép",
                "thám mã",
                "thử mò có rủi ro",
            ],
            Gate::Attunement => &[
                "nghi thức chuyển ràng buộc",
                "giết chủ cũ",
                "phá giao ước và chịu hậu quả",
            ],
            Gate::Physical => &["chìa khóa", "phá khóa", "cưỡng lực", "dịch chuyển"],
            Gate::Cost => &["tích tài nguyên", "tìm nguồn nạp"],
            // `Risk` không chặn ai, nên "vượt" nó là chuẩn bị chứ không phải mở khóa.
            Gate::Risk => &["chấp nhận rủi ro", "chuẩn bị phòng hộ"],
        }
    }

    /// Cổng này có **chặn** không, hay chỉ cảnh báo.
    ///
    /// `Risk` là cổng duy nhất không chặn — và đó là lý do nó nguy hiểm: người
    /// chơi dùng được ngay, và chỉ biết cái giá sau đó.
    pub fn blocks(self) -> bool {
        self != Gate::Risk
    }
}

/// Một yêu cầu cụ thể của một cổng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRequirement {
    /// Cổng nào.
    pub gate: Gate,
    /// Nội dung: node tri thức, tên chỉ số, khẩu quyết, loại khóa.
    pub detail: String,
    /// Ngưỡng, nếu là cổng có ngưỡng.
    pub threshold: i64,
}

/// Số lần dùng còn lại.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Charges {
    /// Tối đa.
    pub max: u32,
    /// Hiện tại.
    pub current: u32,
    /// Nạp lại bao nhiêu mỗi ngày, phần nghìn.
    pub recharge_per_day: u32,
}

/// Hành vi mà một vật phẩm mang.
///
/// **Tham chiếu module, không phải mã nguồn.**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Behaviour {
    /// Module đã đăng ký: `law.rune.frost_lance`.
    pub module: String,
    /// **Phiên bản đóng băng lúc phù phép** (`§13.9.5`).
    ///
    /// Một cây trượng cũ **không đổi hành vi** vì hôm nay Yuu chỉnh cân bằng.
    pub module_version: u32,
    /// Tham số đã đóng băng. Fixed-point, không phải biểu thức.
    pub bound_params: BTreeMap<String, i64>,
    /// Các cổng phải qua.
    pub gates: Vec<GateRequirement>,
    /// Lần dùng.
    pub charges: Charges,
    /// Trần fuel riêng cho vật phẩm này.
    pub fuel_budget: u64,
}

/// Những gì một người mang theo khi thử dùng vật phẩm.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bearer {
    /// Ai.
    pub who: Option<EntityId>,
    /// Ngôn ngữ đọc được.
    pub languages: Vec<String>,
    /// Node tri thức và bậc.
    pub knowledge: BTreeMap<String, i64>,
    /// Chỉ số.
    pub stats: BTreeMap<String, i64>,
    /// Khẩu quyết đã biết.
    pub command_words: Vec<String>,
    /// Đã ràng buộc với vật phẩm nào.
    pub attuned_to: Vec<String>,
    /// Chìa khóa đang giữ.
    pub keys: Vec<String>,
    /// Tài nguyên.
    pub resources: BTreeMap<String, i64>,
}

/// Vì sao chưa dùng được.
///
/// Không `Deserialize`: `routes` là bảng tra tĩnh của engine, không phải dữ liệu
/// đọc từ file. Đây là **kết quả tính toán**, không phải state — nó không đi vào
/// save, và đường vượt qua một cổng là chuyện engine biết chứ không phải chuyện
/// content pack khai.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Blocked {
    /// Cổng nào.
    pub gate: Gate,
    /// Cần gì.
    pub detail: String,
    /// **Cách vượt qua** — luôn có ít nhất một.
    pub routes: &'static [&'static str],
}

impl Behaviour {
    /// Những cổng đang chặn người này.
    ///
    /// Trả về **cả đường vượt**, không chỉ "không dùng được". Một cánh cửa khóa
    /// mà không gợi ý gì là một cánh cửa người chơi bỏ qua; một cánh cửa nói
    /// *"cần khẩu quyết — hỏi chủ cũ, tìm ghi chép, hoặc thử mò"* là một nhánh
    /// nhiệm vụ.
    pub fn blocked_for(&self, b: &Bearer) -> Vec<Blocked> {
        let mut ra = Vec::new();
        for g in &self.gates {
            if !g.gate.blocks() {
                continue;
            }
            let qua = match g.gate {
                Gate::Literacy => b.languages.contains(&g.detail),
                Gate::Knowledge => b
                    .knowledge
                    .get(&g.detail)
                    .is_some_and(|v| *v >= g.threshold),
                Gate::Stat => b.stats.get(&g.detail).is_some_and(|v| *v >= g.threshold),
                Gate::CommandWord => b.command_words.contains(&g.detail),
                Gate::Attunement => b.attuned_to.contains(&g.detail),
                Gate::Physical => b.keys.contains(&g.detail),
                Gate::Cost => b
                    .resources
                    .get(&g.detail)
                    .is_some_and(|v| *v >= g.threshold),
                Gate::Risk => true,
            };
            if !qua {
                ra.push(Blocked {
                    gate: g.gate,
                    detail: g.detail.clone(),
                    routes: g.gate.escape_routes(),
                });
            }
        }
        ra
    }

    /// Dùng được không.
    pub fn usable_by(&self, b: &Bearer) -> bool {
        self.charges.current > 0 && self.blocked_for(b).is_empty()
    }

    /// **Cổng nào không có đường vượt** — bộ kiểm content pack.
    ///
    /// Rỗng là hợp lệ. Không rỗng nghĩa là có một cái khóa tùy tiện, và
    /// `§8.10.2` gọi đó là lỗi dữ liệu chứ không phải một lựa chọn thiết kế.
    pub fn arbitrary_locks(&self) -> Vec<Gate> {
        self.gates
            .iter()
            .map(|g| g.gate)
            .filter(|g| g.escape_routes().is_empty())
            .collect()
    }

    /// Vật phẩm này có mở được **cửa sau** nào không.
    ///
    /// Kiểm rằng nó không xin fuel vượt trần của một spell thường. Nếu có, nó đã
    /// trở thành một hệ thống luật song song — đúng thứ `§8.10.1` cấm.
    pub fn exceeds_spell_budget(&self, normal_spell_budget: u64) -> bool {
        self.fuel_budget > normal_spell_budget
    }
}

/// Thiên phú: một khả năng **di truyền**, không học được (`§13.8`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Talent {
    /// Định danh.
    pub id: String,
    /// Hệ số di truyền, `0`–`1000` — nối vào `mow-life::quantgen`.
    pub heritability: u16,
    /// Node tri thức mà nó mở khóa sớm.
    pub unlocks: Vec<String>,
}

/// Một khải thị: tri thức đến **không qua đường học**.
///
/// `§8.10.6`: revelation phải có **provenance điều tra được**, và **tháo ngược
/// trả về node tri thức**.
///
/// Vì sao cả hai vế đều cần: một khải thị không có provenance là một món quà từ
/// hư không, và người chơi không có cách nào tìm hiểu hay tái tạo nó. Một khải
/// thị không tháo ngược được là một đường tắt vĩnh viễn — ai nhận được thì giữ
/// mãi, và tri thức đó không bao giờ vào được kho chung của nền văn minh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revelation {
    /// Nhận cái gì.
    pub grants: String,
    /// **Từ đâu** — điều tra được: một vị thần, một giấc mơ, một di vật, một nơi.
    pub source: String,
    /// Event đã ghi lại nó.
    pub event_seq: u64,
    /// Node tri thức tương đương, nếu tháo ngược.
    ///
    /// `None` nghĩa là **không tháo ngược được**, và đó là một cảnh báo thiết kế
    /// chứ không phải một lựa chọn: xem [`Revelation::is_dead_end`].
    pub reducible_to: Option<String>,
}

impl Revelation {
    /// Khải thị này có phải một **ngõ cụt** không.
    ///
    /// Ngõ cụt nghĩa là: người nhận dùng được, nhưng không ai khác học lại được,
    /// và nền văn minh không giàu thêm chút nào. Một world đầy khải thị ngõ cụt
    /// là một world mà tiến bộ chỉ đến từ may mắn.
    pub fn is_dead_end(&self) -> bool {
        self.reducible_to.is_none()
    }

    /// Có điều tra được nguồn gốc không.
    pub fn is_investigable(&self) -> bool {
        !self.source.is_empty() && self.event_seq > 0
    }
}

/// NPC tự tổng hợp một module mới (`§8.10.4`, `§22.41`).
///
/// Ba ràng buộc, và bỏ cái nào cũng cho ra một cửa hậu:
///
/// 1. **Chỉ ghép từ node đã biết.** Không thì NPC phát minh ra thứ chưa ai nghĩ tới.
/// 2. **Trần độ phức tạp theo skill.** Không thì một học trò tạo ra thứ mà đại
///    sư không tạo nổi.
/// 3. **Qua đúng validator như luật Yuu sinh.** Không thì có hai đường vào state
///    với hai mức nghiêm ngặt khác nhau, và đường lỏng hơn sẽ thắng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Synthesis {
    /// Ai tổng hợp.
    pub author: EntityId,
    /// Ghép từ những node nào.
    pub from_nodes: Vec<String>,
    /// Độ phức tạp của thứ định làm.
    pub complexity: u32,
}

/// Vì sao một lần tổng hợp bị từ chối.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynthesisError {
    /// Dùng node chưa biết.
    UnknownNode(String),
    /// Vượt trần độ phức tạp.
    TooComplex {
        /// Định làm gì.
        wanted: u32,
        /// Trần theo skill.
        cap: u32,
    },
}

/// Kiểm một lần tổng hợp.
///
/// `skill` `0`–`1000`. Trần độ phức tạp tỉ lệ thẳng với skill: một người mới học
/// ghép được hai node đơn giản, một đại sư ghép được cả một hệ.
pub fn check_synthesis(
    s: &Synthesis,
    known: &BTreeMap<String, i64>,
    skill: u16,
    complexity_per_skill: u32,
) -> Vec<SynthesisError> {
    let mut loi = Vec::new();
    for n in &s.from_nodes {
        if !known.contains_key(n) {
            loi.push(SynthesisError::UnknownNode(n.clone()));
        }
    }
    let tran = u32::from(skill) * complexity_per_skill / 1_000;
    if s.complexity > tran {
        loi.push(SynthesisError::TooComplex {
            wanted: s.complexity,
            cap: tran,
        });
    }
    loi
}
