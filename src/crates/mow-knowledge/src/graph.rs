//! Đồ thị tri thức và thang hiểu biết (`idea.md §13.1`–`§13.5`, `PD-16`).
//!
//! ## `tech_points` **không phải tiền để mua node**
//!
//! `§13.1` nói rõ: điểm tồn tại như **progress nội bộ**, không phải một loại
//! tiền trừ đi để đổi lấy tri thức.
//!
//! Khác biệt này không phải chuyện chữ nghĩa. Với mô hình "mua node", một nền
//! văn minh cày đủ điểm là mở khóa được mọi thứ theo bất kỳ thứ tự nào, và tri
//! thức trở thành một cây kỹ năng. Với mô hình ở đây, [`Requirements`] phải
//! **thật sự thỏa** — có người biết dạy, có vật liệu, có bằng chứng quan sát
//! được — nên một nền văn minh không có mỏ sắt sẽ không luyện được thép dù giàu
//! tới đâu, và đó là chỗ địa lý biến thành lịch sử.
//!
//! ## Thang hiểu biết có sáu bậc, và **nghe nói không phải là biết**
//!
//! ```text
//! UNKNOWN → HEARD_OF → CONCEPTUAL → PRACTICED → PROFICIENT → MASTERED
//! ```
//!
//! *"Nghe nói về cổng liên-world không đồng nghĩa biết xây cổng."* Sáu bậc là để
//! diễn đạt đúng câu đó. Với một cờ `known: bool`, tin đồn và chuyên môn trở
//! thành cùng một thứ.
//!
//! ## Nhiều trường phái có thể cùng đúng cho tới khi thử nghiệm
//!
//! `§13.3`: *"Nhiều trường phái có thể cùng giải thích một hiện tượng bằng mô
//! hình khác nhau; thử nghiệm quyết định mô hình nào dự báo tốt hơn."*
//!
//! Nên [`Node`] có `school`, và hai node cùng giải thích một hiện tượng có thể
//! cùng tồn tại. [`Understanding`] của một người ghi họ theo trường phái nào —
//! và đó là hạt giống của ly giáo, tranh luận học thuật, và những dị giáo sinh
//! ra từ một lỗi dịch.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Bậc hiểu biết của một người về một node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Chưa từng nghe.
    Unknown,
    /// Nghe nói tới. **Không phải biết.**
    HeardOf,
    /// Hiểu khái niệm, chưa làm được.
    Conceptual,
    /// Đã làm, còn vụng.
    Practiced,
    /// Thành thạo.
    Proficient,
    /// Tinh thông — dạy lại được mà không mất mát nhiều.
    Mastered,
}

impl Level {
    /// Bậc này có **làm được** không.
    pub fn can_practise(self) -> bool {
        self >= Level::Practiced
    }

    /// Bậc này có **dạy được** không.
    ///
    /// Phải thành thạo mới dạy nổi. Một người vừa mới làm được đã đi dạy là
    /// nguồn chính của tri thức bị truyền sai — xem [`crate::teaching`].
    pub fn can_teach(self) -> bool {
        self >= Level::Proficient
    }

    /// Bậc kế tiếp, nếu còn.
    pub fn next(self) -> Option<Level> {
        match self {
            Level::Unknown => Some(Level::HeardOf),
            Level::HeardOf => Some(Level::Conceptual),
            Level::Conceptual => Some(Level::Practiced),
            Level::Practiced => Some(Level::Proficient),
            Level::Proficient => Some(Level::Mastered),
            Level::Mastered => None,
        }
    }
}

/// Những gì một node đòi hỏi mới khám phá được.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Requirements {
    /// Node tiền đề, và bậc tối thiểu cần đạt ở mỗi cái.
    pub prerequisites: Vec<(String, Level)>,
    /// Bằng chứng quan sát được cần có.
    ///
    /// Đây là thứ ngăn "cày điểm là mở khóa": không quan sát được hiện tượng thì
    /// không có gì để giải thích.
    pub evidence: Vec<String>,
    /// Vật liệu và công cụ.
    pub materials: Vec<String>,
    /// Cần bao nhiêu người phối hợp.
    pub collaborators: u32,
    /// Cần bao nhiêu chuyên môn khác nhau.
    pub distinct_specialties: u32,
}

/// Một node trong đồ thị tri thức.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    /// Định danh có namespace.
    pub id: String,
    /// Lĩnh vực: `metallurgy`, `evocation`, `agronomy`.
    pub domain: String,
    /// **Trường phái** giải thích. Hai node cùng lĩnh vực, khác trường phái có
    /// thể cùng tồn tại và cùng có người tin.
    pub school: String,
    /// Điều kiện.
    pub requirements: Requirements,
    /// Mở khóa cái gì: action, recipe, spell, node tiếp theo.
    pub unlocks: Vec<String>,
    /// Mức bí mật, `0`–`1000`. Cao thì người biết không muốn dạy.
    pub secrecy: u16,
    /// Độ khó truyền dạy, `0`–`1000`. Cao thì dạy hao hụt nhiều.
    pub teaching_difficulty: u16,
    /// Tỉ lệ thất bại khi thử nghiệm, phần nghìn.
    pub failure_rate: u16,
    /// Node này **dự báo tốt tới đâu**, `0`–`1000`.
    ///
    /// Đây là thứ thử nghiệm đo được, và là cách một trường phái thắng trường
    /// phái khác mà không cần ai tuyên bố nó đúng.
    pub predictive_power: u16,
}

