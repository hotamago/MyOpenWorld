//! Tiền tệ: thang tiến hóa, xu là vật phẩm, vòi và cống (`idea.md §12.8.2`–
//! `§12.8.4`, `PD-11`).
//!
//! ## Tiền **không phải điểm khởi đầu**
//!
//! `§12.8.2` theo Graeber: tín dụng và sổ nợ xuất hiện **trước** tiền đúc, còn
//! "nền kinh tế đổi chác nguyên thủy" gần như không có bằng chứng khảo cổ.
//!
//! ```text
//! mạng nghĩa vụ tương hỗ → tín dụng và sổ nợ → tiền hàng hóa
//!   → tiền đúc do nhà nước phát hành → tiền đại diện
//!   → tiền ngoại lai: mana, linh hồn, lời thề, ân huệ thần linh
//! ```
//!
//! Một world hoàn toàn có thể **không bao giờ có tiền đúc**, và một world khác
//! dùng ân huệ của thần làm đơn vị thanh toán. Cả hai đều hợp lệ, nên
//! [`MonetaryStage`] là dữ liệu của worldseed chứ không phải một giai đoạn mà
//! mọi nền văn minh phải đi qua.
//!
//! ## Lạm phát xuất hiện vì **niềm tin thay đổi**
//!
//! Đồng xu là item, nên nó có thành phần vật chất. Một nhà nước túng quẫn pha
//! loãng hàm lượng bạc; thương nhân biết thử tuổi kim loại phát hiện ra; và giá
//! tăng vì **người ta thôi tin vào đồng xu**, không vì ai đó chỉnh một biến toàn
//! cục.
//!
//! Từ đó rơi ra miễn phí: cắt xén viền xu, nấu chảy xu lấy kim loại, và **luật
//! Gresham** — tích trữ xu tốt, tiêu xu xấu. Xem [`Coinage::gresham_pressure`].
//!
//! ## Vòi và cống
//!
//! Bài học từ EVE Online: nền kinh tế ổn định cần **vòi** cân bằng **cống**, và
//! rút tiền mà không có hao mòn vật chất dẫn thẳng tới giảm phát vì hàng hóa
//! tích tụ mãi trong khi tiền bị hút bớt.
//!
//! [`EconomyProfile`] bắt mỗi world **khai báo rõ** vòi và cống của mình. Không
//! có mặc định: một profile không khai gì là một nền kinh tế mà không ai biết
//! tiền từ đâu ra, và nó sẽ trôi theo cách không ai giải thích được.

use serde::{Deserialize, Serialize};

/// Nấc tiến hóa tiền tệ mà một nền văn minh đang ở.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonetaryStage {
    /// Mạng nghĩa vụ tương hỗ. Chưa có gì gọi là tiền.
    Reciprocity,
    /// Tín dụng và sổ nợ.
    Credit,
    /// Tiền hàng hóa: muối, vải, gia súc.
    Commodity,
    /// Tiền đúc do nhà nước phát hành.
    Coinage,
    /// Tiền đại diện, tín dụng nhà nước.
    Representative,
    /// Tiền ngoại lai: mana, linh hồn, lời thề, ân huệ thần linh.
    Exotic,
}

/// Tích *độ pha loãng × kỹ năng* cần vượt để một người nhận ra đồng xu có vấn đề.
///
/// ## Vì sao con số này phải nhỏ
///
/// Bản đầu đặt `30`, và kết quả là **không ai phát hiện được** một đồng bị pha
/// loãng 20‰ — kể cả thương nhân kỹ năng tối đa, vì `20 × 1000 / 1000 = 20 < 30`.
/// Nghĩa là một nhà nước có thể pha loãng 2% mỗi lần, vô hạn lần, mà không ai
/// từng nghi ngờ. Cơ chế "niềm tin thay đổi dần" ở `§12.8.3` chết ngay tại chỗ,
/// và không có gì báo lỗi — đồng xu vẫn lưu thông, giá vẫn ổn định.
///
/// `15` nghĩa là: một chuyên gia thật sự (kỹ năng 1000) nhận ra pha loãng từ
/// 15‰ trở lên, còn người thường (kỹ năng 300) phải tới 50‰ mới thấy. Khoảng
/// giữa hai mốc đó chính là chỗ nhà nước còn xoay xở được, và là chỗ đáng chơi.
const NGUONG_PHAT_HIEN: i64 = 15;

