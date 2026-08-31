//! Đường đi của một tội (`idea.md §12.5.2`, `PD-02`).
//!
//! ```text
//! nhu cầu thiếu hụt  §9.7.3
//!   + cơ hội          ← tri giác của NGƯỜI KHÁC: ai đang nhìn, trời có tối không
//!   + năng lực        ← skill, sức mạnh, công cụ, phép
//!   + rủi ro ước lượng theo BELIEF về lực lượng cưỡng chế, KHÔNG theo con số thật
//!   + chi phí đạo đức ← traits × values × mức gắn bó với nạn nhân  §9.9.2
//! → ý định → hành động → nhân chứng cảm nhận → chứng cứ → nghi ngờ
//! → điều tra → buộc tội → xét xử → phán quyết → hình phạt → hệ quả
//! ```
//!
//! ## Điểm quan trọng nhất, và nó là một dòng code
//!
//! `§12.5.2` viết:
//!
//! > Kẻ phạm tội ước lượng rủi ro bằng **belief về mức giám sát**, không bằng
//! > `coverage_by_district` thật.
//!
//! Nên [`Temptation::perceived_risk`] nhận `believed_coverage`, và **không có
//! đường nào** để nó đọc [`crate::norms::Enforcement::coverage`]. Đó không phải
//! là sự cẩn thận thừa: nếu hàm này đọc con số thật, ta mất hai hiện tượng mà cả
//! chương `§12.5` tồn tại để tạo ra:
//!
//! - Một chính quyền chỉ cần **làm cho người ta tin** rằng mình giám sát chặt là
//!   đã giảm được tội phạm — quản trị bằng danh tiếng, rẻ hơn bằng tuần tra.
//! - Một đợt tuyên truyền sai tạo ra làn sóng phạm tội mà **chính quyền không
//!   hiểu vì sao**, vì theo sổ sách thì lực lượng của họ vẫn nguyên.
//!
//! ## Vì sao phát hiện đi qua hệ tri giác có sẵn
//!
//! Vì như vậy những thứ sau **rơi ra miễn phí** thay vì phải viết riêng từng cái:
//! ngoại phạm, vu khống, án oan, phi tang, mua chuộc nhân chứng, tội phạm hoàn
//! hảo, và cả trường hợp cả làng đều biết nhưng không ai dám làm chứng.
//!
//! Viết riêng từng cái sẽ cho ra mười hệ thống nhỏ không nhất quán với nhau; đi
//! qua tri giác cho ra một hệ thống mà tất cả chúng là những trường hợp riêng.

use mow_core::EntityId;
use serde::{Deserialize, Serialize};

/// Cân nhắc của một người trước khi phạm tội.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Temptation {
    /// Ai đang cân nhắc.
    pub actor: EntityId,
    /// Hành vi đang cân nhắc.
    pub act: String,
    /// Mức thiếu hụt đang thúc đẩy, `0`–`1000` (`§9.7.3`).
    pub need: u16,
    /// Giá trị thu được nếu trót lọt.
    pub gain: i64,
    /// **Cơ hội**: có bao nhiêu người có thể trông thấy, và trời có tối không.
    ///
    /// `0` là hoàn toàn kín đáo, `1000` là giữa chợ ban ngày.
    pub exposure: u16,
    /// **Năng lực**: skill, sức mạnh, công cụ, phép. `0`–`1000`.
    pub capability: u16,
    /// **Belief** về độ phủ cưỡng chế ở đây, `0`–`1000`.
    ///
    /// Cố ý **không** phải `coverage` thật. Xem docstring của module.
    pub believed_coverage: u16,
    /// **Chi phí đạo đức**: traits × values × mức gắn bó với nạn nhân, `0`–`1000`.
    pub moral_cost: u16,
    /// Mức nghiêm khắc của chế tài nếu bị bắt, `0`–`1000`.
    pub believed_sanction: u16,
}

/// Một phần đóng góp vào quyết định, để giải thích (`§18.13`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weight {
    /// Tên đọc được.
    pub label: String,
    /// Đóng góp; âm là chống lại.
    pub value: i64,
}

/// Kết quả cân nhắc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    /// Có định làm không.
    pub will_act: bool,
    /// Điểm; dương là làm.
    pub score: i64,
    /// Phân rã, để panel "vì sao?" dựng câu trả lời từ dữ liệu.
    pub factors: Vec<Weight>,
}

impl Temptation {
    /// Xác suất **bị bắt** mà người này tin, `0`–`1000`.
    ///
    /// Tích của "bị nhìn thấy" và "có ai đó đến bắt". Tích chứ không phải tổng:
    /// nếu bất kỳ khâu nào bằng 0 thì bằng 0, và đó đúng là cách tội phạm hoàn
    /// hảo hoạt động — không cần cả hai đều thấp, chỉ cần một khâu bị vô hiệu.
    pub fn perceived_catch_chance(&self) -> i64 {
        i64::from(self.exposure) * i64::from(self.believed_coverage) / 1_000
    }

