//! Sổ ký ức có trần của một cá thể (`idea.md §1.2.2`, `§3.2`).
//!
//! [`Recollection`] là **toàn bộ** những gì một người còn nhớ. Nó không biết
//! gì về `Store`, `Sim`, hay thời gian thực — chỉ biết cộng dồn
//! [`crate::Memory`] qua [`Recollection::witness`]/[`Recollection::hear`], và
//! làm chúng mờ đi qua [`Recollection::decay`]. Mọi hành vi đọc được từ ký ức
//! ([`crate::bond_of`], [`crate::preferred_company`], [`crate::would_help`])
//! chỉ được phép đi qua [`Recollection::about`] — không có đường nào khác để
//! lấy dữ liệu ra, giống hệt cách `mow_action::perception::CognitionContext`
//! là cửa duy nhất vào tri giác.

use crate::memory::{Memory, STRENGTH_MAX, STRENGTH_MIN};
use mow_core::EntityId;
use mow_math::rng::streams;
use mow_math::{CanonicalHash, RngStreams, StateHasher, WorldSeed};
use serde::{Deserialize, Serialize};

/// Số ký ức tối đa một [`Recollection`] giữ cùng lúc.
///
/// # Vì sao phải có trần
///
/// Một ván chơi dài chạy hàng trăm nghìn tick; nếu mỗi lần chạm mặt ai đó đều
/// thêm một [`Memory`] không giới hạn, sổ ký ức của một cư dân sống lâu sẽ
/// phình vô hạn — đúng nghĩa rò bộ nhớ, chỉ khác nó rò dữ liệu gameplay thay
/// vì rò cấp phát. `64` đủ để nhớ vài chục người quen biết gần, chuyện xảy ra
/// gần đây, và các sự kiện mạnh còn đọng lại — nhiều hơn một người bình
/// thường thực sự "nhớ chi tiết" về những người quanh mình.
///
/// # Vì sao đầy thì quên cái **yếu nhất**, không phải cái **cũ nhất**
///
/// Cái cũ nhất có thể là sự kiện mạnh nhất đời một người — bị thần can thiệp
/// năm xưa không nên biến mất chỉ vì có một trăm lần chạm mặt tầm thường xảy
/// ra sau đó. Cái yếu nhất luôn là thứ ít ảnh hưởng nhất tới
/// [`crate::bond_of`] ngay lúc bị bỏ, bất kể nó mới hay cũ — đó chính là thứ
/// ta muốn đánh đổi khi hết chỗ.
pub const MEMORY_CAP: usize = 64;

/// Sổ ký ức của một cá thể. Xem tài liệu module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recollection {
    /// `Vec`, không phải map: thứ tự là thứ tự ghi nhận, và chính thứ tự đó
    /// quyết định ai thắng khi hai ký ức hòa điểm sức mạnh lúc dọn trần (xem
    /// [`Recollection::evict_weakest`]). Một `HashMap` sẽ xóa mất thứ tự đó
    /// và làm việc dọn trần phụ thuộc bố cục bộ nhớ của lần chạy — chính xác
    /// điều `§P10.2.1` cấm.
    memories: Vec<Memory>,
}

impl Recollection {
    /// Sổ rỗng.
    #[must_use]
    pub fn new() -> Recollection {
        Recollection::default()
    }

    /// Ghi một ký ức **tự chứng kiến**.
    ///
    /// Không chiết khấu gì cả — tự mắt thấy là dạng bằng chứng mạnh nhất mà
    /// một cá thể có thể có. Nếu người gọi có ý muốn một quan sát mờ (sương
    /// mù, xa, tiếng động không rõ nguồn), việc chiết khấu `strength` phải
    /// làm **trước** khi gọi hàm này, ngay khi dựng `Memory` — xem
    /// `mow_action::perception::Observation::fidelity` ở crate hàng xóm cho
    /// một ví dụ độ tin cậy đã có sẵn.
    pub fn witness(&mut self, m: Memory) {
        self.insert(m);
    }