/// Một loại xu đang lưu hành. **Là vật phẩm** (`§8.5`), nên nó có thành phần.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coinage {
    /// Định danh.
    pub id: String,
    /// Mệnh giá danh nghĩa.
    pub face_value: i64,
    /// Hàm lượng kim loại quý, phần nghìn.
    ///
    /// Đây là trường mà một nhà nước túng quẫn sẽ giảm, và là chỗ lạm phát bắt
    /// đầu — không phải ở một hệ số nào đó.
    pub fineness: u16,
    /// Hàm lượng lúc mới phát hành, để so.
    pub original_fineness: u16,
    /// Khối lượng, để phát hiện cắt xén viền.
    pub weight: u32,
    /// Khối lượng lúc mới đúc.
    pub original_weight: u32,
}

impl Coinage {
    /// Giá trị **nội tại** — kim loại trong nó đáng bao nhiêu.
    ///
    /// Đây là sàn: một đồng xu không bao giờ rẻ hơn thỏi kim loại làm ra nó, vì
    /// người ta sẽ nấu chảy. Nấu chảy xu là hành vi hợp lý, không phải exploit.
    pub fn intrinsic_value(&self) -> i64 {
        self.face_value * i64::from(self.fineness) / 1_000 * i64::from(self.weight)
            / i64::from(self.original_weight.max(1))
    }

    /// Đã bị pha loãng bao nhiêu so với lúc phát hành, phần nghìn.
    pub fn debasement(&self) -> u16 {
        self.original_fineness.saturating_sub(self.fineness)
    }

    /// Đã bị cắt xén viền bao nhiêu, phần nghìn.
    pub fn clipping(&self) -> u16 {
        let mat = self.original_weight.saturating_sub(self.weight);
        u16::try_from(u64::from(mat) * 1_000 / u64::from(self.original_weight.max(1)))
            .unwrap_or(1_000)
    }

    /// Một thương nhân có kỹ năng `skill` (`0`–`1000`) có phát hiện ra không.
    ///
    /// Pha loãng nhẹ thì phải rất giỏi mới thấy; pha loãng nặng thì ai cũng thấy.
    /// Đây là chỗ **niềm tin thay đổi dần**, chứ không sập một lần.
    pub fn detectable_by(&self, skill: u16) -> bool {
        let ro_rang = i64::from(self.debasement()) + i64::from(self.clipping());
        ro_rang * i64::from(skill) / 1_000 >= NGUONG_PHAT_HIEN
    }

    /// **Áp lực Gresham**: xu xấu đuổi xu tốt khỏi lưu thông.
    ///
    /// Trả về `> 0` khi đồng này *tốt hơn* đồng kia và vì thế sẽ bị tích trữ.
    /// Không ai quyết định "hãy tích trữ"; nó là hệ quả của việc hai đồng có
    /// cùng mệnh giá danh nghĩa mà khác giá trị nội tại.
    pub fn gresham_pressure(&self, other: &Coinage) -> i64 {
        if self.face_value != other.face_value {
            return 0;
        }
        self.intrinsic_value() - other.intrinsic_value()
    }
}

/// Một **vòi**: nguồn bơm tiền hoặc hàng vào nền kinh tế.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faucet {
    /// Tên đọc được: `mining`, `state_minting`, `foreign_trade`.
    pub id: String,
    /// Bơm bao nhiêu mỗi kỳ.
    pub rate: i64,
}

/// Một **cống**: đường rút tiền hoặc hàng ra.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sink {
    /// Tên đọc được.
    pub id: String,
    /// Rút bao nhiêu mỗi kỳ.
    pub rate: i64,
    /// Có phải **cống vật chất** không — hao mòn, cháy, chiến tranh.
    ///
    /// `§12.8.4`: hao mòn ở `§8.6.3` là cống vật chất chính. Rút tiền mà không
    /// có cống vật chất dẫn thẳng tới giảm phát, vì hàng hóa tích tụ mãi trong
    /// khi tiền bị hút bớt. Trường này là thứ [`EconomyProfile::audit`] đọc.
    pub physical: bool,
    /// Người ta có **tự nguyện** đi vào cống này không.
    ///
    /// Lễ hội, xây đền, sính lễ, đấu giá địa vị là cống tự nguyện — và là cống
    /// hiệu quả nhất. Ép thuế là cống kém nhất, và nó tạo ra `§12.5`.
    pub voluntary: bool,
}

