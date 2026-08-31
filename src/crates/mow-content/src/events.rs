//! Sự kiện khai báo được trong content pack.
//!
//! ## `EventDef` không phải bản ghi trong event log
//!
//! Phân biệt này quan trọng và rất dễ trượt. `§22` nói về **event log**: sổ ghi
//! những chuyện *đã xảy ra*, thứ replay dựng lại thế giới từ đó, và không content
//! pack nào được ghi vào.
//!
//! [`EventDef`] ở đây là **khuôn của một chuyện có thể xảy ra** — động đất, sương
//! giá đầu mùa, lễ hội mùa màng. Nó là dữ liệu tĩnh: đạo diễn chọn một khuôn, và
//! thứ đi vào event log là *command* sinh ra từ lựa chọn đó, không phải cái khuôn.
//! Giữ hai thứ này cùng tên sẽ dẫn tới câu hỏi sai "vì sao pack ghi được vào
//! event log", và câu trả lời là nó không ghi được.

use crate::error::ContentError;
use crate::loader::{load_directory, parse_simple, DefRegistry, Definition};
use crate::text::LocalizedText;
use serde::Serialize;
use std::path::Path;

/// Phiên bản schema mà bộ nạp này hiểu.
pub const EVENT_SCHEMA: &str = "event_def/v1";

/// Sổ sự kiện, tra theo id và lặp theo id tăng dần.
pub type EventRegistry = DefRegistry<EventDef>;

/// Định nghĩa một loại sự kiện.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventDef {
    /// Định danh ổn định, trùng tên thư mục chứa nó.
    pub id: String,
    /// Tên hiển thị theo ngôn ngữ.
    pub name: LocalizedText,
    /// Nhãn phân loại, đã sắp xếp và khử trùng lặp.
    pub tags: Vec<String>,
    /// Đường dẫn tương đối tới script quyết định điều kiện và hệ quả, nếu có.
    pub script: Option<String>,
}

impl Definition for EventDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl EventDef {
    /// Sự kiện này có tag đó không.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Đọc một `metadata.yaml` đã có sẵn trong bộ nhớ.
    ///
    /// `path` chỉ dùng để dựng thông báo lỗi; nó không cần tồn tại thật.
    pub fn from_metadata(
        path: &Path,
        directory_name: &str,
        text: &str,
    ) -> Result<EventDef, ContentError> {
        let p = parse_simple(path, directory_name, text, EVENT_SCHEMA)?;
        Ok(EventDef {
            id: p.id,
            name: p.name,
            tags: p.tags,
            script: p.script,
        })
    }
}

/// Nạp mọi sự kiện từ một thư mục `events/`.
pub fn load_events(dir: impl AsRef<Path>) -> Result<EventRegistry, ContentError> {
    let map = load_directory(dir.as_ref(), EventDef::from_metadata)?;
    Ok(DefRegistry::from_map(map))
}

#[cfg(test)]
mod tests {
    use super::{load_events, EventDef};
    use crate::error::ContentError;
    use std::path::{Path, PathBuf};

    fn events_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core/events")
    }

    #[test]
    fn nap_su_kien_tu_content_core() {
        let r = load_events(events_dir()).expect("content/core/events phải nạp được");
        let ids: Vec<&str> = r.ids().collect();
        assert_eq!(ids, ["earthquake", "first_frost", "harvest_festival"]);
        assert!(r.get("earthquake").expect("có").has_tag("hazard"));
    }

    #[test]
    fn schema_sai_bi_tu_choi() {
        let path = PathBuf::from("content/core/events/earthquake/metadata.yaml");
        let text = "schema: item_def/v1\nid: earthquake\nname: { en: \"Earthquake\" }\n";
        let e = EventDef::from_metadata(&path, "earthquake", text).expect_err("phải lỗi");
        let s = e.to_string();
        assert!(matches!(e, ContentError::UnknownSchema { .. }), "{s}");
        assert!(
            s.contains("event_def/v1"),
            "lỗi phải nói ra thứ nó chờ: {s}"
        );
    }
}
