//! Ngân sách hiệu năng và cổng CI (`plan.md §P8.1`, `PF-11`).
//!
//! > `§23` liệt kê các mục tiêu đo được của thế giới. Bảng này biến chúng thành
//! > cổng tự động — **mục tiêu không có cổng thì sớm muộn cũng trôi**.
//!
//! ## Ngân sách **theo phase**, không phải một mốc ở cuối
//!
//! `§P8.1` chỉ ra một mâu thuẫn mà nhiều plan mắc phải:
//!
//! > Plan không thể vừa nói *"vượt ngân sách là CI fail"* vừa hoãn việc đạt
//! > ngân sách tới Giai đoạn F — như vậy **hoặc CI luôn đỏ, hoặc câu "fail CI"
//! > là giả**.
//!
//! Nên [`Budget`] mang một [`Phase`], và [`check`] chỉ áp ngân sách của phase
//! đang chạy. Giai đoạn F **siết lại**, không phải lần đầu đo.
//!
//! ## Đơn vị đo phải chống được việc "đạt bằng cách làm ít đi"
//!
//! Một trần `tick_duration_ms` đạt được bằng cách bỏ bớt việc trong tick. Nên
//! mỗi [`Measurement`] mang theo **quy mô** — số thực thể, số chunk — và
//! [`check`] từ chối một phép đo có quy mô nhỏ hơn quy mô ngân sách yêu cầu.
//! Không có nó, cổng này tự mở sau một lần tối ưu sai hướng.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Giai đoạn phát triển — ngân sách siết dần theo đây (`§P8.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Giai đoạn A — bắt đầu đo.
    A,
    /// Giai đoạn B.
    B,
    /// Giai đoạn C.
    C,
    /// Giai đoạn D.
    D,
    /// Giai đoạn E.
    E,
    /// Giai đoạn F — quy mô đầy đủ.
    F,
}

/// Một chỉ số đo được.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    /// `tick_duration_ms` p99, một khu định cư active.
    TickP99Ms,
    /// Sinh một chunk `32×32×16`, ms.
    ChunkGenMs,
    /// Round-trip command → ack qua gateway, p95 ms.
    CommandAckP95Ms,
    /// Rebuild chunk texture ở frontend, ms.
    TextureRebuildMs,
}

impl Metric {
    /// Tên ổn định, dùng trong báo cáo CI.
    pub fn as_str(self) -> &'static str {
        match self {
            Metric::TickP99Ms => "tick_duration_ms_p99",
            Metric::ChunkGenMs => "chunk_gen_ms",
            Metric::CommandAckP95Ms => "command_ack_ms_p95",
            Metric::TextureRebuildMs => "texture_rebuild_ms",
        }
    }

    /// Quy mô tối thiểu để một phép đo có nghĩa.
    ///
    /// Đây là thứ chặn việc "đạt bằng cách làm ít đi": một tick 2 ms với 3 thực
    /// thể không chứng minh gì về một tick với 1200 thực thể.
    pub fn min_scale(self) -> u64 {
        match self {
            // Một khu định cư active — `§P8.1` nói rõ điều kiện đo.
            Metric::TickP99Ms => 1_000,
            // Một chunk đầy đủ `32×32×16`.
            Metric::ChunkGenMs => 32 * 32 * 16,
            Metric::CommandAckP95Ms => 1,
            Metric::TextureRebuildMs => 32 * 32,
        }
    }

    /// Quy mô đo bằng đơn vị gì — để báo cáo đọc được.
    pub fn scale_unit(self) -> &'static str {
        match self {
            Metric::TickP99Ms => "thực thể active",
            Metric::ChunkGenMs | Metric::TextureRebuildMs => "cell",
            Metric::CommandAckP95Ms => "command",
        }
    }
}

/// Một dòng ngân sách.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// Chỉ số nào.
    pub metric: Metric,
    /// Trần, tính bằng ms.
    pub limit_ms: u32,
    /// Áp từ phase nào trở đi.
    pub from_phase: Phase,
}

/// Bảng ngân sách của `§P8.1`.
///
/// > Giá trị khởi điểm, hiệu chỉnh sau lần profiling đầu.
///
/// Bảng nằm **trong code**, không trong một file YAML: một ngân sách sửa được
/// mà không qua review là một ngân sách sẽ được nới mỗi lần nó đỏ.
pub const BANG_NGAN_SACH: &[Budget] = &[
    Budget {
        metric: Metric::TickP99Ms,
        limit_ms: 40,
        from_phase: Phase::A,
    },
    Budget {
        metric: Metric::ChunkGenMs,
        limit_ms: 8,
        from_phase: Phase::A,
    },
    Budget {
        metric: Metric::CommandAckP95Ms,
        limit_ms: 50,
        from_phase: Phase::B,
    },
    Budget {
        metric: Metric::TextureRebuildMs,
        limit_ms: 4,
        from_phase: Phase::C,
    },
];

