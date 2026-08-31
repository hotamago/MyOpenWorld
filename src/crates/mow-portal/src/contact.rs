//! Chế độ tiếp xúc và kiểm dịch cổng (`idea.md §6.4`, `PE-11`).
//!
//! > Thiếu các thỏa thuận này, cổng **vẫn hoạt động** — nhưng nó trở thành ổ
//! > dịch, chợ đen, trại tị nạn hoặc đầu cầu xâm lược.
//!
//! Câu trên là toàn bộ thiết kế của module. Chú ý *"vẫn hoạt động"*: chế độ
//! tiếp xúc **không phải điều kiện để cổng mở**. Một cổng không có thỏa thuận
//! nào vẫn cho người đi qua — nó chỉ không kiểm được cái gì đi cùng.
//!
//! Nên [`ContactRegime`] mặc định là **rỗng**, không phải "chặn hết". Mặc định
//! chặn hết sẽ biến một cổng chưa ai thỏa thuận thành một cánh cửa khóa, và
//! khóa thì không sinh ra ổ dịch. Cái phải mô phỏng được là một cổng **mở toang
//! mà không ai kiểm** — vì đó là thứ thật sự xảy ra khi hai bên chưa ngồi lại.
//!
//! ## Bảy điều khoản, và cái mỗi điều khoản ngăn được
//!
//! | Điều khoản | Thiếu nó thì |
//! |---|---|
//! | [`Quarantine`] | mầm bệnh world A gặp quần thể chưa từng phơi nhiễm ở world B |
//! | [`Tariff`] | chênh giá hai world hút hàng qua cổng cho tới khi một bên sập giá |
//! | [`Measures`] | hai bên ký hợp đồng "100 đơn vị" với hai nghĩa khác nhau |
//! | [`LegalPersonhood`] | một công ty world A không kiện được ở world B, nên không ai dám giao dịch |
//! | [`Residency`] | người qua cổng không có tư cách gì, thành lao động không quyền |
//! | [`TransportLaw`] | sinh vật, linh hồn, hạt giống đi qua mà không ai ghi nhận |
//! | [`DisputeForum`] | mọi tranh chấp xuyên world thành chuyện vũ lực |
//!
//! Bảng này là lý do các điều khoản **không gộp thành một chỉ số "quan hệ"**.
//! Hai world có thể kiểm dịch rất chặt mà vẫn không công nhận pháp nhân của
//! nhau, và tình huống đó cho ra một loại cổng cụ thể — cổng thương mại lậu có
//! kiểm dịch — mà một chỉ số duy nhất không diễn tả nổi.
//!
//! Tham khảo: WHO International Health Regulations (2005).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Kiểm dịch sinh học và ma thuật.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Quarantine {
    /// Số tick cách ly trước khi cho vào.
    pub hold_ticks: u64,
    /// Có sàng mầm bệnh không.
    pub screens_pathogens: bool,
    /// Có sàng nhiễm ma thuật không — một lời nguyền cũng lây.
    pub screens_taint: bool,
    /// Được quyền từ chối nhập cảnh không.
    ///
    /// Sàng mà không có quyền từ chối thì chỉ là một cuốn sổ ghi ai đã mang
    /// bệnh vào.
    pub may_refuse: bool,
}

/// Thuế quan và hàng cấm.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Tariff {
    /// Phần nghìn giá trị hàng.
    pub rate_permille: u32,
    /// Loại hàng bị cấm hẳn.
    pub contraband: BTreeSet<String>,
}

/// Chuẩn đo lường chung.
///
/// `§6.4`: *"hai world không mặc định dùng cùng đơn vị"*. Không có chuẩn thì
/// hợp đồng xuyên world vẫn ký được — và vẫn tranh chấp được.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Measures {
    /// Đã thống nhất chuẩn nào (`mass.kg`, `mana.mmu`, …).
    pub agreed: BTreeSet<String>,
}

/// Quy chế pháp nhân xuyên world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LegalPersonhood {
    /// Tổ chức world nguồn có tồn tại về mặt pháp lý ở world đích không.
    pub recognized: bool,
    /// Hợp đồng ký ở world nguồn có hiệu lực ở world đích không.
    pub contracts_enforceable: bool,
}

