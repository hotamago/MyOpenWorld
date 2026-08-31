//! Ký ức đổi hành vi (`idea.md §3.2`, cuối vòng lặp: "điều kiện mới").
//!
//! Đây là mắt khép vòng lặp mà tài liệu thiết kế đặt ra: ký ức và quan hệ
//! ([`crate::bond_of`]) không có ích gì nếu không có gì đọc chúng để ra quyết
//! định khác đi. Hai hàm ở đây **thuần** như `mow_society::routine::decide`
//! — không `Sim`, không RNG, không I/O — để lớp gọi ghép chúng vào bất kỳ
//! vòng quyết định nào (kể cả `mow_society::routine::Situation`, nếu người
//! nối dây muốn cho `nearest`/`with` một giá trị có căn cứ thay vì luôn
//! `None`).

use crate::bond::bond_of;
use crate::recollection::Recollection;
use mow_core::EntityId;
use std::cmp::Reverse;

/// Ngưỡng `trust + warmth` để [`would_help`] trả `true`.
///
/// # Vì sao ngưỡng này, không phải `0`
///
/// Ngưỡng `0` sẽ khiến một người xa lạ (`Bond::default()`, mọi trường bằng
/// `0`) nhận được cùng câu trả lời với một người mới chỉ gặp một lần ở giếng
/// (một [`crate::MemoryKind::Met`] duy nhất cộng vài điểm thiện cảm/tin
/// tưởng nhỏ giọt) — không phân biệt được "chưa từng biết" với "vừa mới
/// quen". `150` đòi hỏi nhiều hơn một lần chạm mặt thoáng qua (một `Met` ở
/// sức mạnh tối đa chỉ góp `trust + warmth = 30`), nhưng chỉ vài lần trò
/// chuyện thật sự (`Spoke` góp `40` mỗi lần) là đủ vượt ngưỡng — không đòi
/// hỏi một tình bạn lâu năm.
/// Đây là một quyết định gameplay, không phải hằng số vật lý — giống
/// `mow_society::routine::HUNGER_URGENT` ở crate hàng xóm — và có thể chỉnh
/// theo trải nghiệm mong muốn mà không đổi tính đúng của hàm.
pub const HELP_THRESHOLD: i32 = 150;

/// Có nên giúp người này không, theo những gì đã nhớ.
///
/// # Vì sao dùng `trust + warmth`, không dùng riêng một trường
///
/// Giúp đỡ cần cả hai: tin tưởng thuần túy không kèm thiện cảm (một người
/// làm việc chăm chỉ nhưng lạnh lùng, chưa từng trò chuyện) vẫn đủ để đứng ra
/// giúp — và ngược lại, thiện cảm không kèm tin tưởng (một người dễ mến
/// nhưng chưa ai biết có đáng tin không) cũng vậy. Cộng hai trường cho phép
/// một trong hai đủ mạnh **bù** cho trường còn lại yếu, thay vì đòi cả hai
/// cùng cao — khớp với trực giác rằng có nhiều đường khác nhau để trở nên
/// đáng được giúp.
///
/// `familiarity` cố tình **không** tham gia: quen mặt (kể cả quen rất rõ,
/// như từng chứng kiến ai đó bị thần can thiệp) không tự nó là lý do để giúp
/// — xem tài liệu [`crate::bond::contribution`] về việc `Intervened` chỉ đẩy
/// `familiarity`, không đẩy `trust`/`warmth`, đúng vì valence của nó không
/// biết được.
#[must_use]
pub fn would_help(r: &Recollection, who: EntityId) -> bool {
    let b = bond_of(r, who);
    b.trust + b.warmth >= HELP_THRESHOLD
}

