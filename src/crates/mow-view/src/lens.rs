//! Ống kính: ai đang xem, ở chế độ nào (`idea.md §18.9`, `PC-15`).

use mow_action::perception::CognitionContext;
use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Chế độ nhận thức.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Hóa thân: chỉ những gì avatar quan sát được hoặc tin.
    Embodied,
    /// Quan sát: sự thật của vùng đang xem, có nhãn rõ ràng.
    Observer,
    /// True God: mọi thứ, cộng provenance.
    TrueGod,
}

impl Mode {
    /// Tên ổn định trên dây.
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Embodied => "embodied",
            Mode::Observer => "observer",
            Mode::TrueGod => "true_god",
        }
    }

    /// Chế độ này có thấy được provenance không.
    pub fn sees_provenance(self) -> bool {
        matches!(self, Mode::TrueGod)
    }
}

/// Ống kính mà một phiên xem đang dùng.
///
/// ## Vì sao `Embodied` mang theo cả `CognitionContext`
///
/// Vì "avatar biết gì" **không** suy ra được từ `EntityId` cộng một khoảng
/// cách. Nó là kết quả của tri giác: sương mù, cải trang, ánh sáng, tiếng động.
/// [`CognitionContext`] đã là kiểu chứa đúng tập đó (`§22.4`), nên dùng lại nó
/// giữ cho hai đường — cái mà NPC dùng để nghĩ, và cái mà người chơi được nhìn —
/// **không thể lệch nhau**.
///
/// Nếu tách làm hai nguồn, chúng sẽ lệch, và bug sẽ có dạng: người chơi hóa thân
/// nhìn thấy một thứ mà chính nhân vật của họ không thể lập kế hoạch dựa vào.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lens {
    mode: Mode,
    /// Chỉ có ở chế độ hóa thân.
    ctx: Option<CognitionContext>,
}

impl Lens {
    /// Ống kính hóa thân.
    pub fn embodied(ctx: CognitionContext) -> Lens {
        Lens {
            mode: Mode::Embodied,
            ctx: Some(ctx),
        }
    }

    /// Ống kính quan sát.
    pub fn observer() -> Lens {
        Lens {
            mode: Mode::Observer,
            ctx: None,
        }
    }

    /// Ống kính True God.
    pub fn true_god() -> Lens {
        Lens {
            mode: Mode::TrueGod,
            ctx: None,
        }
    }

    /// Chế độ.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Người đang hóa thân, nếu có.
    pub fn viewer(&self) -> Option<EntityId> {
        self.ctx.as_ref().map(|c| c.self_id)
    }

    /// Ngữ cảnh nhận thức, nếu có.
    pub fn context(&self) -> Option<&CognitionContext> {
        self.ctx.as_ref()
    }

    /// Ống kính này có **nhận ra** thực thể `e` không.
    ///
    /// Ở chế độ hóa thân: chỉ khi có một quan sát mang danh tính đó, hoặc `e`
    /// chính là mình. Một bóng người trong sương **không** làm hàm này trả
    /// `true`, và đó là toàn bộ điểm mấu chốt.
    pub fn identifies(&self, e: EntityId) -> bool {
        match self.mode {
            Mode::Observer | Mode::TrueGod => true,
            Mode::Embodied => self.ctx.as_ref().is_some_and(|c| {
                c.self_id == e || c.observations.iter().any(|o| o.identity == Some(e))
            }),
        }
    }

    /// Số bóng người mà ống kính thấy nhưng **không nhận ra**.
    ///
    /// Chúng cố tình **không** trả về `EntityId`. Trả về id sẽ là chính cái rò
    /// rỉ mà `§18.9` cấm: người chơi mở devtool, đọc id, và biết bóng người
    /// trong sương là ai — trong khi nhân vật của họ thì không.
    ///
    /// Chúng đi ra dây dưới dạng [`crate::project::PresenceView`], một kiểu
    /// riêng không có chỗ nào để nhét danh tính vào.
    pub fn anonymous_sightings(&self) -> usize {
        match self.mode {
            Mode::Observer | Mode::TrueGod => 0,
            Mode::Embodied => self.ctx.as_ref().map_or(0, |c| {
                c.observations
                    .iter()
                    .filter(|o| o.identity.is_none())
                    .count()
            }),
        }
    }
}
