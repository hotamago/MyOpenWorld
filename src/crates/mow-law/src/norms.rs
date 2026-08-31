//! `norm_set`, thẩm quyền chồng lấn, và độ phủ cưỡng chế (`idea.md §12.5.1`,
//! `§12.14`, `§22.49`, `PD-01`, `PD-06`).
//!
//! ## Câu mở đầu của `§12.5.1` là toàn bộ thiết kế
//!
//! > Tội **không phải thuộc tính của hành động**. Nó là **quan hệ giữa một hành
//! > động và một bộ chuẩn mực đang có hiệu lực tại nơi hành động xảy ra**.
//!
//! Nên trong crate này không có, và sẽ không có, một trường `is_crime: bool` ở
//! bất kỳ đâu. Cùng một việc hợp pháp ở nước này và tử hình ở nước bên cạnh, và
//! một cờ boolean không diễn đạt được điều đó — nó buộc phải trả lời "có" hoặc
//! "không" ở một chỗ không có ngữ cảnh nào để trả lời.
//!
//! [`judge`] nhận `(hành vi, nơi chốn, lúc nào)` và trả về những [`Charge`] có
//! thể có. Không có ngữ cảnh thì không có câu trả lời.
//!
//! ## Bất công **sinh ra có cấu trúc**, không phải được bịa thêm
//!
//! Hai trường làm việc đó, và chúng là dữ liệu văn hóa chứ không phải hằng số:
//!
//! - [`Enforcement::coverage_by_district`] — cùng một tội, ở bến cảng gần như
//!   không bị phát hiện.
//! - [`Rule::enforced_against`] — người thường bị xử nặng hơn quý tộc.
//!
//! Không có hai trường này thì để có bất công, ai đó phải viết tay từng vụ. Có
//! chúng thì bất công là **hệ quả** của cách nhà nước được tổ chức, và người
//! chơi lật ngược được từ một bản án bất công về tận cái ngân sách đã tạo ra nó.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Loại chế tài.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SanctionKind {
    /// Phạt tiền.
    Fine,
    /// Nhục hình.
    Corporal,
    /// Giam giữ.
    Imprisonment,
    /// Lưu đày.
    Exile,
    /// Tử hình.
    Capital,
}

/// Một chế tài cụ thể.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sanction {
    /// Loại.
    pub kind: SanctionKind,
    /// Mức độ `0`–`1000`.
    pub severity: u16,
}

/// Loại chứng cứ mà một điều luật đòi hỏi.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofRequirement {
    /// Cần ít nhất bấy nhiêu nhân chứng.
    WitnessCount(u32),
    /// Cần vật chứng.
    PhysicalEvidence,
    /// Cần văn bản.
    Document,
    /// Cần phép truy vấn sự thật.
    TruthSpell,
}

/// Cần **tất cả** hay chỉ cần **một** trong các yêu cầu chứng cứ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofMode {
    /// Đủ một là được.
    AnyOf,
    /// Phải đủ hết.
    AllOf,
}

/// Một điều luật.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Hành vi bị điều chỉnh: `theft`, `unlicensed_magic`, `usury`.
    pub act: String,
    /// Chỉ áp dụng khi giá trị vượt ngưỡng này. `None` là mọi mức.
    pub value_above: Option<i64>,
    /// Chế tài.
    pub sanction: Sanction,
    /// Chứng cứ đòi hỏi.
    pub proof_required: Vec<ProofRequirement>,
    /// Cần đủ hết hay chỉ cần một.
    pub proof_mode: ProofMode,
    /// **Chỉ áp dụng với những tầng lớp này.** Rỗng nghĩa là mọi người.
    ///
    /// "Luật áp dụng không đều là chuyện thường" (`§12.5.1`). Đây là chỗ điều đó
    /// được nói ra bằng dữ liệu, thay vì bị giấu trong một nhánh `if` nào đó.
    pub enforced_against: Vec<String>,
}

