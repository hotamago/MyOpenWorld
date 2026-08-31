//! Yuu Auditor và Historian (`idea.md §15.1`, `§22.17`, `PF-07`).
//!
//! ## Auditor dùng **chung** bộ invariant với harness
//!
//! Đây là toàn bộ nội dung của nửa đầu module. Cám dỗ là viết cho Auditor một
//! bộ kiểm riêng, "nhẹ hơn, hợp để chạy lúc runtime". Kết quả là hai bộ, và
//! hai bộ thì **trôi khỏi nhau**: harness bắt được một lỗi mà Auditor không
//! thấy, hoặc tệ hơn, Auditor báo xanh cho một thế giới mà CI đã báo đỏ.
//!
//! Nên [`Auditor`] không định nghĩa invariant nào. Nó nhận một
//! `&InvariantRunner` — **cùng cái** mà test harness dùng — và chỉ thêm phần
//! Auditor có mà harness không có: quét rò rỉ prompt, tri thức bất hợp lệ, dữ
//! liệu mâu thuẫn.
//!
//! ## Historian chỉ dùng event có thật
//!
//! `§22.17`, và nó là bất biến khó giữ nhất trong cả tài liệu vì vi phạm nó
//! làm ra thứ **đọc hay hơn**:
//!
//! > Không có giải thích do model viết sau. Mọi thứ truy được về event thật.
//!
//! Nên [`Chronicle::compose`] không nhận một model. Nó nhận một danh sách
//! `EventSeq` và một hàm render, và mọi câu nó sinh ra mang theo `EventSeq`
//! sinh ra câu đó. Một câu không có nguồn thì **không vào biên niên sử** —
//! [`ChronicleError::UnsourcedClaim`].

use mow_core::{EntityId, EventSeq, InvariantReport};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

// ─────────────────────────────── Auditor ───────────────────────────────

/// Một phát hiện của Auditor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finding {
    /// Một bất biến bị phá — **chép từ harness**, không tự nghĩ ra.
    InvariantViolated {
        /// Bất biến nào, dạng `INV-22-<n>`.
        id: String,
        /// Chi tiết.
        detail: String,
    },
    /// Bí mật lọt vào prompt.
    PromptLeak {
        /// Prompt gửi cho ai.
        viewer: EntityId,
        /// Loại bí mật.
        kind: String,
    },
    /// Một thực thể biết thứ nó không có đường nào để biết.
    ///
    /// `§10.2`: *"Một entity không được biết portal bí mật, prompt, stat người
    /// khác hoặc event ở xa **nếu chưa có kênh thông tin**"*.
    UnreachableKnowledge {
        /// Ai.
        who: EntityId,
        /// Biết cái gì.
        node: String,
    },
    /// Hai nguồn nói hai điều trái nhau về cùng một sự việc.
    ContradictoryData {
        /// Về cái gì.
        subject: String,
        /// Hai lời khai.
        claims: (String, String),
    },
}

impl Finding {
    /// Mức nghiêm trọng: có phải thứ phải dừng lại ngay không.
    ///
    /// `PromptLeak` là nghiêm trọng theo `§22.40` — một bí mật đã gửi đi thì
    /// không rút lại được. Một bất biến bị phá cũng vậy. Hai cái còn lại là dữ
    /// liệu sai: tệ, nhưng sửa được.
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Finding::InvariantViolated { .. } | Finding::PromptLeak { .. }
        )
    }
}

/// Báo cáo của Auditor.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AuditReport {
    /// Mọi phát hiện.
    pub findings: Vec<Finding>,
}

impl AuditReport {
    /// Thế giới có sạch không.
    pub fn clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Những phát hiện phải dừng lại ngay.
    pub fn critical(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.is_critical()).collect()
    }
}

/// Kênh thông tin mà một thực thể có, dùng để kiểm tri thức bất hợp lệ.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Channels {
    /// Ai → những node họ có đường để biết.
    pub reachable: BTreeMap<EntityId, BTreeSet<String>>,
}

/// Auditor (`§15.1`).
///
/// **Không định nghĩa invariant nào.** Nó nhận báo cáo từ cùng
/// `InvariantRunner` mà test harness chạy, và chuyển thành [`Finding`].
#[derive(Debug, Clone, Default)]
pub struct Auditor;