/// Chẩn đoán của Auditor (`§15.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoneyDiagnosis {
    /// Cân bằng.
    Balanced,
    /// Lạm phát, kèm **nguyên nhân**.
    Inflation {
        /// Vì sao.
        cause: String,
        /// Chênh lệch mỗi kỳ.
        surplus: i64,
    },
    /// Giảm phát, kèm nguyên nhân.
    Deflation {
        /// Vì sao.
        cause: String,
        /// Chênh lệch mỗi kỳ.
        deficit: i64,
    },
}

/// Hồ sơ kinh tế của một world. **Bắt buộc khai vòi và cống.**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyProfile {
    /// Nấc tiền tệ.
    pub stage: MonetaryStage,
    /// Các vòi.
    pub faucets: Vec<Faucet>,
    /// Các cống.
    pub sinks: Vec<Sink>,
    /// Cung tiền hiện tại.
    pub money_supply: i64,
    /// Lượng hàng hóa hiện tại.
    pub goods_supply: i64,
}

impl EconomyProfile {
    /// Tổng bơm vào mỗi kỳ.
    pub fn faucet_rate(&self) -> i64 {
        self.faucets.iter().map(|f| f.rate).sum()
    }

    /// Tổng rút ra mỗi kỳ.
    pub fn sink_rate(&self) -> i64 {
        self.sinks.iter().map(|s| s.rate).sum()
    }

    /// Có cống vật chất không.
    pub fn has_physical_sink(&self) -> bool {
        self.sinks.iter().any(|s| s.physical && s.rate > 0)
    }

    /// **Báo nguyên nhân**, không âm thầm chỉnh một hệ số (`§12.8.4`).
    ///
    /// Đây là điểm khác biệt giữa một Auditor có ích và một cái van tự động: van
    /// giữ cho con số đẹp và giấu mất chuyện gì đang xảy ra; Auditor nói ra và
    /// để người chơi quyết định.
    pub fn audit(&self) -> MoneyDiagnosis {
        let vao = self.faucet_rate();
        let ra = self.sink_rate();
        let chenh = vao - ra;

        // Chưa có tiền thì không có lạm phát tiền tệ để mà bàn.
        if self.stage <= MonetaryStage::Credit {
            return MoneyDiagnosis::Balanced;
        }

        if chenh > 0 {
            let lon_nhat = self
                .faucets
                .iter()
                .max_by_key(|f| (f.rate, std::cmp::Reverse(f.id.clone())))
                .map_or_else(|| "không rõ".to_owned(), |f| f.id.clone());
            return MoneyDiagnosis::Inflation {
                cause: format!("vòi `{lon_nhat}` bơm nhiều hơn tổng cống"),
                surplus: chenh,
            };
        }

        if chenh < 0 {
            // Chỗ đặc trưng nhất: rút tiền mà **không có hao mòn vật chất**.
            let cause = if self.has_physical_sink() {
                "cống rút nhiều hơn vòi bơm".to_owned()
            } else {
                "rút tiền mà không có cống vật chất — hàng hóa tích tụ mãi \
                 trong khi tiền bị hút bớt (§12.8.4)"
                    .to_owned()
            };
            return MoneyDiagnosis::Deflation {
                cause,
                deficit: -chenh,
            };
        }

        MoneyDiagnosis::Balanced
    }

    /// Mức giá tương đối: cung tiền trên lượng hàng.
    ///
    /// Không phải "lạm phát" — chỉ là tỉ số. Lạm phát là *thay đổi* của tỉ số
    /// này, và nó phải quan sát được qua thời gian chứ không đọc được từ một
    /// thời điểm.
    pub fn price_level(&self) -> i64 {
        if self.goods_supply <= 0 {
            return i64::MAX;
        }
        self.money_supply * 1_000 / self.goods_supply
    }
}
