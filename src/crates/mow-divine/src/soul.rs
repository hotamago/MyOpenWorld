//! Linh hồn, triệu hồi và thăng thần (`idea.md §14.3`, `§8.9.4`, `PE-13`).
//!
//! ## Thăng thần không xóa lịch sử cũ
//!
//! > Identity, ký ức, quan hệ và **lời hứa** được giữ theo luật chuyển đổi.
//! > Thăng thần **không xóa lịch sử cũ**.
//!
//! Câu cuối là bất biến của module. Cách sai thì tiện: thăng thần sinh ra một
//! entity mới với stat thần thánh, còn entity cũ bị đánh dấu đã chết. Làm thế
//! thì mọi món nợ, mọi mối thù, mọi lời thề của người đó biến mất — và cách rẻ
//! nhất để xù nợ trong thế giới này là **thành thần**.
//!
//! Nên [`Ascension`] giữ nguyên `EntityId`, và [`Ascension::carried_over`] liệt
//! kê đúng những gì đi theo. Một lời hứa đã ghi thì sau khi thăng thần vẫn là
//! một lời hứa đã ghi, chỉ là bây giờ người hứa mạnh hơn nhiều.
//!
//! ## Năm con đường thăng thần
//!
//! `§14.3` cho năm đường, và **bốn trong năm đi từ dưới lên**: tích lũy, kế
//! thừa, nghi lễ tập thể, tín ngưỡng. Chỉ đường thứ năm — True God ban trực
//! tiếp — là từ trên xuống. Tỉ lệ đó là cố ý: một pantheon mà mọi vị thần đều
//! do người chơi chỉ định là một pantheon không có lịch sử riêng.

use mow_core::{EntityId, EventSeq};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Chính sách linh hồn của một world (`§14`).
///
/// Đây là **dữ liệu của world**, không phải hằng số của engine: một world có
/// luân hồi và một world không có là hai world hợp lệ, và chúng phải cùng chạy
/// trên một bộ mã.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulPolicy {
    /// Chết rồi linh hồn còn tồn tại không.
    pub persists_after_death: bool,
    /// Có luân hồi không.
    pub reincarnates: bool,
    /// Neo vào vật được không (`§8.9.4`, `§13.6`).
    pub bindable_to_items: bool,
    /// Triệu hồi được không.
    pub summonable: bool,
    /// Ký ức có đi theo qua luân hồi không.
    ///
    /// Tách khỏi `reincarnates`: một world có luân hồi mà không giữ ký ức thì
    /// luân hồi không quan sát được từ bên trong, và đó là một thiết kế khác
    /// hẳn.
    pub memory_persists: bool,
}

impl SoulPolicy {
    /// Chính sách của một world không có siêu hình: chết là hết.
    pub fn materialist() -> SoulPolicy {
        SoulPolicy {
            persists_after_death: false,
            reincarnates: false,
            bindable_to_items: false,
            summonable: false,
            memory_persists: false,
        }
    }
}

/// Trạng thái của một linh hồn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoulState {
    /// Đang ở trong một cơ thể sống.
    Embodied {
        /// Cơ thể nào.
        body: EntityId,
    },
    /// Đã rời cơ thể, chưa đi đâu.
    Unbound,
    /// Bị neo vào một vật (`§8.9.4`).
    BoundToItem {
        /// Vật nào.
        item: u64,
    },
    /// Đã bị triệu hồi và đang chịu ràng buộc.
    Summoned {
        /// Người triệu.
        by: EntityId,
        /// Hết hạn ở tick nào.
        until: u64,
    },
    /// Đã luân hồi sang thân mới.
    Reincarnated {
        /// Thân mới.
        into: EntityId,
    },
    /// Đã tan.
    Dissolved,
}

/// Một linh hồn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Soul {
    /// Định danh — **giữ nguyên qua mọi lần chuyển trạng thái**.
    pub id: EntityId,
    /// Trạng thái hiện tại.
    pub state: SoulState,
    /// Lời hứa và lời thề chưa hoàn thành, trỏ vào event thật.
    ///
    /// Đây là thứ mà thăng thần và luân hồi **không được xóa**.
    pub unfulfilled_vows: Vec<EventSeq>,
}

/// Vì sao một thao tác linh hồn bị từ chối.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SoulError {
    /// World này không cho phép.
    #[error("chính sách linh hồn của world không cho phép `{what}`")]
    PolicyForbids {
        /// Thao tác nào.
        what: &'static str,
    },
    /// Linh hồn đang ở trạng thái không làm được việc này.
    #[error("linh hồn đang ở trạng thái không cho phép thao tác này")]
    WrongState,
}

