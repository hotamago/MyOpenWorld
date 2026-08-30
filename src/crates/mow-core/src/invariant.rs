//! Bộ chạy bất biến (`idea.md §22`, `plan.md §P7.4`).
//!
//! Mỗi bất biến có một **ID vĩnh viễn** dạng `INV-22-<n>`. Số không bao giờ
//! được dùng lại và không bao giờ đánh số lại — đó là lý do một báo cáo lỗi từ
//! sáu tháng trước vẫn trỏ đúng chỗ.
//!
//! Ba mức chi phí, vì chạy tất cả ở mọi tick là bất khả thi và chạy tất cả
//! theo yêu cầu thì quá muộn:
//!
//! | Mức | Chạy khi nào | Ví dụ |
//! |---|---|---|
//! | [`Cost::Cheap`] | mọi tick, kể cả khi chơi | "mọi thực thể đang sống đều có sự kiện sinh ra" |
//! | [`Cost::Medium`] | mỗi N tick, và trong CI | "vật phẩm nằm ở đúng một nơi" |
//! | [`Cost::Expensive`] | soak và theo yêu cầu | "tổng của cải khớp với vòi và cống" |
//!
//! Bất biến **không phải** là test. Test hỏi "hàm này có đúng không"; bất biến
//! hỏi "thế giới có còn nhất quán không", và nó hỏi câu đó liên tục trong lúc
//! thế giới chạy hàng trăm năm mô phỏng. Một bất biến vi phạm luôn là bug của
//! engine, không bao giờ là lỗi người chơi.

use crate::sim::Sim;
use crate::value::Value;
use std::collections::BTreeSet;

/// Mức chi phí của một bất biến.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cost {
    /// Đủ rẻ để chạy mọi tick.
    Cheap,
    /// Chạy định kỳ và trong CI.
    Medium,
    /// Chỉ soak và khi được yêu cầu.
    Expensive,
}

/// Một vi phạm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// ID bất biến, ví dụ `INV-22-33`.
    pub id: &'static str,
    /// Chi tiết đủ để bắt đầu gỡ lỗi mà không cần chạy lại.
    pub detail: String,
}

/// Kết quả một lần chạy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvariantReport {
    /// Các bất biến đã kiểm.
    pub checked: Vec<&'static str>,
    /// Các vi phạm tìm thấy.
    pub violations: Vec<Violation>,
}

impl InvariantReport {
    /// Sạch hay không.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    /// Gộp hai báo cáo.
    pub fn merge(&mut self, other: InvariantReport) {
        self.checked.extend(other.checked);
        self.violations.extend(other.violations);
    }
}

impl core::fmt::Display for InvariantReport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_clean() {
            return write!(f, "{} bất biến, không vi phạm", self.checked.len());
        }
        writeln!(
            f,
            "{} bất biến, {} vi phạm:",
            self.checked.len(),
            self.violations.len()
        )?;
        for v in &self.violations {
            writeln!(f, "  {} — {}", v.id, v.detail)?;
        }
        Ok(())
    }
}

/// Một bất biến.
pub trait Invariant: Send + Sync + 'static {
    /// ID vĩnh viễn.
    fn id(&self) -> &'static str;
    /// Chi phí.
    fn cost(&self) -> Cost;
    /// Mô tả một dòng, hiện trong báo cáo và trong `mow-mcp`.
    fn describe(&self) -> &'static str;
    /// Kiểm tra. Đẩy vi phạm vào `out`.
    fn check(&self, sim: &Sim, out: &mut Vec<Violation>);
}

/// Bộ chạy.
pub struct InvariantRunner {
    items: Vec<Box<dyn Invariant>>,
    max_cost: Cost,
}

impl Default for InvariantRunner {
    fn default() -> Self {
        Self::standard(Cost::Medium)
    }
}

impl InvariantRunner {
    /// Bộ chạy rỗng.
    pub fn empty(max_cost: Cost) -> InvariantRunner {
        InvariantRunner {
            items: Vec::new(),
            max_cost,
        }
    }

    /// Bộ chạy với toàn bộ bất biến của engine, lọc theo mức chi phí.
    pub fn standard(max_cost: Cost) -> InvariantRunner {
        let mut r = InvariantRunner::empty(max_cost);
        r.add(Box::new(Inv1EntityCoSuKienSinh));
        r.add(Box::new(Inv11ToaDoLaSoNguyen));
        r.add(Box::new(Inv24NhuCauCoMocVaMien));
        r.add(Box::new(Inv33VatPhamMotNoi));
        r.add(Box::new(Inv3SapientCoHopDongNhanThuc));
        r
    }

    /// Thêm một bất biến.
    pub fn add(&mut self, inv: Box<dyn Invariant>) -> &mut Self {
        self.items.push(inv);
        self
    }

