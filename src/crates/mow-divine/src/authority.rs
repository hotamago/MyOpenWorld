//! Ba loại "thần" và **domain authority** (`idea.md §14.1`, `§14.2`, `§22.1`,
//! `PE-13`).
//!
//! ## Câu quyết định của `§14.2`
//!
//! > Một thần bão **không trực tiếp đặt `city.destroyed = true`**.
//!
//! Đây là `INV-22-1` áp cho thần: một state change authoritative chỉ commit qua
//! simulation handler. Thần không phải ngoại lệ — thần chỉ là một entity rất
//! mạnh **vẫn nằm trong law**.
//!
//! Thần được làm gì thì `§14.2` liệt kê hết, và chú ý mọi mục đều là *tác động
//! vào một trường*, không phải *đặt một kết quả*:
//!
//! 1. Tích lũy divine energy / tín ngưỡng.
//! 2. Chọn vùng mà mình có **liên kết**.
//! 3. Tạo hoặc khuếch đại **trường thời tiết**, trong giới hạn domain.
//! 4. Đối đầu counter-domain của thần khác.
//! 5. Chịu hậu quả chính trị, lời thề và phản ứng tín đồ.
//!
//! > Kết quả cuối cùng vẫn đi qua **weather, vật liệu công trình, cảnh báo và
//! > hành động cư dân**.
//!
//! Nên [`DomainAct`] không có biến thể nào đặt kết quả, và [`Intervention`] trả
//! về một **đề xuất tác động lên trường** — thứ mà một handler bình thường xử
//! lý, không phân biệt nó đến từ thần hay từ mùa.
//!
//! Hệ quả chơi được: một thành phố xây bằng đá, có hệ cảnh báo, và có dân biết
//! chạy thì **sống sót một cơn bão thần thánh**. Nếu thần đặt thẳng
//! `destroyed = true` thì cả ba khoản đầu tư đó thành vô nghĩa.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Ba loại "thần" (`§14.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GodKind {
    /// Entity rất mạnh, có domain/cult, **vẫn nằm trong law**.
    ///
    /// World 3 chủ yếu gồm loại này.
    Ascended,
    /// World administrator được ủy quyền — capability do True God cấp, **phạm
    /// vi rõ và có thể thu hồi**.
    ///
    /// Chữ "có thể thu hồi" là điều phân biệt loại này với loại dưới: quyền
    /// mượn được thì lấy lại được.
    Administrator,
    /// True God — người chơi. Quyền ở **tầng ngoài simulation**.
    ///
    /// Chỉ người chơi là loại này.
    True,
}

impl GodKind {
    /// Loại này có nằm trong law không.
    pub fn bound_by_law(self) -> bool {
        !matches!(self, GodKind::True)
    }

    /// Quyền của loại này có thu hồi được không.
    pub fn revocable(self) -> bool {
        matches!(self, GodKind::Administrator)
    }
}

/// Một domain — phạm vi ảnh hưởng của thần.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Domain {
    /// Tên: `storm`, `harvest`, `plague`, `forge`.
    pub name: String,
    /// Trường nào domain này chạm được: `weather.wind`, `weather.rain`,
    /// `soil.fertility`.
    ///
    /// **Danh sách trắng**, và nó là ranh giới quyền lực thật sự: một thần bão
    /// mạnh tới đâu cũng không chạm được `soil.fertility`.
    pub fields: Vec<String>,
    /// Domain nào chống lại domain này.
    pub counters: Vec<String>,
}

/// Một vị thần.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct God {
    /// Entity id — thần **là** một entity.
    pub who: EntityId,
    /// Loại.
    pub kind: GodKind,
    /// Các domain nắm giữ.
    pub domains: Vec<Domain>,
    /// Divine energy hiện có.
    pub energy: u64,
    /// Số tín đồ.
    pub followers: u64,
    /// Vùng mà thần có liên kết — **chỉ can thiệp được ở đây**.
    ///
    /// Không có liên kết thì không với tới. Đây là thứ biến địa lý tín ngưỡng
    /// thành địa lý quyền lực, và là lý do việc phá một đền thờ có ý nghĩa
    /// chiến lược.
    pub anchored_regions: Vec<u64>,
}

