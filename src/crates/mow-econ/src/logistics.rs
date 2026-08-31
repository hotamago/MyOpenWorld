//! Lao động, hãng, và **hàng hóa không teleport** (`idea.md §12.17`, `PD-13`).
//!
//! ## Lỗ hổng mà `§12.17.2` gọi tên
//!
//! > Đây là một lỗ hổng dễ mắc: hàng **không được teleport** giữa hai kho.
//!
//! Dễ mắc vì bản đầu tiên của mọi hệ thống kinh tế đều làm thế: kho A giảm, kho
//! B tăng, xong. Nó chạy, nó cân bằng, và nó xóa mất toàn bộ địa lý kinh tế —
//! khoảng cách không còn tốn gì, nên không có thương nhân, không có tuyến đường,
//! không có cướp đường, không có thành phố cảng.
//!
//! Ở đây mỗi lô hàng là một [`Shipment`] **có mặt trong thế giới**: có người
//! chở, có tuyến, có thời điểm khởi hành, có hao hụt dọc đường, và có **chuỗi
//! bàn giao trách nhiệm**.
//!
//! Nhờ vậy một cây cầu sập lan thành thiếu hàng → tăng giá → vi phạm hợp đồng,
//! **với cause chain đầy đủ** — chứ không phải một sự kiện "giá tăng" xuất hiện
//! từ hư không.
//!
//! ## Chuỗi bàn giao là chỗ trách nhiệm nằm
//!
//! [`Shipment::liable_at`] trả lời câu hỏi *"lúc nó mất thì là lỗi của ai"*.
//! Không có chuỗi này thì mọi mất mát đều rơi vào người gửi, và bảo hiểm, áp
//! tải, hợp đồng vận chuyển đều trở nên vô nghĩa.
//!
//! ## Chuyên môn hóa phải **nảy sinh**, không được gán sẵn (`§12.17.3`)
//!
//! Phát hiện từ mô phỏng nhiều tác tử: vai trò nghề nghiệp phân hóa **chỉ khi**
//! các cá thể quan sát được nhau đang làm gì. Bị chặn tri giác xã hội thì tất cả
//! làm cùng một việc. [`specialize`] vì thế nhận `visible_peers` — và trả về
//! cùng một nghề cho mọi người khi danh sách đó rỗng.

use mow_core::{EntityId, Tick};
use serde::{Deserialize, Serialize};

/// Một chặng trong tuyến đường.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leg {
    /// Từ đâu.
    pub from: String,
    /// Tới đâu.
    pub to: String,
    /// Mất bao nhiêu tick.
    pub travel_ticks: u64,
    /// Hao hụt trên chặng này, phần nghìn.
    pub spoilage: u16,
    /// Rủi ro bị cướp, phần nghìn.
    pub banditry: u16,
    /// Chặng này còn đi được không. Cầu sập thì `false`.
    pub passable: bool,
}

/// Ai đang chịu trách nhiệm ở một chặng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handover {
    /// Từ tick nào.
    pub from_tick: u64,
    /// Ai chịu trách nhiệm.
    pub custodian: EntityId,
}

/// Một lô hàng đang đi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shipment {
    /// Định danh.
    pub id: u64,
    /// Hàng gì.
    pub goods: String,
    /// Số lượng lúc khởi hành.
    pub quantity: i64,
    /// Người gửi.
    pub shipper: EntityId,
    /// Người nhận.
    pub consignee: EntityId,
    /// Tuyến.
    pub route: Vec<Leg>,
    /// Khởi hành lúc nào.
    pub departed: Tick,
    /// Chuỗi bàn giao, theo thứ tự thời gian.
    pub chain: Vec<Handover>,
    /// Có áp tải không — giảm rủi ro cướp.
    pub escorted: bool,
}

/// Lô hàng đi tới đâu rồi, và còn lại bao nhiêu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Đã qua bao nhiêu chặng.
    pub legs_done: usize,
    /// Còn lại bao nhiêu hàng.
    pub remaining: i64,
    /// Đã tới nơi chưa.
    pub arrived: bool,
    /// Bị kẹt vì chặng không đi được.
    pub blocked_at: Option<String>,
    /// Đã mất bao nhiêu vì hao hụt.
    pub spoiled: i64,
    /// Đã mất bao nhiêu vì cướp.
    pub raided: i64,
}

impl Shipment {
    /// Tổng thời gian đi hết tuyến.
    pub fn total_travel(&self) -> u64 {
        self.route.iter().map(|l| l.travel_ticks).sum()
    }

