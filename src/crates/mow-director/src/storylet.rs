//! Storylet và Director (`idea.md §15.6`, `§15.4`, `§22.53`, `PD-17`).
//!
//! ## Vì sao không để LLM tự nghĩ ra sự kiện
//!
//! `§15.6` mở đầu bằng ba lý do, và cả ba đều đúng:
//!
//! > Nếu để LLM tự nghĩ ra sự kiện mỗi lần thì **mất tính kiểm chứng**, **mất
//! > khả năng mod**, và **rất dễ lặp**.
//!
//! Cái thứ ba là cái bất ngờ: một mô hình được hỏi "chuyện gì nên xảy ra bây
//! giờ" sẽ trả lời bằng những mẫu hình quen thuộc nhất của nó, nên sau vài chục
//! giờ chơi người ta nhận ra mọi ngôi làng đều gặp cùng một loại tai họa.
//!
//! ## Bốn quy tắc, và quy tắc 1 là cả thiết kế
//!
//! > **Storylet chỉ đặt điều kiện, không bao giờ đặt kết quả.** Trường
//! > `outcomes` cố ý luôn rỗng.
//!
//! Nên trong [`Storylet`] **không có trường `outcomes`**. Không phải để trống —
//! không tồn tại. Một trường luôn rỗng là một trường sớm muộn sẽ có người điền,
//! và ngày đó `§17.3` trở thành một lời hứa suông.
//!
//! Cái storylet làm là [`Storylet::perturbation`]: **đổi điều kiện thế giới**.
//! Mỏ ngập nước. Sản lượng quặng tụt. Chuyện gì xảy ra sau đó là việc của những
//! người sống ở đó, và không ai — kể cả Director — biết trước.
//!
//! ## Quy tắc 3 giữ Director khỏi bám lấy người chơi
//!
//! > Chọn theo salience **trong ngân sách và cooldown**, nên Director không thể
//! > liên tục nhắm vào một entity chỉ vì người chơi đang xem.
//!
//! `player_focus_region` **được phép** cộng salience — nhìn vào đâu thì chỗ đó
//! nên sống động hơn. Nhưng `cooldown` chặn việc cùng một tai họa giáng xuống
//! mãi, và `budget` chặn việc mọi tai họa cùng giáng xuống một lúc.

use serde::{Deserialize, Serialize};

/// Một vị từ trên **world state thật** — không phải văn bản.
///
/// `§15.6` quy tắc 2: *"Không có storylet nào kích hoạt được nếu thế giới chưa
/// có nguyên nhân."* Đó là nguyên tắc "khuếch đại nguyên nhân đã tồn tại" ở
/// `§15.4`, dưới dạng một kiểu dữ liệu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precondition {
    /// Hạ tầng loại này có tồn tại không.
    InfrastructureExists {
        /// Loại.
        kind: String,
    },
    /// Một áp lực đã vượt ngưỡng chưa.
    Pressure {
        /// Tên áp lực.
        name: String,
        /// Ngưỡng, `0`–`1000`.
        min: u16,
    },
    /// Chuyện này **chưa** xảy ra gần đây.
    NotRecent {
        /// Storylet nào.
        storylet: String,
        /// Trong bao nhiêu tick.
        within: u64,
    },
    /// Một cờ trạng thái của thế giới.
    Flag {
        /// Tên.
        name: String,
    },
}

/// Thế giới đang thế nào — đầu vào để kiểm vị từ.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldFacts {
    /// Hạ tầng đang có.
    pub infrastructure: Vec<String>,
    /// Các áp lực và mức của chúng.
    pub pressures: Vec<(String, u16)>,
    /// Storylet nào đã kích hoạt lần cuối ở tick nào.
    pub last_fired: Vec<(String, u64)>,
    /// Cờ đang bật.
    pub flags: Vec<String>,
    /// Bây giờ là tick nào.
    pub now: u64,
    /// Người chơi đang nhìn vùng nào.
    pub player_focus: Option<String>,
}

impl Precondition {
    /// Vị từ này có thỏa không.
    pub fn holds(&self, w: &WorldFacts) -> bool {
        match self {
            Precondition::InfrastructureExists { kind } => w.infrastructure.contains(kind),
            Precondition::Pressure { name, min } => {
                w.pressures.iter().any(|(n, v)| n == name && *v >= *min)
            }
            Precondition::NotRecent { storylet, within } => !w
                .last_fired
                .iter()
                .any(|(s, t)| s == storylet && w.now.saturating_sub(*t) < *within),
            Precondition::Flag { name } => w.flags.contains(name),
        }
    }

    /// Mô tả đọc được — cho audit view ở `§15.4`.
    pub fn describe(&self) -> String {
        match self {
            Precondition::InfrastructureExists { kind } => format!("có hạ tầng `{kind}`"),
            Precondition::Pressure { name, min } => format!("áp lực `{name}` ≥ {min}"),
            Precondition::NotRecent { storylet, within } => {
                format!("`{storylet}` chưa xảy ra trong {within} tick")
            }
            Precondition::Flag { name } => format!("cờ `{name}` đang bật"),
        }
    }
}

/// Một điều kiện cộng thêm salience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Boost {
    /// Khi nào.
    pub when: Precondition,
    /// Cộng bao nhiêu, `0`–`1000`.
    pub by: u16,
}

/// Storylet chỉ **đổi điều kiện thế giới**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perturbation {
    /// Áp một effect lên một vùng.
    ApplyEffect {
        /// Effect nào.
        effect: String,
        /// Ở đâu.
        target: String,
    },
    /// Đổi sản lượng một tài nguyên, phần nghìn.
    ResourceDelta {
        /// Tài nguyên nào.
        resource: String,
        /// Đổi bao nhiêu.
        delta: i32,
    },
    /// Bật một cờ.
    SetFlag {
        /// Tên.
        name: String,
    },
}