/// Quyền cư trú và lao động.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Residency {
    /// Ở được bao nhiêu tick trước khi phải xin phép. 0 = phải xin ngay.
    pub visa_free_ticks: u64,
    /// Được làm việc hợp pháp không.
    ///
    /// Cư trú mà cấm lao động là công thức của kinh tế ngầm — và đó là một kết
    /// quả hợp lệ của mô phỏng, không phải một lỗi cần chặn.
    pub may_work: bool,
}

/// Luật mang sinh vật, vật phẩm và linh hồn qua cổng.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TransportLaw {
    /// Được mang sinh vật sống không.
    pub living_creatures: bool,
    /// Được mang hạt giống, bào tử không.
    pub seeds: bool,
    /// Được mang linh hồn (bình chứa, di hài có hồn) không.
    pub souls: bool,
}

/// Cơ chế giải quyết tranh chấp xuyên world.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DisputeForum {
    /// Có diễn đàn nào không, và tên nó.
    pub forum: Option<String>,
    /// Có phiên dịch được công nhận không.
    pub interpreters: bool,
}

/// Chế độ tiếp xúc của một cổng — bảy điều khoản của `§6.4`.
///
/// [`Default`] là **rỗng hoàn toàn**: một cổng vừa mở, chưa ai thỏa thuận gì.
/// Đó là trạng thái nguy hiểm nhất và cũng là trạng thái mặc định của thực tế.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContactRegime {
    /// Kiểm dịch.
    pub quarantine: Quarantine,
    /// Thuế quan và hàng cấm.
    pub tariff: Tariff,
    /// Chuẩn đo lường.
    pub measures: Measures,
    /// Pháp nhân.
    pub personhood: LegalPersonhood,
    /// Cư trú.
    pub residency: Residency,
    /// Luật vận chuyển.
    pub transport: TransportLaw,
    /// Tranh chấp.
    pub dispute: DisputeForum,
}

/// Cổng không được quản trở thành cái gì (`§6.4`).
///
/// Không phải một thang điểm: một cổng có thể vừa là ổ dịch vừa là chợ đen,
/// và gộp hai thứ đó vào một con số sẽ mất đúng cái thông tin cần để chữa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// Ổ dịch — không kiểm dịch.
    DiseaseVector,
    /// Chợ đen — không thuế, không hàng cấm.
    BlackMarket,
    /// Trại tị nạn — không quy chế cư trú.
    RefugeeCamp,
    /// Đầu cầu xâm lược — không quyền từ chối, không luật vận chuyển sinh vật.
    Beachhead,
    /// Tranh chấp giải quyết bằng vũ lực — không diễn đàn.
    ForceOnly,
}

impl Failure {
    /// Câu mô tả, để hiện trong UI và log.
    pub fn describe(self) -> &'static str {
        match self {
            Failure::DiseaseVector => {
                "không kiểm dịch: mầm bệnh đi qua cổng gặp quần thể chưa từng phơi nhiễm"
            }
            Failure::BlackMarket => "không thuế, không hàng cấm: chênh giá hai world hút hàng lậu",
            Failure::RefugeeCamp => "không quy chế cư trú: người qua cổng không có tư cách pháp lý",
            Failure::Beachhead => {
                "không quyền từ chối và không luật vận chuyển: cổng thành đầu cầu"
            }
            Failure::ForceOnly => {
                "không diễn đàn tranh chấp: mọi bất đồng xuyên world thành vũ lực"
            }
        }
    }
}

/// Một quyết định của trạm kiểm soát với một lần đi qua.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// Cho qua.
    Allow,
    /// Giữ ở vùng cách ly `ticks` tick rồi xét lại.
    ///
    /// `§6.2` bước 3 nói rõ bước này *"có thể từ chối hoặc **giữ lại ở vùng
    /// cách ly** thay vì cho qua"* — giữ lại là một kết quả riêng, không phải
    /// một dạng từ chối lịch sự.
    Hold {
        /// Bao nhiêu tick.
        ticks: u64,
        /// Vì sao.
        reason: &'static str,
    },
    /// Từ chối.
    Refuse {
        /// Vì sao.
        reason: &'static str,
    },
}

