//! Một điều mà **một** cá thể tin là đã xảy ra (`idea.md §1.2.2`, `§3.2`).
//!
//! ## Vì sao [`Memory`] không phải là bản sao của [`mow_core::Event`]
//!
//! `EventLog` (`mow-core`) là sự thật của thế giới: ghi một lần, không sửa,
//! đúng như đã xảy ra. Nếu mỗi cư dân cứ đọc thẳng `EventLog` thì `idea.md
//! §1.2.2` sụp ngay lập tức — "dữ liệu thật của thế giới không tự động trở
//! thành kiến thức của nhân vật" sẽ chỉ còn là một câu trong tài liệu, không
//! phải một ràng buộc thật.
//!
//! `Memory` là bản **riêng, mờ được, và có thể sai** của một sự kiện, giữ
//! trong đầu một người cụ thể. Khác `Event` ở đúng ba chỗ:
//!
//! - **Không đầy đủ.** Chỉ giữ `about`, `kind`, `tick`, không giữ payload gốc.
//! - **Mờ dần.** `strength` tụt theo thời gian; `Event` thì bất biến mãi mãi.
//! - **Có thể sai.** Một `Memory` nghe kể có thể lệch so với `Event` thật, vì
//!   [`crate::Recollection::hear`] không cho hai đường quay lại kiểm chứng.
//!
//! ## Tập [`MemoryKind`] cố tình nhỏ, và cố tình bám sát Event thật
//!
//! Mỗi biến thể dưới đây ghi rõ nó tương ứng `EventKind` nào **đã thật sự**
//! được engine phát ra (xem [`source_event`]) — không bịa loại ký ức mà không
//! sự kiện nào trong engine từng sinh ra. Hai biến thể ([`MemoryKind::Met`],
//! [`MemoryKind::Saw`]) không có `EventKind` vì chúng đến từ tri giác
//! (`mow_action::perception::Observation`), một kênh không đi qua nhật ký sự
//! kiện — xem ghi chú ở từng biến thể.

use mow_core::{EntityId, EventSeq};
use mow_math::{CanonicalHash, Rate, StateHasher};
use serde::{Deserialize, Serialize};

/// Sức mạnh thấp nhất và cao nhất một ký ức có thể có.
///
/// `0` nghĩa là quên hẳn; [`crate::Recollection::decay`] dọn khỏi sổ những ký
/// ức chạm đáy này, xem lý do ở đó.
pub const STRENGTH_MIN: i32 = 0;

/// Xem [`STRENGTH_MIN`].
pub const STRENGTH_MAX: i32 = 1000;

/// Chuỗi `EventKind` **thật** mà engine đã phát, dùng để nối [`MemoryKind`]
/// với `mow_core::Event::kind` lúc chuyển một sự kiện đã chứng kiến thành một
/// [`Memory`].
///
/// Gom vào một chỗ vì lý do giống hệt `mow_math::rng::streams`: đây là hợp
/// đồng giữa `mow-memory` và phần code (chưa thuộc quyền sở hữu của crate
/// này) sẽ gọi [`crate::Recollection::witness`] — gom một chỗ để chuỗi không
/// bị gõ lại (và gõ sai) ở nhiều nơi.
pub mod source_event {
    /// `about` di chuyển sang ô khác. Phát bởi handler `core.walk`.
    pub const MOVED: &str = "core.entity.moved";
    /// `about` đổi ý định sang làm việc. Phát bởi `npc.intend`, phân biệt với
    /// các ý định khác qua `payload.intent == "work"` — bản thân `EventKind`
    /// dùng chung cho mọi ý định, nên người gọi phải tự lọc theo payload
    /// trước khi tạo [`super::MemoryKind::Worked`].
    pub const WORKED: &str = "npc.intended";
    /// `about` vừa ăn. Phát bởi handler `core.eat`.
    pub const ATE: &str = "core.item.eaten";
    /// `about` vừa nói. Phát bởi handler `core.speak`.
    pub const SPOKE: &str = "core.speech.uttered";
    /// Một quyền năng ngoài thế giới (True God) can thiệp trực tiếp lên
    /// `about`. Phát bởi handler `truegod.set_attr`.
    pub const INTERVENED: &str = "truegod.intervened";
}