/// Một storylet.
///
/// **Không có trường `outcomes`.** Xem docstring của module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Storylet {
    /// Định danh có namespace — điểm mở rộng của plugin (`§19.7`).
    pub id: String,
    /// Vị từ phải thỏa **hết**.
    pub preconditions: Vec<Precondition>,
    /// Salience nền, `0`–`1000`.
    pub base_salience: u16,
    /// Cộng thêm khi có điều kiện.
    pub boosts: Vec<Boost>,
    /// **Chỉ đổi điều kiện thế giới.**
    pub perturbation: Vec<Perturbation>,
    /// Tốn bao nhiêu ngân sách.
    pub budget_cost: u32,
    /// Nghỉ bao nhiêu tick trước khi có thể kích hoạt lại.
    pub cooldown: u64,
    /// Ai đóng góp: `core`, `yuu_director`, tên pack.
    pub provenance: String,
}

/// Vì sao một storylet **không** được chọn, hoặc đã được chọn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit {
    /// Storylet nào.
    pub storylet: String,
    /// Từng vị từ, và nó có thỏa không — **cho audit view**.
    pub preconditions: Vec<(String, bool)>,
    /// Salience đã tính.
    pub salience: u16,
    /// Phân rã salience.
    pub salience_parts: Vec<(String, u16)>,
    /// Có được chọn không.
    pub fired: bool,
    /// Nếu không thì vì sao.
    pub rejected_because: Option<String>,
}

impl Storylet {
    /// Mọi vị từ có thỏa không.
    pub fn eligible(&self, w: &WorldFacts) -> bool {
        self.preconditions.iter().all(|p| p.holds(w))
    }

    /// Salience hiện tại, `0`–`1000`, kèm phân rã.
    pub fn salience(&self, w: &WorldFacts) -> (u16, Vec<(String, u16)>) {
        let mut parts = vec![("nền".to_owned(), self.base_salience)];
        let mut tong = u32::from(self.base_salience);
        for b in &self.boosts {
            if b.when.holds(w) {
                parts.push((b.when.describe(), b.by));
                tong += u32::from(b.by);
            }
        }
        (u16::try_from(tong.min(1_000)).unwrap_or(1_000), parts)
    }

    /// Còn trong thời gian nghỉ không.
    pub fn on_cooldown(&self, w: &WorldFacts) -> bool {
        w.last_fired
            .iter()
            .any(|(s, t)| *s == self.id && w.now.saturating_sub(*t) < self.cooldown)
    }
}

/// Một storylet đã chấm điểm, chờ xét ngân sách.
///
/// Đặt tên thay vì viết tuple sáu tầng ngay trong thân hàm: một kiểu mà người
/// đọc phải đếm dấu ngoặc mới hiểu là một kiểu sẽ bị hiểu sai lúc sửa.
type Cham<'a> = (
    u16,
    &'a Storylet,
    Vec<(String, bool)>,
    Vec<(String, u16)>,
    Option<String>,
);

/// Director chọn storylet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Director {
    /// Ngân sách mỗi lần chọn.
    ///
    /// `§15.6` quy tắc 3: ngân sách là thứ ngăn mọi tai họa cùng giáng xuống một
    /// lúc. Không có nó, một thế giới đang khủng hoảng sẽ hút mọi storylet về
    /// cùng một chỗ.
    pub budget: u32,
}

impl Director {
    /// Chọn storylet để kích hoạt, và trả về **audit đầy đủ cho tất cả**.
    ///
    /// Trả cả những cái **không** được chọn kèm lý do. `§15.6` nói storylet là
    /// dữ liệu auditable: audit view hiển thị được đúng storylet nào đã kích
    /// hoạt, vì precondition nào, và salience bao nhiêu — *"thay vì một câu giải
    /// thích do LLM viết ra sau khi mọi chuyện đã xong"*.
    ///
    /// Muốn thế thì phải giữ lại cả những cái trượt: câu hỏi hay gặp nhất không
    /// phải "vì sao chuyện này xảy ra" mà là "vì sao chuyện kia **không** xảy ra".
    pub fn select(&self, pool: &[Storylet], w: &WorldFacts) -> Vec<Audit> {
        let mut cham: Vec<Cham<'_>> = Vec::new();

        for s in pool {
            let vi_tu: Vec<(String, bool)> = s
                .preconditions
                .iter()
                .map(|p| (p.describe(), p.holds(w)))
                .collect();
            let (sal, parts) = s.salience(w);

            let ly_do = if !s.eligible(w) {
                Some("thế giới chưa có nguyên nhân".to_owned())
            } else if s.on_cooldown(w) {
                Some(format!("còn nghỉ, cooldown {} tick", s.cooldown))
            } else {
                None
            };
            cham.push((sal, s, vi_tu, parts, ly_do));
        }

        // Salience cao trước; phá hòa bằng `id` để kết quả xác định.
        cham.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));

        let mut con_lai = self.budget;
        let mut ra = Vec::new();
        for (sal, s, vi_tu, parts, ly_do) in cham {
            let mut ly_do = ly_do;
            let mut fired = false;
            if ly_do.is_none() {
                if s.budget_cost <= con_lai {
                    con_lai -= s.budget_cost;
                    fired = true;
                } else {
                    ly_do = Some(format!(
                        "hết ngân sách: cần {}, còn {con_lai}",
                        s.budget_cost
                    ));
                }
            }
            ra.push(Audit {
                storylet: s.id.clone(),
                preconditions: vi_tu,
                salience: sal,
                salience_parts: parts,
                fired,
                rejected_because: ly_do,
            });
        }
        ra
    }
}
