//! Quan hệ suy ra từ ký ức, không lưu riêng (`idea.md §3.2`).
//!
//! ## Vì sao không có một trường `Bond` lưu sẵn trong `Recollection`
//!
//! Cách làm hiển nhiên là thêm `BTreeMap<EntityId, Bond>` cạnh
//! [`crate::Recollection`] và cộng dồn vào đó mỗi khi có chuyện xảy ra —
//! đúng như `mow_society::social::SocialState` đã làm cho quan hệ xã hội tầng
//! trên. Với thứ *đó* điều đó hợp lý: `SocialState` theo dõi những trao đổi
//! đã **thương lượng xong** (`Exchange`/`Volition`), và một cuộc thương
//! lượng, một khi xong, không còn lý do gì để nhớ lại nó đã diễn ra thế nào.
//!
//! Ký ức thì khác. Nếu quan hệ ở đây cũng là một con số lưu riêng, nó sẽ
//! **trôi khỏi** những gì thật sự đã xảy ra: sửa một dòng cộng dồn sai, một
//! bản save cũ phục hồi thiếu một bước cập nhật, một handler quên gọi hàm
//! cộng dồn — bất kỳ cái nào trong số đó để lại một con số quan hệ không ai
//! còn giải thích được. Và khi người chơi hỏi "vì sao hai người này ghét
//! nhau", câu trả lời phải luôn tồn tại và luôn tính lại được giống hệt lần
//! trước — đó là thứ toàn bộ trò chơi này được dựng để làm được. Một
//! [`Bond`] **là** một hàm thuần của [`crate::Recollection`]
//! ([`bond_of`]): hỏi lại bất cứ lúc nào cũng ra đúng câu trả lời cũ, và câu
//! trả lời luôn trỏ thẳng được về những `Memory` cụ thể đã tạo ra nó.
//!
//! Cái giá: `bond_of` phải duyệt lại [`crate::Recollection::about`] mỗi lần
//! gọi, thay vì đọc một trường đã tính sẵn. Với trần
//! [`crate::MEMORY_CAP`] là 64 ký ức mỗi sổ, đây là một vòng lặp ngắn, không
//! phải một chi phí đáng lo.

use crate::memory::{MemoryKind, STRENGTH_MAX};
use crate::recollection::Recollection;
use mow_core::EntityId;
use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Quan hệ **suy ra** từ ký ức về một người — không phải trạng thái lưu
/// riêng. Xem tài liệu module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bond {
    /// Tin tưởng, `-1000`..=`1000`. Chủ yếu tới từ những lần thấy người kia
    /// làm việc đều đặn hay chủ động bắt chuyện — hành vi nói lên độ đáng
    /// tin, không phải một lời tự nhận.
    pub trust: i32,
    /// Thiện cảm, `-1000`..=`1000`. Tới từ giao tiếp và những lần chạm mặt
    /// tích lũy dần.
    pub warmth: i32,
    /// Mức độ quen biết/nhớ về người này, `-1000`..=`1000`, bất kể quý hay
    /// ghét. Một người bị thần can thiệp ngay trước mắt bạn là người bạn
    /// **nhớ rất rõ**, dù chuyện đó không nói lên bạn quý hay ghét họ.
    pub familiarity: i32,
}

impl CanonicalHash for Bond {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(i64::from(self.trust));
        h.write_i64(i64::from(self.warmth));
        h.write_i64(i64::from(self.familiarity));
    }
}