    /// Liệt kê, theo thứ tự ID.
    pub fn list(&self) -> Vec<(&'static str, Cost, &'static str)> {
        let mut v: Vec<_> = self
            .items
            .iter()
            .map(|i| (i.id(), i.cost(), i.describe()))
            .collect();
        v.sort_by_key(|x| x.0);
        v
    }

    /// Chạy.
    pub fn run(&self, sim: &Sim) -> InvariantReport {
        let mut rep = InvariantReport::default();
        for inv in &self.items {
            if inv.cost() > self.max_cost {
                continue;
            }
            rep.checked.push(inv.id());
            inv.check(sim, &mut rep.violations);
        }
        rep.checked.sort_unstable();
        rep.violations.sort_by(|a, b| a.id.cmp(b.id));
        rep
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Năm bất biến đầu tiên
// ─────────────────────────────────────────────────────────────────────────────

/// `INV-22-1` — mọi state change authoritative chỉ commit qua transaction handler.
///
/// Không kiểm được trực tiếp lúc chạy: "đã đi qua handler chưa" không phải một
/// thuộc tính của state. Nhưng nó có một **hệ quả quan sát được**: mọi thực thể
/// đang tồn tại phải để lại dấu vết trong nhật ký. Một thực thể xuất hiện mà
/// không có sự kiện sinh ra chỉ có thể tới từ một đường ghi lén.
struct Inv1EntityCoSuKienSinh;

impl Invariant for Inv1EntityCoSuKienSinh {
    fn id(&self) -> &'static str {
        "INV-22-1"
    }
    fn cost(&self) -> Cost {
        Cost::Medium
    }
    fn describe(&self) -> &'static str {
        "mọi thực thể đang tồn tại đều có sự kiện sinh ra trong nhật ký"
    }
    fn check(&self, sim: &Sim, out: &mut Vec<Violation>) {
        let mut da_sinh = BTreeSet::new();
        for ev in sim.log().iter() {
            if ev.kind.0.ends_with(".spawned") {
                if let Some(s) = ev.subject.or(ev.actor) {
                    da_sinh.insert(s);
                }
            }
        }
        for id in sim.store().ids() {
            if !da_sinh.contains(&id) {
                out.push(Violation {
                    id: "INV-22-1",
                    detail: format!(
                        "thực thể {id} tồn tại nhưng không có sự kiện `*.spawned` — \
                         có đường ghi state đi vòng qua transaction handler"
                    ),
                });
            }
        }
    }
}

/// `INV-22-11` — phép toán tọa độ dùng số nguyên có kiểm tra.
///
/// Hệ quả kiểm được: thuộc tính tọa độ phải là [`Value::Int`]. Nếu một content
/// pack hay một handler nhét vào đó một [`Value::Fixed`] hay một chuỗi, thì
/// đâu đó đã có phép quy đổi ngầm, và quy đổi ngầm ở tọa độ là đường ngắn nhất
/// tới lỗi lệch một ô mà không ai nhìn thấy.
struct Inv11ToaDoLaSoNguyen;

const KHOA_TOA_DO: &[&str] = &["core.pos.x", "core.pos.y", "core.pos.z"];

impl Invariant for Inv11ToaDoLaSoNguyen {
    fn id(&self) -> &'static str {
        "INV-22-11"
    }
    fn cost(&self) -> Cost {
        Cost::Cheap
    }
    fn describe(&self) -> &'static str {
        "mọi thuộc tính tọa độ là số nguyên i64, không phải fixed-point hay chuỗi"
    }
    fn check(&self, sim: &Sim, out: &mut Vec<Violation>) {
        for id in sim.store().ids() {
            for k in KHOA_TOA_DO {
                match sim.store().attr(id, k) {
                    None | Some(Value::Int(_)) => {}
                    Some(other) => out.push(Violation {
                        id: "INV-22-11",
                        detail: format!("{id}.{k} là `{}`, phải là int", other.type_name()),
                    }),
                }
            }
        }
    }
}

/// `INV-22-24` — nhu cầu không tick theo từng thực thể.
///
/// Giá trị nhu cầu phải suy ra bằng tích phân đóng từ `last_update_tick`, và
/// mốc đó phải kèm miền đồng hồ (`§4.5`). Thiếu mốc thì có ai đó đang chạy một
/// vòng lặp per-tick per-entity; thiếu miền thì cái đói sẽ nhảy sai khi thực
/// thể đi qua cổng.
struct Inv24NhuCauCoMocVaMien;