    /// Ghi một ký ức **nghe kể** từ `from`, với độ tin `trust` (kẹp về
    /// `0..=1000`).
    ///
    /// # Vì sao nghe kể luôn yếu hơn và có thể sai
    ///
    /// Ba việc xảy ra ở đây mà [`Recollection::witness`] không làm, và cả ba
    /// đều là hệ quả trực tiếp của "người khác đã diễn giải hộ mình"
    /// (`idea.md §3.2`):
    ///
    /// 1. **Sức mạnh bị chiết khấu theo `trust`.** Tin một người kể dở thì
    ///    chuyện họ kể cũng chỉ đọng lại yếu ớt trong đầu mình.
    /// 2. **`tick` có thể lệch.** Người kể nhớ nhầm, hoặc mình nghe không kỹ.
    ///    Độ lệch tăng khi `trust` giảm (chi tiết ở hàm nội bộ tính biên độ
    ///    lệch, xem module `rumor` trong mã nguồn).
    /// 3. **`source` luôn thành `None`**, bất kể `m.source` trước đó là gì.
    ///    Người nghe không tự tay đọc được `EventSeq` mà `from` nhắc tới; giữ
    ///    nguyên `source` sẽ ngầm cho phép một lời kể tự xưng "tôi thấy tận
    ///    mắt sự kiện #482" đi thẳng vào sổ của người khác như thể **họ** đã
    ///    kiểm chứng được `EventSeq` đó — trong khi họ chỉ nghe kể.
    ///
    /// **Điều `hear` không làm**: đổi `kind` hay `about`. Một tin đồn có thể
    /// làm người ta ít chắc chuyện đã xảy ra tới đâu (`strength`) và khi nào
    /// (`tick`), nhưng không tự bịa ra một chuyện khác hẳn — đó là ranh giới
    /// giữa "méo" và "bịa" mà đề bài yêu cầu.
    pub fn hear(&mut self, m: Memory, from: EntityId, trust: i32) {
        let da_meo = rumor::distort(m, from, trust);
        self.insert(da_meo);
    }

    /// Cho mọi ký ức mờ đi `ticks` tick.
    ///
    /// Ký ức đã mờ hẳn (`strength == 0`) bị dọn khỏi sổ ngay trong lần gọi
    /// này. Không giữ chúng lại "cho đủ": một ký ức sức mạnh `0` không còn
    /// ảnh hưởng gì tới [`crate::bond_of`] hay trần [`MEMORY_CAP`], nên giữ
    /// nó chỉ tổ chiếm một chỗ mà một ký ức mới, còn ý nghĩa, xứng đáng có
    /// hơn.
    pub fn decay(&mut self, ticks: u64) {
        for m in &mut self.memories {
            m.decay(ticks);
        }
        self.memories.retain(|m| m.strength > STRENGTH_MIN);
    }

    /// Mọi ký ức về một người, theo đúng thứ tự đã ghi nhận.
    pub fn about(&self, who: EntityId) -> impl Iterator<Item = &Memory> {
        self.memories.iter().filter(move |m| m.about == who)
    }

    /// Số ký ức đang giữ (sau khi đã dọn ký ức mờ hẳn ở lần `decay` gần nhất).
    #[must_use]
    pub fn len(&self) -> usize {
        self.memories.len()
    }

    /// Sổ có đang rỗng không.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
    }

    /// Thêm một ký ức, dọn trần nếu cần.
    ///
    /// Luôn thêm trước rồi mới xét trần — không xét trần trước rồi mới quyết
    /// định có thêm không. Nhờ vậy [`Recollection::evict_weakest`] so sánh
    /// được **cả** ký ức vừa tới lẫn những ký ức đã có: một ký ức mới nhưng
    /// yếu hơn tất cả những gì đã có (ví dụ một tin đồn bị chiết khấu gần hết)
    /// hoàn toàn có thể là chính thứ bị dọn ngay sau khi vừa được thêm vào —
    /// và đó là hành vi đúng, không phải một trường hợp biên bị bỏ sót.
    fn insert(&mut self, m: Memory) {
        self.memories.push(m);
        if self.memories.len() > MEMORY_CAP {
            self.evict_weakest();
        }
    }

    /// Bỏ đúng một ký ức: cái yếu nhất trong sổ.
    ///
    /// Hòa điểm sức mạnh thì bỏ ký ức **xuất hiện trước** trong `memories`
    /// (`Vec::iter().enumerate().min_by_key` trả phần tử đầu tiên khi hòa —
    /// đây là hành vi đã tài liệu của thư viện chuẩn, không phải một giả định
    /// ngầm). Vì [`Recollection::insert`] luôn đẩy ký ức mới vào cuối, hòa
    /// điểm nghĩa là **ký ức cũ hơn bị bỏ, ký ức mới giữ lại** — một thiên vị
    /// nhẹ về phía thông tin gần đây, nhất quán với trực giác "hai chuyện
    /// nhạt như nhau thì nhớ chuyện mới hơn".
    fn evict_weakest(&mut self) {
        let Some((idx, _)) = self
            .memories
            .iter()
            .enumerate()
            .min_by_key(|(_, m)| m.strength)
        else {
            return;
        };
        self.memories.remove(idx);
    }
}