    /// Thiệt hại **kỳ vọng** nếu làm, trên cùng thang với lợi ích.
    ///
    /// ## Vì sao không phải là `exposure × coverage × sanction` rồi trừ thẳng
    ///
    /// Bản đầu tiên viết đúng như thế, và nó **không bao giờ răn đe được ai**:
    /// tích ba số `0`–`1000` chuẩn hóa lại vẫn nằm trong `0`–`1000`, trong khi
    /// vế lợi ích là `need + gain`, tức tới `2000`. Một kẻ trộm ở mức thiếu thốn
    /// trung bình vẫn ra tay dù tin chắc mình đang bị canh gắt — không phải vì
    /// nó liều, mà vì hai vế **không cùng thang**.
    ///
    /// Đây là dạng đúng: kỳ vọng thiệt hại = *xác suất bị bắt* × *mất mát khi bị
    /// bắt*. Mất mát gồm cả chế tài lẫn món lợi bị tịch thu — bị bắt thì không
    /// những bị phạt mà còn mất luôn thứ vừa lấy, và bỏ vế thứ hai làm cho việc
    /// trộm món đắt tiền trở nên **an toàn hơn** trộm món rẻ.
    pub fn expected_loss(&self) -> i64 {
        let p = self.perceived_catch_chance();
        // Chế tài quy về cùng thang với `need + gain` (tối đa 2000).
        let mat_che_tai = i64::from(self.believed_sanction) * 2;
        let mat_mon_loi = self.gain.clamp(0, 1_000);
        p * (mat_che_tai + mat_mon_loi) / 1_000
    }

    /// Cân nhắc, và trả về **cả phân rã**.
    pub fn deliberate(&self) -> Intent {
        let mut factors = vec![
            Weight {
                label: "đang thiếu thốn".into(),
                value: i64::from(self.need),
            },
            Weight {
                label: "món lợi".into(),
                value: self.gain.clamp(0, 1_000),
            },
            Weight {
                label: "rủi ro ước lượng".into(),
                value: -self.expected_loss(),
            },
            Weight {
                label: "chi phí đạo đức".into(),
                value: -i64::from(self.moral_cost),
            },
        ];

        // Năng lực không **thúc đẩy**, nó **cho phép**. Một người muốn trộm mà
        // không mở nổi ổ khóa thì không phải là người ít động cơ hơn — họ chỉ
        // đơn giản là không làm được. Nên nó nhân vào, không cộng vào.
        let thuan: i64 = factors.iter().map(|w| w.value).sum();
        let kha_nang = i64::from(self.capability);
        let score = if thuan > 0 {
            thuan * kha_nang / 1_000
        } else {
            thuan
        };
        factors.push(Weight {
            label: "năng lực".into(),
            value: score - thuan,
        });

        Intent {
            will_act: score > 0,
            score,
            factors,
        }
    }
}

/// Một nhân chứng đã **cảm nhận** được gì đó.
///
/// Là **belief của một entity**, không phải sự thật (`§12.5.3`). Có thể sai thật
/// lòng, có thể nói dối. Hai chuyện đó khác nhau và cả hai đều phải diễn đạt được.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Witness {
    /// Ai chứng kiến.
    pub who: EntityId,
    /// Họ tin ai đã làm. `None` nghĩa là "thấy có người" mà không nhận ra ai.
    pub believes_actor: Option<EntityId>,
    /// Độ chắc chắn, `0`–`1000`.
    pub confidence: u16,
    /// **Động cơ khai báo**, `-1000`..`1000`.
    ///
    /// Âm là có lý do im lặng hoặc nói sai: sợ trả thù, mang ơn thủ phạm, thù
    /// người bị buộc tội. Đây là trường làm nên "cả làng đều biết nhưng không ai
    /// dám làm chứng" — một tình huống không viết riêng dòng nào.
    pub motive_to_testify: i16,
}

impl Witness {
    /// Nhân chứng này có ra làm chứng không.
    pub fn will_testify(&self) -> bool {
        self.motive_to_testify > 0 && self.believes_actor.is_some()
    }

    /// Lời khai có **đúng** không, so với sự thật.
    ///
    /// Dùng cho audit và cho `§18.11` biên niên sử hai lớp — chứ không dùng
    /// trong quá trình xử án. Tòa không có quyền truy cập hàm này, và đó là
    /// toàn bộ điểm mấu chốt: sự lệch giữa phán quyết và sự thật là chất liệu
    /// của lịch sử, không phải một lỗi cần sửa.
    pub fn is_truthful(&self, actual_actor: EntityId) -> bool {
        self.believes_actor == Some(actual_actor)
    }
}