/// Đồ thị tri thức của một world.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    nodes: BTreeMap<String, Node>,
}

impl KnowledgeGraph {
    /// Rỗng.
    pub fn new() -> KnowledgeGraph {
        KnowledgeGraph::default()
    }

    /// Thêm một node.
    pub fn add(&mut self, n: Node) -> &mut KnowledgeGraph {
        self.nodes.insert(n.id.clone(), n);
        self
    }

    /// Lấy một node.
    pub fn get(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Số node.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Các trường phái cùng giải thích một lĩnh vực.
    pub fn rival_schools(&self, domain: &str) -> Vec<&Node> {
        let mut v: Vec<&Node> = self.nodes.values().filter(|n| n.domain == domain).collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    /// Trường phái nào **dự báo tốt hơn** trong world này.
    ///
    /// Không phải "trường phái nào đúng". Thử nghiệm chỉ so được khả năng dự
    /// báo, và một mô hình sai về bản chất vẫn có thể dự báo tốt trong phạm vi
    /// đã thử — đó là lý do các trường phái sai vẫn sống rất lâu.
    pub fn best_predictor(&self, domain: &str) -> Option<&Node> {
        self.rival_schools(domain)
            .into_iter()
            .max_by_key(|n| (n.predictive_power, std::cmp::Reverse(n.id.clone())))
    }
}

/// Một người biết gì.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Understanding {
    /// `node → (bậc, độ tin cậy, học từ đâu)`.
    levels: BTreeMap<String, (Level, u16, Option<EntityId>)>,
}

impl Understanding {
    /// Rỗng.
    pub fn new() -> Understanding {
        Understanding::default()
    }

    /// Bậc hiện tại. Chưa từng nghe thì [`Level::Unknown`].
    pub fn level(&self, node: &str) -> Level {
        self.levels.get(node).map_or(Level::Unknown, |(l, _, _)| *l)
    }

    /// Độ tin cậy, `0`–`1000`.
    pub fn confidence(&self, node: &str) -> u16 {
        self.levels.get(node).map_or(0, |(_, c, _)| *c)
    }

    /// Học từ ai (`provenance`).
    ///
    /// `§13.2`: mỗi cạnh kèm confidence và provenance. Không có provenance thì
    /// không truy được một sai lệch về tận người dạy đầu tiên, và "dị giáo sinh
    /// ra từ một lỗi dịch" trở thành không kiểm chứng được.
    pub fn learned_from(&self, node: &str) -> Option<EntityId> {
        self.levels.get(node).and_then(|(_, _, f)| *f)
    }

    /// Đặt bậc.
    pub fn set(&mut self, node: &str, l: Level, confidence: u16, from: Option<EntityId>) {
        self.levels.insert(node.to_owned(), (l, confidence, from));
    }

    /// Mọi node đã biết ít nhất tới một bậc.
    pub fn at_least(&self, l: Level) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .levels
            .iter()
            .filter(|(_, (x, _, _))| *x >= l)
            .map(|(k, _)| k.as_str())
            .collect();
        v.sort_unstable();
        v
    }
}

/// Vì sao một khám phá chưa làm được.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Blocker {
    /// Thiếu node tiền đề, hoặc chưa đủ bậc.
    Prerequisite {
        /// Node nào.
        node: String,
        /// Cần bậc nào.
        need: Level,
        /// Đang ở bậc nào.
        have: Level,
    },
    /// Chưa quan sát được hiện tượng.
    MissingEvidence(String),
    /// Thiếu vật liệu hoặc công cụ.
    MissingMaterial(String),
    /// Không đủ người.
    NotEnoughCollaborators {
        /// Cần bao nhiêu.
        need: u32,
        /// Có bao nhiêu.
        have: u32,
    },
    /// Đủ người nhưng thiếu chuyên môn khác nhau.
    NotEnoughSpecialties {
        /// Cần bao nhiêu.
        need: u32,
        /// Có bao nhiêu.
        have: u32,
    },
}

/// Những gì đang chặn một khám phá.
///
/// Trả về **danh sách**, không phải `bool`. `§18.13` nguyên tắc 2: mọi con số
/// đều bấm được về nguồn — người chơi phải thấy *thiếu cái gì*, không phải thấy
/// một nút bấm bị làm mờ.
pub fn blockers(
    node: &Node,
    who: &Understanding,
    evidence: &BTreeSet<String>,
    materials: &BTreeSet<String>,
    collaborators: u32,
    specialties: u32,
) -> Vec<Blocker> {
    let mut ra = Vec::new();

    for (id, need) in &node.requirements.prerequisites {
        let have = who.level(id);
        if have < *need {
            ra.push(Blocker::Prerequisite {
                node: id.clone(),
                need: *need,
                have,
            });
        }
    }
    for e in &node.requirements.evidence {
        if !evidence.contains(e) {
            ra.push(Blocker::MissingEvidence(e.clone()));
        }
    }
    for m in &node.requirements.materials {
        if !materials.contains(m) {
            ra.push(Blocker::MissingMaterial(m.clone()));
        }
    }
    if collaborators < node.requirements.collaborators {
        ra.push(Blocker::NotEnoughCollaborators {
            need: node.requirements.collaborators,
            have: collaborators,
        });
    }
    if specialties < node.requirements.distinct_specialties {
        ra.push(Blocker::NotEnoughSpecialties {
            need: node.requirements.distinct_specialties,
            have: specialties,
        });
    }

    ra
}
