//! Soak và World Health Report (`plan.md §P7.7`, `§P8.1`, `PF-10`).
//!
//! `soak-nightly.yml` chạy **3 world song song, mỗi world 200 năm**, bơm nhiễu
//! loạn có kiểm soát, và xuất một World Health Report.
//!
//! > Đây là cách phát hiện các lỗi **chỉ lộ ra sau hàng chục giờ** — đúng loại
//! > lỗi mà một người ngồi chơi thử không bao giờ bắt được.
//!
//! ## Rò rỉ không đo bằng "mỗi năm mô phỏng"
//!
//! `§P8.1` nói thẳng, và đây là bài học đắt nhất trong cả mục:
//!
//! > Một trần dạng *"RAM tăng dưới 50 MB mỗi năm"* cho phép một world 200 năm
//! > phình **10 GB** — đó là **cấp phép cho rò rỉ** chứ không phải phát hiện rò
//! > rỉ.
//!
//! Nên [`MemoryTrace::has_plateaued`] hỏi *"RAM đã đạt mặt bằng ổn định
//! chưa"*, không hỏi *"tăng bao nhiêu mỗi năm"*. Một hệ thống lành mạnh dùng
//! nhiều RAM lúc khởi động rồi **phẳng ra**; một hệ thống rò thì đường RAM cứ
//! dốc lên mãi, dù độ dốc có nhỏ.
//!
//! ## Cảnh báo phải nói được **triệu chứng**, không chỉ con số
//!
//! `§P7.7` đưa hai ví dụ, và cả hai đều là câu chứ không phải số: *"lạm phát
//! không giải thích được"*, *"quần thể loài X sụp"*. Nên [`Warning`] mang một
//! `symptom` — và cái phân biệt nó với một ngưỡng thường là chữ **"không giải
//! thích được"**: lạm phát có nguyên nhân truy được thì **không** là cảnh báo,
//! dù con số có cao.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Số world chạy song song mỗi đêm (`§P7.7`).
pub const SO_WORLD: usize = 3;

/// Số năm mỗi world chạy.
pub const SO_NAM: u32 = 200;

/// Một mẫu đo tại một mốc trong lần chạy soak.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sample {
    /// Năm thứ mấy.
    pub year: u32,
    /// Dân số.
    pub population: u64,
    /// Chỉ số giá — để phát hiện lạm phát.
    pub price_index: u64,
    /// Cung tiền.
    pub money_supply: u64,
    /// Số node tri thức đã mở.
    pub knowledge_nodes: u64,
    /// Số event mỗi ngày.
    pub events_per_day: u64,
    /// Tỉ lệ vùng đang active, phần nghìn.
    pub active_region_permille: u32,
    /// Quần thể từng loài.
    pub species_population: BTreeMap<String, u64>,
    /// RAM đang dùng, MB.
    pub rss_mb: u64,
    /// Số object còn sống — bắt rò rỉ mà RAM chưa lộ.
    pub live_objects: u64,
    /// Kích thước save, byte.
    pub save_bytes: u64,
    /// Số event đã ghi — để tính byte trên mỗi event.
    pub events_total: u64,
    /// Độ trễ tick p99, ms.
    pub tick_p99_ms: u32,
    /// Số vi phạm bất biến tích lũy.
    pub invariant_violations: u64,
    /// Số entity bị rò — tạo mà không ai tham chiếu.
    pub leaked_entities: u64,
}

/// Vết RAM qua cả lần chạy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryTrace {
    /// RSS theo từng năm.
    pub rss_by_year: Vec<(u32, u64)>,
}

/// Bao nhiêu năm đầu tính là **giai đoạn khởi động**.
///
/// Trong giai đoạn này RAM tăng là bình thường: chunk được nạp, quần thể lớn
/// lên, chỉ mục được dựng. Đo rò rỉ ở đây sẽ báo động giả.
pub const NAM_KHOI_DONG: u32 = 50;

/// Dao động cho phép quanh mặt bằng, phần nghìn.
///
/// Không phải 0: RAM thật luôn nhấp nhô theo mùa vụ, theo chunk đang giữ. Cái
/// phải bắt là **xu hướng dốc lên**, không phải nhiễu.
pub const DAO_DONG_CHO_PHEP: u64 = 150;