impl CanonicalHash for Recollection {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.memories.iter(), |hh, m| m.canonical_hash(hh));
    }
}

/// Méo tin đồn — tách riêng để [`Recollection::hear`] đọc như một câu chuyện,
/// không phải một khối toán.
mod rumor {
    use super::{
        CanonicalHash, EntityId, Memory, RngStreams, StateHasher, WorldSeed, STRENGTH_MAX,
        STRENGTH_MIN,
    };
    use rand::Rng;

    /// Biên độ lệch tick tối đa một lần kể có thể gây ra, theo độ tin `trust`
    /// (đã kẹp `0..=1000`) dành cho người kể.
    ///
    /// Tin tuyệt đối (`trust = 1000`) cho biên độ `0`: tin ai đó tuyệt đối
    /// nghĩa là tin luôn cả thời điểm họ kể, không tự suy diễn thêm nhiễu.
    /// Không tin chút nào (`trust = 0`) cho biên độ `100` tick — đủ để một
    /// chuyện "hôm qua" nghe thành "cách đây cả tuần" sau vài lần truyền tay,
    /// nhưng không đủ để một chuyện hôm nay bị kể thành chuyện năm ngoái. Hệ
    /// số `10` là một lựa chọn gameplay, không phải hằng số vật lý — giống
    /// các ngưỡng khác của dự án này (`HUNGER_URGENT`,
    /// `IDENTIFY_THRESHOLD_PERCENT`), có thể chỉnh mà không đổi tính đúng của
    /// phép méo.
    pub(super) fn max_tick_shift(trust: i32) -> i64 {
        i64::from((STRENGTH_MAX - trust.clamp(0, STRENGTH_MAX)) / 10)
    }

    /// Seed cục bộ cho một lần kể cụ thể — dẫn xuất từ **nội dung** câu
    /// chuyện, không phải từ seed thế giới.
    ///
    /// # Đánh đổi
    ///
    /// Mọi nơi khác trong dự án dẫn `RngStreams` từ [`WorldSeed`] **thật** của
    /// thế giới (`mow-life/src/genome.rs::recombination_seed` là ví dụ gần
    /// nhất). `mow-memory` cố tình không đi đường đó: đây là một thư viện
    /// thuần, chữ ký [`super::Recollection::hear`] mà nhiệm vụ này phải theo
    /// không có chỗ cho một tham số seed thế giới, và thêm một tham số như
    /// vậy sẽ buộc mọi lời gọi tầng trên phải xuyên qua một seed mà bản thân
    /// việc "một tin đồn méo đi bao nhiêu" không có lý do gì phải biết.
    ///
    /// Seed ở đây dẫn từ chính nội dung: ai kể, chuyện về ai, loại ký ức nào,
    /// tin là xảy ra lúc nào, và độ tin — bốn thứ y hệt nhau luôn cho cùng
    /// một seed, nên cùng một lời kể luôn méo giống hệt nhau trên mọi máy.
    /// Đơn vị được đảm bảo tái lập là "cùng nội dung", không phải "cùng thế
    /// giới" — hẹp hơn cách làm thông thường, nhưng đủ cho một hàm thuần
    /// không giữ trạng thái thế giới. Domain riêng (`"mow.memory.rumor_seed.
    /// v1"`) tách không gian seed này khỏi không gian seed thế giới thật, để
    /// hai thứ không bao giờ trùng nhau dù trùng số.
    fn rumor_seed(from: EntityId, m: &Memory, trust: i32) -> WorldSeed {
        let mut h = StateHasher::with_domain("mow.memory.rumor_seed.v1");
        from.canonical_hash(&mut h);
        m.about.canonical_hash(&mut h);
        m.kind.canonical_hash(&mut h);
        h.write_u64(m.tick);
        h.write_i64(i64::from(trust));
        let bytes = h.finish().0;
        let mut tam = [0u8; 8];
        tam.copy_from_slice(&bytes[..8]);
        WorldSeed(u64::from_le_bytes(tam))
    }

