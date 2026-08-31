//! Chống trôi persona (`idea.md §20.11`, `§15.1`, `PC-13`).
//!
//! > State là mỏ neo. Auditor so hành vi với trait và báo lệch **không có
//! > nguyên nhân**.
//!
//! ## Vì sao cần một auditor, khi kiến trúc đã miễn nhiễm phần lớn
//!
//! `§20.11.1` đã cắt nguyên nhân gốc của trôi persona: không có chuỗi hội thoại
//! dài để mà mục ruỗng, vì persona được dựng lại từ state ở **mỗi** chu trình
//! nhận thức. Vậy còn gì để kiểm?
//!
//! Còn hai thứ, và cả hai đều im lặng:
//!
//! 1. **Ghi tắt.** Một handler, một plugin, một bản save đã sửa — bất cứ đường
//!    nào đổi `traits` mà không đi qua [`Personality::apply_change`]. Nhân vật
//!    vẫn nhất quán với chính nó ở mọi thời điểm, nên không có gì trông sai.
//!    Chỉ có lịch sử là không cộng lại thành hiện tại.
//! 2. **Hành vi lệch tính cách mà không ai đổi tính cách.** Model liên tục chọn
//!    những hành động của một người hào phóng, trong khi `agreeableness` của
//!    nhân vật là 120. Không có lệnh nào sai, không có event nào lạ. Chỉ có một
//!    nhân vật đang dần trở thành người khác.
//!
//! ## Ranh giới mà `§20.11.4` vạch ra
//!
//! ```text
//!         lệch vượt ngưỡng
//!                │
//!      ┌─────────┴─────────┐
//!      │                   │
//!  có nguyên nhân     không nguyên nhân
//!  (event + cause)          │
//!      │                    │
//!   CỐT TRUYỆN            BUG
//!  nhân vật đang       ghi log, cảnh báo,
//!  phát triển          KHÔNG âm thầm bỏ qua
//! ```
//!
//! Không có ô ở giữa. Đó là điều làm cho một vấn đề "khó đo" trở thành một bất
//! biến kiểm được: mỗi lệch hoặc chỉ được vào một `event_seq` có thật, hoặc là
//! một phát hiện phải báo cáo.

use crate::personality::{CauseKind, CauseRef, Personality, TraitField, Traits};
use serde::{Deserialize, Serialize};

/// Một hành động đã quan sát được, kèm mức đặc điểm mà nó **hàm ý**.
///
/// Ánh xạ từ hành động sang đặc điểm là dữ liệu nội dung, không phải hằng số
/// engine: một nền văn hóa có thể coi việc từ chối lời mời là bất lịch sự, một
/// nền văn hóa khác coi đó là biết điều. Auditor không cần biết ánh xạ đó — nó
/// chỉ nhận kết quả đã quy đổi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Act {
    /// Lúc nào.
    pub at_tick: u64,
    /// Hành động này nói về đặc điểm nào.
    pub field: TraitField,
    /// Mức đặc điểm mà một người làm việc này thường có, `0`–`1000`.
    pub implied: u16,
}

/// Một khoảng thời gian có nguyên nhân hợp lệ đang tác động.
///
/// Khác [`crate::personality::TraitChange`] ở chỗ: `TraitChange` là *tính cách
/// đã đổi thật*, còn cái này là *đang có thứ gì đó tác động lên hành vi* — bùa
/// điều khiển tâm trí, cơn say, một lời thề đang ràng buộc. Hành vi lệch trong
/// khoảng này là có giải thích, dù đặc điểm nền không đổi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveCause {
    /// Bắt đầu.
    pub from_tick: u64,
    /// Kết thúc, bao gồm.
    pub to_tick: u64,
    /// Sự kiện và loại.
    pub cause: CauseRef,
}

impl ActiveCause {
    /// Có phủ một tick không.
    pub fn covers(&self, tick: u64) -> bool {
        (self.from_tick..=self.to_tick).contains(&tick)
    }
}

