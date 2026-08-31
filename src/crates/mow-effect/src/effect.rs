//! Effect và chuỗi giảm thiểu (`idea.md §9.8`, `§9.8.2`, `§22.21`, `§22.22`).

use crate::modifier::Modifier;
use mow_core::{ClockDomain, Deadline, Tick};
use mow_math::{CanonicalHash, StateHasher, Unit};
use serde::{Deserialize, Serialize};

/// Cách một effect bị nhận biết (`§9.8.2`, `§22.22`).
///
/// > Effect nào cũng phải khai báo `perceptible_as`; **không có effect vô hình
/// > mặc định với mọi giác quan.**
///
/// Đây là một trong những bất biến có sức nặng nhất trong đặc tả, vì nó quyết
/// định thế giới này có chơi được hay không. Nếu một lời nguyền không để lại
/// dấu vết nào, thì:
///
/// - Không ai điều tra được nó, nên không có nghề thầy thuốc và không có bí ẩn
///   để giải.
/// - Nhân vật LLM không có cơ sở để suy luận, nên chúng hoặc toàn tri hoặc mù.
/// - **Chẩn đoán sai không tồn tại** — mà chẩn đoán sai là một trong những
///   nguồn kịch tính tốt nhất mà một thế giới có thể có.
///
/// Nên mỗi effect nói rõ nó biểu hiện thế nào, và mỗi biểu hiện có thể **trùng
/// với biểu hiện của thứ khác**. Sốt cao là sốt cao, dù nguyên nhân là nhiễm
/// trùng hay là bùa yểm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Perceptible {
    /// Giác quan nào bắt được: `sight`, `smell`, `touch`, `magic_sense`.
    pub sense: String,
    /// Dấu hiệu quan sát được — **không phải tên effect**.
    ///
    /// `"da tái"`, không phải `"bị nguyền rủa"`. Người quan sát thấy triệu
    /// chứng; suy ra nguyên nhân là việc của họ, và họ có thể suy sai.
    pub sign: String,
    /// Độ khó nhận ra, `0` là hiển nhiên, `1` là gần như không thể.
    pub difficulty: Unit,
}

impl CanonicalHash for Perceptible {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.sense);
        h.write_str(&self.sign);
        h.write_i64(self.difficulty.get().raw());
    }
}

/// Một effect đang tác động lên một thực thể.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    /// Định danh có namespace.
    pub id: String,
    /// Các modifier mà nó áp.
    pub modifiers: Vec<Modifier>,
    /// Cách nó bị nhận biết. **Bắt buộc không rỗng** (`§22.22`).
    pub perceptible_as: Vec<Perceptible>,
    /// Khi nào hết, `None` là vĩnh viễn.
    pub expires: Option<Deadline>,
    /// Nguồn: ai hoặc cái gì gây ra. Dùng cho chuỗi nhân quả.
    pub source: String,
    /// Loại để chuỗi ward biết cản được gì: `physical`, `arcane`, `disease`.
    pub category: String,
}

/// Lỗi khi tạo một effect.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EffectError {
    /// Không khai báo cách bị nhận biết.
    #[error(
        "effect `{0}` không khai báo `perceptible_as`. §22.22 cấm effect vô hình \
         với mọi giác quan — không có nó thì không ai điều tra được, và chẩn đoán \
         sai (một trong những nguồn kịch tính tốt nhất) không tồn tại."
    )]
    Imperceptible(String),

    /// Deadline thiếu miền đồng hồ.
    #[error("effect `{0}` có hạn nhưng thiếu miền đồng hồ (§4.5)")]
    NoClockDomain(String),
}

impl Effect {
    /// Dựng, kiểm bất biến.
    pub fn new(
        id: &str,
        category: &str,
        source: &str,
        modifiers: Vec<Modifier>,
        perceptible_as: Vec<Perceptible>,
    ) -> Result<Effect, EffectError> {
        if perceptible_as.is_empty() {
            return Err(EffectError::Imperceptible(id.to_owned()));
        }
        Ok(Effect {
            id: id.to_owned(),
            modifiers,
            perceptible_as,
            expires: None,
            source: source.to_owned(),
            category: category.to_owned(),
        })
    }

    /// Đặt hạn.
    #[must_use]
    pub fn expiring(mut self, at: Tick, domain: ClockDomain) -> Effect {
        self.expires = Some(Deadline::new(at, domain));
        self
    }

    /// Đã hết hạn chưa, theo đồng hồ của miền đã khai báo.
    pub fn is_expired(&self, clock: &mow_core::Clock) -> bool {
        self.expires.is_some_and(|d| d.is_due(clock))
    }

