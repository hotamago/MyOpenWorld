//! Mô hình thương tích theo **bộ phận cơ thể** (`idea.md §9.4`, `PB-22`).
//!
//! > Body part có mô, chức năng, máu, đau, nhiễm trùng; `vitality` chỉ là chỉ
//! > số **suy ra** cho UI.
//!
//! ## Vì sao không phải một thanh máu
//!
//! Một thanh máu duy nhất trả lời được đúng một câu hỏi: "sắp chết chưa". Nó
//! không trả lời được câu nào trong số những câu mà thế giới này cần:
//!
//! - *Vì sao người này đi khập khiễng?* — chân trái bị thương.
//! - *Vì sao ông ta không cầm nổi rìu?* — mất hai ngón.
//! - *Vì sao vết thương không lành?* — nhiễm trùng, và không ai biết vì sao.
//! - *Vì sao ông ta ngất?* — mất máu, dù không bộ phận nào bị phá hủy.
//!
//! Cái giá là một mô hình phức tạp hơn hẳn. Nó xứng đáng vì mọi hệ thống khác
//! đọc từ đây: chiến đấu, y học, tri thức y khoa, địa vị xã hội của người tàn
//! tật, và cả những câu chuyện mà biên niên sử kể lại.
//!
//! ## `vitality` là **suy ra**, không phải lưu
//!
//! Đây là điều dễ làm sai nhất. Nếu `vitality` là một trường được lưu, thì sẽ
//! có hai nguồn sự thật: các bộ phận, và con số tổng. Chúng sẽ lệch nhau — luôn
//! luôn — và không ai biết cái nào đúng. Ở đây `vitality` là một **hàm**.

use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Loại mô của một bộ phận.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tissue {
    /// Da và mô mềm ngoài.
    Skin,
    /// Cơ.
    Muscle,
    /// Xương.
    Bone,
    /// Nội tạng.
    Organ,
    /// Mô thần kinh.
    Nerve,
}

impl Tissue {
    /// Số tick để hồi phục hoàn toàn từ tổn thương nhẹ.
    ///
    /// Xương lành chậm hơn da nhiều lần, và nội tạng gần như không tự lành. Đó
    /// là thứ khiến một vết chém và một vết đâm thủng bụng là hai câu chuyện
    /// khác nhau chứ không phải hai con số khác nhau.
    pub fn heal_ticks(self) -> u64 {
        match self {
            Tissue::Skin => 3_000,
            Tissue::Muscle => 9_000,
            Tissue::Bone => 60_000,
            Tissue::Organ => 200_000,
            // Mô thần kinh gần như không tái tạo. Con số này lớn tới mức thực
            // tế là "không lành trong một đời người", và đó là chủ đích.
            Tissue::Nerve => 5_000_000,
        }
    }

    /// Bộ phận thuộc mô này có gây mất máu nhiều không.
    pub fn bleeds(self) -> bool {
        matches!(self, Tissue::Skin | Tissue::Muscle | Tissue::Organ)
    }
}

impl CanonicalHash for Tissue {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(match self {
            Tissue::Skin => "skin",
            Tissue::Muscle => "muscle",
            Tissue::Bone => "bone",
            Tissue::Organ => "organ",
            Tissue::Nerve => "nerve",
        });
    }
}

/// Loại thương tích.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjuryKind {
    /// Vết cắt.
    Cut,
    /// Vết đâm.
    Pierce,
    /// Chấn thương kín.
    Blunt,
    /// Bỏng.
    Burn,
    /// Tê cóng.
    Frostbite,
    /// Mất hẳn bộ phận.
    Severed,
}

impl CanonicalHash for InjuryKind {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(match self {
            InjuryKind::Cut => "cut",
            InjuryKind::Pierce => "pierce",
            InjuryKind::Blunt => "blunt",
            InjuryKind::Burn => "burn",
            InjuryKind::Frostbite => "frostbite",
            InjuryKind::Severed => "severed",
        });
    }
}

/// Một thương tích trên một bộ phận.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Injury {
    /// Loại.
    pub kind: InjuryKind,
    /// Mức độ, `0`–`100`.
    pub severity: u8,
    /// Tick lúc bị thương.
    pub at_tick: u64,
    /// Đã nhiễm trùng chưa.
    ///
    /// Nhiễm trùng là một **trạng thái riêng**, không phải mức độ nặng hơn. Một
    /// vết xước nhiễm trùng giết người, còn một vết chém sạch thì lành — và đó
    /// là toàn bộ lý do y học tồn tại như một nhánh tri thức trong thế giới này.
    pub infected: bool,
}

impl CanonicalHash for Injury {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.kind.canonical_hash(h);
        h.write_u64(u64::from(self.severity));
        h.write_u64(self.at_tick);
        h.write_bool(self.infected);
    }
}

