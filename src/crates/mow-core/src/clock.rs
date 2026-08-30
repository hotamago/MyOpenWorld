//! Đồng hồ và **miền đồng hồ** (`idea.md §4.4`, `§4.5`).
//!
//! Mỗi tiến trình có thời hạn — cái đói, thai kỳ, thời gian ủ bệnh, hạn hợp
//! đồng, deadline nghiên cứu — phải khai báo nó đếm theo đồng hồ nào. Nếu
//! không, chuyện sau sẽ xảy ra và sẽ rất khó truy: một người đang ủ bệnh bước
//! qua cổng sang thế giới chảy nhanh gấp mười, và **khỏi bệnh hoặc chết ngay
//! lập tức** vì deadline của họ được đổi đồng loạt cùng đồng hồ thế giới.
//!
//! Cách chữa không phải là "đừng đổi đồng hồ" mà là mỗi deadline nhớ *nó thuộc
//! miền nào*, rồi khi qua cổng thì [`rebase`] từng cái theo miền của chính nó.
//! Nhờ vậy chênh lệch thời gian giữa hai thế giới trở thành thứ chơi được: gửi
//! người sang thế giới chảy nhanh để nghiên cứu, giam kẻ thù ở nơi chảy chậm.

use mow_math::{CanonicalHash, MathResult, Rate, StateHasher};
use serde::{Deserialize, Serialize};

/// Một mốc thời gian, đếm bằng tick.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Tick(pub u64);

impl Tick {
    /// Mốc khởi tạo thế giới.
    pub const GENESIS: Tick = Tick(0);

    /// Cộng thêm số tick.
    pub fn plus(self, d: u64) -> Option<Tick> {
        self.0.checked_add(d).map(Tick)
    }

    /// Số tick từ `earlier` tới `self`; `None` nếu `earlier` ở sau.
    pub fn since(self, earlier: Tick) -> Option<u64> {
        self.0.checked_sub(earlier.0)
    }
}

impl core::fmt::Display for Tick {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "t{}", self.0)
    }
}

impl CanonicalHash for Tick {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.0);
    }
}

/// Miền đồng hồ của một tiến trình (`§4.5`).
///
/// Đây là **trường bắt buộc** trên mọi deadline. Bất biến `§22.24` cấm một tiến
/// trình có thời hạn mà không khai báo miền, và invariant runner kiểm điều đó
/// ở mọi tick — không phải vì lập trình viên hay quên, mà vì mặc định sai ở
/// đây tạo ra lỗi chỉ lộ ra sau khi ai đó đi qua cổng lần đầu, có thể là hàng
/// trăm giờ chơi sau khi code được viết.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockDomain {
    /// Đồng hồ của thế giới nơi tiến trình diễn ra.
    ///
    /// Mùa màng, thời tiết, lịch xã hội, hạn hợp đồng. Một hợp đồng ký ở đây
    /// đáo hạn theo mùa **ở đây**, kể cả khi người ký đã đi nơi khác.
    WorldLocal,

    /// Đồng hồ thần thánh, chung cho toàn đa vũ trụ.
    ///
    /// Điều phối liên-world, lịch của Yuu, thứ tự event toàn multiverse. Đây là
    /// đồng hồ duy nhất so sánh được giữa hai thế giới bất kỳ.
    Divine,

    /// Thời gian riêng của thực thể.
    ///
    /// Tuổi, đói, ủ bệnh, hồi phục. Đi theo thực thể qua cổng — đó là toàn bộ
    /// lý do miền này tồn tại.
    Proper,

    /// Đồng hồ đặc biệt do luật quy định.
    ///
    /// Lời nguyền theo tuần trăng, giao ước theo chu kỳ thần. Tỉ lệ của nó do
    /// một luật cung cấp chứ không phải do thế giới, nên nó có thể dừng hẳn
    /// hoặc chạy ngược mà không vi phạm gì.
    LawDefined,
}

impl CanonicalHash for ClockDomain {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(match self {
            ClockDomain::WorldLocal => "world_local",
            ClockDomain::Divine => "divine",
            ClockDomain::Proper => "proper",
            ClockDomain::LawDefined => "law_defined",
        });
    }
}