/// Một tác động mà thần được phép làm (`§14.2`).
///
/// **Không có biến thể nào đặt kết quả.** Danh sách này là đóng, và mở rộng nó
/// phải qua review — vì một biến thể `SetState` thêm vào đây sẽ vô hiệu hóa
/// `INV-22-1` cho toàn bộ hệ thần linh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainAct {
    /// Khuếch đại một trường sẵn có, theo phần nghìn.
    Amplify {
        /// Trường nào.
        field: String,
        /// Vùng nào.
        region: u64,
        /// Nhân thêm bao nhiêu phần nghìn.
        permille: u32,
    },
    /// Tạo một trường mới ở một vùng.
    Manifest {
        /// Trường nào.
        field: String,
        /// Vùng nào.
        region: u64,
        /// Cường độ.
        magnitude: i64,
    },
    /// Chống lại tác động của một thần khác.
    Oppose {
        /// Thần nào.
        rival: EntityId,
        /// Ở vùng nào.
        region: u64,
    },
}

impl DomainAct {
    /// Trường bị chạm, nếu có.
    pub fn field(&self) -> Option<&str> {
        match self {
            DomainAct::Amplify { field, .. } | DomainAct::Manifest { field, .. } => Some(field),
            DomainAct::Oppose { .. } => None,
        }
    }

    /// Vùng bị chạm.
    pub fn region(&self) -> u64 {
        match self {
            DomainAct::Amplify { region, .. }
            | DomainAct::Manifest { region, .. }
            | DomainAct::Oppose { region, .. } => *region,
        }
    }

    /// Tốn bao nhiêu divine energy.
    pub fn cost(&self) -> u64 {
        match self {
            DomainAct::Amplify { permille, .. } => u64::from(*permille),
            DomainAct::Manifest { magnitude, .. } => magnitude.unsigned_abs() * 2,
            DomainAct::Oppose { .. } => 500,
        }
    }
}

/// Vì sao một can thiệp bị từ chối.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DivineError {
    /// Trường không thuộc domain nào của thần.
    #[error(
        "trường `{field}` không thuộc domain nào của thần — một thần bão mạnh tới \
         đâu cũng không chạm được độ phì của đất"
    )]
    OutsideDomain {
        /// Trường nào.
        field: String,
    },
    /// Thần không có liên kết với vùng đó.
    #[error("thần không có liên kết với vùng {region} nên không với tới")]
    NoAnchor {
        /// Vùng nào.
        region: u64,
    },
    /// Không đủ divine energy.
    #[error("cần {need} divine energy, chỉ có {have}")]
    NotEnoughEnergy {
        /// Cần bao nhiêu.
        need: u64,
        /// Có bao nhiêu.
        have: u64,
    },
    /// True God không đi qua đường này.
    #[error(
        "True God có quyền ở tầng ngoài simulation (§14.1), không đi qua domain \
         authority — dùng God Interface"
    )]
    TrueGodUsesAnotherPath,
}

/// Một đề xuất tác động lên trường, đã qua kiểm domain.
///
/// **Đây là thứ duy nhất mà thần sinh ra.** Nó đi vào handler thường như mọi
/// nguồn khác — và handler đó không cần biết nó đến từ thần hay từ mùa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldProposal {
    /// Thần nào đề xuất.
    pub from: EntityId,
    /// Trường nào.
    pub field: String,
    /// Vùng nào.
    pub region: u64,
    /// Thay đổi bao nhiêu.
    pub delta: i64,
    /// Đã trừ bao nhiêu năng lượng.
    pub energy_spent: u64,
}

