//! Giải quyết đồng thời (`idea.md §10.9`, `§22.43`).
//!
//! > Gom mọi impact cùng tick, chạy theo **tầng cố định**, và `EntityId` **chỉ
//! > dùng để sắp xếp ổn định, không quyết định thắng thua**.
//!
//! ## Vì sao phân biệt này quan trọng
//!
//! Hai người cùng với tay lấy một quả táo. Ai được?
//!
//! Câu trả lời **phải** đến từ một thứ quan sát được trong thế giới: ai nhanh
//! tay hơn, ai gần hơn, ai bắt đầu trước. Nếu nó đến từ `EntityId`, thì thế
//! giới có một luật vô hình mà không ai chơi game đoán được — và tệ hơn, luật
//! đó ổn định: cùng một người sẽ **luôn** thắng, mãi mãi, và không ai hiểu vì
//! sao.
//!
//! `EntityId` vẫn cần, nhưng chỉ ở **bước cuối cùng**, khi mọi thứ luật quan
//! tâm đã bằng nhau. Lúc đó nó chỉ đảm bảo thứ tự xác định để replay đúng.
//!
//! Bài property test ở `PB-10` chứng minh điều đó: đảo `EntityId` của mọi thực
//! thể không được đổi kết quả.

use mow_core::{EntityId, StableKey};
use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Tầng giải quyết. **Thứ tự khai báo là thứ tự chạy.**
///
/// Mỗi tầng là một loại tác động, và thứ tự giữa chúng là luật vật lý của thế
/// giới: không thể chặn một đòn sau khi nó đã trúng, và không thể né sau khi
/// đã bị trói.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Hủy bỏ: choáng, chết, mất mục tiêu. Chạy trước hết vì nó xóa mọi thứ sau.
    Nullify,
    /// Khống chế: trói, đẩy, làm ngã.
    Control,
    /// Phòng thủ: đỡ, né, khiên.
    Defense,
    /// Di chuyển.
    Movement,
    /// Gây tác động: đánh, cắt, đốt.
    Impact,
    /// Chuyển quyền sở hữu: nhặt, trao, trộm.
    Transfer,
    /// Ghi nhận: nói, ra hiệu, quan sát.
    Record,
}

impl Tier {
    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Nullify => "nullify",
            Tier::Control => "control",
            Tier::Defense => "defense",
            Tier::Movement => "movement",
            Tier::Impact => "impact",
            Tier::Transfer => "transfer",
            Tier::Record => "record",
        }
    }
}

impl CanonicalHash for Tier {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

/// Một đề xuất tác động đã tới pha `impact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contention {
    /// Ai.
    pub actor: EntityId,
    /// Tầng.
    pub tier: Tier,
    /// Loại hành động.
    pub action: String,
    /// Mục tiêu tranh chấp, nếu có. Hai đề xuất cùng mục tiêu thì cạnh tranh.
    pub target: Option<String>,
    /// **Điểm phân định, quan sát được trong thế giới.**
    ///
    /// Ai nhanh tay hơn, ai gần hơn, ai chuẩn bị lâu hơn. Lớn hơn thì thắng.
    /// Đây là thứ mà người chơi có thể nhìn thấy và tính toán — khác hẳn với
    /// `EntityId`, thứ mà không ai nhìn thấy.
    pub priority: i64,
    /// Khóa ổn định, chỉ để phá hòa ở bước cuối.
    pub key: StableKey,
}

/// Kết quả giải quyết một nhóm tranh chấp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Đề xuất được thực hiện.
    ///
    /// `None` khi **mọi** đề xuất trong nhóm đều bị một tầng trước vô hiệu hóa.
    /// Đó là một kết quả hợp lệ, không phải một nhóm rỗng cần bỏ qua: chuỗi
    /// nhân quả phải trả lời được câu *"vì sao không có gì xảy ra"*, và câu trả
    /// lời nằm ở danh sách `losers` với lý do [`LossReason::Nullified`].
    pub winner: Option<Contention>,
    /// Những đề xuất bị gạt, và vì sao.
    pub losers: Vec<(Contention, LossReason)>,
}