impl Rule {
    /// Điều luật này có với tới một người thuộc tầng lớp `class` không.
    pub fn applies_to_class(&self, class: &str) -> bool {
        self.enforced_against.is_empty() || self.enforced_against.iter().any(|c| c == class)
    }
}

/// Cơ quan cưỡng chế và **năng lực thật** của nó.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Enforcement {
    /// Tổ chức chịu trách nhiệm.
    pub agency: String,
    /// Độ phủ theo khu, `0`–`1000`.
    ///
    /// **Sinh ra từ năng lực nhà nước** (`§12.13.1`), không phải hằng số viết
    /// tay — xem [`crate::state`]. Ở đây nó là kết quả đã tính, vì luật chỉ cần
    /// biết con số, không cần biết nó từ đâu ra.
    pub coverage_by_district: BTreeMap<String, u16>,
}

impl Enforcement {
    /// Độ phủ ở một khu. Khu không khai báo thì **bằng 0**, không phải trung bình.
    ///
    /// Mặc định 0 là chiều đúng: một khu mà nhà nước chưa từng nhắc tới là một
    /// khu nhà nước không với tới. Lấy trung bình sẽ tạo ra một sự hiện diện mà
    /// ngân sách chưa bao giờ trả tiền cho.
    pub fn coverage(&self, district: &str) -> u16 {
        self.coverage_by_district
            .get(district)
            .copied()
            .unwrap_or(0)
    }
}

/// Phạm vi hiệu lực của một bộ luật.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    /// Tổ chức ban hành.
    pub jurisdiction: String,
    /// Có hiệu lực theo lãnh thổ hay theo thành viên.
    ///
    /// Khác biệt này quyết định chuyện chạy trốn có tác dụng không: luật lãnh
    /// thổ thì ra khỏi biên là thoát, luật thành viên thì mang theo suốt đời.
    pub territorial: bool,
    /// Những khu mà nó phủ, nếu theo lãnh thổ.
    pub districts: Vec<String>,
    /// Những nhóm mà nó ràng buộc, nếu theo thành viên.
    pub members: Vec<String>,
}

impl Scope {
    /// Bộ luật này có với tới một hành vi ở `district` do một thành viên của
    /// `groups` thực hiện không.
    pub fn covers(&self, district: &str, groups: &[String]) -> bool {
        if self.territorial {
            self.districts.iter().any(|d| d == district)
        } else {
            self.members.iter().any(|m| groups.contains(m))
        }
    }
}

/// Một bộ chuẩn mực đã version hóa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormSet {
    /// Định danh có namespace.
    pub id: String,
    /// **Phiên bản.**
    ///
    /// `§12.14`: *"Version của luật tại thời điểm hành vi xảy ra, tách khỏi luật
    /// thủ tục tại thời điểm xét xử."* Con số này đi vào `Event::norm_set_version`
    /// lúc hành vi xảy ra, và tòa xử theo nó — không theo bộ luật hôm nay.
    pub version: u32,
    /// Bậc ưu tiên; **nhỏ hơn thắng** khi hai hệ luật mâu thuẫn (`§12.14`).
    pub precedence: u8,
    /// Phạm vi.
    pub scope: Scope,
    /// Các điều luật.
    pub rules: Vec<Rule>,
    /// Cưỡng chế.
    pub enforcement: Enforcement,
}

/// Một cáo buộc: hành vi này, theo bộ luật kia, ở phiên bản đó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Charge {
    /// Bộ luật.
    pub norm_set: String,
    /// **Phiên bản lúc hành vi xảy ra**, không phải lúc xét xử.
    pub norm_set_version: u32,
    /// Bậc ưu tiên của bộ luật đó.
    pub precedence: u8,
    /// Điều luật.
    pub act: String,
    /// Chế tài.
    pub sanction: Sanction,
    /// Chứng cứ đòi hỏi.
    pub proof_required: Vec<ProofRequirement>,
    /// Cách tính đủ chứng cứ.
    pub proof_mode: ProofMode,
}