/// Ai trong số những người quanh đây là người muốn nói chuyện cùng nhất.
///
/// `None` khi `nearby` rỗng — không có ai thì không có ai để chọn, và đây
/// không phải một trường hợp lỗi (xem `mow_society::routine::Intent::Idle`
/// cho một quyết định thiết kế tương tự: không có việc gì đáng làm là một
/// kết quả hợp lệ, không phải một khoảng trống cần lấp).
///
/// # Vì sao hòa điểm phải kẹp về một thứ tự cố định
///
/// `nearby` là một lát cắt do người gọi đưa vào theo bất kỳ thứ tự nào (ví
/// dụ thứ tự duyệt `Store` của lớp không gian) — thứ tự đó **không** phải một
/// phần của trạng thái thế giới, nên hai lời gọi với cùng một tập người
/// nhưng liệt kê khác thứ tự bắt buộc phải ra cùng một câu trả lời. Vì vậy
/// khi hai người hòa điểm ưu tiên, hàm chọn người có [`EntityId`] nhỏ hơn —
/// một quy tắc tùy ý nhưng **cố định**, độc lập với thứ tự của `nearby`.
#[must_use]
pub fn preferred_company(r: &Recollection, nearby: &[EntityId]) -> Option<EntityId> {
    nearby
        .iter()
        .copied()
        .map(|id| {
            let b = bond_of(r, id);
            (b.trust + b.warmth, id)
        })
        // `Reverse(id)`: `max_by_key` giữ phần tử **cuối** khi hòa điểm
        // (đã tài liệu trong `core::iter::Iterator::max_by_key`), nên đảo
        // chiều so sánh trên `id` biến "cuối theo Reverse" thành "id nhỏ
        // nhất" — kết quả không phụ thuộc thứ tự `nearby` đưa vào.
        .max_by_key(|&(score, id)| (score, Reverse(id)))
        .map(|(_, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryKind};

    fn eid(n: u64) -> EntityId {
        EntityId::new(n)
    }

    #[test]
    fn preferred_company_with_nobody_nearby_is_none() {
        let r = Recollection::new();
        assert_eq!(preferred_company(&r, &[]), None);
    }

    #[test]
    fn preferred_company_picks_the_strongest_bond() {
        let mut r = Recollection::new();
        r.witness(Memory::new(eid(2), MemoryKind::Spoke, 0, 1000, None));
        // eid(3) khong co ky uc nao ca -> bond mac dinh, yeu hon.
        let nearby = [eid(3), eid(2)];
        assert_eq!(preferred_company(&r, &nearby), Some(eid(2)));
    }

    #[test]
    fn preferred_company_tie_break_is_the_smallest_id_regardless_of_input_order() {
        let r = Recollection::new();
        // Ca hai deu la nguoi la: bond mac dinh, hoa diem tuyet doi.
        assert_eq!(preferred_company(&r, &[eid(5), eid(2)]), Some(eid(2)));
        assert_eq!(
            preferred_company(&r, &[eid(2), eid(5)]),
            Some(eid(2)),
            "doi thu tu dau vao khong duoc doi ket qua"
        );
    }

    #[test]
    fn preferred_company_ignores_strangers_not_in_nearby() {
        let mut r = Recollection::new();
        r.witness(Memory::new(eid(99), MemoryKind::Intervened, 0, 1000, None));
        // eid(99) manh nhat nhung khong o gan, khong duoc chon.
        assert_eq!(preferred_company(&r, &[eid(1)]), Some(eid(1)));
    }

    #[test]
    fn would_help_is_false_for_a_total_stranger() {
        let r = Recollection::new();
        assert!(!would_help(&r, eid(1)));
    }

    #[test]
    fn would_help_is_false_after_a_single_fleeting_encounter() {
        let mut r = Recollection::new();
        r.witness(Memory::new(eid(1), MemoryKind::Met, 0, 1000, None));
        assert!(
            !would_help(&r, eid(1)),
            "mot lan cham mat khong du de duoc giup"
        );
    }

    #[test]
    fn would_help_becomes_true_after_enough_positive_history() {
        let mut r = Recollection::new();
        for i in 0..5u64 {
            r.witness(Memory::new(eid(1), MemoryKind::Spoke, i, 1000, None));
        }
        assert!(
            would_help(&r, eid(1)),
            "nam lan noi chuyen manh phai du de duoc giup"
        );
    }

    #[test]
    fn would_help_does_not_count_familiarity_alone() {
        let mut r = Recollection::new();
        // Intervened chi day familiarity, khong day trust/warmth (xem
        // `bond::contribution`) — du manh bao nhieu cung khong nen du de
        // duoc giup theo tieu chi hien tai.
        for i in 0..10u64 {
            r.witness(Memory::new(eid(1), MemoryKind::Intervened, i, 1000, None));
        }
        assert!(!would_help(&r, eid(1)));
    }
}
