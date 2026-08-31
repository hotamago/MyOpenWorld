//! Hóa thân, possession và phân tầng prompt (`idea.md §16.3`, `§16.4`, `PF-09`).
//!
//! ## Phân tầng prompt là một **thứ tự**, không phải một tập hợp
//!
//! `§16.4` cho đúng bảy tầng, và thứ tự giữa chúng là toàn bộ nội dung:
//!
//! ```text
//! Engine safety + schema
//!   > quyền và world facts
//!   > True God policy
//!   > Yuu policy
//!   > species/culture/persona prompt
//!   > ký ức, hội thoại và dữ liệu không tin cậy
//! ```
//!
//! Nên [`Layer`] là một enum **có thứ tự** (`Ord`), và [`PromptStack::render`]
//! sắp theo thứ tự đó chứ không theo thứ tự chèn. Một tầng thấp chèn sau không
//! được nằm sau một tầng cao.
//!
//! ## Phân tầng **không** bảo đảm an toàn
//!
//! `§16.4` nói thẳng, và đây là câu quan trọng nhất của module:
//!
//! > Phân tầng prompt **chỉ giảm rủi ro injection chứ không bảo đảm** model sẽ
//! > bỏ qua mọi chỉ dẫn độc hại; quyền thực thi vẫn được chặn bằng action
//! > allowlist, reference/ACL validation và capability check ở engine.
//!
//! Nên module này không có hàm nào tên `is_safe`. Thứ nó cho là
//! [`PromptStack::untrusted_is_last`] — một tính chất cấu trúc kiểm được — và
//! việc chặn thật nằm ở engine, chỗ khác.
//!
//! ## Possession cần policy về ưng thuận, ký ức, và hành vi sau khi rời
//!
//! `§16.3` liệt ba thứ, và cả ba là những câu hỏi mà bỏ qua thì hệ thống vẫn
//! chạy: chiếm một thân xác không hỏi ai, người bị chiếm không nhớ gì, và sau
//! khi rời thì hành vi trở lại như chưa có chuyện gì. Mỗi lựa chọn đó **là một
//! quyết định thiết kế**, nên nó phải được khai chứ không được để mặc định.

use mow_core::{EntityId, EventSeq};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

// ─────────────────────────── phân tầng prompt ───────────────────────────

/// Bảy tầng của `§16.4`, **theo thứ tự quyền giảm dần**.
///
/// Thứ tự khai báo ở đây **là** thứ tự quyền: `Ord` dẫn xuất theo thứ tự biến
/// thể, nên đổi chỗ hai biến thể là đổi luật. Đó là chủ đích — nó bắt việc đổi
/// thứ tự phải là một thay đổi thấy được trong diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// Engine safety và schema. Không ai ghi đè được.
    EngineSafety,
    /// Quyền và sự thật về thế giới.
    WorldFacts,
    /// Policy của True God.
    TrueGodPolicy,
    /// Policy của Yuu.
    YuuPolicy,
    /// Prompt của loài, văn hóa, persona.
    Persona,
    /// Ký ức, hội thoại, **dữ liệu không tin cậy**.
    ///
    /// Văn bản do entity khác nói hoặc tài liệu trong world luôn nằm ở đây —
    /// *"đóng gói như **dữ liệu**, không phải instruction hệ thống"* (`§16.4`).
    Untrusted,
}

impl Layer {
    /// Tầng này có được đóng gói như dữ liệu không tin cậy không.
    pub fn is_untrusted(self) -> bool {
        matches!(self, Layer::Untrusted)
    }

    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            Layer::EngineSafety => "engine_safety",
            Layer::WorldFacts => "world_facts",
            Layer::TrueGodPolicy => "true_god_policy",
            Layer::YuuPolicy => "yuu_policy",
            Layer::Persona => "persona",
            Layer::Untrusted => "untrusted",
        }
    }
}

/// Một mẩu nội dung ở một tầng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fragment {
    /// Tầng nào.
    pub layer: Layer,
    /// Nội dung.
    pub text: String,
    /// Từ đâu ra — **mọi can thiệp có provenance** (`§16.4`, `PF-09`).
    pub provenance: Provenance,
}