/// Loại điều mà một [`Memory`] ghi nhớ.
///
/// # Vì sao tập này không có `Helped`, `Fed`, `Refused`, `Quarrelled`
///
/// Bản phác thảo ban đầu của tài liệu thiết kế gợi ý những loại đó — chúng
/// đúng là thứ tạo kịch tính. Nhưng tại thời điểm viết module này, không
/// handler nào của engine phát ra một `Event` cho "A giúp B", "A từ chối B"
/// hay "A cãi nhau với B" (`mow-scenario/src/slice.rs` chỉ có sáu động từ:
/// đi, nhặt, ăn, nói, định làm gì, và True God đặt thuộc tính). Đưa những
/// loại đó vào đây sẽ tạo ra một kiểu dữ liệu không handler nào từng lấp đầy
/// — một cái bẫy cho người nối dây sau này, vì trình biên dịch không báo được
/// "không ai từng gọi `Memory::new` với biến thể này".
///
/// Kịch tính không biến mất — nó dời xuống tầng diễn giải
/// ([`crate::would_help`], [`crate::bond_of`]): một người chưa từng được thấy
/// `Worked` hay `Met` cùng ai đó vẫn "chưa quen" theo [`crate::bond_of`], và
/// [`crate::would_help`] tự nhiên trả `false` — không cần một sự kiện
/// "Refused" nào ghi nhận sự từ chối đó, vì bản thân việc *thiếu* ký ức tích
/// cực đã là câu trả lời.
///
/// Khi engine có thật một sự kiện xung đột (một lời từ chối, một trận cãi
/// vã), thêm một biến thể mới ở đây — kèm dòng tương ứng trong
/// [`source_event`] — sẽ tự động chảy vào [`crate::bond_of`] mà không cần sửa
/// hàm đó, miễn là [`crate::bond::contribution`] được dạy trọng số cho biến
/// thể mới.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// Thấy `about` di chuyển. `EventKind`: [`source_event::MOVED`].
    Moved,
    /// Thấy `about` đang làm việc. `EventKind`: [`source_event::WORKED`].
    Worked,
    /// Thấy `about` ăn. `EventKind`: [`source_event::ATE`].
    Ate,
    /// Nghe/thấy `about` nói. `EventKind`: [`source_event::SPOKE`].
    ///
    /// Cố tình **không** giữ nội dung câu nói. Một `Memory` chỉ nặng vài
    /// trường nguyên; giữ cả câu thoại sẽ làm `about()` phải kéo theo chuỗi
    /// dài mỗi lần lặp, và một tin đồn về "đã nói gì đó" là đủ để tạo kịch
    /// tính (ai nói với ai, không phải nói cái gì) mà không mở đường cho
    /// [`crate::Recollection::hear`] bịa ra một câu nói không ai từng nói.
    Spoke,
    /// Một quyền năng ngoài thế giới can thiệp trực tiếp lên `about`.
    /// `EventKind`: [`source_event::INTERVENED`].
    ///
    /// Đây là ứng viên cho "chuyện mạnh nhớ lâu" — xem tốc độ mờ dần chậm hơn
    /// hẳn các loại khác ở [`fade_rate`].
    Intervened,
    /// Chạm mặt `about` gần tới mức nhận ra được ai đó là ai.
    ///
    /// **Không có `EventKind` tương ứng.** Tri giác
    /// (`mow_action::perception::observe`) không ghi vào `EventLog` — nó chỉ
    /// trả `Vec<Observation>` cho lớp quyết định đọc ngay trong tick đó rồi
    /// bỏ. `Met` là chỗ `mow-memory` biến một `Observation` thoáng qua thành
    /// thứ còn lại sau khi tick đó trôi qua. Vì không có sự kiện gốc, đây là
    /// một trong hai loại mà [`Memory::source`] hợp lệ để là `None` **kể cả
    /// khi tự chứng kiến** — khác với ký ức nghe kể, `None` ở đây có nghĩa
    /// "không có gì để trỏ tới", không phải "không đáng tin".
    Met,
    /// Nhận ra một dấu hiệu (`sign.*`) nơi `about`, ví dụ dấu vết một cuộc ẩu
    /// đả hay mùi máu — cùng khái niệm "dấu hiệu" mà
    /// `mow_action::perception::Observation::signs` đã lọc theo giác quan.
    ///
    /// Không import thẳng kiểu `Observation`: `mow-memory` cố tình không phụ
    /// thuộc `mow-action` (xem tài liệu ở gốc crate) nên biến thể này chỉ giữ
    /// lại đúng cái tên dấu hiệu dạng chuỗi, để người nối dây tự chuyển từ
    /// `Observation::signs` sang đây.
    ///
    /// Cùng lý do với [`MemoryKind::Met`]: không có `EventKind`, `source`
    /// hợp lệ để là `None` cả khi tự chứng kiến.
    Saw {
        /// Tên dấu hiệu, ví dụ `"blood"` — phần sau `sign.<giác quan>.` trong
        /// khóa thuộc tính mà `perception::qua_kenh` đã lọc.
        sign: String,
    },
}