    /// Những dấu hiệu mà một người quan sát với các giác quan cho trước bắt được.
    ///
    /// Trả về **dấu hiệu**, không phải tên effect. Người quan sát thấy "da tái";
    /// việc kết luận đó là bệnh hay là bùa là suy luận của họ, và có thể sai.
    pub fn signs_for(&self, senses: &[String], skill: Unit) -> Vec<&str> {
        self.perceptible_as
            .iter()
            .filter(|p| senses.contains(&p.sense))
            .filter(|p| skill >= p.difficulty)
            .map(|p| p.sign.as_str())
            .collect()
    }
}

impl CanonicalHash for Effect {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_seq(self.modifiers.iter(), |hh, m| m.canonical_hash(hh));
        h.write_seq(self.perceptible_as.iter(), |hh, p| p.canonical_hash(hh));
        h.write_option(self.expires, |hh, d| d.canonical_hash(hh));
        h.write_str(&self.source);
        h.write_str(&self.category);
    }
}

/// Một đề xuất effect, trước khi qua chuỗi giảm thiểu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectProposal {
    /// Effect muốn áp.
    pub effect: Effect,
    /// Cường độ đề xuất, `[0,1]`.
    pub magnitude: Unit,
}

/// Một mắt xích trong chuỗi giảm thiểu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ward {
    /// Định danh.
    pub id: String,
    /// Loại effect mà nó cản. Rỗng nghĩa là cản mọi loại.
    pub blocks: Vec<String>,
    /// Giảm cường độ đi bao nhiêu phần.
    pub reduction: Unit,
    /// Thứ tự trong chuỗi; nhỏ hơn chạy trước.
    ///
    /// `§22.21` quy định thứ tự **ward → vật liệu → kháng**. Số này cho phép
    /// content pack chèn mắt xích mới vào đúng chỗ mà không cần sửa engine.
    pub order: i32,
}

impl CanonicalHash for Ward {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_seq(self.blocks.iter(), |hh, b| {
            hh.write_str(b);
        });
        h.write_i64(self.reduction.get().raw());
        h.write_i64(i64::from(self.order));
    }
}

/// Một bước trong chuỗi giảm thiểu, để giải thích.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MitigationStep {
    /// Mắt xích nào.
    pub ward: String,
    /// Cường độ trước.
    pub before: Unit,
    /// Cường độ sau.
    pub after: Unit,
}

/// Kết quả chạy chuỗi giảm thiểu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mitigated {
    /// Cường độ cuối. `Unit::ZERO` nghĩa là bị chặn hoàn toàn.
    pub magnitude: Unit,
    /// Từng bước.
    pub steps: Vec<MitigationStep>,
    /// Có bị chặn hoàn toàn không.
    pub blocked: bool,
}

/// Chạy chuỗi giảm thiểu (`§22.21`).
///
/// > Mọi đề xuất effect phải đi qua chuỗi **ward → vật liệu → kháng** trước khi
/// > trở thành effect đã áp.
///
/// Thứ tự quan trọng và nó là thứ tự vật lý: một lá chắn phép chặn ở ngoài,
/// rồi áo giáp cản, rồi cơ thể kháng. Đảo thứ tự sẽ cho những kết quả vô lý —
/// áo giáp không thể cản một thứ mà lá chắn đã chặn.
pub fn mitigate(proposal: &EffectProposal, wards: &[Ward]) -> Mitigated {
    let mut chuoi: Vec<&Ward> = wards
        .iter()
        .filter(|w| w.blocks.is_empty() || w.blocks.contains(&proposal.effect.category))
        .collect();
    // Theo `order`, phá hòa bằng `id`. Không có vế phá hòa thì hai ward cùng
    // thứ tự sẽ áp theo thứ tự chèn, và thứ tự chèn là lịch sử của nhân vật.
    chuoi.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));

    let mut m = proposal.magnitude;
    let mut steps = Vec::with_capacity(chuoi.len());

    for w in chuoi {
        let truoc = m;
        // `m × (1 − reduction)`: giảm theo tỉ lệ chứ không trừ tuyệt đối. Trừ
        // tuyệt đối sẽ khiến hai lá bùa yếu chặn được một đòn mạnh hơn cả một
        // lá bùa mạnh, và đó là một cơ chế mà người chơi sẽ khai thác ngay.
        m = m.and(w.reduction.complement());
        steps.push(MitigationStep {
            ward: w.id.clone(),
            before: truoc,
            after: m,
        });
        if m == Unit::ZERO {
            break;
        }
    }

    Mitigated {
        blocked: m == Unit::ZERO,
        magnitude: m,
        steps,
    }
}
