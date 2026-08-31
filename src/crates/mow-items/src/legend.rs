//! Vật phẩm huyền thoại và di sản (`idea.md §8.9`, `§22.17`, `PE-16`).
//!
//! ## Không có "tỉ lệ rơi đồ huyền thoại"
//!
//! `§8.9.1` mở đầu bằng đúng câu đó, và nó loại bỏ cách làm phổ biến nhất: một
//! bảng xác suất sinh ra `Legendary Sword of Flame +5` từ hư không. Ở đây một
//! món đồ phi thường **luôn** qua ít nhất một trong [`bốn con đường`](Path), và
//! mỗi con đường là một chuỗi sự kiện có thật, truy ngược được.
//!
//! Hệ quả trực tiếp: [`Legendary::why`] không bao giờ trả về danh sách rỗng cho
//! một món đồ huyền thoại. Nếu nó rỗng thì món đồ đó **không** huyền thoại — dù
//! ai đó đã gắn nhãn gì lên nó.
//!
//! ## Truyền thuyết không phải lịch sử — và đây là chỗ dễ làm sai nhất
//!
//! `§8.9.2` nêu thẳng một game làm ngược lại:
//!
//! > *Caves of Qud* sinh lịch sử bằng cách tạo sự kiện trước rồi hợp lý hóa sau
//! > — hiệu quả cho việc tạo huyền thoại, nhưng **vi phạm trực tiếp `§22.17`**.
//! > Ta làm ngược lại: **sự kiện có thật trước, truyền thuyết là ảnh biến dạng
//! > của nó.**
//!
//! Nên có **hai** kiểu, và chúng không đổi chỗ cho nhau được:
//!
//! | | Là gì | Sai được không |
//! |---|---|---|
//! | [`Provenance`] | chuỗi sự kiện đã ghi | không — nó *là* lịch sử |
//! | [`Legend`] | belief về chuỗi đó | có, và thường sai |
//!
//! `§18.3` Legends view hiển thị **hai lớp cạnh nhau**, và khoảng cách giữa hai
//! lớp chính là nội dung chơi được: [`Legend::discrepancies`] tính đúng khoảng
//! cách đó, để một học giả có thể dành cả đời chứng minh thanh kiếm quốc bảo
//! thực ra được rèn sau ngày lập quốc một trăm năm.
//!
//! ## Hủy diệt là thật
//!
//! [`Fate::Destroyed`] không hoàn tác được. Một huyền thoại phá được thì mới có
//! ai buồn phá; một huyền thoại luôn tái sinh thì mọi hành động nhắm vào nó đều
//! vô nghĩa — nhưng [`Legend`] về nó **vẫn sống tiếp**, và đó mới là phần đáng
//! chơi.

use mow_core::{EntityId, EventSeq};
use serde::{Deserialize, Serialize};

/// Bốn con đường thành huyền thoại (`§8.9.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Path {
    /// Tay nghề tuyệt đỉnh — đuôi trên của phân phối `§8.7`.
    Masterwork,
    /// Lịch sử tích lũy — giá trị ở provenance, không ở vật liệu.
    ///
    /// Đây là con đường mà một thanh kiếm **tầm thường** đi được, và nó là con
    /// đường thú vị nhất vì nó không cần bất kỳ thứ gì đặc biệt lúc rèn.
    AccumulatedHistory,
    /// Ràng buộc phép thuật — nghi thức, domain, hoặc một linh hồn bị neo vào.
    MagicalBinding,
    /// Nguồn gốc thần thánh hoặc dị thường — khải thị, mảnh vỡ rift, di vật
    /// world khác.
    DivineOrAnomalous,
}

/// Một sự kiện có thật trong đời một món đồ.
///
/// `seq` là số thứ tự event **thật** trong log, không phải một nhãn. Đó là thứ
/// làm chuỗi này truy ngược được, và là thứ mà một chuỗi "sinh trước, hợp lý
/// hóa sau" không thể có.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deed {
    /// Event nào trong log.
    pub seq: EventSeq,
    /// Loại: `forged`, `wielded_at_battle`, `slew`, `changed_hands`,
    /// `reforged`, `stolen`, `displayed`.
    pub kind: String,
    /// Ai liên quan.
    pub who: Option<EntityId>,
    /// Mô tả ngắn, dựng từ dữ liệu event chứ không viết tay.
    pub detail: String,
}

/// Số phận cuối của một món đồ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fate {
    /// Còn tồn tại, đang ở đâu đó.
    Extant,
    /// Mất tích — không ai biết ở đâu, **nhưng vẫn tồn tại**.
    ///
    /// Khác hẳn `Destroyed`: mất tích thì tìm lại được, và cả một thể loại
    /// nhiệm vụ sống nhờ khác biệt này.
    Lost {
        /// Từ event nào thì mất dấu.
        since: EventSeq,
    },
    /// **Bị hủy — thật.** Không hoàn tác.
    Destroyed {
        /// Event nào.
        at: EventSeq,
        /// Bằng cách nào.
        how: String,
    },
}