impl MemoryKind {
    /// Tên ổn định, dùng cho hash canonical và cho log dễ đọc.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryKind::Moved => "moved",
            MemoryKind::Worked => "worked",
            MemoryKind::Ate => "ate",
            MemoryKind::Spoke => "spoke",
            MemoryKind::Intervened => "intervened",
            MemoryKind::Met => "met",
            MemoryKind::Saw { .. } => "saw",
        }
    }
}

impl CanonicalHash for MemoryKind {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
        // Chỉ `Saw` mang dữ liệu thêm; các biến thể khác không có gì để hash
        // ngoài tên, và `write_str(as_str())` đã đủ phân biệt chúng vì mỗi
        // biến thể có một tên duy nhất.
        if let MemoryKind::Saw { sign } = self {
            h.write_str(sign);
        }
    }
}

/// Một điều mà **một** cá thể tin là đã xảy ra. Xem tài liệu module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    /// Chuyện này về ai.
    pub about: EntityId,
    /// Loại chuyện.
    pub kind: MemoryKind,
    /// Tick mà người giữ ký ức này **tin** là chuyện đã xảy ra.
    ///
    /// Với ký ức tự chứng kiến, đây gần như luôn là tick thật. Với ký ức nghe
    /// kể, [`crate::Recollection::hear`] có thể làm trường này lệch khỏi tick
    /// thật — đó chính là "diễn giải sai" mà `idea.md §3.2` mô tả, không phải
    /// một lỗi.
    pub tick: u64,
    /// Sức mạnh hiện tại, luôn nằm trong [`STRENGTH_MIN`]..=[`STRENGTH_MAX`].
    ///
    /// Đây là giá trị **hiện tại**, đã trừ phần mờ dần tính tới lần gọi
    /// [`crate::Recollection::decay`] gần nhất — không phải giá trị lúc ghi.
    /// [`crate::Recollection::about`] trả thẳng trường này, nên nó luôn phải
    /// đã cập nhật, không phải một bản chưa mờ chờ ai đó tính lại.
    pub strength: i32,
    /// Sự kiện gốc nếu tự chứng kiến và tri giác đó có một `Event` đứng sau
    /// (xem [`MemoryKind::Met`], [`MemoryKind::Saw`] về trường hợp không có
    /// sự kiện gốc dù vẫn tự chứng kiến); `None` vô điều kiện nếu nghe kể —
    /// [`crate::Recollection::hear`] xóa trường này ngay cả khi người kể nói
    /// rõ họ đã thấy tận mắt, vì người nghe không có cách nào tự xác minh một
    /// `EventSeq` mà chính họ chưa từng đọc từ nhật ký.
    pub source: Option<EventSeq>,
    /// Số dư (đơn vị: phần nghìn sức mạnh) chưa kịp trừ từ lần mờ dần trước.
    ///
    /// # Vì sao trường này tồn tại
    ///
    /// Đây là nơi tránh đúng lỗi đã lặp lại ba lần trong dự án này: tốc độ
    /// nhỏ (ví dụ mờ 1 điểm mỗi 400 tick) mà chia trước khi cộng dồn thì
    /// `decay(1)` gọi liên tục **không bao giờ** trừ được gì —
    /// `1 điểm * 1 tick / 400 tick = 0` mọi lần, dù đã qua hàng nghìn tick.
    /// Đó là chính xác dạng lỗi "tốc độ ×0.001 đóng băng" được nhắc trong đề
    /// bài, chỉ khác miền.
    ///
    /// [`mow_math::Rate::integrate`] đã giải bài này một lần cho cả dự án
    /// bằng số dư mang theo: tổng nhân trước (`num * ticks + carry`), chia
    /// **một lần**, phần dư giữ lại cho lần sau. Chuỗi gọi
    /// `decay(1)` × 400 lần cộng dồn đúng bằng một lần `decay(400)` — có test
    /// khẳng định đúng bất biến đó. `carry` ở đây chính là "phần dư" đó, neo
    /// theo từng `Memory` vì mỗi loại ký ức mờ theo một tốc độ khác nhau
    /// ([`fade_rate`]) nên không thể dùng chung một số dư cho cả sổ.
    ///
    /// Không công khai: nó là chi tiết triển khai của phép mờ dần, không
    /// phải một sự thật về ký ức mà người gọi cần đọc. Nó **có** nằm trong
    /// [`CanonicalHash`] — bỏ nó ra khỏi hash sẽ để lọt một phần trạng thái
    /// thật sự ảnh hưởng tới tương lai (tốc độ mờ tiếp theo) mà hai thế giới
    /// giống hệt nhau trên mọi trường công khai vẫn có thể rẽ nhánh sau đó.
    pub(crate) carry: i64,
}

