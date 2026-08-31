//! [`Dossier`] — tất cả sự thật mà Yuu được phép dùng để tư vấn (`idea.md
//! §3.1` bước 2, `§1.2.4`).
//!
//! ## Vì sao struct này phải nghèo nàn
//!
//! `§1.2.4`: *"LLM là tầng nhận thức, không phải engine vật lý"*. Yuu đọc đồ
//! thị nhân quả thành tiếng người — nó không được cầm cả engine để tự đi tìm
//! thêm bằng chứng. Cám dỗ ở đây giống hệt cám dỗ đã gặp ở `mow_mind::Observation`:
//! "thêm một chút ngữ cảnh cho Yuu thông minh hơn". Mỗi lần thêm như vậy đều rẻ,
//! và tổng của chúng là một Yuu biết những thứ True God không đưa cho nó xem —
//! đúng thứ phá vỡ lời hứa "mọi câu Yuu nói truy được về một sự kiện có thật".
//!
//! Nên [`Dossier`] chỉ có bốn trường, và **engine phải tự lọc** trước khi dựng
//! nó. Không có con trỏ nào để prompt hay [`crate::without_model`] lần ngược ra
//! state thật. Thứ không có mặt ở đây thì không có đường vào cả prompt lẫn câu
//! trả lời không cần model.
//!
//! ## Vì sao là bản rút gọn của `mow_core::Event`, không phải chính nó
//!
//! Giữ nguyên `mow_core::Event` ở đây sẽ kéo `branch`, `world`, `payload` thô,
//! `law_version` — mọi thứ engine đổi kiểu dữ liệu lõi sẽ buộc crate diễn giải
//! này phải sửa theo, dù Yuu chẳng bao giờ cần tới `payload` thô để nói một câu
//! có trích dẫn. [`EventBrief`] giữ đúng năm thứ Yuu cần: `seq` để trích dẫn,
//! `tick`/`kind`/`actor`/`summary` để kể, và `cause` — cạnh của đồ thị nhân quả
//! mà toàn bộ tính năng này tồn tại để đọc.
//!
//! ## Chuẩn hoá nằm ở đây, dùng chung cho cả hai đường ra
//!
//! [`crate::prompt_of`] và [`crate::without_model`] đều cần "kho đã sắp, đã
//! khử trùng lặp" và "dân làng đã sắp, đã khử trùng lặp". Nếu mỗi hàm tự chuẩn
//! hoá lấy một bản, một ngày nào đó chúng sẽ lệch — và triệu chứng sẽ là prompt
//! nói kho còn một con số, còn câu trả lời không cần model nói một con số khác
//! cho **cùng một** `Dossier`. Nên phép chuẩn hoá sống đúng một chỗ:
//! [`Dossier::canonical_stock`] và [`Dossier::canonical_folk`], và dùng
//! `BTreeMap` chứ không `HashMap` — thứ tự chỗ gọi truyền `stock`/`folk` vào
//! không được phép lọt ra ngoài, vì nó sẽ lọt vào khóa của bản ghi `REPLAY`
//! qua [`crate::prompt_of`].

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Một cư dân, tóm tắt đủ để Yuu nói về họ mà không bịa thêm điều gì.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolkBrief {
    /// Định danh thực thể trong engine — khóa duy nhất trong [`Dossier::folk`].
    pub id: u64,
    /// Tên hiển thị.
    pub name: String,
    /// Vai của người này trong làng, ví dụ `farmer`.
    pub role: String,
    /// Ý định hiện tại, dạng nhãn ngắn do engine gán — không phải suy diễn của
    /// Yuu, và Yuu không được đổi nó thành gì khác.
    pub intent: String,
    /// Mức đói: `0` là no, càng cao càng đói. Số nguyên (`§P10.2.1`).
    pub hunger: i64,
}

/// Một sự kiện, kèm cạnh nhân quả — bản rút gọn của `mow_core::Event` dành
/// riêng cho Yuu. Xem tài liệu module vì sao rút gọn thay vì dùng thẳng.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBrief {
    /// Số thứ tự sự kiện trong nhật ký — khóa trích dẫn **duy nhất** mà Yuu
    /// được dùng. Ứng với `mow_core::EventSeq`.
    pub seq: u64,
    /// Nhịp lúc sự kiện xảy ra.
    pub tick: u64,
    /// Loại sự kiện, ví dụ `econ.harvest.failed`. Ứng với `mow_core::EventKind`.
    pub kind: String,
    /// Ai gây ra, nếu có chủ thể. Ứng với `mow_core::Event::actor`.
    pub actor: Option<u64>,
    /// Sự kiện nào đã dẫn tới sự kiện này — cạnh của đồ thị nhân quả
    /// (`§18.10`). Ứng với `mow_core::Event::cause`.
    pub cause: Option<u64>,
    /// Tóm tắt bằng một câu, do engine dựng từ payload thật — Yuu không tự
    /// viết lại nó, chỉ trích lại nguyên văn hoặc ghép nhiều tóm tắt lại.
    pub summary: String,
}