/// Một bộ phận cơ thể.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyPart {
    /// Định danh: `core.head`, `core.arm_left`.
    pub id: String,
    /// Mô chính.
    pub tissue: Tissue,
    /// Chức năng mà bộ phận này đóng góp, và đóng góp bao nhiêu phần trăm.
    ///
    /// Một người có hai tay, mỗi tay đóng góp 50% cho `manipulation`. Mất một
    /// tay không làm mất khả năng cầm nắm, nó làm giảm một nửa — và mô hình
    /// phải nói được điều đó.
    pub functions: Vec<(String, u8)>,
    /// Mất bộ phận này là chết ngay.
    pub vital: bool,
    /// Thương tích hiện có.
    pub injuries: Vec<Injury>,
    /// Bộ phận cha, nếu có. Chặt cánh tay thì mất luôn bàn tay.
    pub parent: Option<String>,
}

impl BodyPart {
    /// Bộ phận có còn không.
    pub fn is_severed(&self) -> bool {
        self.injuries.iter().any(|i| i.kind == InjuryKind::Severed)
    }

    /// Mức hiệu quả còn lại, `0`–`100`.
    pub fn efficiency(&self) -> u8 {
        if self.is_severed() {
            return 0;
        }
        let tong: u32 = self.injuries.iter().map(|i| u32::from(i.severity)).sum();
        100u32.saturating_sub(tong).min(100) as u8
    }

    /// Có đang chảy máu không.
    pub fn is_bleeding(&self) -> bool {
        self.tissue.bleeds()
            && self.injuries.iter().any(|i| {
                i.severity > 20
                    && matches!(
                        i.kind,
                        InjuryKind::Cut | InjuryKind::Pierce | InjuryKind::Severed
                    )
            })
    }
}

impl CanonicalHash for BodyPart {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        self.tissue.canonical_hash(h);
        h.write_seq(self.functions.iter(), |hh, (f, w)| {
            hh.write_str(f);
            hh.write_u64(u64::from(*w));
        });
        h.write_bool(self.vital);
        h.write_seq(self.injuries.iter(), |hh, i| i.canonical_hash(hh));
        h.write_option(self.parent.as_deref(), |hh, p| {
            hh.write_str(p);
        });
    }
}

/// Toàn bộ cơ thể.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BodyPlan {
    parts: Vec<BodyPart>,
    /// Lượng máu còn lại, `0`–`100`.
    ///
    /// Riêng biệt với thương tích: mất máu giết người **mà không bộ phận nào bị
    /// phá hủy**, và đó là một cách chết khác hẳn.
    pub blood: u8,
    /// Mức đau tích lũy, `0`–`100`.
    ///
    /// Đau không giết ai. Nó làm ngất, và ngất trong một trận đánh thì tương
    /// đương chết — nhưng ngất khi đang mổ thì lại là điều mong muốn.
    pub pain: u8,
}

impl BodyPlan {
    /// Cơ thể lành lặn với một bộ phận cho trước.
    pub fn new(parts: Vec<BodyPart>) -> BodyPlan {
        let mut p = parts;
        // Sắp theo id: thứ tự duyệt đi vào state hash và vào thứ tự chọn mục
        // tiêu khi ra đòn.
        p.sort_by(|a, b| a.id.cmp(&b.id));
        BodyPlan {
            parts: p,
            blood: 100,
            pain: 0,
        }
    }

    /// Mọi bộ phận, theo thứ tự id.
    pub fn parts(&self) -> impl Iterator<Item = &BodyPart> {
        self.parts.iter()
    }

    /// Một bộ phận.
    pub fn part(&self, id: &str) -> Option<&BodyPart> {
        self.parts.iter().find(|p| p.id == id)
    }

    /// Một bộ phận, để sửa.
    pub fn part_mut(&mut self, id: &str) -> Option<&mut BodyPart> {
        self.parts.iter_mut().find(|p| p.id == id)
    }

    /// Mức thực hiện được của một chức năng, `0`–`100`.
    ///
    /// Tổng đóng góp của các bộ phận còn hoạt động. Đây là thứ mà mọi hành động
    /// hỏi: đi lại hỏi `locomotion`, chế tác hỏi `manipulation`, nói hỏi
    /// `speech`.
    pub fn function_level(&self, function: &str) -> u8 {
        let tong: u32 = self
            .parts
            .iter()
            .flat_map(|p| {
                p.functions
                    .iter()
                    .filter(|(f, _)| f == function)
                    .map(move |(_, w)| u32::from(*w) * u32::from(p.efficiency()) / 100)
            })
            .sum();
        tong.min(100) as u8
    }

    /// Có bộ phận sinh tử nào đã mất không.
    pub fn is_dead(&self) -> bool {
        self.blood == 0 || self.parts.iter().any(|p| p.vital && p.is_severed())
    }

    /// Có đang bất tỉnh không.
    pub fn is_unconscious(&self) -> bool {
        self.pain >= 80 || self.blood < 30 || self.function_level("consciousness") < 30
    }

    /// Có bộ phận nào đang chảy máu không.
    pub fn is_bleeding(&self) -> bool {
        self.parts.iter().any(BodyPart::is_bleeding)
    }