impl Memory {
    /// Dựng một ký ức mới, sức mạnh được kẹp về `[STRENGTH_MIN, STRENGTH_MAX]`
    /// ngay từ đầu — không có `Memory` nào lọt ra ngoài khoảng hợp lệ.
    #[must_use]
    pub fn new(
        about: EntityId,
        kind: MemoryKind,
        tick: u64,
        strength: i32,
        source: Option<EventSeq>,
    ) -> Memory {
        Memory {
            about,
            kind,
            tick,
            strength: strength.clamp(STRENGTH_MIN, STRENGTH_MAX),
            source,
            carry: 0,
        }
    }

    /// Áp phép mờ dần qua `ticks` tick, đúng tốc độ của [`MemoryKind`] này.
    ///
    /// `pub(crate)`: chỉ [`crate::Recollection::decay`] được gọi, vì chỉ nó
    /// biết khi nào một mốc thời gian mới thật sự trôi qua cho **toàn bộ**
    /// sổ ký ức — một `Memory` đơn lẻ không có khái niệm "bây giờ".
    pub(crate) fn decay(&mut self, ticks: u64) {
        let rate = fade_rate(&self.kind);
        // `Rate::integrate` chỉ lỗi khi tràn số — với `ticks` tối đa
        // `u64::MAX` và tử số tối đa vài chục, phép nhân trung gian dùng
        // `i128` nên không thể tràn thật. Nhánh `unwrap_or` chỉ là lưới an
        // toàn, không phải đường chạy bình thường.
        let (delta, carry_out) = rate.integrate(ticks, self.carry).unwrap_or((0, self.carry));
        self.carry = carry_out;
        let sau = i64::from(self.strength) + delta;
        self.strength = i32::try_from(sau.clamp(i64::from(STRENGTH_MIN), i64::from(STRENGTH_MAX)))
            .unwrap_or(STRENGTH_MIN);
    }
}

impl CanonicalHash for Memory {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.about.canonical_hash(h);
        self.kind.canonical_hash(h);
        h.write_u64(self.tick);
        h.write_i64(i64::from(self.strength));
        h.write_option(self.source, |hh, s| s.canonical_hash(hh));
        h.write_i64(self.carry);
    }
}

