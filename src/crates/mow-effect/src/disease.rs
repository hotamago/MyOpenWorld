//! Bệnh và dịch (`idea.md §9.8.5`, `PB-06`).
//!
//! Hai tầng, và tách chúng ra là điều làm cả hệ thống khả thi:
//!
//! | Tầng | Mô hình | Khi nào |
//! |---|---|---|
//! | Cá thể | ủ bệnh, lây theo **tiếp xúc thật** | thực thể ở mức `Active` |
//! | Khu định cư | ngăn S/E/I/R | mọi thứ ở mức `Near` và `Far` |
//!
//! ## Vì sao hai tầng chứ không phải một
//!
//! Mô hình cá thể cho những câu chuyện mà mô hình quần thể không cho được: *ai*
//! lây cho *ai*, ở *chỗ nào*, và ai đã ở đó mà không biết. Đó là chất liệu của
//! điều tra dịch tễ, của nghi kỵ, của cách ly sai người.
//!
//! Nhưng nó không chạy nổi cho một thành phố mười nghìn dân ở mức `Far`. Ở đó,
//! ngăn S/E/I/R cho cùng đường cong dịch với một phần triệu chi phí.
//!
//! Điều kiện để hai tầng không mâu thuẫn: **chuyển giữa chúng phải bảo toàn số
//! người** (`§22.14`). [`Compartments::from_individuals`] và
//! [`Compartments::total`] tồn tại để kiểm điều đó.

use mow_math::{CanonicalHash, Prob, StateHasher};
use serde::{Deserialize, Serialize};

/// Giai đoạn bệnh của một cá thể.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Chưa nhiễm, có thể nhiễm.
    Susceptible,
    /// Đã nhiễm nhưng chưa lây được — thời gian ủ bệnh.
    ///
    /// Đây là giai đoạn khiến dịch bệnh thành một bài toán khó trong thế giới:
    /// người đang ủ bệnh **không có triệu chứng** và vẫn đi lại bình thường,
    /// nên cách ly luôn muộn.
    Exposed,
    /// Đang lây được.
    Infectious,
    /// Đã khỏi và miễn dịch.
    Recovered,
    /// Đã chết vì bệnh.
    Dead,
}

/// Định nghĩa một loại bệnh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pathogen {
    /// Định danh.
    pub id: String,
    /// Xác suất lây trong một lần tiếp xúc.
    ///
    /// [`Prob`] chứ không phải Q16.16: tỉ lệ lây của một bệnh khó lây có thể
    /// nhỏ hơn `1e-5`, và Q16.16 sẽ làm tròn nó về 0 — bệnh biến mất khỏi thế
    /// giới, đúng như cách tỉ lệ đột biến từng biến mất.
    pub transmission_per_contact: Prob,
    /// Số tick ủ bệnh.
    pub incubation_ticks: u64,
    /// Số tick lây được.
    pub infectious_ticks: u64,
    /// Xác suất chết trong toàn đợt bệnh.
    pub lethality: Prob,
    /// Effect áp khi phát bệnh.
    pub symptom_effect: String,
}

impl CanonicalHash for Pathogen {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_u64(self.transmission_per_contact.raw());
        h.write_u64(self.incubation_ticks);
        h.write_u64(self.infectious_ticks);
        h.write_u64(self.lethality.raw());
        h.write_str(&self.symptom_effect);
    }
}

/// Tình trạng nhiễm của một cá thể.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Infection {
    /// Bệnh nào.
    pub pathogen: String,
    /// Giai đoạn.
    pub stage: Stage,
    /// Tick vào giai đoạn hiện tại.
    pub since_tick: u64,
    /// Ai lây cho.
    ///
    /// Có trường này thì điều tra dịch tễ trở thành một hoạt động thật trong
    /// thế giới: truy ngược được chuỗi lây, và **truy sai** cũng được, vì
    /// không ai đọc được trường này trừ True God.
    pub infected_by: Option<u64>,
}

