//! Dựng prompt tư vấn từ một [`Dossier`] và một câu hỏi — xác định, và không
//! lấy gì ngoài chúng.
//!
//! ## Vì sao tính xác định ở đây là bắt buộc chứ không phải đáng có
//!
//! Cùng lý do với `mow_mind::prompt`: `mow_llm::Request::hash` gồm cả chuỗi đã
//! render, và hash đó là khóa của bản ghi `REPLAY`. Một prompt đổi giữa hai
//! lần chạy cho cùng một `Dossier` — vì `stock`/`folk` duyệt khác thứ tự, vì
//! đồng hồ, vì một biến môi trường — là một bộ ghi không bao giờ trúng và một
//! CI phải gọi mạng thật để xanh. [`prompt_of`] là hàm thuần của đúng hai đối
//! số của nó: không đọc biến môi trường, không đọc tệp, không đọc đồng hồ,
//! không dùng RNG.
//!
//! ## Chuẩn hoá không sống ở đây
//!
//! Khác với `mow_mind::prompt::canonical_registry`, việc sắp/khử trùng lặp
//! `stock` và `folk` sống ở [`Dossier::canonical_stock`]/[`Dossier::canonical_folk`]
//! — vì [`crate::without_model`] cần đúng phép chuẩn hoá đó và không nên chép
//! lại nó. `prompt_of` chỉ gọi, không tự làm lại.

use crate::dossier::Dossier;

/// Định danh prompt, dùng làm khóa stub và tên tệp bản ghi.
pub const PROMPT_ID: &str = "yuu.advise";

/// Phiên bản prompt. Tăng số này mỗi lần đổi một ký tự trong prompt (`§20.10`).
pub const PROMPT_VERSION: u32 = 1;

/// Số sự kiện tối đa đưa vào prompt — trần chứ không phải toàn bộ lịch sử,
/// cùng lý do `mow_mind::prompt::MAX_RECENT` tồn tại.
pub const MAX_EVENTS: usize = 12;

const HEADER: &str = "Bạn là Yuu — trợ lý phân tích của True God trong một thế giới mô phỏng.
Việc của bạn không phải là kể chuyện: nó là ĐỌC hồ sơ dưới đây và trả lời đúng
những gì hồ sơ chứng minh được.

== HỒ SƠ ==
Phần dưới do engine dựng từ đồ thị nhân quả có thật. Đây là toàn bộ những gì
bạn biết lúc này, không có gì thêm.
";

const RULES: &str = "
== LUẬT ==
1. Chỉ nói những gì hồ sơ ở trên chứng minh được. Không thêm sự kiện, không
   thêm số liệu, không đoán điều hồ sơ không có.
2. Mỗi câu trong `lines` phải kèm ít nhất một `cites` trỏ đúng tới `seq` của
   một sự kiện có trong hồ sơ trên. Một câu không trích dẫn được sẽ bị hệ
   thống cắt bỏ trước khi tới người chơi — không phải để người chơi tự đánh
   giá.
3. Mỗi đề xuất trong `proposals` phải nêu đúng một `power` có thật và ít nhất
   một `cites`. Không tự đặt tên quyền năng.
4. Trả về đúng một đối tượng JSON, không lời dẫn, không hàng rào mã.

== HÌNH DẠNG TRẢ LỜI ==
{\"lines\": [{\"text\": \"<một câu>\", \"cites\": [<seq>, ...]}], \"proposals\": [{\"power\": \"<tên quyền năng>\", \"why\": \"<một câu>\", \"cites\": [<seq>, ...]}]}
";

/// Ghi một dòng `khóa: giá trị`.
fn field(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value);
    out.push('\n');
}

/// Prompt tư vấn cho một [`Dossier`] và một câu hỏi.
///
/// Công khai để test khẳng định về prompt mà **không cần model**: hình dạng
/// của prompt là một hợp đồng, và một hợp đồng chỉ kiểm được qua mạng thì trên
/// thực tế là không kiểm.
#[must_use]
pub fn prompt_of(d: &Dossier, question: &str) -> String {
    let stock = d.canonical_stock();
    let folk = d.canonical_folk();
    let events = d.recent_events(MAX_EVENTS);

    let mut out = String::with_capacity(1024);
    out.push_str(HEADER);

    field(&mut out, "tick", &d.tick.to_string());

    out.push_str("kho:\n");
    if stock.is_empty() {
        out.push_str("  (trống)\n");
    } else {
        for (name, qty) in &stock {
            out.push_str("  - ");
            out.push_str(name);
            out.push_str(": ");
            out.push_str(&qty.to_string());
            out.push('\n');
        }
    }

    out.push_str("dân làng:\n");
    if folk.is_empty() {
        out.push_str("  (không có ai)\n");
    } else {
        for f in &folk {
            out.push_str(&format!(
                "  - #{} {} ({}): ý định {}, đói {}\n",
                f.id, f.name, f.role, f.intent, f.hunger
            ));
        }
    }

    out.push_str("sự kiện gần đây (cũ trước, mới sau):\n");
    if events.is_empty() {
        out.push_str("  (không có gì)\n");
    } else {
        for (i, e) in events.iter().enumerate() {
            let actor = e
                .actor
                .map_or_else(|| "(không rõ)".to_owned(), |a| format!("#{a}"));
            let cause = e
                .cause
                .map_or_else(|| "(không có)".to_owned(), |c| format!("seq={c}"));
            out.push_str(&format!(
                "  {}. [seq={}] tick={} loại={} tác nhân={} nguyên nhân={} — {}\n",
                i + 1,
                e.seq,
                e.tick,
                e.kind,
                actor,
                cause,
                e.summary
            ));
        }
    }

    out.push_str("\n== CÂU HỎI ==\n");
    out.push_str(question.trim());
    out.push('\n');

    out.push_str(RULES);
    out
}