/// Tốc độ mờ dần theo loại ký ức: mất `|num|` phần nghìn sức mạnh mỗi `den`
/// tick (xem [`mow_math::Rate`]).
///
/// # Vì sao không đều
///
/// `idea.md §3.2` cần "chuyện mạnh nhớ lâu hơn chuyện thường" — bằng chứng cụ
/// thể trong đề bài là "bị từ chối lúc đói" so với "gặp ngoài đường". Vì tập
/// [`MemoryKind`] hiện tại không có sự kiện từ chối (xem tài liệu ở đó), ứng
/// viên "chuyện mạnh" thật sự có trong engine là [`MemoryKind::Intervened`]
/// — một quyền năng ngoài thế giới ra tay là chuyện không ai quên nhanh được.
/// Nó mờ chậm hơn [`MemoryKind::Met`]/[`MemoryKind::Saw`] (chạm mặt thoáng
/// qua, dấu hiệu mơ hồ) tới bốn mươi lần.
///
/// Các mốc cụ thể là quyết định gameplay, không phải hằng số vật lý — giống
/// `mow_action::perception::IDENTIFY_THRESHOLD_PERCENT` ở crate hàng xóm:
/// chỉnh chúng không đổi tính đúng của phép mờ dần, chỉ đổi nhịp độ.
fn fade_rate(kind: &MemoryKind) -> Rate {
    let (num, den) = match kind {
        // Thoáng qua: chạm mặt ở giếng, một dấu hiệu mơ hồ. Mờ nhanh nhất vì
        // bản thân sự kiện gốc (nếu có) cũng nhạt — không ai nhớ lâu một
        // khuôn mặt lướt qua ở quảng trường.
        MemoryKind::Met | MemoryKind::Saw { .. } => (-4, 100),
        // Sinh hoạt thường ngày: đi lại, ăn. Xảy ra mỗi ngày nên không đọng
        // lại lâu, nhưng đáng tin hơn một cái nhìn thoáng qua.
        MemoryKind::Moved | MemoryKind::Ate => (-3, 100),
        // Có chủ đích quan sát được: làm việc, nói chuyện. Nói lên điều gì đó
        // về người kia (chăm chỉ, cởi mở), nên đọng lại lâu hơn sinh hoạt
        // thuần túy.
        MemoryKind::Worked | MemoryKind::Spoke => (-2, 100),
        // Chuyện mạnh — xem tài liệu hàm.
        MemoryKind::Intervened => (-1, 400),
    };
    // `den` luôn dương và cố định trong mã nguồn nên `Rate::new` không thể
    // lỗi thật; `unwrap_or` chỉ để tránh `unwrap()` trần theo quy ước của
    // crate này, không phải vì nhánh lỗi có thể xảy ra.
    Rate::new(num, den).unwrap_or(Rate::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mow_core::EntityId;

    fn eid(n: u64) -> EntityId {
        EntityId::new(n)
    }

    #[test]
    fn new_clamps_strength_above_max() {
        let m = Memory::new(eid(1), MemoryKind::Met, 0, 5000, None);
        assert_eq!(m.strength, STRENGTH_MAX);
    }

    #[test]
    fn new_clamps_strength_below_min() {
        let m = Memory::new(eid(1), MemoryKind::Met, 0, -5000, None);
        assert_eq!(m.strength, STRENGTH_MIN);
    }

    #[test]
    fn different_kinds_hash_differently() {
        let a = MemoryKind::Met.state_hash();
        let b = MemoryKind::Worked.state_hash();
        assert_ne!(a, b);
    }

    #[test]
    fn saw_hashes_by_sign_text() {
        let a = MemoryKind::Saw {
            sign: "blood".to_owned(),
        }
        .state_hash();
        let b = MemoryKind::Saw {
            sign: "smoke".to_owned(),
        }
        .state_hash();
        assert_ne!(a, b, "hai dau hieu khac nhau phai hash khac nhau");
    }

    #[test]
    fn decay_reduces_strength_over_one_big_call() {
        let mut m = Memory::new(eid(1), MemoryKind::Met, 0, 1000, None);
        m.decay(1000);
        assert!(m.strength < 1000, "sau 1000 tick phai mo di it nhieu");
    }

    #[test]
    fn decay_does_not_go_below_floor() {
        let mut m = Memory::new(eid(1), MemoryKind::Met, 0, 50, None);
        m.decay(1_000_000);
        assert_eq!(m.strength, STRENGTH_MIN);
    }

    #[test]
    fn decay_leaves_identity_fields_untouched() {
        let mut m = Memory::new(eid(7), MemoryKind::Worked, 42, 800, Some(EventSeq(3)));
        m.decay(10);
        assert_eq!(m.about, eid(7));
        assert_eq!(m.kind, MemoryKind::Worked);
        assert_eq!(m.tick, 42, "decay khong duoc sua thoi diem tin la xay ra");
        assert_eq!(m.source, Some(EventSeq(3)));
    }

    /// Bài quan trọng nhất của module: mờ dần qua nhiều bước nhỏ phải cộng
    /// dồn đúng bằng một bước lớn — đây là bất biến mà `carry` tồn tại để
    /// giữ. Dùng [`MemoryKind::Intervened`] vì nó có tốc độ chậm nhất
    /// (1/400), tức dễ dính lỗi "chia trước khi cộng dồn" nhất nếu ai đó lỡ
    /// bỏ `carry` đi.
    #[test]
    fn many_small_decay_steps_equal_one_big_step() {
        let mut buoc_nho = Memory::new(eid(1), MemoryKind::Intervened, 0, 1000, None);
        for _ in 0..2000 {
            buoc_nho.decay(1);
        }
        let mut buoc_lon = Memory::new(eid(1), MemoryKind::Intervened, 0, 1000, None);
        buoc_lon.decay(2000);
        assert_eq!(buoc_nho.strength, buoc_lon.strength);
        assert_eq!(buoc_nho.carry, buoc_lon.carry);
    }

    /// Cùng bài trên nhưng cắt ở mốc lẻ, để đảm bảo bất biến không chỉ đúng
    /// khi số bước chia hết cho mẫu số của tốc độ.
    #[test]
    fn many_small_decay_steps_equal_one_big_step_at_odd_boundary() {
        let mut buoc_nho = Memory::new(eid(1), MemoryKind::Worked, 0, 1000, None);
        for _ in 0..777 {
            buoc_nho.decay(1);
        }
        let mut buoc_lon = Memory::new(eid(1), MemoryKind::Worked, 0, 1000, None);
        buoc_lon.decay(777);
        assert_eq!(buoc_nho.strength, buoc_lon.strength);
    }

    /// Khóa lại chính lỗi "tốc độ ×0.001 đóng băng": một ký ức mờ rất chậm
    /// (1/400) gọi `decay(1)` đủ nhiều lần **phải** thấy sức mạnh giảm, chứ
    /// không được đứng yên mãi vì mỗi lần chia ra 0.
    #[test]
    fn slow_fading_memory_does_not_freeze_under_many_tiny_steps() {
        let mut m = Memory::new(eid(1), MemoryKind::Intervened, 0, 1000, None);
        for _ in 0..399 {
            m.decay(1);
        }
        // Chưa đủ 400 tick nên vẫn có thể còn nguyên — đây không phải bug.
        for _ in 0..2 {
            m.decay(1);
        }
        // Nhưng qua mốc 400 thì phải đã giảm — nếu đứng yên ở đây là đóng
        // băng thật.
        assert!(m.strength < 1000, "ky uc cham mo van phai mo sau 401 tick");
    }

    #[test]
    fn intervened_fades_slower_than_met() {
        let mut than = Memory::new(eid(1), MemoryKind::Intervened, 0, 1000, None);
        let mut gap = Memory::new(eid(1), MemoryKind::Met, 0, 1000, None);
        than.decay(200);
        gap.decay(200);
        assert!(
            than.strength > gap.strength,
            "than can thiep phai nho lau hon gap go thoang qua"
        );
    }

    #[test]
    fn zero_ticks_is_a_no_op() {
        let mut m = Memory::new(eid(1), MemoryKind::Met, 0, 500, None);
        m.decay(0);
        assert_eq!(m.strength, 500);
        assert_eq!(m.carry, 0);
    }

    #[test]
    fn source_event_strings_match_what_engine_actually_emits() {
        // Bài này không gọi vào engine (crate cố tình không phụ thuộc
        // mow-server) — nó khóa các hằng số lại đúng chuỗi đã xác nhận bằng
        // tay trong `mow-scenario/src/slice.rs`, để một lần đổi tên sự kiện
        // ở đó mà quên đổi ở đây sẽ lộ ra thành một bài test đỏ khi ai đó
        // cập nhật hằng số theo engine mới.
        assert_eq!(source_event::MOVED, "core.entity.moved");
        assert_eq!(source_event::WORKED, "npc.intended");
        assert_eq!(source_event::ATE, "core.item.eaten");
        assert_eq!(source_event::SPOKE, "core.speech.uttered");
        assert_eq!(source_event::INTERVENED, "truegod.intervened");
    }

    #[test]
    fn as_str_is_stable_and_unique_per_variant() {
        let ds = [
            MemoryKind::Moved,
            MemoryKind::Worked,
            MemoryKind::Ate,
            MemoryKind::Spoke,
            MemoryKind::Intervened,
            MemoryKind::Met,
            MemoryKind::Saw {
                sign: "x".to_owned(),
            },
        ];
        let mut ten: Vec<&str> = ds.iter().map(MemoryKind::as_str).collect();
        let truoc = ten.len();
        ten.sort_unstable();
        ten.dedup();
        assert_eq!(ten.len(), truoc, "hai bien the khong duoc trung ten");
    }
}
