//! [`without_model`] — đọc thẳng đồ thị nhân quả thành câu, không cần model.
//!
//! ## Đây là đáy, không phải một phương án phụ
//!
//! `§20.10`/`§10.3` đòi ba tầng dưới khiến thế giới chạy đúng khi LLM chậm hay
//! mất kết nối. Với Yuu, "chạy đúng" nghĩa là: True God hỏi, và luôn nhận được
//! một câu trả lời có căn cứ — không phải một màn hình trống. [`without_model`]
//! là đường lui khi không có provider, **và cũng là đáy mà mọi câu trả lời của
//! Yuu phải đứng trên**: [`crate::Yuu::ask`] rơi về đây bất cứ khi nào model im
//! lặng, trả rác, hoặc mọi câu nó nói đều bị [`crate::read_answer`] cắt sạch.
//! Vì vậy hàm này phải **hữu ích**, không chỉ đúng — và bài kiểm thật của thiết
//! kế là nó phải trả lời được cả ba câu ở [`suggested_questions`].
//!
//! ## Không có NLU, chỉ có heuristic từ khóa — và đó là lựa chọn có chủ ý
//!
//! Hàm này không hiểu câu hỏi. Nó khớp vài từ khóa tiếng Việt để chọn một
//! trong ba cách đọc đồ thị (kho, dân làng, xu hướng), và khi không khớp gì nó
//! đọc thẳng các sự kiện gần nhất. Một NLU thật sẽ cần chính LLM mà hàm này
//! tồn tại để **thay thế** khi LLM không có — nên "chỉ khớp từ khóa" không
//! phải là một thiếu sót cần vá, mà là toàn bộ lý do hàm này xác định, không
//! mạng, và không tốn token.
//!
//! ## Không bao giờ bịa một sự kiện tương lai
//!
//! Câu hỏi "nếu ta không làm gì" hỏi về tương lai, nhưng [`Dossier`] chỉ có sự
//! kiện đã xảy ra. [`about_trend`] không bịa một sự kiện mới — nó tìm chuỗi
//! nhân quả **đã có thật** đang phát triển sâu nhất (nhiều mắt xích nhất) và
//! để người đọc tự suy ra xu hướng từ chuỗi đó. Đây là ngoại suy có căn cứ, chứ
//! không phải tiên tri.
//!
//! ## `proposals` luôn rỗng ở đây — một đánh đổi có ghi lại
//!
//! `without_model` không nhận `known_powers`. Đề xuất một quyền năng mà không
//! có tập quyền năng thật để kiểm là đúng thứ toàn bộ crate này tồn tại để cấm
//! (`UnknownPower`). Nên đáy này chỉ làm việc nó chắc chắn làm đúng — kể
//! nguyên nhân và hậu quả — và để việc đề xuất can thiệp cho model, nơi
//! [`crate::Yuu::ask`] có tập quyền năng thật để kiểm.

use crate::answer::{Answer, Line, StripReason, Stripped};
use crate::dossier::{Dossier, EventBrief};
use std::collections::BTreeMap;

/// Ba câu hỏi đặt sẵn cho giao diện hiện thành nút — ba câu người chơi sẽ hỏi
/// đầu tiên, từ một buổi tư vấn thiết kế (`idea.md §3.1` bước 2).
pub const SUGGESTED_QUESTIONS: [&str; 3] = [
    "Vì sao kho lương đang cạn?",
    "Dân làng đang gặp chuyện gì?",
    "Nếu ta không làm gì, chuyện gì sẽ tới?",
];

/// [`SUGGESTED_QUESTIONS`] dạng slice, cho giao diện không phải biết kích cỡ
/// mảng.
#[must_use]
pub fn suggested_questions() -> &'static [&'static str] {
    &SUGGESTED_QUESTIONS
}

/// Độ sâu tối đa khi truy ngược một chuỗi nhân quả — cùng vai trò với
/// `mow_core::EventLog::cause_chain`, viết lại vì [`Dossier`] không giữ một
/// `EventLog` thật (`§1.2.4`: Yuu cầm bản rút gọn, không cầm engine).
const MAX_CHAIN: usize = 6;

/// Số sự kiện tối đa trong câu trả lời "chung chung" khi câu hỏi không khớp
/// heuristic nào.
const MAX_GENERAL: usize = 6;

