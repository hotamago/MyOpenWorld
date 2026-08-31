//! Modifier pipeline (`idea.md §9.8`, `§22.20`).
//!
//! > Effect chỉ tác động qua modifier pipeline và **không bao giờ ghi base
//! > stat**; thứ tự áp dụng sắp theo khóa ổn định.
//!
//! ## Vì sao không ghi thẳng
//!
//! Cám dỗ: một lời nguyền làm giảm sức mạnh, nên trừ 5 vào `strength`. Nó chạy
//! ngay và ít code hơn hẳn. Nó cũng hỏng ngay khi có cái thứ hai:
//!
//! - Gỡ lời nguyền thì cộng lại 5 — nhưng nếu trong lúc đó nhân vật đã lên
//!   cấp và `strength` đã đổi thì cộng lại 5 là sai.
//! - Hai lời nguyền chồng nhau, gỡ một cái, và không ai biết cái nào đã trừ
//!   bao nhiêu.
//! - Người chơi hỏi "vì sao sức mạnh của tôi là 12", và câu trả lời không tồn
//!   tại ở đâu cả — chỉ có một con số 12.
//!
//! Với pipeline, base stat **không bao giờ đổi**. Giá trị hiệu dụng được tính
//! lại từ base cộng danh sách modifier đang có, và danh sách đó chính là câu
//! trả lời cho "vì sao".
//!
//! ## Thứ tự áp dụng phải ổn định
//!
//! `+5` rồi `×2` cho 34; `×2` rồi `+5` cho 29. Nếu thứ tự phụ thuộc vào effect
//! nào được áp trước, thì hai nhân vật giống hệt nhau sẽ có chỉ số khác nhau
//! tùy vào lịch sử — và không cách nào tái hiện.
//!
//! Nên thứ tự là **hàm của dữ liệu**: theo [`Op`] trước, rồi theo `source` để
//! phá hòa.

use mow_math::{CanonicalHash, Fx, StateHasher};
use serde::{Deserialize, Serialize};

/// Phép mà một modifier thực hiện.
///
/// Thứ tự khai báo **chính là** thứ tự áp dụng. Đổi thứ tự các biến thể ở đây
/// sẽ đổi kết quả của mọi chỉ số trong thế giới, nên nó cần một migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// Đặt giá trị nền, ghi đè base. Dùng cho biến hình.
    Set,
    /// Cộng vào.
    Add,
    /// Nhân với một tỉ lệ.
    Multiply,
    /// Kẹp xuống trần.
    Cap,
    /// Nâng lên sàn.
    Floor,
}

/// Chính sách chồng chập khi nhiều effect cùng nguồn tác động lên một chỉ số
/// (`§9.8`).
///
/// Năm chính sách, và mỗi cái mô tả một loại hiện tượng khác nhau trong thế
/// giới. Chọn sai làm cơ chế trở nên vô lý theo cách người chơi cảm thấy được
/// nhưng không diễn đạt được.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stacking {
    /// Cộng dồn hết. Ba vết thương thì đau gấp ba.
    Additive,
    /// Chỉ cái mạnh nhất có tác dụng. Ba nguồn ánh sáng thì sáng bằng cái mạnh nhất.
    HighestOnly,
    /// Áp cái mới nhất, gỡ cái cũ. Một trạng thái thời tiết thay một trạng thái khác.
    Replace,
    /// Hiệu quả giảm dần theo số lượng.
    ///
    /// Cái thứ hai tác dụng một nửa, cái thứ ba một phần tư. Đây là chính sách
    /// đúng cho hầu hết buff: nó ngăn được việc chồng hai mươi lá bùa nhỏ để
    /// đạt hiệu quả vô hạn, mà không cần một trần cứng nào.
    DiminishingReturns,
    /// Không chồng được: nguồn thứ hai thất bại.
    Exclusive,
}

/// Một modifier đang tác động.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifier {
    /// Chỉ số bị tác động: `core.strength`, `core.move_speed`.
    pub stat: String,
    /// Phép.
    pub op: Op,
    /// Giá trị. Với [`Op::Multiply`] đây là tỉ lệ, `Fx::ONE` là không đổi.
    pub value: Fx,
    /// Effect nào sinh ra nó. Dùng để phá hòa thứ tự và để trả lời "vì sao".
    pub source: String,
    /// Chính sách chồng chập.
    pub stacking: Stacking,
}

impl CanonicalHash for Modifier {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.stat);
        h.write_str(match self.op {
            Op::Set => "set",
            Op::Add => "add",
            Op::Multiply => "multiply",
            Op::Cap => "cap",
            Op::Floor => "floor",
        });
        h.write_i64(self.value.raw());
        h.write_str(&self.source);
    }
}