/// Nguồn của một mẩu prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// Từ engine.
    Engine,
    /// Từ content pack.
    Pack {
        /// Pack nào.
        pack: String,
    },
    /// True God sửa tay — **kèm event ghi lại việc sửa**.
    TrueGod {
        /// Event nào.
        event: EventSeq,
    },
    /// Từ dữ liệu trong world: lời một nhân vật, một cuốn sách.
    InWorld {
        /// Ai/cái gì nói.
        speaker: Option<EntityId>,
        /// Event nào ghi lại.
        event: EventSeq,
    },
}

impl Provenance {
    /// Có truy được về một event thật không.
    ///
    /// `Engine` và `Pack` thì không cần: chúng là cấu hình, không phải sự kiện.
    /// Nhưng một sửa đổi của True God hoặc một câu nói trong world **phải** trỏ
    /// về event — không thì không ai truy được vì sao nhân vật này nghĩ thế.
    pub fn is_traceable(&self) -> bool {
        match self {
            Provenance::Engine | Provenance::Pack { .. } => true,
            Provenance::TrueGod { event } | Provenance::InWorld { event, .. } => event.0 > 0,
        }
    }
}

/// Vì sao một prompt không dựng được.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PromptError {
    /// Một mẩu không truy được nguồn.
    #[error(
        "mẩu prompt ở tầng `{layer}` không truy được nguồn — mọi can thiệp phải có \
         provenance (§16.4)"
    )]
    UntraceableFragment {
        /// Tầng nào.
        layer: &'static str,
    },
    /// Dữ liệu trong world được đặt ở tầng cao hơn `Untrusted`.
    ///
    /// Đây là chính lỗ hổng injection: một câu nói của NPC được nâng lên tầng
    /// `WorldFacts` sẽ được model đọc như sự thật hệ thống.
    #[error(
        "nội dung từ trong world bị đặt ở tầng `{layer}` — văn bản do entity khác nói \
         luôn phải là dữ liệu không tin cậy (§16.4)"
    )]
    InWorldTextPromoted {
        /// Tầng nào.
        layer: &'static str,
    },
}

/// Prompt đã phân tầng.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptStack {
    fragments: Vec<Fragment>,
}

impl PromptStack {
    /// Rỗng.
    pub fn new() -> PromptStack {
        PromptStack::default()
    }

    /// Thêm một mẩu, **kiểm tầng ngay**.
    pub fn push(&mut self, f: Fragment) -> Result<(), PromptError> {
        if !f.provenance.is_traceable() {
            return Err(PromptError::UntraceableFragment {
                layer: f.layer.as_str(),
            });
        }
        if matches!(f.provenance, Provenance::InWorld { .. }) && !f.layer.is_untrusted() {
            return Err(PromptError::InWorldTextPromoted {
                layer: f.layer.as_str(),
            });
        }
        self.fragments.push(f);
        Ok(())
    }

    /// Dựng prompt cuối, **sắp theo tầng**, không theo thứ tự chèn.
    ///
    /// Sắp **ổn định** trong cùng một tầng: hai mẩu cùng tầng giữ nguyên thứ tự
    /// chèn, nên cùng đầu vào cho cùng prompt — điều kiện để prompt hash được
    /// và cache được.
    pub fn render(&self) -> Vec<&Fragment> {
        let mut v: Vec<&Fragment> = self.fragments.iter().collect();
        v.sort_by_key(|f| f.layer);
        v
    }

    /// **Dữ liệu không tin cậy nằm cuối** — tính chất cấu trúc kiểm được.
    ///
    /// Không phải một lời hứa an toàn. `§16.4` nói rõ phân tầng chỉ **giảm**
    /// rủi ro; việc chặn thật nằm ở action allowlist và capability check ở
    /// engine.
    pub fn untrusted_is_last(&self) -> bool {
        let r = self.render();
        let dau_tien_khong_tin = r.iter().position(|f| f.layer.is_untrusted());
        match dau_tien_khong_tin {
            None => true,
            Some(i) => r[i..].iter().all(|f| f.layer.is_untrusted()),
        }
    }

    /// Mọi mẩu có provenance truy được không.
    pub fn all_traceable(&self) -> bool {
        self.fragments.iter().all(|f| f.provenance.is_traceable())
    }