impl Outcome {
    /// Có đề xuất nào được thực hiện không.
    pub fn happened(&self) -> bool {
        self.winner.is_some()
    }
}

/// Vì sao một đề xuất bị gạt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossReason {
    /// Thua về điểm phân định — **quan sát được**.
    LowerPriority,
    /// Bằng điểm, thua ở khóa phá hòa.
    ///
    /// Đây là lý do duy nhất mà `EntityId` được dùng, và nó chỉ xuất hiện khi
    /// mọi thứ luật quan tâm đã bằng nhau. Nếu lý do này xuất hiện thường
    /// xuyên, đó là dấu hiệu `priority` chưa phân định đủ.
    TieBreak,
    /// Bị một tầng chạy trước vô hiệu hóa.
    Nullified,
}

/// Giải quyết mọi tranh chấp cùng tick.
///
/// Chạy theo tầng; trong mỗi tầng, nhóm theo mục tiêu; trong mỗi nhóm, chọn
/// theo `priority` rồi mới tới khóa ổn định.
pub fn resolve_all(mut items: Vec<Contention>) -> Vec<Outcome> {
    // Sắp theo `(tier, target, -priority, key)`. Toàn phần, và mọi vế trừ vế
    // cuối đều là dữ liệu của thế giới.
    items.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then(a.target.cmp(&b.target))
            .then(b.priority.cmp(&a.priority))
            .then(a.key.cmp(&b.key))
    });

    let mut ket_qua: Vec<Outcome> = Vec::new();
    let mut da_vo_hieu: std::collections::BTreeSet<EntityId> = std::collections::BTreeSet::new();

    let mut i = 0;
    while i < items.len() {
        let tier = items[i].tier;
        let target = items[i].target.clone();

        // Gom nhóm cùng tầng cùng mục tiêu.
        let mut j = i;
        while j < items.len() && items[j].tier == tier && items[j].target == target {
            j += 1;
        }
        let nhom = &items[i..j];
        i = j;

        // Loại những ai đã bị tầng trước vô hiệu hóa.
        let (con_hieu_luc, bi_vo_hieu): (Vec<_>, Vec<_>) = nhom
            .iter()
            .cloned()
            .partition(|c| !da_vo_hieu.contains(&c.actor));

        let mut losers: Vec<(Contention, LossReason)> = bi_vo_hieu
            .into_iter()
            .map(|c| (c, LossReason::Nullified))
            .collect();

        let winner = match con_hieu_luc.split_first() {
            Some((thang, thua)) => {
                // Ai bị tầng `Nullify` chạm tới thì mất mọi hành động sau đó.
                if tier == Tier::Nullify {
                    if let Some(t) = &thang.target {
                        if let Ok(id) = t.parse::<u64>() {
                            da_vo_hieu.insert(EntityId(id));
                        }
                    }
                }
                losers.extend(thua.iter().map(|c| {
                    let ly_do = if c.priority < thang.priority {
                        LossReason::LowerPriority
                    } else {
                        LossReason::TieBreak
                    };
                    (c.clone(), ly_do)
                }));
                Some(thang.clone())
            }
            // Cả nhóm bị vô hiệu hóa. Vẫn ghi lại — xem tài liệu của `Outcome`.
            None if losers.is_empty() => continue,
            None => None,
        };

        ket_qua.push(Outcome { winner, losers });
    }

    ket_qua
}

/// Có bao nhiêu tranh chấp phải dùng tới khóa phá hòa.
///
/// Công cụ chẩn đoán, không phải một phần của mô phỏng. Tỉ lệ cao nghĩa là
/// `priority` chưa phân định đủ, và thế giới đang quyết định bằng một luật vô
/// hình nhiều hơn mức nên có.
pub fn tiebreak_ratio(outcomes: &[Outcome]) -> (usize, usize) {
    let tong: usize = outcomes.iter().map(|o| o.losers.len()).sum();
    let hoa = outcomes
        .iter()
        .flat_map(|o| &o.losers)
        .filter(|(_, r)| *r == LossReason::TieBreak)
        .count();
    (hoa, tong)
}