impl Infection {
    /// Giai đoạn tại một tick, suy ra theo thời gian đã trôi.
    ///
    /// Cùng nguyên tắc với homeostasis: trạng thái là hàm của thời gian, không
    /// phải một biến được cập nhật mỗi tick. Nhờ vậy một người ủ bệnh ở mức
    /// `Far` vẫn phát bệnh đúng lúc khi quay lại `Active`.
    pub fn stage_at(&self, p: &Pathogen, now: u64) -> Stage {
        if matches!(
            self.stage,
            Stage::Recovered | Stage::Dead | Stage::Susceptible
        ) {
            return self.stage;
        }
        let troi = now.saturating_sub(self.since_tick);
        match self.stage {
            Stage::Exposed if troi >= p.incubation_ticks => Stage::Infectious,
            Stage::Infectious if troi >= p.infectious_ticks => Stage::Recovered,
            other => other,
        }
    }

    /// Có lây được không tại một tick.
    pub fn is_infectious_at(&self, p: &Pathogen, now: u64) -> bool {
        self.stage_at(p, now) == Stage::Infectious
    }
}

impl CanonicalHash for Infection {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.pathogen);
        h.write_str(match self.stage {
            Stage::Susceptible => "susceptible",
            Stage::Exposed => "exposed",
            Stage::Infectious => "infectious",
            Stage::Recovered => "recovered",
            Stage::Dead => "dead",
        });
        h.write_u64(self.since_tick);
        h.write_option(self.infected_by, |hh, v| {
            hh.write_u64(v);
        });
    }
}

/// Ngăn S/E/I/R ở mức khu định cư.
///
/// Số **người**, không phải tỉ lệ. Tỉ lệ mất thông tin về quy mô, và quy mô là
/// thứ quyết định dịch có bùng hay tắt.
///
/// ## Vì sao có ba trường `carry`
///
/// Đây là **cùng một lớp lỗi** với tỉ lệ đột biến bị Q16.16 nuốt, chỉ khoác một
/// cái áo khác. Trong một ngôi làng ba mươi người với thời gian ủ bệnh 2000
/// tick, luồng chuyển từ "đang ủ" sang "đang lây" là `1 / 2000` người mỗi tick.
/// Chia số nguyên cho ra **0**, mỗi tick, mãi mãi:
///
/// ```text
/// exposed / incubation_ticks  =  1 / 2000  =  0
/// ```
///
/// Dịch không tắt, cũng không lan. Nó **đứng im** — và đứng im là trạng thái
/// khó phát hiện nhất, vì nó trông giống "chưa tới lúc".
///
/// Cách chữa giống hệt [`mow_math::Rate::integrate`]: mang theo số dư. Luồng
/// phân số tích lũy qua các tick cho tới khi đủ một người, rồi một người
/// chuyển. Qua đúng `incubation_ticks` tick thì đúng một người đã chuyển.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Compartments {
    /// Chưa nhiễm.
    pub susceptible: u64,
    /// Đang ủ.
    pub exposed: u64,
    /// Đang lây.
    pub infectious: u64,
    /// Đã khỏi.
    pub recovered: u64,
    /// Đã chết.
    pub dead: u64,

    /// Số dư của luồng lây nhiễm, thang `2^32`.
    pub carry_infection: u64,
    /// Số dư của luồng phát bệnh, đơn vị người·tick.
    pub carry_incubation: u64,
    /// Số dư của luồng khỏi bệnh, đơn vị người·tick.
    pub carry_recovery: u64,
    /// Số dư của luồng tử vong, thang `2^32`.
    pub carry_death: u64,
}

impl Compartments {
    /// Tổng số người, **kể cả người đã chết**.
    ///
    /// Kể cả người chết vì `§22.14` đòi bảo toàn dân số khi chuyển LOD, và một
    /// người chết vẫn là một người đã từng tồn tại — bỏ họ ra khỏi tổng sẽ làm
    /// phép kiểm bảo toàn báo động giả sau mỗi trận dịch.
    pub fn total(self) -> u64 {
        self.susceptible + self.exposed + self.infectious + self.recovered + self.dead
    }