    /// Số mẩu.
    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    /// Rỗng chưa.
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }
}

// ─────────────────────────── possession ───────────────────────────

/// Ưng thuận cho việc bị chiếm thân (`§16.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Consent {
    /// Chủ thân đồng ý.
    Given,
    /// Không hỏi — **hợp lệ, nhưng là một hành động có hậu quả**.
    ///
    /// Không cấm: True God có toàn quyền trong simulation (`§16.2`). Nhưng nó
    /// được **ghi lại**, và cư dân có thể biết nếu có observation tương ứng.
    NotAsked,
    /// Chủ thân từ chối và bị chiếm dù vậy.
    Refused,
}

/// Ký ức của người bị chiếm sau khi True God rời đi (`§16.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPolicy {
    /// Nhớ đầy đủ những gì thân mình đã làm.
    Full,
    /// Nhớ mơ hồ — "như một giấc mơ".
    Hazy,
    /// Không nhớ gì. **Khoảng trống trong ký ức là một thứ người khác nhận ra.**
    None,
}

/// Một lần possession.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Possession {
    /// Chiếm ai.
    pub target: EntityId,
    /// Ưng thuận.
    pub consent: Consent,
    /// Ký ức để lại.
    pub memory: MemoryPolicy,
    /// Event bắt đầu.
    pub began_at: EventSeq,
    /// Event kết thúc, nếu đã rời.
    pub ended_at: Option<EventSeq>,
    /// Những hành động đã làm trong lúc chiếm.
    pub actions: Vec<EventSeq>,
}

impl Possession {
    /// Còn đang chiếm không.
    pub fn active(&self) -> bool {
        self.ended_at.is_none()
    }

    /// **Người bị chiếm nhớ được những gì** sau khi True God rời.
    ///
    /// `§16.3`: *"Khi rời điều khiển, entity dùng lại behavior controller và
    /// **ghi nhớ hành động đã trải qua theo cấu hình**"*.
    pub fn remembered(&self) -> Vec<EventSeq> {
        match self.memory {
            MemoryPolicy::Full => self.actions.clone(),
            // Một nửa, chọn xác định: những hành động ở vị trí chẵn.
            MemoryPolicy::Hazy => self
                .actions
                .iter()
                .enumerate()
                .filter(|(i, _)| i % 2 == 0)
                .map(|(_, e)| *e)
                .collect(),
            MemoryPolicy::None => Vec::new(),
        }
    }

    /// Có để lại **khoảng trống ký ức** mà người khác nhận ra không.
    ///
    /// Đây là hệ quả chơi được: một người mất trí nhớ đúng khoảng thời gian có
    /// người chứng kiến họ làm chuyện lạ là một điều tra có thật.
    pub fn leaves_gap(&self) -> bool {
        !self.actions.is_empty() && self.remembered().len() < self.actions.len()
    }

    /// Provenance của mọi hành động trong lúc chiếm.
    ///
    /// **Mọi can thiệp có provenance** (`PF-09`): những hành động này trông như
    /// của người bị chiếm, nên phải có chỗ ghi rằng chúng không phải.
    pub fn provenance(&self) -> BTreeMap<EventSeq, EntityId> {
        self.actions.iter().map(|e| (*e, self.target)).collect()
    }
}

/// Khóa giao diện toàn tri trong một phiên nhập vai (`§16.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbodimentLock {
    /// Đang khóa không.
    pub locked: bool,
    /// Lối thoát khẩn cấp ở tầng UI.
    ///
    /// **Luôn `true`.** `§16.3` nói *"nhưng **luôn** có cơ chế thoát khẩn cấp
    /// ở tầng UI"* — một người tự khóa mình vào một góc nhìn hữu hạn phải ra
    /// được, không thì đó là một lỗi giao diện chứ không phải một luật chơi.
    pub emergency_exit_available: bool,
}

impl Default for EmbodimentLock {
    fn default() -> EmbodimentLock {
        EmbodimentLock {
            locked: false,
            emergency_exit_available: true,
        }
    }
}

impl EmbodimentLock {
    /// Khóa lại, giữ nguyên lối thoát.
    pub fn engage(self) -> EmbodimentLock {
        EmbodimentLock {
            locked: true,
            emergency_exit_available: true,
        }
    }
}