fn index_by_seq(d: &Dossier) -> BTreeMap<u64, &EventBrief> {
    d.events.iter().map(|e| (e.seq, e)).collect()
}

/// Truy ngược từ `from` theo cạnh `cause`, chặn vòng lặp và chặn độ sâu.
fn cause_chain<'a>(
    index: &BTreeMap<u64, &'a EventBrief>,
    from: u64,
    max_depth: usize,
) -> Vec<&'a EventBrief> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut cur = Some(from);
    while let Some(seq) = cur {
        if out.len() >= max_depth || !seen.insert(seq) {
            break;
        }
        let Some(ev) = index.get(&seq) else { break };
        out.push(*ev);
        cur = ev.cause;
    }
    out
}

/// Sự kiện gần nhất (`seq` lớn nhất — `seq` đơn điệu tăng nên đó là "mới
/// nhất") mà `kind` hoặc `summary` chứa `needle`, không phân biệt hoa thường.
fn most_relevant<'a>(d: &'a Dossier, needle: &str) -> Option<&'a EventBrief> {
    let needle = needle.to_lowercase();
    d.events
        .iter()
        .filter(|e| {
            e.kind.to_lowercase().contains(&needle) || e.summary.to_lowercase().contains(&needle)
        })
        .max_by_key(|e| e.seq)
}

/// Ghép một chuỗi nhân quả thành một [`Line`], trích dẫn toàn bộ chuỗi.
fn chain_line(lead: &str, chain: &[&EventBrief]) -> Line {
    let mut text = lead.to_owned();
    for (i, ev) in chain.iter().enumerate() {
        if i > 0 {
            text.push_str(" → ");
        }
        text.push_str(&ev.summary);
    }
    Line {
        text,
        cites: chain.iter().map(|e| e.seq).collect(),
    }
}

/// "Vì sao kho lương đang cạn?" và mọi câu hỏi về kho nói chung: đọc từng mặt
/// hàng đã chuẩn hoá, tìm sự kiện gần nhất nhắc tới đúng tên mặt hàng đó, kể
/// xuôi cả chuỗi nhân quả dẫn tới nó.
fn about_stock(d: &Dossier) -> (Vec<Line>, Vec<Stripped>) {
    let index = index_by_seq(d);
    let mut lines = Vec::new();
    let mut stripped = Vec::new();
    for (name, qty) in d.canonical_stock() {
        match most_relevant(d, &name) {
            Some(ev) => {
                let chain = cause_chain(&index, ev.seq, MAX_CHAIN);
                lines.push(chain_line(
                    &format!("Kho \"{name}\" hiện còn {qty}, vì: "),
                    &chain,
                ));
            }
            None => stripped.push(Stripped {
                text: format!(
                    "Kho \"{name}\" hiện còn {qty}, nhưng không có sự kiện nào ghi lại vì sao."
                ),
                reason: StripReason::NoCitation,
            }),
        }
    }
    (lines, stripped)
}

/// "Dân làng đang gặp chuyện gì?": mỗi cư dân đã chuẩn hoá, sự kiện gần nhất
/// họ là tác nhân, kể xuôi chuỗi nhân quả dẫn tới nó.
fn about_folk(d: &Dossier) -> (Vec<Line>, Vec<Stripped>) {
    let index = index_by_seq(d);
    let mut lines = Vec::new();
    let mut stripped = Vec::new();
    for f in d.canonical_folk() {
        let related = d
            .events
            .iter()
            .filter(|e| e.actor == Some(f.id))
            .max_by_key(|e| e.seq);
        match related {
            Some(ev) => {
                let chain = cause_chain(&index, ev.seq, MAX_CHAIN);
                let lead = format!(
                    "{} ({}, ý định {}, đói {}): ",
                    f.name, f.role, f.intent, f.hunger
                );
                lines.push(chain_line(&lead, &chain));
            }
            None => stripped.push(Stripped {
                text: format!("{} không có sự kiện nào gắn với mình gần đây.", f.name),
                reason: StripReason::NoCitation,
            }),
        }
    }
    (lines, stripped)
}

