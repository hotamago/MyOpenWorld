//! Chiếm hữu, quyền sở hữu và bó quyền tài sản (`idea.md §12.8.1`, `§12.8.7`,
//! `PD-10`).
//!
//! ## Hai khái niệm tách hẳn, và tại sao đó là cả một hệ thống
//!
//! - **Possession** — ground truth vật lý: món đồ đang nằm trong tay ai. Engine
//!   biết chính xác.
//! - **Claim** — belief xã hội: ai *được công nhận* là chủ, theo `norm_set` nào.
//!
//! Từ **một** sự tách đôi này rơi ra, không cần hệ thống riêng cho cái nào:
//!
//! | Hiện tượng | Là gì, dưới dạng possession/claim |
//! |---|---|
//! | Trộm cắp | chuyển possession, **không** chuyển claim |
//! | Tiêu thụ đồ gian | mua từ người có possession mà không có claim |
//! | Tẩy nguồn gốc | tạo một chuỗi claim giả để che chỗ đứt |
//! | Chiếm hữu lâu ngày thành quyền | possession đủ lâu **sinh ra** claim |
//! | Chiến lợi phẩm | claim hợp lệ theo luật bên thắng, không theo luật bên bại |
//! | Tranh chấp thừa kế | nhiều claim cùng hạng, không cái nào tự thắng |
//!
//! Nếu gộp hai thứ này thành một trường `owner: EntityId`, **mọi dòng trong
//! bảng trên đều biến mất cùng một lúc**. Không phải khó làm hơn — không diễn
//! đạt được nữa.
//!
//! ## Không claim nào **tự thực thi**
//!
//! Muốn đòi lại phải qua đúng bộ máy `§12.5`: phát hiện, chứng cứ, thủ tục,
//! cưỡng chế. Nên trong module này không có hàm nào tên `enforce` hay
//! `take_back` — chỉ có [`Ownership::claims_on`] trả về những gì các bên *nói*,
//! và việc ai thắng là chuyện của `mow-law`.

use mow_core::{EntityId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Một quyền cụ thể trong **bó quyền tài sản** (`§12.8.7`).
///
/// "Sở hữu" không phải một thứ nguyên khối. Một người có thể được dùng mà không
/// được bán; được thu hoa lợi mà không được đổi; được cho thuê mà không được
/// phá. Tá điền, thái ấp, quyền chăn thả, quyền câu cá, quyền khai thác — tất
/// cả là những **tổ hợp con** khác nhau của cùng một bó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Right {
    /// Được vào, được chạm.
    Access,
    /// Được lấy hoa lợi.
    Withdrawal,
    /// Được quyết định ai khác được dùng.
    Management,
    /// Được loại trừ người khác.
    Exclusion,
    /// Được chuyển nhượng, bán, cho.
    Alienation,
    /// Được phá hủy.
    Destruction,
}

/// Sáu quyền, để lặp.
pub const RIGHTS: [Right; 6] = [
    Right::Access,
    Right::Withdrawal,
    Right::Management,
    Right::Exclusion,
    Right::Alienation,
    Right::Destruction,
];

/// Vì sao một người nói mình là chủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// Tự làm ra.
    Creation,
    /// Mua lại.
    Purchase,
    /// Được cho, được tặng.
    Gift,
    /// Thừa kế.
    Inheritance,
    /// **Chiếm hữu lâu ngày.** Đủ lâu thì thành quyền, ở nhiều `norm_set`.
    AdversePossession,
    /// Chiến lợi phẩm.
    Conquest,
}

/// Một lời tuyên bố quyền sở hữu.
///
/// **Là belief xã hội, không phải sự thật.** Nhiều claim mâu thuẫn cùng tồn tại
/// được, và đó là chuyện bình thường chứ không phải trạng thái hỏng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    /// Ai nói.
    pub holder: EntityId,
    /// Theo bộ luật nào. Cùng một món có thể có claim hợp lệ ở hai bộ luật khác
    /// nhau, và cả hai đều đúng trong phạm vi của mình.
    pub under_norm_set: String,
    /// Bó quyền mà claim này đòi.
    pub rights: Vec<Right>,
    /// Căn cứ.
    pub basis: Basis,
    /// Từ lúc nào.
    pub since: Tick,
}