impl Auditor {
    /// Gộp báo cáo bất biến của harness vào báo cáo Auditor.
    ///
    /// Chữ ký nhận `&InvariantReport` — kiểu mà harness sinh ra. Đó là cách
    /// "dùng chung bộ invariant" được thi hành bằng kiểu chứ bằng kỷ luật:
    /// không có đường nào để Auditor có một danh sách bất biến riêng.
    pub fn from_invariants(report: &InvariantReport) -> AuditReport {
        AuditReport {
            findings: report
                .violations
                .iter()
                .map(|v| Finding::InvariantViolated {
                    id: v.id.to_owned(),
                    detail: v.detail.clone(),
                })
                .collect(),
        }
    }

    /// Tìm tri thức mà thực thể không có kênh nào để biết (`§10.2`).
    pub fn unreachable_knowledge(
        knows: &BTreeMap<EntityId, BTreeSet<String>>,
        channels: &Channels,
    ) -> Vec<Finding> {
        let rong = BTreeSet::new();
        knows
            .iter()
            .flat_map(|(who, nodes)| {
                let toi_duoc = channels.reachable.get(who).unwrap_or(&rong);
                nodes
                    .difference(toi_duoc)
                    .map(move |n| Finding::UnreachableKnowledge {
                        who: *who,
                        node: n.clone(),
                    })
            })
            .collect()
    }

    /// Tìm dữ liệu mâu thuẫn: cùng một chủ đề, hai giá trị khác nhau.
    pub fn contradictions(claims: &[(String, String)]) -> Vec<Finding> {
        let mut theo_chu_de: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (chu_de, gia_tri) in claims {
            theo_chu_de
                .entry(chu_de.as_str())
                .or_default()
                .insert(gia_tri.as_str());
        }
        theo_chu_de
            .into_iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(chu_de, v)| {
                let mut it = v.into_iter();
                let a = it.next().unwrap_or("").to_owned();
                let b = it.next().unwrap_or("").to_owned();
                Finding::ContradictoryData {
                    subject: chu_de.to_owned(),
                    claims: (a, b),
                }
            })
            .collect()
    }
}

// ─────────────────────────────── Historian ───────────────────────────────

/// Một câu trong biên niên sử, **kèm nguồn**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// Câu.
    pub text: String,
    /// Event nào sinh ra nó. Nhiều event thì câu tổng hợp nhiều event.
    ///
    /// **Không rỗng được** — [`Chronicle::compose`] từ chối một câu không nguồn.
    pub sources: Vec<EventSeq>,
}

/// Vì sao một biên niên sử không dựng được.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChronicleError {
    /// Một câu không có event nào đằng sau.
    #[error(
        "câu \"{text}\" không có event nào đằng sau — §22.17 cấm giải thích do model \
         viết sau, kể cả khi nó đọc hay hơn"
    )]
    UnsourcedClaim {
        /// Câu nào.
        text: String,
    },
    /// Một câu trỏ tới event không có trong log.
    #[error("câu \"{text}\" trỏ tới event {seq:?} không có trong nhật ký")]
    DanglingSource {
        /// Câu nào.
        text: String,
        /// Event nào.
        seq: EventSeq,
    },
}

/// Biên niên sử đã dựng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chronicle {
    /// Các câu, theo thứ tự thời gian.
    pub lines: Vec<Line>,
}

impl Chronicle {
    /// Dựng biên niên sử, **chỉ từ event có thật** (`§22.17`).
    ///
    /// Không nhận một model. `render` là một hàm thuần từ event sang câu — và
    /// vì nó thuần, cùng nhật ký cho cùng biên niên sử, nên hai người đọc cùng
    /// một thế giới thấy cùng một lịch sử.
    pub fn compose(
        log: &BTreeSet<EventSeq>,
        lines: Vec<Line>,
    ) -> Result<Chronicle, ChronicleError> {
        for l in &lines {
            if l.sources.is_empty() {
                return Err(ChronicleError::UnsourcedClaim {
                    text: l.text.clone(),
                });
            }
            for s in &l.sources {
                if !log.contains(s) {
                    return Err(ChronicleError::DanglingSource {
                        text: l.text.clone(),
                        seq: *s,
                    });
                }
            }
        }
        Ok(Chronicle { lines })
    }

    /// Mọi event mà biên niên sử này dựa vào.
    pub fn sources(&self) -> BTreeSet<EventSeq> {
        self.lines
            .iter()
            .flat_map(|l| l.sources.iter().copied())
            .collect()
    }

    /// Một câu cụ thể dựa vào những event nào — **affordance "vì sao?"**.
    pub fn why(&self, index: usize) -> Option<&[EventSeq]> {
        self.lines.get(index).map(|l| l.sources.as_slice())
    }
}
