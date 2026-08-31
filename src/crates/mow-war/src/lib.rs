//! # `mow-war` — chiến tranh (`idea.md §12.4`, `PD-23`)
//!
//! Bốn câu trong `§12.4`, và mỗi câu bác bỏ một cách làm rẻ:
//!
//! | `§12.4` nói | Cách rẻ bị bác bỏ |
//! |---|---|
//! | thắng bại phụ thuộc **hậu cần, chỉ huy, morale, địa hình, bệnh tật** | so tổng `combat_score` |
//! | quyết định có thể dựa trên **tin cũ hoặc sai** | mọi bên biết mọi thứ tức thời |
//! | hòa bình có **treaty thực thi được** | một biến `at_war = false` |
//!
//! ## Vì sao "không chỉ tổng combat score" là điều khó giữ
//!
//! Vì tổng điểm *hoạt động*. Nó cho ra kết quả hợp lý trong phần lớn trường hợp,
//! và chỉ sai ở đúng những trận đáng nhớ: đạo quân lớn hơn thua vì hết lương,
//! vì tướng chết, vì dịch tả, vì phải đánh ngược dốc. Nếu bỏ những trường hợp đó
//! thì chiến tranh trong world này chỉ là một phép so sánh số.
//!
//! Nên [`Army::effective_strength`] **nhân** các yếu tố với nhau thay vì cộng:
//! quân số nhiều gấp ba không bù được morale bằng không, vì bất kỳ thừa số nào
//! bằng 0 làm cả tích bằng 0. Đó là hình dạng đúng của bài toán.
//!
//! ## Hậu cần là thứ giết quân đội, không phải kẻ địch
//!
//! [`Campaign::step`] rút lương mỗi tick và giết dần khi hết. Một đạo quân bị
//! cắt đường tiếp tế **tự tan** mà không cần đánh — và đó là cách phần lớn các
//! cuộc vây hãm trong lịch sử kết thúc.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]

use mow_core::{EntityId, Tick};
use serde::{Deserialize, Serialize};

/// Vì sao một cuộc chiến bắt đầu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CasusBelli {
    /// Tranh lãnh thổ.
    Territory(String),
    /// Tranh tài nguyên.
    Resource(String),
    /// Khác niềm tin.
    Faith(String),
    /// Thù cũ.
    Grievance {
        /// Event nào đã gây ra.
        since_event: u64,
    },
    /// Bị kéo vào vì cam kết đồng minh.
    AllianceObligation {
        /// Với ai.
        ally: EntityId,
    },
}

/// Địa hình chỗ đánh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    /// Đồng bằng: không ai lợi.
    Open,
    /// Rừng: bên ít quân lợi.
    Forest,
    /// Dốc: bên trên lợi.
    Slope,
    /// Sông: bên phòng thủ lợi.
    River,
    /// Thành lũy: bên phòng thủ lợi rất nhiều.
    Fortified,
}

impl Terrain {
    /// Hệ số cho bên **phòng thủ**, phần nghìn.
    pub fn defender_multiplier(self) -> i64 {
        match self {
            Terrain::Open => 1_000,
            Terrain::Forest => 1_200,
            Terrain::Slope => 1_400,
            Terrain::River => 1_500,
            Terrain::Fortified => 2_500,
        }
    }
}

/// Một đạo quân.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Army {
    /// Định danh.
    pub id: String,
    /// Quân số.
    pub troops: u32,
    /// **Morale**, `0`–`1000`.
    ///
    /// Nhân vào, không cộng: morale bằng 0 nghĩa là đạo quân đã tan, và không có
    /// quân số nào bù được điều đó.
    pub morale: u16,
    /// Chất lượng chỉ huy, `0`–`1000`.
    pub command: u16,
    /// Trang bị và công nghệ, `0`–`1000`.
    pub equipment: u16,
    /// Lương thực còn lại, tính bằng khẩu phần.
    pub supplies: i64,
    /// Ăn bao nhiêu mỗi tick.
    pub consumption_per_tick: i64,
    /// Mức bệnh tật, `0`–`1000`.
    ///
    /// Trong lịch sử, bệnh giết nhiều lính hơn giao tranh. Một mô hình bỏ nó đi
    /// sẽ làm mọi chiến dịch dài thành khả thi.
    pub disease: u16,
}