    /// Méo một ký ức trước khi nó vào sổ của người nghe.
    ///
    /// Ba việc, không hơn: chiết khấu `strength` theo `trust`, lệch `tick`
    /// một khoảng bị chặn bởi [`max_tick_shift`], và xóa `source`. `kind` và
    /// `about` không đổi — xem lý do ở [`super::Recollection::hear`].
    pub(super) fn distort(m: Memory, from: EntityId, trust: i32) -> Memory {
        let trust = trust.clamp(0, STRENGTH_MAX);

        // Nhân trước, chia một lần: cả hai thừa số đã là số nguyên đầy đủ
        // trước phép chia duy nhất này, nên không có bước làm tròn trung
        // gian nào ăn mất độ chính xác.
        let suc_manh_tho = i64::from(m.strength) * i64::from(trust) / i64::from(STRENGTH_MAX);
        let suc_manh =
            i32::try_from(suc_manh_tho.clamp(0, i64::from(STRENGTH_MAX))).unwrap_or(STRENGTH_MIN);

        let bien_do = max_tick_shift(trust);
        // Tin tuyệt đối thì biên độ bằng 0: bỏ qua RNG hẳn, đúng nghĩa "không
        // suy diễn thêm nhiễu nào" chứ không phải rút một số ngẫu nhiên rồi
        // nó tình cờ bằng 0.
        let lech: i64 = if bien_do == 0 {
            0
        } else {
            let seed = rumor_seed(from, &m, trust);
            let mut rng = seed.rumor_stream();
            rng.gen_range(-bien_do..=bien_do)
        };
        let tick_moi = m.tick.saturating_add_signed(lech);

        Memory::new(m.about, m.kind, tick_moi, suc_manh, None)
    }

    /// Dòng RNG đặt tên `society.message.drift` — đã có sẵn trong
    /// `mow_math::rng::streams` từ trước, chưa ai dùng. Bọc thành một
    /// extension trait để lời gọi tại chỗ dùng đọc gọn
    /// (`seed.rumor_stream()`), thay vì `RngStreams::new(seed).stream(...)`
    /// lặp lại tên dòng bằng tay ở đây.
    pub(super) trait StreamsExt {
        /// Dòng ngẫu nhiên nhiễu tin đồn cho seed này.
        fn rumor_stream(self) -> mow_math::DetRng;
    }

    impl StreamsExt for WorldSeed {
        fn rumor_stream(self) -> mow_math::DetRng {
            RngStreams::new(self).stream(super::streams::SOCIAL_RUMOR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryKind;
    use mow_core::EventSeq;

    fn eid(n: u64) -> EntityId {
        EntityId::new(n)
    }

    fn met(about: EntityId, tick: u64, strength: i32) -> Memory {
        Memory::new(about, MemoryKind::Met, tick, strength, None)
    }

    #[test]
    fn witness_then_about_returns_it() {
        let mut r = Recollection::new();
        r.witness(met(eid(2), 10, 500));
        let found: Vec<&Memory> = r.about(eid(2)).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].strength, 500);
    }

    #[test]
    fn len_and_is_empty_track_witness() {
        let mut r = Recollection::new();
        assert!(r.is_empty());
        r.witness(met(eid(2), 0, 100));
        assert_eq!(r.len(), 1);
        assert!(!r.is_empty());
    }

    #[test]
    fn about_filters_by_entity() {
        let mut r = Recollection::new();
        r.witness(met(eid(2), 0, 100));
        r.witness(met(eid(3), 0, 100));
        r.witness(met(eid(2), 1, 200));
        assert_eq!(r.about(eid(2)).count(), 2);
        assert_eq!(r.about(eid(3)).count(), 1);
        assert_eq!(r.about(eid(9)).count(), 0);
    }

    #[test]
    fn cap_is_never_exceeded() {
        let mut r = Recollection::new();
        for i in 0..(MEMORY_CAP * 3) {
            r.witness(met(eid(1), i as u64, i32::try_from(i % 1000).unwrap_or(0)));
        }
        assert_eq!(r.len(), MEMORY_CAP);
    }

