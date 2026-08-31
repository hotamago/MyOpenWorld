//! Biên niên sử hai lớp (`idea.md §18.11`, `§8.9.2`, `PD-18`).
//!
//! > Giao diện phải cho thấy **cả hai lớp cạnh nhau**:
//! >
//! > - **Đã xảy ra**: dựng từ event log.
//! > - **Người ta tin là đã xảy ra**: dựng từ belief đang lưu hành.
//!
//! ## Vì sao đây không phải là "hiển thị hai danh sách"
//!
//! Cái khó nằm ở câu tiếp theo:
//!
//! > Chỗ hai lớp lệch nhau được **đánh dấu**, và bấm vào là thấy lệch **từ đâu**
//! > — ai kể lại sai, ở đời nào, vì động cơ gì.
//!
//! Nghĩa là phải giữ được **chuỗi truyền** của mỗi truyền thuyết, không chỉ nội
//! dung cuối cùng của nó. Một biên niên sử chỉ lưu "người ta tin X" thì đánh dấu
//! được chỗ lệch mà không trả lời được vì sao lệch — và câu "vì sao" mới là thứ
//! đáng bấm vào.
//!
//! Nên [`Legend`] mang `chain: Vec<Retelling>`, mỗi mắt ghi ai kể, ở tick nào,
//! và **động cơ** của họ. Đó là dữ liệu đủ để trả lời cả ba vế của câu hỏi.
//!
//! ## Với vật phẩm, cùng khung này
//!
//! > Cùng khung này hiển thị **chuỗi đổi chủ thật** đặt cạnh **truyền thuyết về
//! > nó**.
//!
//! Nên [`Chronicle`] không phân biệt "sự kiện" với "vật phẩm". Cả hai đều là một
//! chuỗi sự thật, đặt cạnh một chuỗi lời kể.

use mow_core::{EntityId, Tick};
use serde::{Deserialize, Serialize};

/// Một mắt trong lịch sử **thật** — dựng từ event log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    /// Event nào.
    pub event_seq: u64,
    /// Lúc nào.
    pub at: Tick,
    /// Ai làm.
    pub actor: Option<EntityId>,
    /// Chuyện gì.
    pub what: String,
}

/// Một lần kể lại, và **vì sao nó lệch đi**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retelling {
    /// Ai kể.
    pub teller: EntityId,
    /// Lúc nào.
    pub at: Tick,
    /// Đời thứ mấy kể từ người chứng kiến.
    pub generation: u32,
    /// **Vì sao** họ kể khác đi.
    ///
    /// Đây là trường trả lời vế thứ ba của `§18.11`. Không có nó, người chơi
    /// thấy được chỗ lệch và đời nào lệch, nhưng không biết vì sao — và "vì sao"
    /// mới là thứ biến một sai lệch thành một câu chuyện.
    pub motive: String,
    /// Nội dung sau lần kể này.
    pub says: String,
}

/// Một truyền thuyết: **ảnh biến dạng** của chuỗi provenance thật (`§8.9.2`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Legend {
    /// Nói về sự kiện thật nào.
    pub about_event: u64,
    /// Ai đang tin: một văn hóa, một tổ chức, hay một cá thể.
    pub believed_by: String,
    /// **Chuỗi truyền đầy đủ**, từ người chứng kiến tới bản đang lưu hành.
    pub chain: Vec<Retelling>,
}

impl Legend {
    /// Bản đang lưu hành.
    pub fn current(&self) -> Option<&Retelling> {
        self.chain.last()
    }

    /// Lần kể **đầu tiên làm nội dung đổi** so với lần trước.
    ///
    /// Đây là thứ mà bấm vào chỗ lệch phải trả về: không phải người kể cuối
    /// cùng, mà **người kể đã bẻ nó**. Người kể cuối có thể hoàn toàn trung
    /// thực với thứ họ nghe được.
    pub fn first_divergence(&self) -> Option<&Retelling> {
        self.chain
            .windows(2)
            .find(|w| w[0].says != w[1].says)
            .map(|w| &w[1])
    }
}

/// Hai lớp đặt cạnh nhau.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Chronicle {
    /// Lớp "đã xảy ra".
    pub facts: Vec<Fact>,
    /// Lớp "người ta tin là đã xảy ra".
    pub legends: Vec<Legend>,
}

/// Một chỗ hai lớp lệch nhau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Divergence {
    /// Về sự kiện nào.
    pub event_seq: u64,
    /// Ai tin bản lệch.
    pub believed_by: String,
    /// Sự thật.
    pub truth: String,
    /// Điều người ta tin.
    pub belief: String,
    /// **Ai kể sai**.
    pub introduced_by: Option<EntityId>,
    /// **Ở đời nào**.
    pub generation: Option<u32>,
    /// **Vì động cơ gì**.
    pub motive: Option<String>,
}

impl Chronicle {
    /// Rỗng.
    pub fn new() -> Chronicle {
        Chronicle::default()
    }

    /// Sự thật về một sự kiện.
    pub fn fact(&self, event_seq: u64) -> Option<&Fact> {
        self.facts.iter().find(|f| f.event_seq == event_seq)
    }

    /// **Mọi chỗ hai lớp lệch nhau**, kèm đủ ba vế: ai, đời nào, vì sao.
    ///
    /// Một truyền thuyết trùng khớp sự thật **không** xuất hiện ở đây. Đó là
    /// điểm: đánh dấu chỗ lệch chỉ có nghĩa khi phần lớn không lệch.
    pub fn divergences(&self) -> Vec<Divergence> {
        let mut ra = Vec::new();
        for l in &self.legends {
            let Some(that) = self.fact(l.about_event) else {
                continue;
            };
            let Some(dang_tin) = l.current() else {
                continue;
            };
            if dang_tin.says == that.what {
                continue;
            }
            let be = l.first_divergence();
            ra.push(Divergence {
                event_seq: l.about_event,
                believed_by: l.believed_by.clone(),
                truth: that.what.clone(),
                belief: dang_tin.says.clone(),
                introduced_by: be.map(|r| r.teller),
                generation: be.map(|r| r.generation),
                motive: be.map(|r| r.motive.clone()),
            });
        }
        ra.sort_by_key(|d| (d.event_seq, d.believed_by.clone()));
        ra
    }

    /// Hai văn hóa có tin **khác nhau** về cùng một sự kiện không.
    ///
    /// Đây là chất liệu của chiến tranh: không phải ai đúng, mà là hai bên đang
    /// kể hai câu chuyện khác nhau về cùng một ngày.
    pub fn contested(&self, event_seq: u64) -> bool {
        let cac: Vec<&str> = self
            .legends
            .iter()
            .filter(|l| l.about_event == event_seq)
            .filter_map(|l| l.current().map(|r| r.says.as_str()))
            .collect();
        cac.windows(2).any(|w| w[0] != w[1])
    }
}