    /// **`vitality` là chỉ số SUY RA**, chỉ dùng cho UI (`PB-22`).
    ///
    /// Không lưu nó ở đâu cả. Nếu lưu, sẽ có hai nguồn sự thật — các bộ phận và
    /// con số tổng — và chúng sẽ lệch nhau, luôn luôn.
    ///
    /// Cũng không dùng nó để quyết định gì trong mô phỏng. Mọi quyết định hỏi
    /// đúng câu hỏi cụ thể: [`BodyPlan::is_dead`], [`BodyPlan::is_unconscious`],
    /// [`BodyPlan::function_level`].
    pub fn vitality(&self) -> u8 {
        if self.is_dead() {
            return 0;
        }
        let hieu_qua: u32 = if self.parts.is_empty() {
            100
        } else {
            self.parts
                .iter()
                .map(|p| u32::from(p.efficiency()))
                .sum::<u32>()
                / self.parts.len() as u32
        };
        let mau = u32::from(self.blood);
        let dau = 100u32.saturating_sub(u32::from(self.pain));
        // Lấy giá trị nhỏ nhất chứ không phải trung bình: một người mất 90% máu
        // đang hấp hối, và trung bình hóa với "các bộ phận đều lành" sẽ vẽ ra
        // một thanh máu đầy ba phần tư ngay trước khi họ chết.
        hieu_qua.min(mau).min(dau) as u8
    }

    /// Áp một thương tích lên một bộ phận.
    ///
    /// Trả `false` nếu không có bộ phận đó. Chặt một bộ phận cũng chặt luôn mọi
    /// bộ phận con — mất cánh tay là mất luôn bàn tay và các ngón.
    pub fn injure(&mut self, part_id: &str, injury: &Injury) -> bool {
        let Some(p) = self.part_mut(part_id) else {
            return false;
        };
        let la_chat = injury.kind == InjuryKind::Severed;
        p.injuries.push(injury.clone());

        if la_chat {
            let mut can_chat = vec![part_id.to_owned()];
            // Lan xuống cây bộ phận con. Vòng lặp thay vì đệ quy để một cây bị
            // hỏng (tự trỏ vào mình) không làm tràn ngăn xếp.
            while let Some(cha) = can_chat.pop() {
                let con: Vec<String> = self
                    .parts
                    .iter()
                    .filter(|c| c.parent.as_deref() == Some(cha.as_str()) && !c.is_severed())
                    .map(|c| c.id.clone())
                    .collect();
                for id in con {
                    if let Some(c) = self.part_mut(&id) {
                        c.injuries.push(injury.clone());
                    }
                    can_chat.push(id);
                }
            }
        }
        true
    }

    /// Sơ đồ cơ thể người, dùng làm mẫu.
    ///
    /// Loài khác có giải phẫu khác, và `PB-21` dựa vào đó: chỗ mặc trang bị suy
    /// ra từ sơ đồ này, nên một loài bốn tay tự nhiên có bốn chỗ đeo găng mà
    /// không cần sửa engine.
    pub fn humanoid() -> BodyPlan {
        let p =
            |id: &str, tissue, functions: &[(&str, u8)], vital, parent: Option<&str>| BodyPart {
                id: id.to_owned(),
                tissue,
                functions: functions
                    .iter()
                    .map(|(f, w)| ((*f).to_owned(), *w))
                    .collect(),
                vital,
                injuries: Vec::new(),
                parent: parent.map(str::to_owned),
            };
        BodyPlan::new(vec![
            p(
                "core.head",
                Tissue::Bone,
                &[("consciousness", 60), ("sight", 100), ("speech", 100)],
                true,
                None,
            ),
            p(
                "core.brain",
                Tissue::Nerve,
                &[("consciousness", 40)],
                true,
                Some("core.head"),
            ),
            p("core.torso", Tissue::Bone, &[], true, None),
            p(
                "core.heart",
                Tissue::Organ,
                &[("circulation", 100)],
                true,
                Some("core.torso"),
            ),
            p(
                "core.lung_left",
                Tissue::Organ,
                &[("breathing", 50)],
                false,
                Some("core.torso"),
            ),
            p(
                "core.lung_right",
                Tissue::Organ,
                &[("breathing", 50)],
                false,
                Some("core.torso"),
            ),
            p(
                "core.arm_left",
                Tissue::Muscle,
                &[("manipulation", 25)],
                false,
                None,
            ),
            p(
                "core.arm_right",
                Tissue::Muscle,
                &[("manipulation", 25)],
                false,
                None,
            ),
            p(
                "core.hand_left",
                Tissue::Muscle,
                &[("manipulation", 25)],
                false,
                Some("core.arm_left"),
            ),
            p(
                "core.hand_right",
                Tissue::Muscle,
                &[("manipulation", 25)],
                false,
                Some("core.arm_right"),
            ),
            p(
                "core.leg_left",
                Tissue::Muscle,
                &[("locomotion", 50)],
                false,
                None,
            ),
            p(
                "core.leg_right",
                Tissue::Muscle,
                &[("locomotion", 50)],
                false,
                None,
            ),
        ])
    }
}

impl CanonicalHash for BodyPlan {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.parts.iter(), |hh, p| p.canonical_hash(hh));
        h.write_u64(u64::from(self.blood));
        h.write_u64(u64::from(self.pain));
    }
}