/// Một hành vi đã xảy ra, chưa gán nhãn gì.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deed {
    /// Ai làm.
    pub actor: EntityId,
    /// Làm gì: `theft`, `usury`.
    pub act: String,
    /// Giá trị liên quan, nếu có.
    pub value: i64,
    /// Ở khu nào.
    pub district: String,
    /// Tầng lớp của người làm.
    pub actor_class: String,
    /// Những nhóm mà người làm thuộc về: phường hội, dòng họ, giáo hội.
    pub actor_groups: Vec<String>,
}

/// Nhiều bộ luật chồng lên nhau (`§12.14`).
///
/// Một cá thể chịu đồng thời luật quốc gia, luật phường hội, luật dòng họ, giáo
/// luật và hiệp ước liên-world. Chúng **chồng lên nhau**, không thay thế nhau.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LegalOrder {
    sets: Vec<NormSet>,
}

impl LegalOrder {
    /// Rỗng.
    pub fn new() -> LegalOrder {
        LegalOrder::default()
    }

    /// Thêm một bộ luật.
    pub fn add(&mut self, ns: NormSet) -> &mut LegalOrder {
        self.sets.push(ns);
        self
    }

    /// Các bộ luật đang có.
    pub fn sets(&self) -> &[NormSet] {
        &self.sets
    }
}

/// Mọi cáo buộc có thể có với một hành vi.
///
/// Trả về **danh sách**, không phải một cáo buộc. Một hành vi có thể vi phạm ba
/// hệ luật cùng lúc, và rút gọn về một cái sẽ xóa mất chính cái xung đột mà
/// `§12.14` tồn tại để mô hình hóa: chạy trốn sang thẩm quyền khác chỉ là một
/// nước đi hợp lệ khi các thẩm quyền thật sự khác nhau.
///
/// Danh sách đã sắp theo `precedence` tăng dần — bộ luật thắng đứng đầu.
pub fn judge(order: &LegalOrder, deed: &Deed) -> Vec<Charge> {
    let mut ra: Vec<Charge> = Vec::new();

    for ns in &order.sets {
        if !ns.scope.covers(&deed.district, &deed.actor_groups) {
            continue;
        }
        for r in &ns.rules {
            if r.act != deed.act {
                continue;
            }
            if !r.applies_to_class(&deed.actor_class) {
                continue;
            }
            if let Some(nguong) = r.value_above {
                if deed.value <= nguong {
                    continue;
                }
            }
            ra.push(Charge {
                norm_set: ns.id.clone(),
                norm_set_version: ns.version,
                precedence: ns.precedence,
                act: r.act.clone(),
                sanction: r.sanction,
                proof_required: r.proof_required.clone(),
                proof_mode: r.proof_mode,
            });
        }
    }

    // Phá hòa bằng `norm_set` để kết quả xác định: hai bộ luật cùng bậc thì thứ
    // tự phải là hàm của dữ liệu, không phải của thứ tự nạp content pack.
    ra.sort_by(|a, b| {
        a.precedence
            .cmp(&b.precedence)
            .then_with(|| a.norm_set.cmp(&b.norm_set))
    });
    ra
}

/// Cáo buộc thắng khi các hệ luật mâu thuẫn.
pub fn governing_charge(order: &LegalOrder, deed: &Deed) -> Option<Charge> {
    judge(order, deed).into_iter().next()
}

/// Một người có được miễn trừ không, và vì sao (`§12.14`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Immunity {
    /// Ai được miễn.
    pub holder: EntityId,
    /// Bộ luật nào không với tới họ.
    pub from_norm_set: String,
    /// Lý do: `office`, `envoy`, `sanctuary`.
    pub basis: String,
}

/// Miễn trừ có chặn được một cáo buộc không.
pub fn immune(immunities: &[Immunity], holder: EntityId, charge: &Charge) -> bool {
    immunities
        .iter()
        .any(|i| i.holder == holder && i.from_norm_set == charge.norm_set)
}