    /// Tình trạng ở tick `now`.
    ///
    /// **Xác định**: hao hụt và cướp là hàm của tuyến, không của xúc xắc. Một
    /// tuyến nguy hiểm thì *luôn* mất nhiều hơn, nên người chơi học được rằng
    /// đường nào đáng đi và khi nào nên thuê áp tải.
    pub fn progress(&self, now: Tick) -> Progress {
        let mut con = self.quantity;
        let mut hao = 0;
        let mut cuop = 0;
        let mut t = self.departed.0;
        let mut xong = 0;

        for l in &self.route {
            if !l.passable {
                return Progress {
                    legs_done: xong,
                    remaining: con,
                    arrived: false,
                    blocked_at: Some(l.from.clone()),
                    spoiled: hao,
                    raided: cuop,
                };
            }
            if now.0 < t + l.travel_ticks {
                break;
            }
            t += l.travel_ticks;
            xong += 1;

            let h = con * i64::from(l.spoilage) / 1_000;
            con -= h;
            hao += h;

            // Áp tải cắt rủi ro cướp đi bốn phần năm — không phải hết. Một đoàn
            // hộ tống đủ mạnh vẫn có thể bị đánh, và đó là lý do người ta vẫn
            // mua bảo hiểm.
            let r = if self.escorted {
                i64::from(l.banditry) / 5
            } else {
                i64::from(l.banditry)
            };
            let c = con * r / 1_000;
            con -= c;
            cuop += c;
        }

        Progress {
            legs_done: xong,
            remaining: con,
            arrived: xong == self.route.len(),
            blocked_at: None,
            spoiled: hao,
            raided: cuop,
        }
    }

    /// **Ai chịu trách nhiệm** ở thời điểm `at`.
    ///
    /// Trả về người giữ hàng gần nhất trước `at`. Trước lần bàn giao đầu tiên thì
    /// là người gửi — hàng chưa rời tay ai thì chưa ai nhận trách nhiệm.
    pub fn liable_at(&self, at: Tick) -> EntityId {
        self.chain
            .iter()
            .filter(|h| h.from_tick <= at.0)
            .max_by_key(|h| h.from_tick)
            .map_or(self.shipper, |h| h.custodian)
    }
}

/// Một hợp đồng lao động (`§12.17.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabourContract {
    /// Người làm.
    pub worker: EntityId,
    /// Người thuê.
    pub employer: EntityId,
    /// Tiền công mỗi kỳ.
    pub wage: i64,
    /// Thời hạn, tính bằng tick. `None` là vô thời hạn.
    pub term_ticks: Option<u64>,
    /// Giờ làm mỗi ngày.
    pub hours_per_day: u8,
    /// Mức rủi ro của công việc, `0`–`1000`.
    pub hazard: u16,
    /// Có quyền nghỉ không.
    pub has_leave: bool,
    /// Ai chịu trách nhiệm nếu hỏng công cụ.
    pub tool_liability: EntityId,
}

impl LabourContract {
    /// Hợp đồng này có **bóc lột** không, theo một chuẩn mực cho trước.
    ///
    /// Không có định nghĩa tuyệt đối: `min_wage`, `max_hours`, `max_hazard` là
    /// dữ liệu của `norm_set`. Cùng một hợp đồng có thể hợp pháp ở nơi này và bị
    /// coi là nô dịch ở nơi khác — đúng như `§12.5.1` nói về mọi thứ khác.
    pub fn is_exploitative(&self, min_wage: i64, max_hours: u8, max_hazard: u16) -> bool {
        self.wage < min_wage
            || self.hours_per_day > max_hours
            || self.hazard > max_hazard
            || !self.has_leave
    }
}

/// Một nghề mà ai đó **quan sát được** người khác đang làm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedTrade {
    /// Nghề gì.
    pub trade: String,
    /// Người đang làm nó làm tốt tới đâu, `0`–`1000`.
    pub visible_skill: u16,
    /// Thu nhập quan sát được.
    pub visible_income: i64,
    /// Đang thiếu bao nhiêu người.
    pub shortage: i32,
}

/// Một người chọn nghề gì (`§12.17.3`).
///
/// **Chuyên môn hóa không được gán sẵn.** Nó phải rơi ra từ việc thấy người khác
/// làm gì, thấy chỗ nào thiếu người, và thấy nghề nào có uy tín và thu nhập.
///
/// Trả `None` khi `visible_peers` rỗng — và đó chính là kết quả của thí nghiệm
/// đối chứng: bị chặn tri giác xã hội thì không ai phân hóa, tất cả làm cùng một
/// việc. Người gọi hiểu `None` là "làm việc mặc định như mọi người khác".
pub fn specialize(
    visible_peers: &[ObservedTrade],
    own_aptitude: &[(String, u16)],
) -> Option<String> {
    if visible_peers.is_empty() {
        return None;
    }

    let mut tot_nhat: Option<(i64, String)> = None;
    for t in visible_peers {
        let nang_khieu = own_aptitude
            .iter()
            .find(|(n, _)| *n == t.trade)
            .map_or(0, |(_, v)| i64::from(*v));

        // Thiếu người thì đáng vào; đã đông thì thôi. Thu nhập quan sát được kéo
        // người ta tới. Năng khiếu nhân lên, vì làm nghề mình không hợp thì thu
        // nhập kia không thành hiện thực.
        let diem = (i64::from(t.shortage.clamp(-1_000, 1_000)) * 2
            + t.visible_income
            + i64::from(t.visible_skill) / 2)
            * (500 + nang_khieu)
            / 1_000;

        // Phá hòa bằng tên nghề để kết quả xác định.
        match &tot_nhat {
            Some((d, n)) if *d > diem || (*d == diem && *n <= t.trade) => {}
            _ => tot_nhat = Some((diem, t.trade.clone())),
        }
    }
    tot_nhat.map(|(_, n)| n)
}