impl Army {
    /// **Sức mạnh thực tế**, sau khi mọi thứ đã nhân vào nhau.
    ///
    /// Nhân chứ không cộng. Cộng thì quân số bù được mọi thứ, và ta quay lại
    /// đúng cái `combat_score` mà `§12.4` bác bỏ.
    pub fn effective_strength(&self, terrain_multiplier: i64) -> i64 {
        let n = i64::from(self.troops);
        let m = i64::from(self.morale);
        let c = i64::from(self.command);
        let e = i64::from(self.equipment);
        let khoe = i64::from(1_000 - self.disease);

        n * m / 1_000 * c / 1_000 * e / 1_000 * khoe / 1_000 * terrain_multiplier / 1_000
    }

    /// Còn đủ ăn bao nhiêu tick nữa.
    pub fn supply_ticks(&self) -> i64 {
        if self.consumption_per_tick <= 0 {
            return i64::MAX;
        }
        self.supplies / self.consumption_per_tick
    }

    /// Đạo quân này đã tan chưa.
    pub fn broken(&self) -> bool {
        self.troops == 0 || self.morale == 0
    }
}

/// Kết quả một trận.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleResult {
    /// Ai thắng.
    pub winner: String,
    /// Bên thắng mất bao nhiêu.
    pub attacker_losses: u32,
    /// Bên thua mất bao nhiêu.
    pub defender_losses: u32,
    /// Phân rã, để `§18.13` trả lời "vì sao bên đông hơn lại thua".
    pub factors: Vec<(String, i64)>,
}

/// Đánh một trận.
///
/// **Hàm thuần và xác định.** Trả về cả phân rã: câu hỏi hay gặp nhất sau một
/// trận thua là *"sao quân tôi đông gấp đôi mà vẫn thua"*, và câu trả lời phải
/// đọc được từ dữ liệu.
pub fn battle(attacker: &Army, defender: &Army, terrain: Terrain) -> BattleResult {
    let a = attacker.effective_strength(1_000);
    let d = defender.effective_strength(terrain.defender_multiplier());

    let factors = vec![
        ("quân số tấn công".to_owned(), i64::from(attacker.troops)),
        ("quân số phòng thủ".to_owned(), i64::from(defender.troops)),
        ("morale tấn công".to_owned(), i64::from(attacker.morale)),
        ("morale phòng thủ".to_owned(), i64::from(defender.morale)),
        ("bệnh tật tấn công".to_owned(), -i64::from(attacker.disease)),
        (
            format!("địa hình {terrain:?} cho bên thủ"),
            terrain.defender_multiplier(),
        ),
        ("sức mạnh thực tế tấn công".to_owned(), a),
        ("sức mạnh thực tế phòng thủ".to_owned(), d),
    ];

    // Bên yếu hơn mất nhiều hơn, nhưng bên thắng cũng mất — một trận thắng sạch
    // sẽ không tồn tại, và đó là lý do chiến tranh làm kiệt quệ cả hai bên.
    let tong = (a + d).max(1);
    let mat_a = u32::try_from(i64::from(attacker.troops) * d / tong / 2).unwrap_or(0);
    let mat_d = u32::try_from(i64::from(defender.troops) * a / tong / 2).unwrap_or(0);

    BattleResult {
        // Hòa thì bên **phòng thủ** giữ được đất — đó là định nghĩa của phòng thủ.
        winner: if a > d {
            attacker.id.clone()
        } else {
            defender.id.clone()
        },
        attacker_losses: mat_a,
        defender_losses: mat_d,
        factors,
    }
}

/// Một cuộc vây hãm hoặc chiến dịch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Campaign {
    /// Đạo quân.
    pub army: Army,
    /// Đường tiếp tế còn thông không.
    ///
    /// Đây là trường mà cắt đường tiếp tế tác động vào. Một đạo quân bị cắt
    /// đường **tự tan** mà không cần đánh.
    pub supply_line_open: bool,
    /// Tiếp tế được bao nhiêu mỗi tick khi đường còn thông.
    pub resupply_per_tick: i64,
}

/// Chuyện gì xảy ra trong một tick chiến dịch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignTick {
    /// Chết vì đói.
    pub starved: u32,
    /// Chết vì bệnh.
    pub died_of_disease: u32,
    /// Morale đổi bao nhiêu.
    pub morale_delta: i32,
    /// Đã tan chưa.
    pub broke: bool,
}