/// "Nếu ta không làm gì, chuyện gì sẽ tới?": không bịa một sự kiện tương lai —
/// tìm chuỗi nhân quả **đã có thật** phát triển sâu nhất (nhiều mắt xích nhất,
/// tức đang là một diễn biến đang tiếp diễn chứ không phải một sự kiện đơn
/// lẻ) và để câu văn chỉ nói xuôi nó lại.
fn about_trend(d: &Dossier) -> (Vec<Line>, Vec<Stripped>) {
    let index = index_by_seq(d);
    let deepest = d
        .events
        .iter()
        .map(|e| (cause_chain(&index, e.seq, MAX_CHAIN), e.seq))
        .max_by_key(|(chain, seq)| (chain.len(), *seq));

    match deepest {
        Some((chain, _)) if !chain.is_empty() => {
            let line = chain_line(
                "Nếu không có can thiệp, diễn biến gần đây nhiều khả năng tiếp tục: ",
                &chain,
            );
            (vec![line], Vec::new())
        }
        _ => (
            Vec::new(),
            vec![Stripped {
                text: "Chưa có sự kiện nào để suy ra xu hướng.".to_owned(),
                reason: StripReason::NoCitation,
            }],
        ),
    }
}

/// Không khớp heuristic nào: đọc thẳng các sự kiện gần nhất, mỗi sự kiện một
/// câu, tự trích dẫn chính nó. Bản đọc "trần trụi" nhất của đồ thị.
fn general(d: &Dossier) -> (Vec<Line>, Vec<Stripped>) {
    let lines = d
        .recent_events(MAX_GENERAL)
        .into_iter()
        .map(|e| Line {
            text: format!("Tick {}: {} — {}", e.tick, e.kind, e.summary),
            cites: vec![e.seq],
        })
        .collect();
    (lines, Vec::new())
}

fn is_stock_question(q: &str) -> bool {
    ["kho", "lương", "cạn", "thực phẩm", "dự trữ"]
        .iter()
        .any(|k| q.contains(k))
}

fn is_folk_question(q: &str) -> bool {
    ["dân làng", "cư dân", "dân chúng"]
        .iter()
        .any(|k| q.contains(k))
}

fn is_trend_question(q: &str) -> bool {
    q.contains("nếu")
        && (q.contains("không làm")
            || q.contains("không can thiệp")
            || q.contains("sẽ tới")
            || q.contains("sẽ ra sao"))
}

/// Trả lời không cần model: đọc thẳng đồ thị nhân quả thành câu.
///
/// Đây là đường lui khi không có provider (`§10.3`), và cũng là **đáy** mà
/// mọi câu trả lời của Yuu phải đứng trên — xem tài liệu module.
///
/// `proposals` luôn rỗng (xem "một đánh đổi có ghi lại" ở tài liệu module).
/// Mọi `Line` được dựng ở đây đều đi qua cùng luật trích dẫn với
/// [`crate::read_answer`]: không có mắt xích nào thì không có `Line`, chỉ có
/// [`Stripped`] nói rõ vì sao Yuu chọn im lặng thay vì bịa.
#[must_use]
pub fn without_model(d: &Dossier, question: &str) -> Answer {
    let q = question.to_lowercase();
    let (mut lines, mut stripped) = if is_stock_question(&q) {
        about_stock(d)
    } else if is_folk_question(&q) {
        about_folk(d)
    } else if is_trend_question(&q) {
        about_trend(d)
    } else {
        general(d)
    };

    // Một câu hỏi khớp đúng chủ đề nhưng hồ sơ chưa có gì về chủ đề đó vẫn phải
    // được trả lời bằng **cái đang có**, không phải bằng im lặng.
    //
    // Ví dụ thật đã bắt được điều này: người chơi hỏi *"Vì sao kho lương đang
    // cạn?"* trong một thế giới mà kho **chưa** cạn. Nhánh kho khớp, không tìm
    // thấy sự kiện thiếu hụt nào, và trả về rỗng — màn hình trắng. Nhưng thế
    // giới lúc đó vẫn có hàng chục sự kiện đáng kể để nói.
    //
    // Rơi về [`general`] chứ không bịa: vẫn là những sự kiện có thật, vẫn kèm
    // trích dẫn, chỉ là rộng hơn câu hỏi. Nói "đây là những gì tôi thấy" thì
    // trung thực hơn nói không gì cả.
    if lines.is_empty() {
        let (rong, them) = general(d);
        lines = rong;
        stripped.extend(them);
    }

    Answer {
        lines,
        proposals: Vec::new(),
        stripped,
    }
}