/// Trọng số một [`MemoryKind`] đóng góp vào `(trust, warmth, familiarity)`,
/// tính trên mỗi `1000` đơn vị `strength` (tức trọng số này là kết quả khi
/// một ký ức loại đó còn nguyên sức mạnh tối đa).
///
/// Thang điểm chọn sao cho một vài lần tương tác **có ý nghĩa** (vài lần nói
/// chuyện, vài lần thấy làm việc) đã đủ đẩy `trust`/`warmth` qua
/// [`crate::HELP_THRESHOLD`], mà không cần lấp đầy cả [`crate::MEMORY_CAP`]
/// mới thấy khác biệt — một mối quan hệ hình thành trong vài lần gặp, không
/// phải sau sáu mươi tư lần.
///
/// # Vì sao không có trọng số âm nào ở đây
///
/// Không phải vì quan hệ trong crate này không thể xấu đi — [`Bond`] cho
/// phép cả ba trường xuống tới `-1000`. Lý do là [`crate::MemoryKind`] hiện
/// tại không có biến thể nào ứng với một sự kiện **xấu** giữa hai người, vì
/// engine chưa phát sự kiện nào như vậy (xem tài liệu ở
/// [`crate::MemoryKind`] về `Helped`/`Refused`/`Quarrelled` đã bị bỏ khỏi
/// tập ký ức). Khi engine có một sự kiện như vậy, thêm dòng trọng số âm cho
/// biến thể mới ở đây là đủ — [`bond_of`] không cần sửa, vì nó chỉ cộng dồn
/// những gì bảng này trả về.
fn contribution(kind: &MemoryKind) -> (i32, i32, i32) {
    match kind {
        // Chạm mặt thoáng qua: chủ yếu là "tôi biết mặt người này", gần như
        // không nói lên gì về tin tưởng hay thiện cảm.
        MemoryKind::Met => (10, 20, 40),
        // Một dấu hiệu mơ hồ (`sign.*`): chỉ đóng góp vào familiarity — dấu
        // hiệu có thể tốt hay xấu, và ta không diễn giải valence ở tầng
        // MemoryKind (xem tài liệu ở đó), nên không gán trust/warmth.
        MemoryKind::Saw { .. } => (0, 0, 20),
        // Thấy làm việc đều: hành vi nói lên độ đáng tin (chăm chỉ, có mặt
        // đúng chỗ) nhiều hơn là nói lên thiện cảm.
        MemoryKind::Worked => (30, 10, 20),
        // Ăn: quan sát sinh tồn thuần túy, gần như trung tính.
        MemoryKind::Ate => (0, 10, 10),
        // Di chuyển: chỉ xác nhận "tôi có để ý người này đi đâu", không nói
        // lên tính cách gì.
        MemoryKind::Moved => (0, 0, 10),
        // Nói chuyện: chủ động giao tiếp xây thiện cảm nhiều hơn tin tưởng.
        MemoryKind::Spoke => (10, 30, 20),
        // Thần can thiệp: một chuyện không ai quên, nhưng không tự nói lên
        // người đó đáng tin hay dễ mến — chỉ đẩy familiarity lên mạnh.
        MemoryKind::Intervened => (0, 0, 60),
    }
}

/// Quan hệ hiện tại với `other`, suy ra từ mọi ký ức về người đó.
///
/// # Vì sao nhân trước rồi mới chia một lần
///
/// Đây là lỗi đã lặp lại ba lần trong dự án này ở những miền khác (tốc độ,
/// tỉ lệ đột biến, khoang SEIR): nếu mỗi ký ức tự chia lấy phần đóng góp của
/// nó (`trong_so * strength / 1000`) rồi mới cộng dồn, một loạt ký ức yếu
/// (`strength` nhỏ, `trong_so` nhỏ) sẽ **mỗi cái đều làm tròn về `0`** trước
/// khi kịp cộng — dù gộp lại chúng đủ tạo ra một quan hệ có ý nghĩa. Hàm này
/// cộng dồn toàn bộ tích `trong_so * strength` trước, rồi mới chia đúng một
/// lần cho `1000` ở cuối.
#[must_use]
pub fn bond_of(r: &Recollection, other: EntityId) -> Bond {
    let mut trust_acc: i64 = 0;
    let mut warmth_acc: i64 = 0;
    let mut fam_acc: i64 = 0;
    for m in r.about(other) {
        let (dt, dw, df) = contribution(&m.kind);
        let suc_manh = i64::from(m.strength);
        trust_acc += i64::from(dt) * suc_manh;
        warmth_acc += i64::from(dw) * suc_manh;
        fam_acc += i64::from(df) * suc_manh;
    }
    Bond {
        trust: chia_va_kep(trust_acc),
        warmth: chia_va_kep(warmth_acc),
        familiarity: chia_va_kep(fam_acc),
    }
}