/// Kết luận của auditor về một lệch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// **Cốt truyện.** Có event giải thích được: nhân vật đang phát triển.
    Story(CauseRef),
    /// **Bug.** Không có nguyên nhân nào: đây là trôi.
    Drift,
    /// **Ghi tắt.** Lịch sử không cộng lại thành hiện tại — có người đã đổi
    /// `traits` không qua đường chính thức. Nặng hơn `Drift`, vì `Drift` chỉ là
    /// hành vi lệch, còn cái này là state đã hỏng.
    Tampered,
}

impl Verdict {
    /// Có phải phát hiện cần báo không.
    ///
    /// [`Verdict::Story`] thì không: nhân vật thay đổi vì một lý do có thật là
    /// điều thế giới này tồn tại để tạo ra, không phải điều cần sửa.
    pub fn is_finding(self) -> bool {
        !matches!(self, Verdict::Story(_))
    }
}

/// Một lệch đã phát hiện.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Đặc điểm nào.
    pub field: TraitField,
    /// Tính cách nói bao nhiêu.
    pub expected: u16,
    /// Hành vi nói bao nhiêu.
    pub observed: u16,
    /// Khoảng quan sát.
    pub from_tick: u64,
    /// Khoảng quan sát.
    pub to_tick: u64,
    /// Dựa trên bao nhiêu hành động.
    pub sample: u32,
    /// Kết luận.
    pub verdict: Verdict,
}

impl Finding {
    /// Độ lệch tuyệt đối.
    pub fn gap(&self) -> u16 {
        self.expected.abs_diff(self.observed)
    }
}

/// Bộ kiểm trôi persona.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftAuditor {
    /// Lệch bao nhiêu thì coi là đáng kể, `0`–`1000`.
    ///
    /// Không có ngưỡng thì mọi nhiễu đều thành báo động, và một auditor kêu
    /// suốt ngày là một auditor không ai đọc.
    pub threshold: u16,
    /// Cần ít nhất bao nhiêu hành động mới kết luận.
    ///
    /// **Đây là tham số quan trọng hơn `threshold`.** Một người hào phóng vẫn
    /// có thể keo kiệt một lần; kết luận từ một hành động là biến mọi nhân vật
    /// có chiều sâu thành một báo cáo lỗi.
    pub min_sample: u32,
}

impl Default for DriftAuditor {
    fn default() -> Self {
        DriftAuditor {
            threshold: 300,
            min_sample: 8,
        }
    }
}

impl DriftAuditor {
    /// Soi một nhân vật.
    ///
    /// **Hàm thuần.** Không đọc đồng hồ, không ghi gì — nó chỉ trả về những gì
    /// nó thấy, và người gọi quyết định ghi event hay cảnh báo. Auditor tự ghi
    /// state sẽ vi phạm `§22.1`, và một auditor vi phạm bất biến nó đi kiểm là
    /// một thứ khó chữa.
    pub fn audit(&self, p: &Personality, acts: &[Act], causes: &[ActiveCause]) -> Vec<Finding> {
        let mut ra = Vec::new();

        // Kiểm mạnh nhất trước: lịch sử có cộng lại thành hiện tại không.
        //
        // Đặt trước phần so hành vi vì nếu state đã hỏng thì mọi so sánh phía
        // sau đều so với một con số không đáng tin, và báo cáo sẽ chỉ vào sai chỗ.
        if !p.history_explains_current() {
            for f in TRAIT_FIELDS {
                let (b, c) = (doc(p.birth_traits(), f), doc(p.traits(), f));
                if b != c {
                    ra.push(Finding {
                        field: f,
                        expected: b,
                        observed: c,
                        from_tick: 0,
                        to_tick: 0,
                        sample: 0,
                        verdict: Verdict::Tampered,
                    });
                }
            }
            return ra;
        }

        for f in TRAIT_FIELDS {
            let lien_quan: Vec<&Act> = acts.iter().filter(|a| a.field == f).collect();
            if u32::try_from(lien_quan.len()).unwrap_or(u32::MAX) < self.min_sample {
                continue;
            }

            let tong: u64 = lien_quan.iter().map(|a| u64::from(a.implied)).sum();
            let tb = u16::try_from(tong / lien_quan.len() as u64).unwrap_or(1000);
            let mong_doi = doc(p.traits(), f);
            if mong_doi.abs_diff(tb) < self.threshold {
                continue;
            }

            let tu = lien_quan.iter().map(|a| a.at_tick).min().unwrap_or(0);
            let den = lien_quan.iter().map(|a| a.at_tick).max().unwrap_or(0);

            ra.push(Finding {
                field: f,
                expected: mong_doi,
                observed: tb,
                from_tick: tu,
                to_tick: den,
                sample: u32::try_from(lien_quan.len()).unwrap_or(u32::MAX),
                verdict: Self::giai_thich(p, f, tu, den, causes),
            });
        }

        ra
    }