impl Fate {
    /// Món đồ còn dùng được không.
    pub fn usable(&self) -> bool {
        matches!(self, Fate::Extant)
    }

    /// Còn tìm lại được không.
    ///
    /// `Lost` thì còn, `Destroyed` thì không. Gộp hai cái này lại là xóa mất
    /// một thể loại nhiệm vụ.
    pub fn recoverable(&self) -> bool {
        matches!(self, Fate::Extant | Fate::Lost { .. })
    }
}

/// Chuỗi provenance — **dữ liệu thật**, `§8.9.2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Món đồ nào.
    pub item: u64,
    /// Event rèn/tạo ra nó.
    pub forged_at: EventSeq,
    /// Người tạo, nếu biết.
    pub maker: Option<EntityId>,
    /// Chất lượng lúc rèn, phần nghìn của đuôi trên (`§8.7`).
    pub craft_percentile: u32,
    /// Mọi việc đã xảy ra, theo thứ tự event.
    pub deeds: Vec<Deed>,
    /// Có bị ràng buộc phép thuật không, và bởi cái gì.
    pub binding: Option<String>,
    /// Nguồn gốc dị thường, nếu có.
    pub anomalous_origin: Option<String>,
    /// Số phận.
    pub fate: Fate,
}

/// Ngưỡng để một chuỗi lịch sử tự nó làm nên huyền thoại.
///
/// Ba việc **đáng kể**, không phải ba lần đổi chủ. Một món đồ đi qua mười chủ
/// tiệm cầm đồ không thành huyền thoại, còn một món có mặt ở ba trận đánh
/// quyết định thì có.
pub const SO_VIEC_DANG_KE_THANH_HUYEN_THOAI: usize = 3;

/// Bách phân vị tay nghề để tính là tuyệt đỉnh — đuôi trên `§8.7`.
pub const NGUONG_TAY_NGHE: u32 = 995;

/// Loại việc được tính là "đáng kể".
const VIEC_DANG_KE: &[&str] = &["wielded_at_battle", "slew", "sealed", "founded"];

impl Provenance {
    /// **Vì sao món này huyền thoại** — trả về những con đường nó thật sự đi.
    ///
    /// Rỗng nghĩa là **không** huyền thoại. Không có tham số nào nới lỏng được
    /// điều này, vì nới lỏng nó là mở lại cánh cửa "tỉ lệ rơi đồ".
    pub fn why(&self) -> Vec<Path> {
        let mut v = Vec::new();
        if self.craft_percentile >= NGUONG_TAY_NGHE {
            v.push(Path::Masterwork);
        }
        if self.significant_deeds() >= SO_VIEC_DANG_KE_THANH_HUYEN_THOAI {
            v.push(Path::AccumulatedHistory);
        }
        if self.binding.is_some() {
            v.push(Path::MagicalBinding);
        }
        if self.anomalous_origin.is_some() {
            v.push(Path::DivineOrAnomalous);
        }
        v
    }

    /// Món này có huyền thoại không.
    pub fn is_legendary(&self) -> bool {
        !self.why().is_empty()
    }

    /// Số việc đáng kể đã làm.
    pub fn significant_deeds(&self) -> usize {
        self.deeds
            .iter()
            .filter(|d| VIEC_DANG_KE.contains(&d.kind.as_str()))
            .count()
    }

    /// Ai đang giữ nó, theo lần đổi chủ gần nhất.
    pub fn current_holder(&self) -> Option<EntityId> {
        self.deeds
            .iter()
            .rev()
            .find(|d| d.kind == "changed_hands")
            .and_then(|d| d.who)
    }
}

/// Một tuyên bố trong truyền thuyết.
///
/// Mỗi tuyên bố **có thể sai**, và cái sai đó là dữ liệu chứ không phải lỗi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    /// Loại: `forged_at`, `maker`, `slew`, `age_years`.
    pub about: String,
    /// Người ta tin là gì.
    pub believed: String,
    /// Bao nhiêu người tin, phần nghìn dân số biết tới món đồ.
    pub held_by_permille: u32,
}

/// Truyền thuyết về một món đồ — **belief**, không phải lịch sử (`§8.9.2`).
///
/// Lan qua kể lại và trôi dạt theo `§12.3`. Nó **không** có `EventSeq` nào,
/// và đó là điểm phân biệt cấu trúc với [`Provenance`]: một tuyên bố truyền
/// thuyết không trỏ vào event nào cả, vì nó không cần có thật để lan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Legend {
    /// Món đồ nào.
    pub item: u64,
    /// Tên người ta gọi nó — có thể khác hẳn tên thật.
    pub called: String,
    /// Những gì người ta tin.
    pub claims: Vec<Claim>,
}

