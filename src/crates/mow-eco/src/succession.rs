//! Diễn thế sinh thái theo thời gian (`idea.md §9.10`, `PE-12`).
//!
//! `§7.3` sinh hệ sinh thái ban đầu và `§8.3` mô phỏng quần thể theo LOD. Thứ
//! thiếu ở giữa là: hệ sinh thái **thay đổi theo thời gian**.
//!
//! ```text
//! đất trọc → cỏ → cây bụi → cây tiên phong → rừng trưởng thành
//! ```
//!
//! Mỗi giai đoạn **nuôi một tập loài khác nhau**, và mất hàng chục tới hàng
//! trăm năm. Đó là hai tính chất, không phải một:
//!
//! - Nếu chỉ có thời gian mà tập loài không đổi thì diễn thế chỉ là một thanh
//!   tiến trình, và phá rừng không mất gì ngoài việc phải chờ.
//! - Nếu chỉ có tập loài mà không có thời gian thì rừng mọc lại sau một mùa, và
//!   quyết định phá rừng không có trọng lượng nào.
//!
//! ## Bốn quá trình mà `§9.10` đòi thêm
//!
//! Ngoài săn mồi và sức tải: **thụ phấn, phân hủy, phát tán hạt, hình thành
//! đất**. Chúng ở đây vì mỗi cái là một đường mà hành động của nền văn minh
//! đi vào hệ sinh thái và quay lại cắn:
//!
//! | Quá trình | Hỏng vì | Hậu quả đọc được |
//! |---|---|---|
//! | thụ phấn | mất loài thụ phấn | mất mùa |
//! | phân hủy | đất bị nén, mất sinh vật đáy | đất không hồi |
//! | phát tán hạt | mất chim/thú mang hạt | diễn thế đứng lại |
//! | hình thành đất | xói mòn sau phá rừng | tụt về giai đoạn sớm hơn |

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Một giai đoạn diễn thế (`§9.10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Đất trọc sau cháy, sạt lở, hoặc khai hoang.
    BareGround,
    /// Cỏ.
    Grass,
    /// Cây bụi.
    Shrub,
    /// Cây tiên phong — mọc nhanh, ưa sáng, sống ngắn.
    Pioneer,
    /// Rừng trưởng thành.
    MatureForest,
}

/// Số năm cần để đi hết một giai đoạn, trong điều kiện tốt.
///
/// Tăng dần: cỏ phủ đất trong vài năm, rừng trưởng thành cần hơn một đời người.
/// Con số ở đây là bậc độ lớn, và chúng nằm một chỗ để hiệu chỉnh được.
pub const NAM_MOI_GIAI_DOAN: [u32; 4] = [4, 15, 40, 120];

impl Stage {
    /// Giai đoạn kế tiếp trong chuỗi.
    pub fn next(self) -> Option<Stage> {
        match self {
            Stage::BareGround => Some(Stage::Grass),
            Stage::Grass => Some(Stage::Shrub),
            Stage::Shrub => Some(Stage::Pioneer),
            Stage::Pioneer => Some(Stage::MatureForest),
            Stage::MatureForest => None,
        }
    }

    /// Số năm để rời giai đoạn này.
    pub fn years_to_advance(self) -> Option<u32> {
        match self {
            Stage::BareGround => Some(NAM_MOI_GIAI_DOAN[0]),
            Stage::Grass => Some(NAM_MOI_GIAI_DOAN[1]),
            Stage::Shrub => Some(NAM_MOI_GIAI_DOAN[2]),
            Stage::Pioneer => Some(NAM_MOI_GIAI_DOAN[3]),
            Stage::MatureForest => None,
        }
    }

    /// Tổng số năm từ đất trọc tới đây.
    pub fn years_from_bare(self) -> u32 {
        let mut s = Stage::BareGround;
        let mut t = 0;
        while s != self {
            let Some(n) = s.years_to_advance() else { break };
            t += n;
            let Some(k) = s.next() else { break };
            s = k;
        }
        t
    }

    /// Tập loài mà giai đoạn này nuôi được.
    ///
    /// **Khác nhau ở từng giai đoạn** — đó là điều làm diễn thế khác một thanh
    /// tiến trình. Một loài chỉ sống ở rừng trưởng thành thì mất chỗ ngay khi
    /// rừng bị đốt, và phải chờ trăm năm mới quay lại được.
    pub fn supports(self) -> &'static [&'static str] {
        match self {
            Stage::BareGround => &["pioneer.weed", "insect.ground_beetle"],
            Stage::Grass => &["grazer.rabbit", "bird.lark", "insect.grasshopper"],
            Stage::Shrub => &["browser.deer", "bird.warbler", "pollinator.bee"],
            Stage::Pioneer => &["bird.woodpecker", "predator.fox", "pollinator.bee"],
            Stage::MatureForest => &[
                "bird.owl",
                "predator.lynx",
                "fungus.mycorrhiza",
                "browser.deer",
                "decomposer.beetle",
            ],
        }
    }
}

/// Bốn quá trình mà `§9.10` đòi thêm, ngoài săn mồi và sức tải.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Process {
    /// Thụ phấn.
    Pollination,
    /// Phân hủy.
    Decomposition,
    /// Phát tán hạt.
    SeedDispersal,
    /// Hình thành đất.
    SoilFormation,
}

impl Process {
    /// Hỏng quá trình này thì hệ sinh thái biểu hiện ra sao.
    pub fn failure(self) -> &'static str {
        match self {
            Process::Pollination => "mất thụ phấn: cây có hoa không kết hạt, mất mùa",
            Process::Decomposition => "mất phân hủy: chất hữu cơ không quay lại đất",
            Process::SeedDispersal => "mất phát tán hạt: diễn thế đứng lại ở giai đoạn hiện tại",
            Process::SoilFormation => "mất hình thành đất: xói mòn, tụt về giai đoạn sớm hơn",
        }
    }
}

/// Một mảnh môi trường sống đang diễn thế.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patch {
    /// Định danh.
    pub id: u64,
    /// Giai đoạn hiện tại.
    pub stage: Stage,
    /// Đã ở giai đoạn này bao nhiêu năm.
    pub years_in_stage: u32,
    /// Độ dày đất, phần nghìn của mức đủ cho rừng trưởng thành.
    pub soil: u32,
    /// Quá trình nào còn hoạt động.
    pub processes: BTreeSet<Process>,
    /// Loài đang sống ở đây.
    pub species: BTreeSet<String>,
}

/// Đất tối thiểu để một giai đoạn tồn tại được, phần nghìn.
///
/// Rừng trưởng thành cần đất dày; cỏ thì gần như không cần. Đây là lý do phá
/// rừng rồi để xói mòn **không** hồi lại được chỉ bằng cách chờ.
pub fn dat_toi_thieu(s: Stage) -> u32 {
    match s {
        Stage::BareGround => 0,
        Stage::Grass => 50,
        Stage::Shrub => 200,
        Stage::Pioneer => 400,
        Stage::MatureForest => 700,
    }
}

/// Một biến cố xảy ra với mảnh đất.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// Tiến thêm `years` năm.
    Time {
        /// Bao nhiêu năm.
        years: u32,
    },
    /// Cháy hoặc bị đốt.
    Fire,
    /// Bị phá để lấy gỗ hoặc lấy đất.
    Cleared,
    /// Xói mòn — mất `permille` phần nghìn đất.
    Erosion {
        /// Mất bao nhiêu.
        permille: u32,
    },
    /// Mất một quá trình sinh thái.
    ProcessLost(Process),
    /// Khôi phục một quá trình.
    ProcessRestored(Process),
}

impl Patch {
    /// Một mảnh rừng trưởng thành khỏe mạnh.
    pub fn mature_forest(id: u64) -> Patch {
        Patch {
            id,
            stage: Stage::MatureForest,
            years_in_stage: 0,
            soil: 900,
            processes: BTreeSet::from([
                Process::Pollination,
                Process::Decomposition,
                Process::SeedDispersal,
                Process::SoilFormation,
            ]),
            species: Stage::MatureForest
                .supports()
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }

    /// Áp một biến cố. **Xác định** — không có RNG ở đây.
    pub fn apply(&mut self, e: &Event) {
        match e {
            Event::Time { years } => self.advance(*years),
            // Cháy đưa về đất trọc nhưng **giữ phần lớn đất**: tro là dinh
            // dưỡng. Đây là chỗ cháy khác hẳn phá rừng lấy đất.
            Event::Fire => {
                self.reset_to(Stage::BareGround);
                self.soil = self.soil.saturating_sub(100);
            }
            // Phá rừng lấy đất thì đất bị nén và mất lớp mặt.
            Event::Cleared => {
                self.reset_to(Stage::BareGround);
                self.soil = self.soil.saturating_sub(300);
                self.processes.remove(&Process::Decomposition);
                self.processes.remove(&Process::SoilFormation);
            }
            Event::Erosion { permille } => {
                self.soil = self.soil.saturating_sub(*permille);
                self.enforce_soil();
            }
            Event::ProcessLost(p) => {
                self.processes.remove(p);
            }
            Event::ProcessRestored(p) => {
                self.processes.insert(*p);
            }
        }
    }

    /// Diễn thế có đi tiếp được không, và **vì sao không**.
    ///
    /// Trả về lý do chứ không phải `bool`: *"rừng không mọc lại"* là một câu
    /// người chơi không hành động được, còn *"không còn loài phát tán hạt"* thì
    /// biết phải thả lại con gì.
    pub fn blocked_by(&self) -> Option<&'static str> {
        if self.stage == Stage::MatureForest {
            return None;
        }
        if !self.processes.contains(&Process::SeedDispersal) {
            return Some(Process::SeedDispersal.failure());
        }
        let can = self.stage.next().map_or(0, dat_toi_thieu);
        if self.soil < can {
            return Some("đất mỏng hơn mức giai đoạn kế tiếp cần");
        }
        None
    }

    fn advance(&mut self, years: u32) {
        let mut con = years;
        while con > 0 {
            if self.blocked_by().is_some() {
                self.years_in_stage = self.years_in_stage.saturating_add(con);
                // Đất vẫn dày lên nếu quá trình hình thành đất còn chạy, nên
                // một mảnh bị chặn vì thiếu đất **có thể tự gỡ chặn**.
                self.grow_soil(con);
                return;
            }
            let Some(can) = self.stage.years_to_advance() else {
                self.years_in_stage = self.years_in_stage.saturating_add(con);
                self.grow_soil(con);
                return;
            };
            let thieu = can.saturating_sub(self.years_in_stage);
            if con < thieu {
                self.years_in_stage += con;
                self.grow_soil(con);
                return;
            }
            con -= thieu;
            self.grow_soil(thieu);
            if let Some(k) = self.stage.next() {
                self.stage = k;
                self.years_in_stage = 0;
                self.species = k.supports().iter().map(|s| (*s).to_owned()).collect();
            }
        }
    }

    fn grow_soil(&mut self, years: u32) {
        if self.processes.contains(&Process::SoilFormation) {
            // Chậm: 1 phần nghìn mỗi năm. Trăm năm mới đủ cho rừng.
            self.soil = (self.soil + years).min(1_000);
        }
    }

    fn reset_to(&mut self, s: Stage) {
        self.stage = s;
        self.years_in_stage = 0;
        self.species = s.supports().iter().map(|x| (*x).to_owned()).collect();
    }

    /// Đất mỏng quá thì **tụt** về giai đoạn còn trụ được.
    fn enforce_soil(&mut self) {
        while dat_toi_thieu(self.stage) > self.soil {
            let lui = match self.stage {
                Stage::MatureForest => Stage::Pioneer,
                Stage::Pioneer => Stage::Shrub,
                Stage::Shrub => Stage::Grass,
                Stage::Grass | Stage::BareGround => Stage::BareGround,
            };
            if lui == self.stage {
                break;
            }
            self.reset_to(lui);
        }
    }
}