impl Invariant for Inv24NhuCauCoMocVaMien {
    fn id(&self) -> &'static str {
        "INV-22-24"
    }
    fn cost(&self) -> Cost {
        Cost::Cheap
    }
    fn describe(&self) -> &'static str {
        "thực thể có nhu cầu phải có `need.last_update_tick` và `need.clock_domain`"
    }
    fn check(&self, sim: &Sim, out: &mut Vec<Violation>) {
        for id in sim.store().ids() {
            let Some(attrs) = sim.store().attrs(id) else {
                continue;
            };
            let co_nhu_cau = attrs.keys().any(|k| {
                k.starts_with("need.") && k != "need.last_update_tick" && k != "need.clock_domain"
            });
            if !co_nhu_cau {
                continue;
            }
            if !attrs.contains_key("need.last_update_tick") {
                out.push(Violation {
                    id: "INV-22-24",
                    detail: format!("{id} có nhu cầu nhưng thiếu `need.last_update_tick`"),
                });
            }
            if !attrs.contains_key("need.clock_domain") {
                out.push(Violation {
                    id: "INV-22-24",
                    detail: format!("{id} có nhu cầu nhưng thiếu `need.clock_domain`"),
                });
            }
        }
    }
}

/// `INV-22-33` — vật phẩm nằm ở **đúng một** trong ba nơi.
///
/// Ô đất, vật chứa, hoặc túi đồ. Không phải hai, và không phải không nơi nào.
/// Vi phạm theo hướng "hai nơi" là nhân đôi của cải; theo hướng "không nơi
/// nào" là của cải bốc hơi. Cả hai đều phá kinh tế theo cách chỉ lộ ra sau
/// hàng nghìn giao dịch.
struct Inv33VatPhamMotNoi;

const KHOA_VI_TRI: &[&str] = &["loc.cell", "loc.container", "loc.inventory"];

impl Invariant for Inv33VatPhamMotNoi {
    fn id(&self) -> &'static str {
        "INV-22-33"
    }
    fn cost(&self) -> Cost {
        Cost::Medium
    }
    fn describe(&self) -> &'static str {
        "vật phẩm nằm ở đúng một trong ba nơi: ô đất, vật chứa, hoặc túi đồ"
    }
    fn check(&self, sim: &Sim, out: &mut Vec<Violation>) {
        for id in sim.store().ids() {
            let Some(attrs) = sim.store().attrs(id) else {
                continue;
            };
            if !attrs.contains_key("item.def") {
                continue;
            }
            let noi: Vec<&str> = KHOA_VI_TRI
                .iter()
                .copied()
                .filter(|k| attrs.contains_key(*k))
                .collect();
            if noi.len() != 1 {
                out.push(Violation {
                    id: "INV-22-33",
                    detail: if noi.is_empty() {
                        format!("vật phẩm {id} không nằm ở đâu cả")
                    } else {
                        format!("vật phẩm {id} nằm cùng lúc ở {noi:?}")
                    },
                });
            }
        }
    }
}

/// `INV-22-3` — thực thể `Sapient` phải có hợp đồng nhận thức đầy đủ.
///
/// Và mặt còn lại, thường bị quên: thực thể chỉ `Animate` **không được** có
/// memory namespace và không được chiếm ngân sách nhận thức. Quên vế này thì
/// đàn cừu sẽ lặng lẽ ăn hết ngân sách LLM của cả khu định cư.
struct Inv3SapientCoHopDongNhanThuc;

const TRUONG_NHAN_THUC: &[&str] = &[
    "cognition.persona_version",
    "cognition.memory_namespace",
    "cognition.branch_scope",
    "cognition.fallback",
];

impl Invariant for Inv3SapientCoHopDongNhanThuc {
    fn id(&self) -> &'static str {
        "INV-22-3"
    }
    fn cost(&self) -> Cost {
        Cost::Medium
    }
    fn describe(&self) -> &'static str {
        "Sapient có đủ hợp đồng nhận thức; Animate thuần không có memory namespace"
    }
    fn check(&self, sim: &Sim, out: &mut Vec<Violation>) {
        for id in sim.store().ids() {
            let Some(attrs) = sim.store().attrs(id) else {
                continue;
            };
            let sapient = attrs.contains_key("tag.sapient");

            if sapient {
                for t in TRUONG_NHAN_THUC {
                    if !attrs.contains_key(*t) {
                        out.push(Violation {
                            id: "INV-22-3",
                            detail: format!("{id} là Sapient nhưng thiếu `{t}`"),
                        });
                    }
                }
            } else if attrs.contains_key("cognition.memory_namespace") {
                out.push(Violation {
                    id: "INV-22-3",
                    detail: format!(
                        "{id} không phải Sapient nhưng có memory namespace — \
                         nó sẽ chiếm ngân sách nhận thức mà không bao giờ dùng tới"
                    ),
                });
            }
        }
    }
}