    /// Có gì giải thích được lệch này không.
    fn giai_thich(
        p: &Personality,
        f: TraitField,
        tu: u64,
        den: u64,
        causes: &[ActiveCause],
    ) -> Verdict {
        // Một nguyên nhân đang tác động, phủ phần lớn khoảng quan sát.
        if let Some(c) = causes
            .iter()
            .find(|c| c.covers(tu) || c.covers(den) || (c.from_tick >= tu && c.to_tick <= den))
        {
            return Verdict::Story(c.cause);
        }

        // Hoặc chính đặc điểm đó vừa đổi, và đổi vì một lý do đã ghi.
        //
        // Nới ra hai phía có chủ đích: một sang chấn ở tick 900 giải thích được
        // hành vi ở tick 1000, và một hành vi bắt đầu lệch từ tick 800 rồi mới
        // được chẩn đoán ở tick 900 cũng vậy. Trôi persona diễn ra chậm, nên
        // một cửa sổ khớp chính xác sẽ bỏ sót đúng những ca nó cần bắt.
        let no = NOI_LONG;
        if let Some(c) = p
            .history()
            .iter()
            .find(|c| c.field == f && c.at_tick + no >= tu && c.at_tick <= den.saturating_add(no))
        {
            return Verdict::Story(c.cause);
        }

        Verdict::Drift
    }
}

/// Nới cửa sổ khớp nguyên nhân, tính bằng tick.
const NOI_LONG: u64 = 2_000;

/// Năm đặc điểm, để lặp.
const TRAIT_FIELDS: [TraitField; 5] = [
    TraitField::Openness,
    TraitField::Conscientiousness,
    TraitField::Extraversion,
    TraitField::Agreeableness,
    TraitField::Neuroticism,
];

/// Đọc một đặc điểm theo tên trường.
fn doc(t: &Traits, f: TraitField) -> u16 {
    match f {
        TraitField::Openness => t.openness,
        TraitField::Conscientiousness => t.conscientiousness,
        TraitField::Extraversion => t.extraversion,
        TraitField::Agreeableness => t.agreeableness,
        TraitField::Neuroticism => t.neuroticism,
    }
}

/// Báo cáo tổng hợp cho một lần chạy auditor.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DriftReport {
    /// Mọi phát hiện, kể cả loại "cốt truyện".
    pub findings: Vec<Finding>,
}

impl DriftReport {
    /// Những thứ **phải báo**.
    ///
    /// `§20.11.4` nói rõ: không âm thầm bỏ qua. Hàm này tồn tại để chỗ gọi
    /// không phải tự nhớ lọc thế nào, vì "tự nhớ" là cách một cảnh báo biến mất.
    pub fn to_report(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.verdict.is_finding())
            .collect()
    }

    /// Những thay đổi có nguyên nhân — tức là cốt truyện, để UI kể lại.
    pub fn story_beats(&self) -> Vec<(&Finding, CauseKind)> {
        self.findings
            .iter()
            .filter_map(|f| match f.verdict {
                Verdict::Story(c) => Some((f, c.kind)),
                _ => None,
            })
            .collect()
    }

    /// Có gì phải báo không.
    pub fn is_clean(&self) -> bool {
        self.to_report().is_empty()
    }
}