/// Thứ đang xin đi qua cổng.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cargo {
    /// Có mang mầm bệnh không (theo hiểu biết của trạm, không phải sự thật).
    pub suspected_pathogen: bool,
    /// Có nhiễm ma thuật không.
    pub suspected_taint: bool,
    /// Loại hàng mang theo.
    pub goods: BTreeSet<String>,
    /// Có sinh vật sống không.
    pub living: bool,
    /// Có hạt giống không.
    pub seeds: bool,
    /// Có linh hồn không.
    pub souls: bool,
}

impl ContactRegime {
    /// Chế độ tiếp xúc của một cổng chưa ai thỏa thuận gì.
    pub fn none() -> ContactRegime {
        ContactRegime::default()
    }

    /// Cổng này thiếu quản trị thì hóa thành những gì.
    ///
    /// Trả về một danh sách, không một nhãn: một cổng bỏ mặc hoàn toàn là **cả
    /// năm** thứ cùng lúc, và biết nó là cả năm thì mới biết vá cái nào trước.
    pub fn failure_modes(&self) -> Vec<Failure> {
        let mut f = Vec::new();
        if self.quarantine.hold_ticks == 0
            && !self.quarantine.screens_pathogens
            && !self.quarantine.screens_taint
        {
            f.push(Failure::DiseaseVector);
        }
        if self.tariff.rate_permille == 0 && self.tariff.contraband.is_empty() {
            f.push(Failure::BlackMarket);
        }
        if self.residency.visa_free_ticks == 0 && !self.residency.may_work {
            f.push(Failure::RefugeeCamp);
        }
        if !self.quarantine.may_refuse && self.transport.living_creatures {
            f.push(Failure::Beachhead);
        }
        if self.dispute.forum.is_none() {
            f.push(Failure::ForceOnly);
        }
        f
    }

    /// **Bước 3 của `§6.2`**: áp chế độ tiếp xúc lên một lần đi qua.
    ///
    /// Thứ tự xét có ý nghĩa: từ chối hẳn thắng giữ lại, và giữ lại thắng cho
    /// qua. Xét ngược lại thì một kiện hàng cấm sẽ được "cho qua sau khi cách
    /// ly", điều mà không hải quan nào làm.
    pub fn screen(&self, cargo: &Cargo) -> Decision {
        // 1. Cấm hẳn.
        if cargo.living && !self.transport.living_creatures {
            return Decision::Refuse {
                reason: "luật cổng cấm mang sinh vật sống qua",
            };
        }
        if cargo.seeds && !self.transport.seeds {
            return Decision::Refuse {
                reason: "luật cổng cấm mang hạt giống, bào tử qua",
            };
        }
        if cargo.souls && !self.transport.souls {
            return Decision::Refuse {
                reason: "luật cổng cấm mang linh hồn qua",
            };
        }
        if let Some(h) = cargo.goods.intersection(&self.tariff.contraband).next() {
            let _ = h;
            return Decision::Refuse {
                reason: "trong hàng có thứ nằm trong danh mục cấm",
            };
        }

        // 2. Giữ lại — chỉ khi trạm **vừa sàng được vừa có quyền từ chối**.
        //    Sàng ra mà không có quyền giữ thì kết quả sàng chỉ là một dòng ghi
        //    chép, nên ở đây nó không đổi được quyết định.
        if self.quarantine.may_refuse {
            if cargo.suspected_pathogen && self.quarantine.screens_pathogens {
                return Decision::Hold {
                    ticks: self.quarantine.hold_ticks,
                    reason: "nghi mang mầm bệnh, giữ ở vùng cách ly",
                };
            }
            if cargo.suspected_taint && self.quarantine.screens_taint {
                return Decision::Hold {
                    ticks: self.quarantine.hold_ticks,
                    reason: "nghi nhiễm ma thuật, giữ ở vùng cách ly",
                };
            }
        }

        Decision::Allow
    }

    /// Một hợp đồng ký ở world nguồn có cưỡng chế được ở world đích không.
    ///
    /// Cần **cả hai**: công nhận pháp nhân *và* có nơi xử. Công nhận mà không
    /// có tòa thì bên bị vi phạm chỉ có một tờ giấy đúng luật và không ai thi
    /// hành.
    pub fn contract_enforceable(&self) -> bool {
        self.personhood.recognized
            && self.personhood.contracts_enforceable
            && self.dispute.forum.is_some()
    }

    /// Hai bên đã thống nhất một chuẩn đo chưa.
    pub fn shares_measure(&self, m: &str) -> bool {
        self.measures.agreed.contains(m)
    }
}