impl Soul {
    /// Neo vào một vật — tạo ra một vật phẩm có tri giác (`§8.9.4`).
    pub fn bind_to_item(&mut self, item: u64, p: &SoulPolicy) -> Result<(), SoulError> {
        if !p.bindable_to_items {
            return Err(SoulError::PolicyForbids {
                what: "neo linh hồn vào vật",
            });
        }
        if !matches!(self.state, SoulState::Unbound) {
            return Err(SoulError::WrongState);
        }
        self.state = SoulState::BoundToItem { item };
        Ok(())
    }

    /// Triệu hồi.
    ///
    /// Có **hạn**: một triệu hồi vĩnh viễn là một cách sinh ra sức mạnh miễn
    /// phí, và nó cũng xóa mất phần thú vị nhất — cái giá của việc gia hạn.
    pub fn summon(&mut self, by: EntityId, until: u64, p: &SoulPolicy) -> Result<(), SoulError> {
        if !p.summonable {
            return Err(SoulError::PolicyForbids {
                what: "triệu hồi"
            });
        }
        if !matches!(
            self.state,
            SoulState::Unbound | SoulState::BoundToItem { .. }
        ) {
            return Err(SoulError::WrongState);
        }
        self.state = SoulState::Summoned { by, until };
        Ok(())
    }

    /// Luân hồi sang thân mới.
    ///
    /// **Lời thề đi theo.** Một world có luân hồi mà nợ được xóa là một world
    /// mà cách trả nợ rẻ nhất là chết.
    pub fn reincarnate(&mut self, into: EntityId, p: &SoulPolicy) -> Result<(), SoulError> {
        if !p.reincarnates {
            return Err(SoulError::PolicyForbids {
                what: "luân hồi"
            });
        }
        self.state = SoulState::Reincarnated { into };
        Ok(())
    }
}

/// Năm con đường thăng thần (`§14.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AscensionPath {
    /// Tích lũy / biến đổi soul và body.
    SoulCultivation,
    /// Kế thừa domain từ thần cũ.
    InheritedDomain,
    /// Hoàn thành ritual/project tập thể.
    CollectiveRitual,
    /// Được tín ngưỡng đủ lớn neo giữ.
    AnchoredByFaith,
    /// True God trực tiếp ban capability.
    ///
    /// Đường **duy nhất** đi từ trên xuống.
    DivineGrant,
}

impl AscensionPath {
    /// Đường này đi từ dưới lên không.
    pub fn bottom_up(self) -> bool {
        !matches!(self, AscensionPath::DivineGrant)
    }
}

/// Những gì đi theo khi thăng thần (`§14.3`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarriedOver {
    /// `EntityId` cũ — **không đổi**.
    pub identity: EntityId,
    /// Số event ký ức giữ lại.
    pub memories: usize,
    /// Quan hệ giữ lại.
    pub relationships: usize,
    /// Lời hứa chưa hoàn thành — **không xóa được**.
    pub vows: Vec<EventSeq>,
}

/// Một lần thăng thần.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ascension {
    /// Ai thăng.
    pub who: EntityId,
    /// Qua đường nào — có thể nhiều đường cùng lúc.
    pub paths: Vec<AscensionPath>,
    /// Event nào đánh dấu.
    pub at: EventSeq,
    /// Những gì mang theo.
    pub carried: CarriedOver,
}

impl Ascension {
    /// Dựng một lần thăng thần, giữ nguyên mọi thứ phải giữ.
    ///
    /// Không có tham số nào cho phép bỏ lời thề. Đó là cách bất biến này được
    /// giữ: không phải bằng một kiểm tra, mà bằng việc không có đường để vi
    /// phạm.
    pub fn new(
        soul: &Soul,
        paths: Vec<AscensionPath>,
        at: EventSeq,
        memories: usize,
        relationships: usize,
    ) -> Ascension {
        Ascension {
            who: soul.id,
            paths,
            at,
            carried: CarriedOver {
                identity: soul.id,
                memories,
                relationships,
                vows: soul.unfulfilled_vows.clone(),
            },
        }
    }

    /// Lịch sử cũ có bị xóa không.
    ///
    /// **Không bao giờ.** Hàm này tồn tại để một test khẳng định được điều đó,
    /// và để chỗ nào đó viết `if !a.erases_history()` thì đọc ra ngay là thừa.
    pub fn erases_history(&self) -> bool {
        false
    }

    /// Thần này có tự lên không, hay do True God chỉ định.
    pub fn self_made(&self) -> bool {
        self.paths.iter().any(|p| p.bottom_up())
    }
}