/// Một chỗ truyền thuyết lệch khỏi sự thật.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discrepancy {
    /// Lệch về chuyện gì.
    pub about: String,
    /// Sự thật, theo provenance.
    pub truth: String,
    /// Điều người ta tin.
    pub belief: String,
    /// Bao nhiêu người tin điều sai đó.
    pub held_by_permille: u32,
}

impl Legend {
    /// **Hai lớp cạnh nhau**: chỗ nào truyền thuyết lệch khỏi provenance.
    ///
    /// `truth` phải là bảng dựng **từ** provenance, không phải một bảng viết
    /// tay: chỉ khi ấy khoảng cách mới là khoảng cách thật.
    pub fn discrepancies(&self, truth: &[(String, String)]) -> Vec<Discrepancy> {
        self.claims
            .iter()
            .filter_map(|c| {
                let (_, that) = truth.iter().find(|(k, _)| *k == c.about)?;
                (that != &c.believed).then(|| Discrepancy {
                    about: c.about.clone(),
                    truth: that.clone(),
                    belief: c.believed.clone(),
                    held_by_permille: c.held_by_permille,
                })
            })
            .collect()
    }

    /// Truyền thuyết này còn sống không khi món đồ đã bị hủy.
    ///
    /// **Luôn còn.** Hàm tồn tại để chỗ gọi không xóa `Legend` khi xử lý
    /// `Fate::Destroyed` — mất chuông vẫn còn tiếng, và tiếng đó mới là thứ
    /// người ta đánh nhau vì.
    pub fn survives_destruction(&self) -> bool {
        true
    }
}

/// Sức mạnh xã hội của một vật phẩm (`§8.9.3`).
///
/// > Quyền uy của một chiếc vương trượng chỉ thật đúng bằng mức người ta tin
/// > vào nó.
///
/// Nên [`SocialPower::authority`] nhân với niềm tin chứ không cộng: một vương
/// miện không ai công nhận có quyền uy **bằng không**, không phải "thấp".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialPower {
    /// Món đồ nào.
    pub item: u64,
    /// Vai trò: `crown`, `seal`, `relic`, `heirloom_blade`.
    pub role: String,
    /// Quyền uy danh nghĩa nếu ai cũng tin, phần nghìn.
    pub nominal: u32,
    /// Bao nhiêu phần nghìn dân tin đây là vật thật.
    pub believed_authentic_permille: u32,
}

impl SocialPower {
    /// Quyền uy thật.
    pub fn authority(&self) -> u32 {
        u32::try_from(u64::from(self.nominal) * u64::from(self.believed_authentic_permille) / 1_000)
            .unwrap_or(u32::MAX)
    }

    /// **Một bản sao được tin là thật** có quyền uy đúng bằng bản thật.
    ///
    /// `§8.9.3` liệt kê đúng trường hợp này: *"một bản sao được tin là thật suốt
    /// hai trăm năm"*. Vì quyền uy tính trên niềm tin chứ không trên vật, hàm
    /// này không cần biết đâu là bản thật — và đó chính là điều đang được
    /// khẳng định.
    pub fn same_authority_as(&self, other: &SocialPower) -> bool {
        self.authority() == other.authority()
    }
}

/// Vật phẩm có tri giác (`§8.9.4`).
///
/// > Nó **không phải trường hợp đặc biệt**: nó tuân thủ toàn bộ `§9.1`, chiếm
/// > ngân sách nhận thức như mọi `Sapient` khác, và chịu mọi ràng buộc ở `§22`.
///
/// Nên kiểu này cố tình **nghèo nàn**: nó chỉ ghi rằng món đồ có `MemoryNamespace`
/// và tag `Sapient`. Mọi thứ còn lại — cognition contract, persona version,
/// fallback, ACL — đi qua đúng đường của mọi entity `Sapient` khác. Thêm trường
/// riêng ở đây là bắt đầu tạo ra một đường vòng, và đường vòng đó sẽ dần dần
/// bỏ qua `INV-22-3`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SapientItem {
    /// Món đồ nào.
    pub item: u64,
    /// Entity id của nó — nó **là** một entity, không phải một component.
    pub as_entity: EntityId,
    /// Namespace ký ức, giống mọi `Sapient`.
    pub memory_namespace: String,
}

impl SapientItem {
    /// Nó có chiếm ngân sách nhận thức không.
    ///
    /// **Có.** Hàm này tồn tại để câu trả lời nằm trong code chứ không nằm
    /// trong trí nhớ của người viết scheduler.
    pub fn consumes_cognition_budget(&self) -> bool {
        true
    }
}