/// Tất cả sự thật mà Yuu được phép dùng để trả lời True God. Không có gì khác.
///
/// Do engine dựng, không bao giờ do model dựng — cùng nguyên tắc với
/// `mow_mind::Observation`. Thứ tự [`Dossier::events`] mang thông tin (một
/// dòng thời gian): xem tài liệu trường.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dossier {
    /// Nhịp hiện tại — "bây giờ" của Yuu, dùng làm mốc khi kể chuyện.
    pub tick: u64,
    /// Tóm tắt kho của làng: `(tên, giá trị)`. Số nguyên, đúng như engine giữ
    /// (`§P10.2.1`) — không quy tròn, không phần trăm suy ra.
    ///
    /// Là một *tập* theo tên: [`Dossier::canonical_stock`] sắp và khử trùng
    /// lặp trước khi bất cứ thứ gì đọc nó, nên thứ tự chỗ gọi truyền vào không
    /// quan trọng.
    pub stock: Vec<(String, i64)>,
    /// Cư dân đáng chú ý hiện tại. Cùng lập luận với `stock`: là một tập theo
    /// `id`, xem [`Dossier::canonical_folk`].
    pub folk: Vec<FolkBrief>,
    /// Các sự kiện gần đây, kèm cạnh nhân quả — **cũ trước, mới sau**.
    ///
    /// Khác với `stock`/`folk`, đây là một dòng thời gian: sắp lại nó là nói
    /// dối thứ tự nhân quả. [`crate::prompt_of`] chỉ cắt bớt
    /// ([`Dossier::recent_events`]), không bao giờ sắp lại.
    pub events: Vec<EventBrief>,
}

impl Dossier {
    /// Tập `seq` mà một trích dẫn được coi là hợp lệ.
    ///
    /// Dùng thẳng cho [`crate::read_answer`] và cho [`crate::Yuu::ask`] — cả
    /// hai đều cần đúng tập này, và tính nó ra ở một chỗ giữ nó không lệch với
    /// chính `Dossier` đã dùng để dựng prompt.
    #[must_use]
    pub fn known_events(&self) -> BTreeSet<u64> {
        self.events.iter().map(|e| e.seq).collect()
    }

    /// Kho đã chuẩn hoá: cắt khoảng trắng, bỏ tên rỗng, khử trùng lặp theo tên
    /// (mục **sau** đè mục **trước** cùng tên — giống ngữ nghĩa "ghi đè" của
    /// một kho thật), sắp theo tên.
    ///
    /// [`crate::prompt_of`] và [`crate::without_model`] gọi đúng hàm này — xem
    /// "Chuẩn hoá nằm ở đây" trong tài liệu module.
    #[must_use]
    pub fn canonical_stock(&self) -> Vec<(String, i64)> {
        let mut m: BTreeMap<String, i64> = BTreeMap::new();
        for (name, qty) in &self.stock {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            m.insert(name.to_owned(), *qty);
        }
        m.into_iter().collect()
    }

    /// Dân làng đã chuẩn hoá: khử trùng lặp theo `id` (mục sau đè mục trước),
    /// sắp theo `id`.
    #[must_use]
    pub fn canonical_folk(&self) -> Vec<FolkBrief> {
        let mut m: BTreeMap<u64, FolkBrief> = BTreeMap::new();
        for f in &self.folk {
            m.insert(f.id, f.clone());
        }
        m.into_values().collect()
    }

    /// `events` đã khử trùng lặp theo `seq` (giữ lần xuất hiện **đầu**, để
    /// không phá thứ tự dòng thời gian), rồi cắt về tối đa `max` mục **cuối**.
    ///
    /// Một dòng thời gian dài hàng nghìn sự kiện nhét hết vào prompt là cách
    /// chắc chắn nhất để hóa đơn tăng mà chất lượng câu trả lời thì không —
    /// cùng lý do `mow_mind::prompt::MAX_RECENT` tồn tại. Giữ **`max` mục mới
    /// nhất** vì phần đáng giữ khi phải cắt một dòng thời gian là phần gần
    /// "bây giờ" nhất.
    #[must_use]
    pub fn recent_events(&self, max: usize) -> Vec<&EventBrief> {
        let mut seen = BTreeSet::new();
        let deduped: Vec<&EventBrief> = self.events.iter().filter(|e| seen.insert(e.seq)).collect();
        let start = deduped.len().saturating_sub(max);
        deduped[start..].to_vec()
    }
}
