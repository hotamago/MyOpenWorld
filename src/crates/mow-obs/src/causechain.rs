//! Khung xem chuỗi nhân quả (`idea.md §18.10`, `§23`, `PC-16`).
//!
//! > `§23` yêu cầu người chơi truy được từ một biến cố lớn về tận nguyên nhân.
//! > Đây là giao diện thực hiện lời hứa đó, và nó là thứ phân biệt "thế giới
//! > sống" với "AI tự nghĩ ra".
//!
//! ## Quy tắc duy nhất, và nó tuyệt đối
//!
//! **Khung này chỉ hiển thị event có thật trong log.**
//!
//! Không có câu giải thích nào do model viết ra sau khi mọi chuyện đã xong
//! (`§22.17`). Điều đó nghe như một hạn chế tự nguyện; nó không phải. Một lời
//! giải thích sinh ra sau sự việc luôn *mạch lạc hơn* sự thật, vì nó được viết
//! khi đã biết kết cục. Người xem sẽ tin nó hơn chuỗi event thật, và lúc đó
//! công cụ này không còn chứng minh được điều gì cả — nó chỉ kể một câu chuyện
//! hay, đúng như thứ mà cả kiến trúc này tồn tại để không phải làm.
//!
//! ## Hai chiều, và vì sao chiều xuôi không suy ra được từ chiều ngược
//!
//! ```text
//!        ┌── nguyên nhân ──┐
//!   e3 ──┤                 │
//!   e5 ──┴──► e9 ──┬──► e12    ← hệ quả
//!                  └──► e13
//! ```
//!
//! Cạnh được ghi ở **con trỏ về cha** (`Event::cause`), nên đi ngược là đi theo
//! con trỏ, còn đi xuôi phải quét. Chỗ hỏng nằm ở đây: cách rẻ là chỉ dựng chỉ
//! mục con trong khoảng đang xem, và khi đó một hệ quả xảy ra muộn hơn cửa sổ
//! sẽ **biến mất** khỏi khung — im lặng, và đúng ở những vụ thú vị nhất, vì hệ
//! quả xa mới là hệ quả đáng xem.
//!
//! Nên [`ChainView::forward`] nhận toàn bộ khoảng cần quét, và [`Chain`] ghi
//! rõ nó đã quét tới đâu bằng [`Chain::scanned_to`]. Người xem thấy được rằng
//! "chưa có hệ quả nào **trong khoảng đã quét**", chứ không thấy một danh sách
//! rỗng trông như "chuyện này chẳng dẫn tới đâu".

use mow_core::{Event, EventSeq};
use serde::Serialize;
use std::collections::BTreeMap;

/// Một mắt xích trong chuỗi.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Link {
    /// Sự kiện.
    pub seq: EventSeq,
    /// Tick lúc xảy ra — để "nhảy tới đúng lúc".
    pub tick: u64,
    /// Loại sự kiện.
    pub kind: String,
    /// Chủ thể.
    pub actor: Option<u64>,
    /// Đối tượng.
    pub subject: Option<u64>,
    /// Phiên bản luật lúc đó.
    pub law_version: Option<u32>,
    /// Phiên bản bộ chuẩn mực lúc đó.
    ///
    /// Cùng một hành vi có thể hợp pháp mà bị khinh, hoặc phạm pháp mà được nể.
    /// Không có trường này, khung xem trả lời được "chuyện gì đã xảy ra" nhưng
    /// không trả lời được "vì sao cả làng phản ứng như thế".
    pub norm_set_version: Option<u32>,
    /// Sâu bao nhiêu bậc so với mắt xích gốc. Âm là ngược lên, dương là xuôi xuống.
    pub depth: i32,
}

impl Link {
    fn from(e: &Event, depth: i32) -> Link {
        Link {
            seq: e.seq,
            tick: e.tick.0,
            kind: e.kind.0.clone(),
            actor: e.actor.map(|a| a.0),
            subject: e.subject.map(|s| s.0),
            law_version: e.law_version,
            norm_set_version: e.norm_set_version,
            depth,
        }
    }
}

/// Kết quả truy chuỗi.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chain {
    /// Mắt xích được hỏi.
    pub root: Link,
    /// Ngược lên: những gì đã dẫn tới nó, gần trước xa sau.
    pub backward: Vec<Link>,
    /// Xuôi xuống: những gì nó đã gây ra.
    pub forward: Vec<Link>,
    /// Đã dừng ngược lên vì chạm giới hạn độ sâu, chứ không phải vì hết nguyên nhân.
    ///
    /// Phân biệt hai chuyện này quan trọng: "đây là khởi nguồn" và "tôi ngừng
    /// tìm ở đây" trông giống hệt nhau trên màn hình nếu không nói ra.
    pub truncated_backward: bool,
    /// Seq lớn nhất đã quét khi tìm hệ quả.
    ///
    /// Người xem cần biết con số này để đọc đúng một danh sách rỗng: "chưa có
    /// hệ quả nào **tính tới đây**", không phải "chuyện này chẳng dẫn tới đâu".
    pub scanned_to: EventSeq,
}

