//! Sổ đăng ký hành động và điều kiện tiên quyết (`idea.md §10.5`, `§22.5`).
//!
//! > Action registry **tự kiểm tra precondition authoritative**; assertion do
//! > LLM hoặc YAML khai báo **không thay thế** state check.
//!
//! ## Vì sao lời khai của LLM không đủ
//!
//! Một mô hình sẽ nói *"tôi lấy ổ bánh từ trong túi"* một cách thuyết phục cả
//! khi túi trống. Nó không nói dối — nó không có cách nào biết. Nếu engine tin
//! lời khai đó, thì bánh mì xuất hiện từ hư không, và kinh tế của thế giới hỏng
//! theo cách chậm và khó truy.
//!
//! Cùng lý do với YAML: một content pack khai báo `requires: [has_bread]` đang
//! mô tả *ý định*, không phải kiểm tra *state*. Engine phải tự nhìn vào túi.
//!
//! Nên [`Precondition`] là một **hàm chạy trên state**, không phải một chuỗi
//! khai báo. Không có cách nào để một chuỗi trong YAML trở thành một cái gật
//! đầu.

use crate::consent::{ConsentCapacity, IntimacyRegistry};
use crate::timeline::PhaseDurations;
use mow_core::{Ctx, EntityId, Failure, FailureCode};
use std::collections::BTreeMap;

/// Kết quả kiểm một điều kiện.
pub type PreconditionResult = Result<(), Failure>;

/// Một điều kiện tiên quyết — **một hàm chạy trên state**.
pub type Precondition = fn(&Ctx<'_>, EntityId) -> PreconditionResult;

/// Định nghĩa một hành động.
pub struct ActionDef {
    /// Định danh có namespace.
    pub id: String,
    /// Tầng giải quyết khi tranh chấp.
    pub tier: crate::resolve::Tier,
    /// Thời lượng ba pha ở tốc độ chuẩn.
    pub durations: PhaseDurations,
    /// Điều kiện tiên quyết, chạy theo thứ tự.
    pub preconditions: Vec<Precondition>,
    /// Có chịu ràng buộc ưng thuận không (`§22.26`).
    pub requires_consent: bool,
}

impl core::fmt::Debug for ActionDef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ActionDef")
            .field("id", &self.id)
            .field("tier", &self.tier)
            .field("preconditions", &self.preconditions.len())
            .field("requires_consent", &self.requires_consent)
            .finish_non_exhaustive()
    }
}

/// Sổ đăng ký hành động.
#[derive(Default)]
pub struct ActionRegistry {
    actions: BTreeMap<String, ActionDef>,
    intimacy: IntimacyRegistry,
}

impl ActionRegistry {
    /// Sổ với ràng buộc ưng thuận chuẩn đã nạp sẵn.
    pub fn new() -> ActionRegistry {
        ActionRegistry {
            actions: BTreeMap::new(),
            intimacy: IntimacyRegistry::standard(),
        }
    }

    /// Đăng ký một hành động.
    ///
    /// Cờ `requires_consent` được **ép bật** cho mọi loại nằm trong
    /// [`IntimacyRegistry`], bất kể content pack khai báo gì. Đây là chỗ
    /// `§22.26` "không plugin nào cấp được ngoại lệ" được thi hành: một pack có
    /// thể quên bật cờ, hoặc cố ý tắt nó, và cả hai đều không có tác dụng.
    pub fn register(&mut self, mut def: ActionDef) {
        if self.intimacy.requires_consent(&def.id) {
            def.requires_consent = true;
        }
        self.actions.insert(def.id.clone(), def);
    }

    /// Mở rộng diện chịu ràng buộc ưng thuận.
    ///
    /// Áp ngược lại cho những hành động đã đăng ký — nếu không, thứ tự đăng ký
    /// sẽ quyết định một hành động có được bảo vệ hay không.
    pub fn require_consent_for(&mut self, kind: &str) {
        self.intimacy.require_consent(kind);
        if let Some(a) = self.actions.get_mut(kind) {
            a.requires_consent = true;
        }
    }

    /// Tra một hành động.
    pub fn get(&self, id: &str) -> Option<&ActionDef> {
        self.actions.get(id)
    }

    /// Mọi id đã đăng ký, theo thứ tự.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.actions.keys().map(String::as_str)
    }

    /// Số hành động.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Kiểm mọi điều kiện của một hành động, **từ state**.
    ///
    /// Thứ tự kiểm là một quyết định: ưng thuận trước, rồi mới tới điều kiện
    /// khác. Nhờ vậy log kiểm toán ghi `consent_violation` chứ không ghi
    /// `insufficient` cho cùng một sự việc — và hai mã đó dẫn tới hai phản ứng
    /// hoàn toàn khác nhau khi đọc lại.
    pub fn validate(
        &self,
        ctx: &Ctx<'_>,
        action_id: &str,
        actor: EntityId,
        consent_parties: &[(EntityId, ConsentCapacity)],
    ) -> PreconditionResult {
        let def = self.actions.get(action_id).ok_or_else(|| {
            Failure::new(
                FailureCode::UnknownCommand,
                format!("không có hành động `{action_id}`"),
            )
        })?;

        // Thực thể có biết hành động này không (`§22.4`).
        if ctx
            .store
            .attr(actor, &format!("knows.{action_id}"))
            .is_none()
        {
            return Err(Failure::new(
                FailureCode::ActionNotKnown,
                format!("{actor} không biết `{action_id}`"),
            ));
        }

        if def.requires_consent {
            crate::consent::validate(consent_parties).map_err(|ds| {
                Failure::new(
                    FailureCode::ConsentViolation,
                    ds.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            })?;
        }

        for p in &def.preconditions {
            p(ctx, actor)?;
        }
        Ok(())
    }
}