impl MemoryTrace {
    /// **RAM đã đạt mặt bằng ổn định chưa** (`§P8.1`).
    ///
    /// So nửa sau của giai đoạn ổn định với nửa đầu của nó. Phẳng thì đạt; dốc
    /// lên đều thì không, **dù độ dốc nhỏ tới đâu** — vì độ dốc nhỏ nhân với
    /// 200 năm vẫn là 10 GB.
    pub fn has_plateaued(&self) -> bool {
        let on_dinh: Vec<u64> = self
            .rss_by_year
            .iter()
            .filter(|(y, _)| *y > NAM_KHOI_DONG)
            .map(|(_, r)| *r)
            .collect();
        if on_dinh.len() < 4 {
            // Chưa đủ mẫu để kết luận — và "chưa kết luận được" phải khác
            // "đã đạt", nên trả `false`.
            return false;
        }
        let giua = on_dinh.len() / 2;
        let tb = |v: &[u64]| v.iter().sum::<u64>() / v.len() as u64;
        let dau = tb(&on_dinh[..giua]);
        let sau = tb(&on_dinh[giua..]);
        if dau == 0 {
            return sau == 0;
        }
        // Nửa sau không được cao hơn nửa đầu quá mức dao động cho phép.
        sau.saturating_sub(dau) * 1_000 / dau <= DAO_DONG_CHO_PHEP
    }

    /// RAM ở cuối lần chạy.
    pub fn final_rss(&self) -> u64 {
        self.rss_by_year.last().map_or(0, |(_, r)| *r)
    }
}

/// Một cảnh báo trong World Health Report (`§P7.7`).
///
/// Không `Deserialize`: `code` là bảng mã tĩnh của engine. Báo cáo là **kết
/// quả tính toán** từ chuỗi mẫu đo, dựng lại được bất cứ lúc nào — nó đi ra
/// file JSON cho CI đọc, không đi vào save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    /// Mã ổn định.
    pub code: &'static str,
    /// **Triệu chứng**, viết cho người đọc — không phải một con số trần trụi.
    pub symptom: String,
    /// Có phải lỗi phải sửa ngay không.
    pub blocking: bool,
}

/// Lạm phát trên mức này mà **không có nguyên nhân** thì là cảnh báo, phần nghìn.
pub const NGUONG_LAM_PHAT: i64 = 200;

/// Quần thể sụt quá mức này so với đỉnh thì là "sụp", phần nghìn.
pub const NGUONG_SUP_QUAN_THE: i64 = 800;

/// World Health Report của một lần chạy (`§P7.7`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthReport {
    /// World nào.
    pub world: String,
    /// Chạy bao nhiêu năm.
    pub years: u32,
    /// Mẫu đo đầu và cuối.
    pub first: Sample,
    /// Mẫu cuối.
    pub last: Sample,
    /// Vết RAM.
    pub memory: MemoryTrace,
    /// Cảnh báo.
    pub warnings: Vec<Warning>,
}

impl HealthReport {
    /// Lần chạy này có đạt không.
    pub fn healthy(&self) -> bool {
        !self.warnings.iter().any(|w| w.blocking)
    }

    /// Byte save trên mỗi event — đo theo `§P8.1`, không theo tổng.
    pub fn bytes_per_event(&self) -> u64 {
        if self.last.events_total == 0 {
            return 0;
        }
        self.last.save_bytes / self.last.events_total
    }
}

/// Nguyên nhân lạm phát mà hệ thống truy được (`§P7.7`).
///
/// Có mặt ở đây vì cảnh báo là *"lạm phát **không giải thích được**"*: một đợt
/// lạm phát có nguyên nhân truy được — mỏ bạc mới mở, chiến phí, tiền bị pha —
/// là mô phỏng đang chạy đúng, không phải một lỗi.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Explanations {
    /// Những nguyên nhân đã tìm thấy cho lạm phát.
    pub inflation_causes: Vec<String>,
    /// Những nguyên nhân đã tìm thấy cho sụt quần thể, theo loài.
    pub population_causes: BTreeMap<String, String>,
}