impl Chain {
    /// Toàn bộ mắt xích, ngược trước rồi gốc rồi xuôi — thứ tự để vẽ.
    pub fn timeline(&self) -> Vec<&Link> {
        let mut v: Vec<&Link> = self.backward.iter().rev().collect();
        v.push(&self.root);
        v.extend(self.forward.iter());
        v
    }
}

/// Chỉ mục để truy chuỗi.
///
/// Dựng từ một lát cắt của nhật ký. Nó **chỉ đọc** và không giữ tham chiếu tới
/// `Sim` — một công cụ chẩn đoán có khả năng ghi là một công cụ sẽ có lúc ghi.
#[derive(Debug, Clone, Default)]
pub struct ChainView {
    events: BTreeMap<EventSeq, Event>,
    /// `cause → các con`. Dựng sẵn vì đi xuôi mà quét lại mỗi lần là bậc hai.
    children: BTreeMap<EventSeq, Vec<EventSeq>>,
    scanned_to: EventSeq,
}

impl ChainView {
    /// Dựng chỉ mục từ một lát cắt nhật ký.
    pub fn new(events: impl IntoIterator<Item = Event>) -> ChainView {
        let mut v = ChainView::default();
        for e in events {
            if let Some(c) = e.cause {
                v.children.entry(c).or_default().push(e.seq);
            }
            v.scanned_to = v.scanned_to.max(e.seq);
            v.events.insert(e.seq, e);
        }
        for con in v.children.values_mut() {
            con.sort_unstable();
        }
        v
    }

    /// Số sự kiện trong chỉ mục.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Truy hai chiều từ một sự kiện.
    ///
    /// `max_depth` áp cho **cả hai** chiều. `§18.10` nói "dừng ở mức người chơi
    /// chọn": một chuỗi nhân quả dài hàng nghìn mắt không đọc được, và cắt nó là
    /// một quyết định của người xem chứ không phải của công cụ.
    pub fn chain(&self, seq: EventSeq, max_depth: u32) -> Option<Chain> {
        let goc = self.events.get(&seq)?;

        // Ngược lên: đi theo con trỏ cha.
        let mut backward = Vec::new();
        let mut truncated_backward = false;
        let mut hien_tai = goc.cause;
        let mut d: i32 = 0;
        while let Some(c) = hien_tai {
            if d >= i32::try_from(max_depth).unwrap_or(i32::MAX) {
                // Còn cha nhưng ta ngừng tìm — nói ra, đừng để nó trông như
                // khởi nguồn.
                truncated_backward = self.events.contains_key(&c);
                break;
            }
            let Some(e) = self.events.get(&c) else {
                // Cha nằm ngoài lát cắt. Cũng là cắt ngắn, cùng lý do.
                truncated_backward = true;
                break;
            };
            d += 1;
            backward.push(Link::from(e, -d));
            hien_tai = e.cause;
        }

        // Xuôi xuống: duyệt theo bề rộng để mắt xích gần hiện trước.
        let mut forward = Vec::new();
        let mut lop = vec![seq];
        for buoc in 1..=max_depth {
            let mut sau = Vec::new();
            for p in lop {
                for c in self.children.get(&p).into_iter().flatten() {
                    if let Some(e) = self.events.get(c) {
                        forward.push(Link::from(e, i32::try_from(buoc).unwrap_or(i32::MAX)));
                        sau.push(*c);
                    }
                }
            }
            if sau.is_empty() {
                break;
            }
            lop = sau;
        }

        Some(Chain {
            root: Link::from(goc, 0),
            backward,
            forward,
            truncated_backward,
            scanned_to: self.scanned_to,
        })
    }

    /// Gốc rễ xa nhất truy được của một sự kiện.
    ///
    /// Đây là câu trả lời cho *"vì sao chuyện này xảy ra"* ở dạng ngắn nhất mà
    /// vẫn có thật. Trả `None` khi chính nó đã là gốc.
    pub fn root_cause(&self, seq: EventSeq) -> Option<EventSeq> {
        let mut hien_tai = self.events.get(&seq)?.cause?;
        let mut da_qua = 0usize;
        loop {
            let Some(e) = self.events.get(&hien_tai) else {
                return Some(hien_tai);
            };
            match e.cause {
                Some(c) => {
                    hien_tai = c;
                    da_qua += 1;
                    // Nhật ký hỏng có thể chứa chu trình. Đi vòng mãi ở một công
                    // cụ chẩn đoán là cách tệ nhất để phát hiện điều đó.
                    if da_qua > self.events.len() {
                        return Some(hien_tai);
                    }
                }
                None => return Some(e.seq),
            }
        }
    }
}