    /// Bài quan trọng nhất của module: trần phải bỏ ký ức **yếu nhất**, kể cả
    /// khi nó cũ hơn nhiều so với ký ức khác.
    #[test]
    fn cap_evicts_weakest_not_oldest() {
        let mut r = Recollection::new();
        // Ký ức đầu tiên, cũ nhất, nhưng rất mạnh — phải sống sót.
        r.witness(met(eid(1), 0, 999));
        for i in 1..MEMORY_CAP {
            r.witness(met(eid(1), i as u64, 500));
        }
        assert_eq!(r.len(), MEMORY_CAP);
        // Thêm một ký ức yếu: phải chính nó bị dọn ngay, không đụng gì khác.
        r.witness(met(eid(1), 9999, 1));
        assert_eq!(r.len(), MEMORY_CAP);
        let manh_nhat = r.about(eid(1)).map(|m| m.strength).max().unwrap_or(0);
        assert_eq!(manh_nhat, 999, "ky uc manh nhat, du cu nhat, phai con");
        assert!(
            r.about(eid(1)).all(|m| m.tick != 9999),
            "ky uc yeu vua them phai la thu bi don"
        );
    }

    #[test]
    fn cap_can_evict_incoming_memory_if_it_is_the_weakest() {
        let mut r = Recollection::new();
        for i in 0..MEMORY_CAP {
            r.witness(met(eid(1), i as u64, 900));
        }
        r.witness(met(eid(1), 12345, 1));
        assert!(
            r.about(eid(1)).all(|m| m.tick != 12345),
            "ky uc moi them nhung yeu nhat phai la thu bi don ngay"
        );
    }

    #[test]
    fn hear_produces_weaker_memory_than_an_equivalent_witness() {
        let mut chung_kien = Recollection::new();
        chung_kien.witness(met(eid(2), 100, 800));

        let mut nghe_ke = Recollection::new();
        nghe_ke.hear(met(eid(2), 100, 800), eid(9), 500);

        let a = chung_kien
            .about(eid(2))
            .next()
            .expect("phai co dung mot ky uc");
        let b = nghe_ke
            .about(eid(2))
            .next()
            .expect("phai co dung mot ky uc");
        assert!(
            b.strength < a.strength,
            "nghe ke phai yeu hon chung kien cung noi dung"
        );
    }

    #[test]
    fn hear_always_clears_source() {
        let mut r = Recollection::new();
        let goc = Memory::new(eid(2), MemoryKind::Worked, 5, 900, Some(EventSeq(7)));
        r.hear(goc, eid(9), 900);
        let nghe = r.about(eid(2)).next().expect("phai co dung mot ky uc");
        assert_eq!(nghe.source, None, "nghe ke khong bao gio giu duoc EventSeq");
    }

    #[test]
    fn hear_never_changes_kind_or_about() {
        let mut r = Recollection::new();
        r.hear(met(eid(2), 5, 900), eid(9), 300);
        let nghe = r.about(eid(2)).next().expect("phai co dung mot ky uc");
        assert_eq!(nghe.kind, MemoryKind::Met);
        assert_eq!(nghe.about, eid(2));
    }

    #[test]
    fn full_trust_hearsay_never_shifts_the_tick() {
        let mut r = Recollection::new();
        r.hear(met(eid(2), 500, 900), eid(9), 1000);
        let nghe = r.about(eid(2)).next().expect("phai co dung mot ky uc");
        assert_eq!(nghe.tick, 500, "tin tuyet doi thi khong tu suy dien nhieu");
    }

    #[test]
    fn low_trust_hearsay_tick_stays_within_the_documented_bound() {
        let mut r = Recollection::new();
        r.hear(met(eid(2), 500, 900), eid(9), 0);
        let nghe = r.about(eid(2)).next().expect("phai co dung mot ky uc");
        let lech = (i64::try_from(nghe.tick).unwrap_or(i64::MAX) - 500).abs();
        assert!(
            lech <= rumor::max_tick_shift(0),
            "do lech {lech} vuot bien do cho phep"
        );
    }