impl Campaign {
    /// Một tick.
    ///
    /// Thứ tự có ý nghĩa: ăn trước, rồi bệnh, rồi morale. Đói làm bệnh nặng hơn,
    /// và cả hai làm morale tụt — nên đảo thứ tự sẽ cho ra một chiến dịch dễ
    /// sống sót hơn thực tế.
    pub fn step(&mut self) -> CampaignTick {
        if self.supply_line_open {
            self.army.supplies += self.resupply_per_tick;
        }
        self.army.supplies -= self.army.consumption_per_tick;

        let mut doi = 0;
        let mut morale_delta: i32 = 0;

        if self.army.supplies < 0 {
            // Hết lương: chết dần, và morale tụt nhanh hơn quân số.
            let thieu = -self.army.supplies;
            doi = u32::try_from(thieu.min(i64::from(self.army.troops))).unwrap_or(0);
            self.army.troops = self.army.troops.saturating_sub(doi);
            self.army.supplies = 0;
            morale_delta -= 100;
            // Đói làm bệnh nặng hơn.
            self.army.disease = self.army.disease.saturating_add(50).min(1_000);
        } else {
            morale_delta += 5;
        }

        // Bệnh giết mỗi tick một phần nghìn theo mức bệnh.
        let benh =
            u32::try_from(i64::from(self.army.troops) * i64::from(self.army.disease) / 1_000 / 100)
                .unwrap_or(0);
        self.army.troops = self.army.troops.saturating_sub(benh);
        if benh > 0 {
            morale_delta -= 10;
        }

        self.army.morale =
            u16::try_from((i64::from(self.army.morale) + i64::from(morale_delta)).clamp(0, 1_000))
                .unwrap_or(0);

        CampaignTick {
            starved: doi,
            died_of_disease: benh,
            morale_delta,
            broke: self.army.broken(),
        }
    }
}

/// Một điều khoản có **cơ chế thực thi**.
///
/// `§12.4`: *"Hòa bình có treaty thực thi được, con tin, thương mại, giám sát
/// hoặc bảo chứng; **một biến `at_war=false` là chưa đủ**."*
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    /// Con tin: vi phạm thì con tin chết.
    Hostage {
        /// Ai bị giữ.
        who: EntityId,
    },
    /// Thương mại: vi phạm thì mất lợi ích.
    Trade {
        /// Giá trị mỗi kỳ.
        value_per_period: i64,
    },
    /// Giám sát: vi phạm thì bị phát hiện.
    Inspection {
        /// Xác suất phát hiện, `0`–`1000`.
        detection: u16,
    },
    /// Bảo chứng của bên thứ ba.
    Guarantor {
        /// Ai bảo chứng.
        who: EntityId,
        /// Họ mạnh tới đâu.
        strength: i64,
    },
}

impl Enforcement {
    /// Cơ chế này răn đe được bao nhiêu, `0`–`1000`.
    pub fn deterrence(&self) -> u16 {
        match self {
            Enforcement::Hostage { .. } => 600,
            Enforcement::Trade { value_per_period } => {
                u16::try_from((*value_per_period / 10).clamp(0, 1_000)).unwrap_or(1_000)
            }
            Enforcement::Inspection { detection } => *detection / 2,
            Enforcement::Guarantor { strength, .. } => {
                u16::try_from((*strength / 10).clamp(0, 1_000)).unwrap_or(1_000)
            }
        }
    }
}

/// Một hiệp ước.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Treaty {
    /// Định danh.
    pub id: String,
    /// Các bên.
    pub parties: Vec<String>,
    /// Ký lúc nào.
    pub signed: Tick,
    /// **Cơ chế thực thi.** Rỗng nghĩa là một tờ giấy.
    pub enforcement: Vec<Enforcement>,
}

impl Treaty {
    /// Hiệp ước này **giữ được** tới mức nào, `0`–`1000`.
    ///
    /// Rỗng thì bằng 0 — không phải một nửa, không phải "tùy thiện chí". Một
    /// hiệp ước không có cơ chế thực thi nào là một tờ giấy, và mô hình phải nói
    /// điều đó ra bằng con số.
    pub fn binding_strength(&self) -> u16 {
        let tong: u32 = self
            .enforcement
            .iter()
            .map(|e| u32::from(e.deterrence()))
            .sum();
        u16::try_from(tong.min(1_000)).unwrap_or(1_000)
    }

    /// Một bên có **đáng** phá hiệp ước không.
    ///
    /// `gain` là cái được nếu phá. So với sức ràng buộc — nên một hiệp ước chỉ
    /// dựa vào thiện chí sẽ bị phá ngay khi có lợi, và điều đó là **đúng**, không
    /// phải một lỗ hổng.
    pub fn worth_breaking(&self, gain: i64) -> bool {
        gain > i64::from(self.binding_strength())
    }
}