/// Kết quả của một lần can thiệp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intervention {
    /// Đề xuất lên trường, nếu tác động chạm trường.
    pub proposal: Option<FieldProposal>,
    /// Hậu quả chính trị: tín đồ phản ứng thế nào (`§14.2` mục 5).
    ///
    /// Có mặt ở đây vì `§14.2` liệt nó ngang hàng với bốn quyền kia: một can
    /// thiệp **luôn** có giá xã hội, kể cả khi thành công.
    pub follower_reaction: i32,
}

impl God {
    /// Thần có chạm được trường này không.
    pub fn may_touch(&self, field: &str) -> bool {
        self.domains
            .iter()
            .any(|d| d.fields.iter().any(|f| f == field))
    }

    /// Thần có liên kết với vùng này không.
    pub fn anchored_at(&self, region: u64) -> bool {
        self.anchored_regions.contains(&region)
    }

    /// **Thi hành một tác động domain** — trả về đề xuất, không phải kết quả.
    ///
    /// Không có đường nào từ hàm này tới việc đặt một trường state trực tiếp.
    /// Đó là toàn bộ điểm của nó.
    pub fn act(&mut self, a: &DomainAct) -> Result<Intervention, DivineError> {
        if self.kind == GodKind::True {
            return Err(DivineError::TrueGodUsesAnotherPath);
        }
        if let Some(f) = a.field() {
            if !self.may_touch(f) {
                return Err(DivineError::OutsideDomain {
                    field: f.to_owned(),
                });
            }
        }
        if !self.anchored_at(a.region()) {
            return Err(DivineError::NoAnchor { region: a.region() });
        }
        let gia = a.cost();
        if self.energy < gia {
            return Err(DivineError::NotEnoughEnergy {
                need: gia,
                have: self.energy,
            });
        }
        self.energy -= gia;

        let proposal = a.field().map(|f| FieldProposal {
            from: self.who,
            field: f.to_owned(),
            region: a.region(),
            delta: match a {
                DomainAct::Amplify { permille, .. } => i64::from(*permille),
                DomainAct::Manifest { magnitude, .. } => *magnitude,
                DomainAct::Oppose { .. } => 0,
            },
            energy_spent: gia,
        });

        Ok(Intervention {
            proposal,
            // Can thiệp lớn làm tín đồ vừa sợ vừa phục; can thiệp thất thường
            // làm họ hoang mang. Ở đây chỉ ghi nhận là **có** phản ứng — mức độ
            // do hệ thống tín ngưỡng ở `mow-culture` xử.
            follower_reaction: i32::try_from(gia / 100).unwrap_or(i32::MAX),
        })
    }

    /// Hai domain đối đầu: bên nào thắng và thắng bao nhiêu.
    ///
    /// Trả về **hiệu**, không phải người thắng: hai thần bão ngang sức triệt
    /// tiêu nhau và trời quang, chứ không phải một bên "thắng".
    pub fn contest(&self, rival: &God, field: &str) -> i64 {
        let a = if self.may_touch(field) {
            i64::try_from(self.energy).unwrap_or(i64::MAX)
        } else {
            0
        };
        let b = if rival.may_touch(field) {
            i64::try_from(rival.energy).unwrap_or(i64::MAX)
        } else {
            0
        };
        a - b
    }
}

/// Capability do True God cấp cho một administrator (`§14.1` loại 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// Cấp cho ai.
    pub to: EntityId,
    /// Quyền gì.
    pub capability: String,
    /// Phạm vi: world nào, vùng nào.
    pub scope: Vec<u64>,
    /// Đã bị thu hồi chưa.
    pub revoked: bool,
}

impl Grant {
    /// Quyền này còn hiệu lực ở vùng đó không.
    pub fn active_at(&self, region: u64) -> bool {
        !self.revoked && self.scope.contains(&region)
    }

    /// **Thu hồi.** Quyền mượn được thì lấy lại được — đó là điều phân biệt
    /// administrator với ascended god.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}