    /// Ngăn khởi đầu với một dân số và một ca chỉ điểm.
    pub fn seeded(population: u64, initial_infectious: u64) -> Compartments {
        let i = initial_infectious.min(population);
        Compartments {
            susceptible: population - i,
            infectious: i,
            ..Compartments::default()
        }
    }

    /// Số người còn sống.
    pub fn alive(self) -> u64 {
        self.total() - self.dead
    }

    /// Gộp từ danh sách cá thể — dùng khi hạ LOD từ `Active` xuống `Far`.
    pub fn from_individuals(stages: &[Stage]) -> Compartments {
        let mut c = Compartments::default();
        for s in stages {
            match s {
                Stage::Susceptible => c.susceptible += 1,
                Stage::Exposed => c.exposed += 1,
                Stage::Infectious => c.infectious += 1,
                Stage::Recovered => c.recovered += 1,
                Stage::Dead => c.dead += 1,
            }
        }
        c
    }

    /// Tiến ngăn một bước.
    ///
    /// Mọi luồng đi qua một số dư, nên một luồng nhỏ hơn một người mỗi tick vẫn
    /// tích lũy thay vì biến mất. Xem tài liệu của struct về lý do.
    #[must_use]
    pub fn step(self, p: &Pathogen, contacts_per_tick: u64) -> Compartments {
        let mut c = self;
        let song = c.alive();
        if song == 0 {
            return c;
        }

        // ── Lây nhiễm ────────────────────────────────────────────────────────
        // Số lần tiếp xúc giữa người lây và người chưa nhiễm, nhân xác suất lây.
        // Nhân trước chia sau: `n / q * p` và `n * p / q` cho kết quả khác nhau
        // với số nhỏ, và cái đầu làm dịch tắt ngóm trong một ngôi làng.
        let tiep_xuc = u128::from(c.infectious) * u128::from(contacts_per_tick);
        let co_hoi = tiep_xuc * u128::from(c.susceptible) / u128::from(song);
        // Thang 2^32 để số dư giữ được phần lẻ mà không tràn.
        let luong = ((co_hoi * u128::from(p.transmission_per_contact.raw())) >> 32) as u64;
        let tich = c.carry_infection.saturating_add(luong);
        let moi = (tich >> 32).min(c.susceptible);
        c.carry_infection = tich - (moi << 32);

        c.susceptible -= moi;
        c.exposed += moi;

        // ── Phát bệnh ────────────────────────────────────────────────────────
        let inc = p.incubation_ticks.max(1);
        let tich_e = c.carry_incubation + c.exposed;
        let phat = (tich_e / inc).min(c.exposed);
        c.carry_incubation = tich_e - phat * inc;
        c.exposed -= phat;
        c.infectious += phat;

        // ── Khỏi hoặc chết ───────────────────────────────────────────────────
        let inf = p.infectious_ticks.max(1);
        let tich_i = c.carry_recovery + c.infectious;
        let ket_thuc = (tich_i / inf).min(c.infectious);
        c.carry_recovery = tich_i - ket_thuc * inf;
        c.infectious -= ket_thuc;

        let luong_chet = ((u128::from(ket_thuc) * u128::from(p.lethality.raw())) >> 32) as u64;
        let tich_d = c.carry_death.saturating_add(luong_chet);
        let chet = (tich_d >> 32).min(ket_thuc);
        c.carry_death = tich_d - (chet << 32);

        c.dead += chet;
        c.recovered += ket_thuc - chet;

        c
    }

    /// Dịch đã tắt chưa.
    pub fn is_over(self) -> bool {
        self.exposed == 0 && self.infectious == 0
    }
}

impl CanonicalHash for Compartments {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.susceptible);
        h.write_u64(self.exposed);
        h.write_u64(self.infectious);
        h.write_u64(self.recovered);
        h.write_u64(self.dead);
        h.write_u64(self.carry_infection);
        h.write_u64(self.carry_incubation);
        h.write_u64(self.carry_recovery);
        h.write_u64(self.carry_death);
    }
}
