//! Hành động tập thể theo ngưỡng (`idea.md §12.11`, `PD-07`).
//!
//! > Một khác biệt nhỏ trong **phân bố ngưỡng** có thể khiến hai đám đông giống
//! > hệt nhau đi tới hai kết cục hoàn toàn khác — một bên giải tán, một bên lật
//! > đổ chính quyền.
//!
//! Đây là mô hình Granovetter, và nó là một trong những kết quả phản trực giác
//! nhất của khoa học xã hội: **hai đám đông có cùng ngưỡng trung bình** có thể
//! cho ra 0 người tham gia và 100 người tham gia. Trung bình không dự đoán được
//! gì cả; chỉ có phân bố mới dự đoán được.
//!
//! ```text
//!  Đám A: ngưỡng 0,1,2,3,...,99   → 100 người tham gia (dây chuyền đủ)
//!  Đám B: ngưỡng 0,2,2,3,...,99   → 1 người tham gia   (đứt ở bậc 1)
//!         ▲
//!         └── một người đổi ý, và cuộc nổi dậy không xảy ra
//! ```
//!
//! ## Vì sao đây là lý do Director **không được phép ép kết quả**
//!
//! `§15.4`: Yuu Director chỉ đặt áp lực, phần còn lại là động lực học của ngưỡng.
//!
//! Nếu Director ép được kết quả thì cuộc nổi dậy trở thành một cảnh đã viết sẵn,
//! và người chơi không bao giờ học được rằng *một người đổi ý ở đúng bậc* mới là
//! thứ quyết định. Còn nếu Director chỉ được đẩy ngưỡng của vài người, thì cùng
//! một hành động của người chơi sẽ có lúc lật đổ được chính quyền và có lúc
//! không — tùy vào chỗ đứt nằm ở đâu, và điều đó **học được**.
//!
//! ## Kỳ vọng tính theo belief, không theo con số thật
//!
//! Người ta quyết định tham gia dựa trên **họ nghĩ bao nhiêu người sẽ tham gia**,
//! không phải con số cuối cùng. Nên [`cascade`] lặp: mỗi vòng, số người đã tham
//! gia trở thành thông tin cho vòng sau. Một tin đồn phóng đại quy mô có thể
//! **tự làm cho mình đúng**.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Một người có thể tham gia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    /// Ai.
    pub who: EntityId,
    /// **Ngưỡng**: cần bao nhiêu phần nghìn người khác tham gia thì mình mới tham gia.
    ///
    /// `0` là người khởi xướng — làm dù chẳng ai làm. `1000` là người không bao
    /// giờ tham gia dù cả làng đã xuống đường.
    pub threshold: u16,
    /// Chi phí mà người này phải chịu nếu tham gia, `0`–`1000`.
    pub cost: u16,
    /// **Kẻ ăn theo**: hưởng kết quả mà không chịu chi phí.
    ///
    /// Không phải một loại người xấu — là một vai trò cấu trúc. Càng nhiều người
    /// ăn theo, ngưỡng thực tế của mọi người càng cao, vì lợi ích chung bị chia
    /// mỏng đi mà chi phí thì không.
    pub free_rider: bool,
}

/// Tín hiệu từ chính quyền.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Signal {
    /// Không làm gì.
    Silence,
    /// Đe dọa đàn áp: đẩy ngưỡng mọi người lên.
    Repression {
        /// Mức, `0`–`1000`.
        severity: u16,
    },
    /// Nhượng bộ: giảm động cơ tham gia của người ôn hòa, nhưng **không** của
    /// người cực đoan — và đó là cách nhượng bộ đôi khi làm phong trào cực đoan hơn.
    Concession {
        /// Mức, `0`–`1000`.
        size: u16,
    },
}

/// Kết quả một vòng lan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cascade {
    /// Ai đã tham gia, theo thứ tự ổn định.
    pub joined: Vec<EntityId>,
    /// Sau bao nhiêu vòng thì dừng.
    pub rounds: u32,
    /// Phần nghìn dân đã tham gia.
    pub participation: u16,
}

impl Cascade {
    /// Phong trào có thành không, theo một ngưỡng thành công cho trước.
    pub fn succeeded(&self, needed: u16) -> bool {
        self.participation >= needed
    }
}

/// Cho phong trào lan tới khi ổn định.
///
/// **Hàm thuần và xác định.** Không có ngẫu nhiên: kết quả khác nhau phải đến từ
/// *phân bố ngưỡng khác nhau*, không từ một lần tung xúc xắc — nếu không, bài
/// học mà `§12.11` muốn dạy biến thành "may rủi".
pub fn cascade(people: &[Participant], signal: Signal, max_rounds: u32) -> Cascade {
    let n = people.len();
    if n == 0 {
        return Cascade {
            joined: Vec::new(),
            rounds: 0,
            participation: 0,
        };
    }

    // Tín hiệu chính quyền dịch ngưỡng của mọi người.
    let dich = |p: &Participant| -> i64 {
        let goc = i64::from(p.threshold) + i64::from(p.cost) / 4;
        match signal {
            Signal::Silence => goc,
            // Đàn áp đẩy ngưỡng lên: phải thấy nhiều người hơn mới dám.
            Signal::Repression { severity } => goc + i64::from(severity),
            // Nhượng bộ chỉ làm nguội người ngưỡng thấp — người đã sẵn sàng đi
            // đầu thì một nhượng bộ nhỏ không đổi được gì. Đây là cơ chế làm
            // phong trào **cực đoan hơn** sau một nhượng bộ nửa vời.
            Signal::Concession { size } => {
                if p.threshold < 500 {
                    goc + i64::from(size)
                } else {
                    goc
                }
            }
        }
    };

    let mut tham_gia = vec![false; n];
    let mut rounds = 0;

    for vong in 1..=max_rounds {
        rounds = vong;
        let da = tham_gia.iter().filter(|x| **x).count();
        // Kỳ vọng của người đang cân nhắc = số người **hiện đang** tham gia.
        let ky_vong = i64::try_from(da * 1_000 / n).unwrap_or(0);

        let mut doi = false;
        for (i, p) in people.iter().enumerate() {
            if tham_gia[i] || p.free_rider {
                continue;
            }
            if ky_vong >= dich(p) {
                tham_gia[i] = true;
                doi = true;
            }
        }
        if !doi {
            break;
        }
    }

    let joined: Vec<EntityId> = people
        .iter()
        .zip(&tham_gia)
        .filter(|(_, t)| **t)
        .map(|(p, _)| p.who)
        .collect();
    let participation = u16::try_from(joined.len() * 1_000 / n).unwrap_or(1_000);

    Cascade {
        joined,
        rounds,
        participation,
    }
}