/// Một hạn có miền. Đây là kiểu duy nhất được phép biểu diễn "khi nào thì xong".
///
/// Không có hàm dựng nào nhận mỗi [`Tick`]: nếu bạn muốn một deadline thì bạn
/// phải nói nó đếm theo đồng hồ nào.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Deadline {
    /// Mốc đáo hạn, tính theo đồng hồ của miền.
    pub at: Tick,
    /// Miền đồng hồ mà `at` được đo theo.
    pub domain: ClockDomain,
}

impl Deadline {
    /// Dựng.
    pub const fn new(at: Tick, domain: ClockDomain) -> Deadline {
        Deadline { at, domain }
    }

    /// Đã tới hạn chưa, theo đồng hồ tương ứng.
    pub fn is_due(self, clock: &Clock) -> bool {
        clock.now_in(self.domain) >= self.at
    }
}

impl CanonicalHash for Deadline {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.at.canonical_hash(h);
        self.domain.canonical_hash(h);
    }
}

/// Đồng hồ của một thế giới, mang cả bốn miền.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clock {
    world_local: Tick,
    divine: Tick,
    /// Tỉ lệ giữa đồng hồ thần và đồng hồ địa phương: bao nhiêu tick địa phương
    /// trôi qua khi một tick thần trôi qua.
    ///
    /// Hữu tỉ chứ không phải số thực, và [`Rate`] mang theo số dư, nên tỉ lệ
    /// `7/3` không tích lũy sai số sau một triệu tick.
    local_per_divine: Rate,
    /// Số dư của phép quy đổi ở trên.
    local_carry: i64,
    /// Tỉ lệ của đồng hồ do luật định nghĩa, nếu thế giới này có.
    law_defined: Option<LawClock>,
}

/// Đồng hồ do luật định nghĩa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawClock {
    /// Mốc hiện tại của đồng hồ này.
    pub now: Tick,
    /// Tốc độ so với đồng hồ địa phương. Có thể là 0 — một lời nguyền có thể
    /// bị treo mà không hết hạn.
    pub per_local: Rate,
    /// Số dư quy đổi.
    pub carry: i64,
}

impl Clock {
    /// Đồng hồ tại genesis, với tỉ lệ địa phương/thần cho trước.
    pub fn new(local_per_divine: Rate) -> Clock {
        Clock {
            world_local: Tick::GENESIS,
            divine: Tick::GENESIS,
            local_per_divine,
            local_carry: 0,
            law_defined: None,
        }
    }

    /// Đồng hồ chạy đúng bằng đồng hồ thần.
    pub fn synchronous() -> Clock {
        Clock::new(Rate::per_tick(1))
    }

    /// Gắn một đồng hồ do luật định nghĩa.
    pub fn with_law_clock(mut self, per_local: Rate) -> Clock {
        self.law_defined = Some(LawClock {
            now: Tick::GENESIS,
            per_local,
            carry: 0,
        });
        self
    }

    /// Mốc hiện tại của một miền.
    ///
    /// [`ClockDomain::Proper`] trả về đồng hồ địa phương: thời gian riêng của
    /// thực thể được đo *tương đối với* nơi nó đang đứng, và phần "riêng" nằm ở
    /// chỗ deadline của nó được [`rebase`] khi nó đi nơi khác chứ không nằm ở
    /// chỗ có một đồng hồ riêng cho từng thực thể.
    pub fn now_in(&self, domain: ClockDomain) -> Tick {
        match domain {
            ClockDomain::WorldLocal | ClockDomain::Proper => self.world_local,
            ClockDomain::Divine => self.divine,
            ClockDomain::LawDefined => self
                .law_defined
                .as_ref()
                .map_or(self.world_local, |c| c.now),
        }
    }

    /// Mốc địa phương hiện tại.
    pub fn local(&self) -> Tick {
        self.world_local
    }

    /// Mốc thần hiện tại.
    pub fn divine(&self) -> Tick {
        self.divine
    }

    /// Tỉ lệ địa phương trên thần.
    pub fn local_per_divine(&self) -> Rate {
        self.local_per_divine
    }