/// Dựng World Health Report từ chuỗi mẫu đo (`§P7.7`).
///
/// `explanations` là những gì hệ thống truy ngược được. Không truyền vào thì
/// mọi biến động lớn đều thành *"không giải thích được"* — và đó là mặc định
/// đúng: một cảnh báo thừa tốn công người đọc, một cảnh báo thiếu tốn cả một
/// bản phát hành.
pub fn health_report(
    world: &str,
    samples: &[Sample],
    explanations: &Explanations,
) -> Option<HealthReport> {
    let first = samples.first()?.clone();
    let last = samples.last()?.clone();

    let memory = MemoryTrace {
        rss_by_year: samples.iter().map(|s| (s.year, s.rss_mb)).collect(),
    };

    let mut warnings = Vec::new();

    // ── Bất biến bị phá: chặn, luôn luôn ──
    if last.invariant_violations > 0 {
        warnings.push(Warning {
            code: "invariant.violated",
            symptom: format!(
                "{} vi phạm bất biến trong {} năm — thế giới đã ở trạng thái không hợp lệ",
                last.invariant_violations, last.year
            ),
            blocking: true,
        });
    }

    // ── Rò entity ──
    if last.leaked_entities > 0 {
        warnings.push(Warning {
            code: "entity.leaked",
            symptom: format!(
                "{} thực thể tồn tại mà không ai tham chiếu — chúng vẫn ăn ngân sách mỗi tick",
                last.leaked_entities
            ),
            blocking: true,
        });
    }

    // ── Rò RAM: hỏi mặt bằng, không hỏi độ dốc ──
    if !memory.has_plateaued() {
        warnings.push(Warning {
            code: "memory.no_plateau",
            symptom: format!(
                "RAM chưa đạt mặt bằng sau {NAM_KHOI_DONG} năm khởi động — cuối kỳ {} MB \
                 và vẫn đang lên",
                memory.final_rss()
            ),
            blocking: true,
        });
    }

    // ── Lạm phát **không giải thích được** ──
    if first.price_index > 0 {
        let tang = (i64::try_from(last.price_index).unwrap_or(i64::MAX)
            - i64::try_from(first.price_index).unwrap_or(0))
            * 1_000
            / i64::try_from(first.price_index).unwrap_or(1);
        if tang > NGUONG_LAM_PHAT && explanations.inflation_causes.is_empty() {
            warnings.push(Warning {
                code: "economy.unexplained_inflation",
                symptom: format!(
                    "giá tăng {tang}‰ trong {} năm mà không truy được nguyên nhân nào — \
                     không có mỏ mới, không có chiến phí, không có tiền bị pha",
                    last.year
                ),
                blocking: true,
            });
        }
    }

    // ── Quần thể loài sụp ──
    for (loai, dau) in &first.species_population {
        let cuoi = last.species_population.get(loai).copied().unwrap_or(0);
        if *dau == 0 {
            continue;
        }
        let sut = (i64::try_from(*dau).unwrap_or(1) - i64::try_from(cuoi).unwrap_or(0)) * 1_000
            / i64::try_from(*dau).unwrap_or(1);
        if sut >= NGUONG_SUP_QUAN_THE {
            let co_ly_do = explanations.population_causes.contains_key(loai);
            warnings.push(Warning {
                code: "ecology.population_collapse",
                symptom: if co_ly_do {
                    format!(
                        "quần thể `{loai}` sụp {sut}‰ — nguyên nhân: {}",
                        explanations.population_causes[loai]
                    )
                } else {
                    format!("quần thể `{loai}` sụp {sut}‰ mà không truy được nguyên nhân")
                },
                // Có nguyên nhân thì là mô phỏng đang chạy đúng, không phải lỗi.
                blocking: !co_ly_do,
            });
        }
    }

    // ── Thế giới đứng im ──
    if last.events_per_day == 0 {
        warnings.push(Warning {
            code: "world.stalled",
            symptom: "không có event nào mỗi ngày — thế giới đã đứng im".to_owned(),
            blocking: true,
        });
    }

    Some(HealthReport {
        world: world.to_owned(),
        years: last.year,
        first,
        last,
        memory,
        warnings,
    })
}

/// Kết quả cả một đêm soak: ba world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoakRun {
    /// Báo cáo từng world.
    pub reports: Vec<HealthReport>,
}

impl SoakRun {
    /// Đêm nay có đạt không.
    ///
    /// **Một world đỏ là cả đêm đỏ.** Trung bình ba world sẽ giấu đúng cái
    /// world hỏng — và một lỗi chỉ lộ ra ở một trong ba cấu hình vẫn là một lỗi.
    pub fn passed(&self) -> bool {
        self.reports.len() == SO_WORLD && self.reports.iter().all(HealthReport::healthy)
    }

    /// Mọi cảnh báo chặn, kèm tên world.
    pub fn blockers(&self) -> Vec<(&str, &Warning)> {
        self.reports
            .iter()
            .flat_map(|r| {
                r.warnings
                    .iter()
                    .filter(|w| w.blocking)
                    .map(move |w| (r.world.as_str(), w))
            })
            .collect()
    }
}