/// Một bước trong lời giải thích: modifier nào, đổi từ bao nhiêu thành bao nhiêu.
///
/// `§18.13` yêu cầu mọi giá trị suy ra phải bấm được về nguồn. Đây là cấu trúc
/// trả lời điều đó — nó được sinh ra **cùng lúc** với phép tính, không phải
/// dựng lại sau.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Nguồn.
    pub source: String,
    /// Phép.
    pub op: Op,
    /// Giá trị trước bước này.
    pub before: Fx,
    /// Giá trị sau.
    pub after: Fx,
}

/// Kết quả tính một chỉ số.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Giá trị nền, **không bao giờ bị sửa**.
    pub base: Fx,
    /// Giá trị hiệu dụng.
    pub value: Fx,
    /// Từng bước, theo thứ tự áp dụng.
    pub steps: Vec<Step>,
}

/// Tính giá trị hiệu dụng của một chỉ số.
///
/// `base` là giá trị nền và nó đi vào đây **theo giá trị**, không phải theo
/// tham chiếu có thể sửa. Đó là cách `§22.20` được thi hành bằng chữ ký hàm:
/// hàm này không có cách nào ghi vào base.
pub fn resolve(base: Fx, mods: &[Modifier]) -> Resolved {
    let ap_dung = apply_stacking(mods);

    let mut v = base;
    let mut steps = Vec::with_capacity(ap_dung.len());

    for m in &ap_dung {
        let truoc = v;
        v = match m.op {
            Op::Set => m.value,
            Op::Add => v.add(m.value).unwrap_or(v),
            Op::Multiply => v.mul(m.value).unwrap_or(v),
            Op::Cap => {
                if v > m.value {
                    m.value
                } else {
                    v
                }
            }
            Op::Floor => {
                if v < m.value {
                    m.value
                } else {
                    v
                }
            }
        };
        steps.push(Step {
            source: m.source.clone(),
            op: m.op,
            before: truoc,
            after: v,
        });
    }

    Resolved {
        base,
        value: v,
        steps,
    }
}

/// Áp chính sách chồng chập, rồi sắp theo thứ tự ổn định.
fn apply_stacking(mods: &[Modifier]) -> Vec<Modifier> {
    use std::collections::BTreeMap;

    // Nhóm theo `(stat, op, stacking)`: chính sách chỉ áp giữa những modifier
    // cùng loại. Một buff cộng và một buff nhân không cạnh tranh nhau.
    let mut nhom: BTreeMap<(String, Op, Stacking), Vec<&Modifier>> = BTreeMap::new();
    for m in mods {
        nhom.entry((m.stat.clone(), m.op, m.stacking))
            .or_default()
            .push(m);
    }

    let mut ra: Vec<Modifier> = Vec::new();
    for ((_, _, policy), mut ds) in nhom {
        // Sắp theo `source` để mọi lựa chọn dưới đây là hàm của dữ liệu, không
        // của thứ tự chèn.
        ds.sort_by(|a, b| a.source.cmp(&b.source));

        match policy {
            Stacking::Additive => ra.extend(ds.into_iter().cloned()),

            Stacking::HighestOnly => {
                if let Some(m) = ds.iter().max_by_key(|m| m.value.raw()) {
                    ra.push((*m).clone());
                }
            }

            Stacking::Replace | Stacking::Exclusive => {
                // Với `Replace`, "mới nhất" được biểu diễn bằng `source` lớn
                // nhất — nguồn mang dấu thời gian trong id. Với `Exclusive`,
                // nguồn thứ hai lẽ ra đã bị từ chối lúc áp; nếu vẫn có mặt thì
                // đây là lưới cuối, và nó chọn một cách xác định thay vì chọn
                // cái tình cờ tới trước.
                if let Some(m) = ds.last() {
                    ra.push((*m).clone());
                }
            }

            Stacking::DiminishingReturns => {
                // Cái mạnh nhất tác dụng đủ, cái sau một nửa, rồi một phần tư.
                let mut theo_manh = ds.clone();
                theo_manh.sort_by(|a, b| {
                    b.value
                        .raw()
                        .cmp(&a.value.raw())
                        .then(a.source.cmp(&b.source))
                });
                for (i, m) in theo_manh.into_iter().enumerate() {
                    let chia = 1i64 << i.min(20);
                    let mut m2 = m.clone();
                    m2.value = Fx::from_raw(m.value.raw() / chia);
                    ra.push(m2);
                }
            }
        }
    }

    // Thứ tự cuối: theo `Op` (thứ tự khai báo của enum), rồi theo `source`.
    // Đây là chỗ `§22.20` "sắp theo khóa ổn định" được thực hiện.
    ra.sort_by(|a, b| a.op.cmp(&b.op).then(a.source.cmp(&b.source)));
    ra
}