    /// Tiến `n` tick **thần**, kéo theo các đồng hồ khác theo tỉ lệ của chúng.
    ///
    /// Đồng hồ thần là đồng hồ chủ vì nó là thứ duy nhất so sánh được giữa hai
    /// thế giới; nếu để đồng hồ địa phương làm chủ thì thứ tự event toàn đa vũ
    /// trụ sẽ không xác định được.
    pub fn advance_divine(&mut self, n: u64) -> MathResult<TickSpan> {
        let divine_truoc = self.divine;
        let local_truoc = self.world_local;

        self.divine = Tick(self.divine.0.saturating_add(n));

        let (d_local, carry) = self.local_per_divine.integrate(n, self.local_carry)?;
        self.local_carry = carry;
        self.world_local = Tick(self.world_local.0.saturating_add(d_local.max(0) as u64));

        if let Some(law) = &mut self.law_defined {
            let d = self.world_local.0.saturating_sub(local_truoc.0);
            let (d_law, c) = law.per_local.integrate(d, law.carry)?;
            law.carry = c;
            law.now = Tick(law.now.0.saturating_add(d_law.max(0) as u64));
        }

        Ok(TickSpan {
            divine_from: divine_truoc,
            divine_to: self.divine,
            local_from: local_truoc,
            local_to: self.world_local,
        })
    }

    /// Quy đổi một khoảng thời gian địa phương của thế giới này sang thế giới
    /// khác, **giữ nguyên độ dài thật của khoảng đó theo đồng hồ thần**.
    ///
    /// Đây là hạt nhân của [`rebase`].
    pub fn convert_local_span(&self, span: u64, dich: &Clock) -> MathResult<u64> {
        // span địa phương ở đây → span thần → span địa phương ở kia.
        let (than, _) = Rate::new(
            self.local_per_divine.den(),
            self.local_per_divine.num().max(1),
        )?
        .integrate(span, 0)?;
        let (kia, _) = dich.local_per_divine.integrate(than.max(0) as u64, 0)?;
        Ok(kia.max(0) as u64)
    }
}

impl CanonicalHash for Clock {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.world_local.canonical_hash(h);
        self.divine.canonical_hash(h);
        self.local_per_divine.canonical_hash(h);
        h.write_i64(self.local_carry);
        h.write_option(self.law_defined.as_ref(), |hh, c| {
            c.now.canonical_hash(hh);
            c.per_local.canonical_hash(hh);
            hh.write_i64(c.carry);
        });
    }
}

/// Khoảng thời gian đã trôi qua trong một lần tiến đồng hồ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickSpan {
    /// Mốc thần trước.
    pub divine_from: Tick,
    /// Mốc thần sau.
    pub divine_to: Tick,
    /// Mốc địa phương trước.
    pub local_from: Tick,
    /// Mốc địa phương sau.
    pub local_to: Tick,
}

/// Rebase một deadline khi thực thể chuyển từ `tu` sang `dich` (`§4.5`, `§22.42`).
///
/// Quy tắc, và mỗi dòng ở đây là một lỗi đã được ngăn:
///
/// - [`ClockDomain::Divine`] **không đổi**. Đồng hồ thần là chung; quy đổi nó
///   sẽ là quy đổi một thứ vốn đã tuyệt đối.
/// - [`ClockDomain::WorldLocal`] **không đổi**. Hợp đồng ký ở thế giới cũ đáo
///   hạn theo lịch của thế giới cũ, kể cả khi người ký đã đi. Đổi nó sẽ khiến
///   một khoản vay đáo hạn tức thì chỉ vì con nợ bỏ trốn qua cổng — điều mà
///   người chơi sẽ lập tức khai thác.
/// - [`ClockDomain::Proper`] **được quy đổi**. Đây là miền duy nhất đi theo
///   thực thể, nên nó là miền duy nhất phải đổi số.
/// - [`ClockDomain::LawDefined`] **không đổi**. Luật sở hữu đồng hồ đó, nên chỉ
///   luật mới được rebase nó; engine tự ý đổi là vượt quyền.
pub fn rebase(deadline: Deadline, tu: &Clock, dich: &Clock) -> MathResult<Deadline> {
    if deadline.domain != ClockDomain::Proper {
        return Ok(deadline);
    }
    let con_lai = deadline.at.0.saturating_sub(tu.local().0);
    let con_lai_moi = tu.convert_local_span(con_lai, dich)?;
    Ok(Deadline {
        at: Tick(dich.local().0.saturating_add(con_lai_moi)),
        domain: ClockDomain::Proper,
    })
}