/// Chia một lần cho `1000` rồi kẹp về khoảng hợp lệ của [`Bond`].
///
/// # Vì sao kẹp ở đây là một đường chạy **thật**, không chỉ lưới an toàn
///
/// Với trọng số tối đa hiện tại (`60`, ở [`MemoryKind::Intervened`]) và trần
/// [`crate::MEMORY_CAP`] (`64`), tổng trước khi chia có thể tới
/// `64 * 60 * 1000 = 3_840_000`, tức sau khi chia tới `3840` — vượt xa `1000`.
/// Một người từng chứng kiến hàng chục lần thần can thiệp lên cùng một người
/// thật sự nên có `familiarity` chạm trần, nên việc kẹp ở đây không phải bù
/// cho một trường hợp không tưởng — nó là điểm bão hòa có chủ đích của thang
/// điểm: quen tới mức tối đa vẫn chỉ là tối đa, không tràn ra ngoài ý nghĩa
/// của chính con số.
fn chia_va_kep(acc: i64) -> i32 {
    let gioi_han = i64::from(STRENGTH_MAX);
    i32::try_from((acc / gioi_han).clamp(-gioi_han, gioi_han)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryKind};
    use mow_core::EntityId;

    fn eid(n: u64) -> EntityId {
        EntityId::new(n)
    }

    #[test]
    fn empty_recollection_is_the_default_bond() {
        let r = Recollection::new();
        assert_eq!(bond_of(&r, eid(1)), Bond::default());
    }

    #[test]
    fn bond_of_is_deterministic() {
        let mut r = Recollection::new();
        r.witness(Memory::new(eid(2), MemoryKind::Worked, 0, 800, None));
        r.witness(Memory::new(eid(2), MemoryKind::Spoke, 1, 600, None));
        assert_eq!(bond_of(&r, eid(2)), bond_of(&r, eid(2)));
    }

    #[test]
    fn only_memories_about_the_target_count() {
        let mut r = Recollection::new();
        r.witness(Memory::new(eid(2), MemoryKind::Intervened, 0, 1000, None));
        // Chuyện của người khác không được rò sang bond của eid(2).
        r.witness(Memory::new(eid(3), MemoryKind::Intervened, 0, 1000, None));
        let b2 = bond_of(&r, eid(2));
        let stranger = bond_of(&r, eid(9));
        assert_ne!(b2, Bond::default());
        assert_eq!(stranger, Bond::default());
    }

    /// Bài khóa lại chính lỗi "chia trước khi cộng dồn": nhiều ký ức yếu, mỗi
    /// cái nếu tự chia riêng sẽ làm tròn về `0`, nhưng cộng dồn đúng cách
    /// phải cho một `familiarity` khác `0`.
    #[test]
    fn many_weak_memories_still_accumulate_instead_of_rounding_to_zero() {
        let mut r = Recollection::new();
        // Mỗi ký ức Met có trọng số familiarity = 40; với strength = 10,
        // `40 * 10 / 1000 = 0.4` chia riêng lẻ mỗi lần vẫn làm tròn xuống
        // `0`. Nhưng 20 ký ức như vậy cộng dồn *trước khi chia*
        // (`20 * 40 * 10 / 1000 = 8`) phải ra một giá trị dương — đây chính
        // là bài kiểm chứng "nhân trước, chia một lần" ở tài liệu [`bond_of`].
        for i in 0..20u64 {
            r.witness(Memory::new(eid(2), MemoryKind::Met, i, 10, None));
        }
        let b = bond_of(&r, eid(2));
        assert!(
            b.familiarity > 0,
            "20 ky uc yeu cong don phai cho familiarity > 0, duoc {}",
            b.familiarity
        );
    }

    #[test]
    fn intervened_builds_more_familiarity_than_a_single_meeting() {
        let mut than = Recollection::new();
        than.witness(Memory::new(eid(2), MemoryKind::Intervened, 0, 1000, None));
        let mut gap = Recollection::new();
        gap.witness(Memory::new(eid(2), MemoryKind::Met, 0, 1000, None));
        assert!(bond_of(&than, eid(2)).familiarity > bond_of(&gap, eid(2)).familiarity);
    }

    #[test]
    fn speaking_builds_more_warmth_than_working() {
        let mut noi = Recollection::new();
        noi.witness(Memory::new(eid(2), MemoryKind::Spoke, 0, 1000, None));
        let mut lam = Recollection::new();
        lam.witness(Memory::new(eid(2), MemoryKind::Worked, 0, 1000, None));
        assert!(bond_of(&noi, eid(2)).warmth > bond_of(&lam, eid(2)).warmth);
    }

    #[test]
    fn working_builds_more_trust_than_speaking() {
        let mut lam = Recollection::new();
        lam.witness(Memory::new(eid(2), MemoryKind::Worked, 0, 1000, None));
        let mut noi = Recollection::new();
        noi.witness(Memory::new(eid(2), MemoryKind::Spoke, 0, 1000, None));
        assert!(bond_of(&lam, eid(2)).trust > bond_of(&noi, eid(2)).trust);
    }

    #[test]
    fn bond_stays_within_documented_range_even_with_a_full_book() {
        let mut r = Recollection::new();
        for i in 0..(crate::MEMORY_CAP as u64) {
            r.witness(Memory::new(eid(2), MemoryKind::Intervened, i, 1000, None));
        }
        let b = bond_of(&r, eid(2));
        assert!((-1000..=1000).contains(&b.trust));
        assert!((-1000..=1000).contains(&b.warmth));
        assert!((-1000..=1000).contains(&b.familiarity));
    }
}