/// Một phép đo từ bench.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measurement {
    /// Chỉ số nào.
    pub metric: Metric,
    /// Đo được bao nhiêu ms.
    pub value_ms: u32,
    /// Quy mô lúc đo.
    pub scale: u64,
}

/// Vì sao một chỉ số trượt cổng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Failure {
    /// Vượt trần.
    OverBudget {
        /// Chỉ số.
        metric: &'static str,
        /// Đo được.
        measured_ms: u32,
        /// Trần.
        limit_ms: u32,
    },
    /// Quy mô đo quá nhỏ để kết luận.
    ///
    /// **Không phải "đạt".** Một phép đo dưới quy mô là một phép đo không nói
    /// gì, và coi nó là đạt biến cổng thành thứ tự mở.
    ScaleTooSmall {
        /// Chỉ số.
        metric: &'static str,
        /// Đo ở quy mô nào.
        scale: u64,
        /// Cần ít nhất bao nhiêu.
        required: u64,
        /// Đơn vị.
        unit: &'static str,
    },
    /// Không có phép đo nào cho một chỉ số đang áp ngân sách.
    ///
    /// Cũng **không phải "đạt"**. Một chỉ số không được đo là một chỉ số đã
    /// trôi — đúng thứ mà `§P8.1` nói cổng tự động sinh ra để ngăn.
    NotMeasured {
        /// Chỉ số.
        metric: &'static str,
    },
}

impl core::fmt::Display for Failure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Failure::OverBudget {
                metric,
                measured_ms,
                limit_ms,
            } => write!(f, "{metric}: {measured_ms} ms, trần {limit_ms} ms"),
            Failure::ScaleTooSmall {
                metric,
                scale,
                required,
                unit,
            } => write!(
                f,
                "{metric}: đo ở quy mô {scale} {unit}, cần ít nhất {required} — \
                 phép đo này không kết luận được gì"
            ),
            Failure::NotMeasured { metric } => {
                write!(f, "{metric}: không có phép đo nào — chỉ số đã trôi")
            }
        }
    }
}

/// Kết quả cổng ngân sách.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetReport {
    /// Chạy ở phase nào.
    pub phase: Phase,
    /// Những chỉ số đã kiểm và đạt.
    pub passed: Vec<&'static str>,
    /// Những chỗ trượt.
    pub failures: Vec<Failure>,
}

impl BudgetReport {
    /// CI có xanh không.
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Áp bảng ngân sách lên một bộ phép đo (`§P8.1`).
///
/// Chỉ áp những dòng có hiệu lực ở `phase`. Đó là cách câu *"vượt ngân sách là
/// CI fail"* thành thật ngay từ Giai đoạn A thay vì thành một lời hứa cho
/// Giai đoạn F.
pub fn check(phase: Phase, measurements: &[Measurement]) -> BudgetReport {
    let theo_chi_so: BTreeMap<Metric, &Measurement> =
        measurements.iter().map(|m| (m.metric, m)).collect();

    let mut passed = Vec::new();
    let mut failures = Vec::new();

    for b in BANG_NGAN_SACH {
        if phase < b.from_phase {
            continue;
        }
        let Some(m) = theo_chi_so.get(&b.metric) else {
            failures.push(Failure::NotMeasured {
                metric: b.metric.as_str(),
            });
            continue;
        };
        if m.scale < b.metric.min_scale() {
            failures.push(Failure::ScaleTooSmall {
                metric: b.metric.as_str(),
                scale: m.scale,
                required: b.metric.min_scale(),
                unit: b.metric.scale_unit(),
            });
            continue;
        }
        if m.value_ms > b.limit_ms {
            failures.push(Failure::OverBudget {
                metric: b.metric.as_str(),
                measured_ms: m.value_ms,
                limit_ms: b.limit_ms,
            });
            continue;
        }
        passed.push(b.metric.as_str());
    }

    BudgetReport {
        phase,
        passed,
        failures,
    }
}

/// Những chỉ số có hiệu lực ở một phase.
pub fn active_at(phase: Phase) -> Vec<Metric> {
    BANG_NGAN_SACH
        .iter()
        .filter(|b| phase >= b.from_phase)
        .map(|b| b.metric)
        .collect()
}