impl Claim {
    /// Claim này có đòi quyền đó không.
    pub fn grants(&self, r: Right) -> bool {
        self.rights.contains(&r)
    }
}

/// Sổ sở hữu: possession thật, và các claim chồng lên nó.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ownership {
    /// `item → ai đang giữ`. **Sự thật vật lý**, engine biết chính xác.
    possession: BTreeMap<u64, EntityId>,
    /// `item → các claim`.
    claims: BTreeMap<u64, Vec<Claim>>,
}

impl Ownership {
    /// Rỗng.
    pub fn new() -> Ownership {
        Ownership::default()
    }

    /// Ai đang **giữ** món này.
    pub fn possessor(&self, item: u64) -> Option<EntityId> {
        self.possession.get(&item).copied()
    }

    /// Chuyển possession.
    ///
    /// **Không** đụng tới claim. Đó là toàn bộ định nghĩa của trộm cắp, và cũng
    /// là lý do hàm này không có tham số nào để "chuyển luôn quyền sở hữu" — nếu
    /// có, ai đó sẽ dùng nó cho đường mua bán và đường trộm cắp sẽ thành ngoại lệ.
    pub fn set_possession(&mut self, item: u64, who: EntityId) {
        self.possession.insert(item, who);
    }

    /// Thêm một claim.
    pub fn add_claim(&mut self, item: u64, c: Claim) {
        self.claims.entry(item).or_default().push(c);
    }

    /// Mọi claim trên một món, theo thứ tự ổn định.
    pub fn claims_on(&self, item: u64) -> Vec<&Claim> {
        let mut v: Vec<&Claim> = self.claims.get(&item).into_iter().flatten().collect();
        v.sort_by(|a, b| {
            a.under_norm_set
                .cmp(&b.under_norm_set)
                .then_with(|| a.holder.0.cmp(&b.holder.0))
        });
        v
    }

    /// Claim hợp lệ **trong một bộ luật cụ thể**.
    ///
    /// Không có "claim hợp lệ" nói chung. Chiến lợi phẩm hợp pháp theo luật bên
    /// thắng và bất hợp pháp theo luật bên bại — hai câu trả lời, cả hai đúng.
    pub fn claims_under(&self, item: u64, norm_set: &str) -> Vec<&Claim> {
        self.claims_on(item)
            .into_iter()
            .filter(|c| c.under_norm_set == norm_set)
            .collect()
    }

    /// Món này đang **tranh chấp** trong một bộ luật không.
    pub fn disputed(&self, item: u64, norm_set: &str) -> bool {
        let cac = self.claims_under(item, norm_set);
        cac.len() > 1 && cac.windows(2).any(|w| w[0].holder != w[1].holder)
    }

    /// Người đang giữ có claim được công nhận không.
    ///
    /// `false` nghĩa là **đồ gian** — theo bộ luật này. Người mua nó sau đó là
    /// tiêu thụ đồ gian, dù họ có thể hoàn toàn không biết.
    pub fn possession_is_lawful(&self, item: u64, norm_set: &str) -> bool {
        let Some(giu) = self.possessor(item) else {
            return false;
        };
        self.claims_under(item, norm_set)
            .iter()
            .any(|c| c.holder == giu)
    }

    /// Chiếm hữu lâu ngày **sinh ra** claim (`§12.8.1`).
    ///
    /// Trả về claim mới nếu đủ điều kiện. Không tự thêm vào sổ: việc một chế độ
    /// công nhận thời hiệu hay không là dữ liệu của `norm_set`, và quyết định đó
    /// phải đi qua đường commit như mọi thay đổi state khác (`§22.1`).
    pub fn ripened_claim(
        &self,
        item: u64,
        norm_set: &str,
        held_since: Tick,
        now: Tick,
        ripening_ticks: u64,
    ) -> Option<Claim> {
        let giu = self.possessor(item)?;
        if now.0.checked_sub(held_since.0)? < ripening_ticks {
            return None;
        }
        // Đã có claim rồi thì không cần chín thêm.
        if self
            .claims_under(item, norm_set)
            .iter()
            .any(|c| c.holder == giu)
        {
            return None;
        }
        Some(Claim {
            holder: giu,
            under_norm_set: norm_set.to_owned(),
            rights: RIGHTS.to_vec(),
            basis: Basis::AdversePossession,
            since: now,
        })
    }
}
