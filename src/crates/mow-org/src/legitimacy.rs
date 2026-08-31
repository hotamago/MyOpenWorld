//! Chính danh và ba động cơ tuân thủ (`idea.md §12.13.2`, `PD-05`).
//!
//! > Ba động cơ cho ra kết quả **giống nhau khi nhà nước mạnh**, và **khác nhau
//! > hoàn toàn vào ngày nhà nước yếu đi** — đó là lúc một chế độ dựa trên sợ hãi
//! > sụp trong một tuần còn chế độ dựa trên niềm tin vẫn đứng.
//!
//! ## Vì sao không gộp thành một chỉ số "ổn định"
//!
//! Vì một chỉ số duy nhất **không phân biệt được** hai chế độ cùng đạt 0.9 ổn
//! định. Chúng trông y hệt nhau trên bảng điều khiển, và rồi một cái sụp trong
//! một tuần còn cái kia không — mà người chơi không có cách nào biết trước, cũng
//! không có cách nào giải thích sau.
//!
//! Ba trường tách rời làm cho câu hỏi *"chế độ này đứng bằng gì"* trả lời được,
//! và làm cho hành động của người chơi có nghĩa: tuyên truyền nuôi [`Motive::Belief`],
//! tuần tra nuôi [`Motive::Fear`], và lễ hội nuôi [`Motive::Conformity`] — ba
//! việc khác nhau, ba cái giá khác nhau, ba kiểu sụp khác nhau.
//!
//! ## Điểm gãy
//!
//! ```text
//!  sức mạnh nhà nước ──────────────────────────────►
//!  1000                          500                      0
//!   │ cả ba đều tuân    │ sợ hãi bắt đầu rơi │ chỉ niềm tin còn
//! ```
//!
//! `Fear` tỉ lệ thuận với năng lực cưỡng chế: nhà nước yếu đi thì nó **tan ngay**.
//! `Conformity` phụ thuộc việc người khác còn tuân không, nên nó sụp theo dây
//! chuyền — chậm hơn sợ hãi một nhịp, rồi rất nhanh. `Belief` không phụ thuộc
//! sức mạnh chút nào, và đó là lý do nó là thứ đắt nhất để xây và bền nhất khi mất.

use serde::{Deserialize, Serialize};

/// Vì sao người ta tuân lệnh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Motive {
    /// Tin rằng luật đúng.
    Belief,
    /// Sợ hình phạt.
    Fear,
    /// Thấy mọi người xung quanh đang tuân.
    Conformity,
}

/// Nguồn chính danh (`§12.13.2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Kết quả đạt được.
    Performance,
    /// Thủ tục công bằng.
    Procedure,
    /// Truyền thống.
    Tradition,
    /// Sức hút cá nhân.
    Charisma,
    /// Tôn giáo.
    Religion,
    /// Bản sắc cộng đồng.
    Identity,
}

/// Chính danh của một chế độ, tách theo động cơ.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Legitimacy {
    /// Bao nhiêu phần dân tuân vì **tin**, `0`–`1000`.
    pub belief: u16,
    /// Bao nhiêu phần tuân vì **sợ**, `0`–`1000`.
    pub fear: u16,
    /// Bao nhiêu phần tuân vì **thấy người khác tuân**, `0`–`1000`.
    pub conformity: u16,
    /// Chính danh đến từ đâu — dùng cho UI và cho việc quyết định hành động nào
    /// nuôi được nó.
    pub sources: Vec<Source>,
}

/// Mức tuân thủ đã tính, kèm phân rã.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compliance {
    /// Tổng, `0`–`1000`.
    pub total: u16,
    /// Từng động cơ đóng góp bao nhiêu.
    pub by_motive: Vec<(Motive, u16)>,
}

impl Compliance {
    /// Động cơ đang đỡ chế độ này nhiều nhất.
    pub fn dominant(&self) -> Option<Motive> {
        self.by_motive
            .iter()
            .max_by_key(|(m, v)| (*v, std::cmp::Reverse(*m)))
            .map(|(m, _)| *m)
    }
}

impl Legitimacy {
    /// Mức tuân thủ ở một mức **sức mạnh nhà nước** cho trước, `0`–`1000`.
    ///
    /// Đây là hàm mà cả `PD-05` xoay quanh. Ba động cơ phản ứng khác nhau với
    /// cùng một sự suy yếu, và đó là toàn bộ nội dung.
    pub fn compliance(&self, state_strength: u16) -> Compliance {
        let s = i64::from(state_strength);

        // Tin thì tuân, bất kể nhà nước còn mạnh hay không.
        let tin = i64::from(self.belief);

        // Sợ tỉ lệ thẳng với năng lực cưỡng chế: không còn ai đến bắt thì không
        // còn ai sợ. Đây là chỗ chế độ dựa trên sợ hãi sụp trong một tuần.
        let so = i64::from(self.fear) * s / 1_000;

        // Hùa theo phụ thuộc **số người còn đang tuân**, không phụ thuộc nhà
        // nước. Nó sụp theo dây chuyền: chậm hơn sợ hãi một nhịp rồi rất nhanh,
        // vì mỗi người bỏ cuộc lại làm người kế tiếp dễ bỏ cuộc hơn.
        let da_tuan = tin + so;
        let hua = i64::from(self.conformity) * da_tuan / 1_000;

        let tong = (tin + so + hua).clamp(0, 1_000);
        Compliance {
            total: u16::try_from(tong).unwrap_or(1_000),
            by_motive: vec![
                (
                    Motive::Belief,
                    u16::try_from(tin.clamp(0, 1_000)).unwrap_or(0),
                ),
                (Motive::Fear, u16::try_from(so.clamp(0, 1_000)).unwrap_or(0)),
                (
                    Motive::Conformity,
                    u16::try_from(hua.clamp(0, 1_000)).unwrap_or(0),
                ),
            ],
        }
    }

    /// Chế độ này sụp ở mức sức mạnh nào.
    ///
    /// Trả về sức mạnh **cao nhất** mà tại đó tuân thủ đã tụt dưới `threshold`.
    /// `None` nghĩa là nó đứng vững tới tận khi nhà nước biến mất hoàn toàn —
    /// điều chỉ xảy ra khi `belief` đủ cao một mình.
    pub fn collapse_point(&self, threshold: u16) -> Option<u16> {
        (0..=1_000u16)
            .rev()
            .find(|s| self.compliance(*s).total < threshold)
    }
}