    /// Tin đồn qua ba người: méo cộng dồn (yếu dần), nhưng không bịa — loại
    /// chuyện và người liên quan phải giữ nguyên suốt chuỗi, và độ lệch tick
    /// tổng cộng vẫn phải nằm trong một biên hợp lý, không nhảy tự do.
    #[test]
    fn gossip_through_three_people_distorts_but_never_fabricates() {
        let goc = met(eid(2), 1000, 1000);

        let mut b = Recollection::new();
        b.hear(goc.clone(), eid(1), 700);
        let cho_b = b
            .about(eid(2))
            .next()
            .cloned()
            .expect("phai co dung mot ky uc");

        let mut c = Recollection::new();
        // `eid(1)` lặp lại ở mỗi hop chỉ vì bài test không quan tâm ai là
        // người kể — `distort` chỉ dùng `from` để trộn vào seed, không đối
        // chiếu với người kể trước đó.
        c.hear(cho_b.clone(), eid(1), 700);
        let cho_c = c
            .about(eid(2))
            .next()
            .cloned()
            .expect("phai co dung mot ky uc");

        let mut d = Recollection::new();
        d.hear(cho_c.clone(), eid(1), 700);
        let cho_d = d
            .about(eid(2))
            .next()
            .cloned()
            .expect("phai co dung mot ky uc");

        // Không bịa: loại chuyện và người liên quan giữ nguyên suốt chuỗi.
        for m in [&cho_b, &cho_c, &cho_d] {
            assert_eq!(m.kind, MemoryKind::Met);
            assert_eq!(m.about, eid(2));
            assert_eq!(m.source, None);
        }

        // Méo cộng dồn: mỗi lần kể lại yếu đi.
        assert!(cho_b.strength < 1000);
        assert!(cho_c.strength < cho_b.strength);
        assert!(cho_d.strength < cho_c.strength);

        // Lệch tick vẫn bị chặn — tổng ba lần lệch không thể vượt quá tổng ba
        // biên độ tối đa cho phép ở trust = 700.
        let bien_toi_da_mot_lan = rumor::max_tick_shift(700);
        let lech_tong = (i64::try_from(cho_d.tick).unwrap_or(i64::MAX) - 1000).abs();
        assert!(
            lech_tong <= 3 * bien_toi_da_mot_lan,
            "lech tong {lech_tong} vuot qua bien cho phep cho ba lan ke"
        );
    }

    #[test]
    fn decay_prunes_fully_faded_memories() {
        let mut r = Recollection::new();
        r.witness(met(eid(2), 0, 10));
        r.decay(1_000_000);
        assert_eq!(r.len(), 0, "ky uc mo het phai bi don khoi so");
    }

    #[test]
    fn decay_keeps_still_meaningful_memories() {
        let mut r = Recollection::new();
        r.witness(Memory::new(eid(2), MemoryKind::Intervened, 0, 1000, None));
        r.decay(10);
        assert_eq!(r.len(), 1, "ky uc con manh khong duoc bi don qua som");
    }

    #[test]
    fn two_recollections_built_from_the_same_history_hash_the_same() {
        let build = || {
            let mut r = Recollection::new();
            r.witness(met(eid(2), 0, 900));
            r.hear(met(eid(3), 5, 800), eid(9), 600);
            r.decay(37);
            r.witness(Memory::new(
                eid(4),
                MemoryKind::Worked,
                40,
                700,
                Some(EventSeq(1)),
            ));
            r.decay(5);
            r
        };
        let a = build();
        let b = build();
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn different_history_hashes_differently() {
        let mut a = Recollection::new();
        a.witness(met(eid(2), 0, 900));
        let mut b = Recollection::new();
        b.witness(met(eid(2), 0, 800));
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn max_tick_shift_is_zero_at_full_trust_and_positive_below_it() {
        assert_eq!(rumor::max_tick_shift(1000), 0);
        assert!(rumor::max_tick_shift(0) > 0);
        assert!(rumor::max_tick_shift(0) >= rumor::max_tick_shift(500));
        assert!(rumor::max_tick_shift(500) >= rumor::max_tick_shift(999));
    }

    #[test]
    fn hear_clamps_out_of_range_trust() {
        let mut acao = Recollection::new();
        acao.hear(met(eid(2), 0, 500), eid(9), 5000);
        let m = acao.about(eid(2)).next().expect("phai co dung mot ky uc");
        assert_eq!(m.strength, 500, "trust > 1000 phai duoc kep ve toi da");

        let mut am = Recollection::new();
        am.hear(met(eid(2), 0, 500), eid(9), -5000);
        let m2 = am.about(eid(2)).next().expect("phai co dung mot ky uc");
        assert_eq!(m2.strength, 0, "trust < 0 phai duoc kep ve toi thieu");
    }
}
