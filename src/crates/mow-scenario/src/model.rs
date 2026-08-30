//! Mô hình dữ liệu của một kịch bản.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Chế độ gọi mô hình khi chạy kịch bản.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LlmMode {
    /// Trả lời cố định. Mặc định, và là chế độ đúng cho hầu hết kịch bản:
    /// ta đang test **luật**, không test mô hình.
    #[default]
    Stub,
    /// Phát lại từ bản ghi.
    Replay,
    /// Ghi lại lời gọi thật.
    Record,
    /// Gọi thật.
    Live,
}

/// Một kịch bản.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Tên, cũng là định danh trong báo cáo.
    pub scenario: String,
    /// Mô tả: kịch bản này chứng minh điều gì.
    #[serde(default)]
    pub description: String,
    /// Worldseed dùng để dựng thế giới.
    pub worldseed: String,
    /// Chế độ mô hình.
    #[serde(default)]
    pub llm_mode: LlmMode,
    /// Ghi đè seed, để hai kịch bản dùng chung worldseed không dính nhau.
    #[serde(default)]
    pub seed_overrides: BTreeMap<String, String>,
    /// Ràng buộc alias. Chạy **một lần** sau genesis.
    #[serde(default)]
    pub bind: BTreeMap<String, Binding>,
    /// Dựng điều kiện.
    #[serde(default)]
    pub given: Vec<Step>,
    /// Chạy.
    #[serde(default)]
    pub when: Vec<Step>,
    /// Kiểm chứng.
    #[serde(default)]
    pub then: Vec<Assertion>,
}

/// Cách chọn thực thể cho một alias.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Loại đối tượng: `entity`, `settlement`, `building`, ...
    pub kind: String,
    /// Giới hạn trong phạm vi của một alias khác.
    #[serde(rename = "in", default)]
    pub within: Option<String>,
    /// Chỉ lấy đối tượng có tag này.
    #[serde(default)]
    pub tag: Option<String>,
    /// `first` hoặc `nth`.
    #[serde(default = "mac_dinh_select")]
    pub select: String,
    /// Với `nth`, lấy cái thứ mấy, đếm từ 1.
    #[serde(default)]
    pub n: Option<usize>,
    /// Thứ tự sắp xếp. **Bắt buộc kết thúc bằng `id asc`.**
    ///
    /// Đây là quy tắc 1 của phần `bind` trong `§P7.3`, và nó không phải chuyện
    /// hình thức: thiếu vế phá hòa thì hai lần chạy có thể chọn hai thực thể
    /// khác nhau, và kịch bản trở nên chập chờn theo cách không ai truy được.
    /// Một kịch bản chập chờn tệ hơn không có kịch bản, vì nó dạy cả đội bỏ qua
    /// màu đỏ.
    pub order: Vec<String>,
}

fn mac_dinh_select() -> String {
    "first".to_owned()
}

/// Một bước trong `given` hoặc `when`.
///
/// Dùng biểu diễn ánh xạ tự do thay vì enum đóng: content pack định nghĩa được
/// bước mới (`§19.7`), và một enum trong engine sẽ chặn điều đó. Bù lại,
/// runner phải kiểm tên bước tại thời điểm chạy và **báo lỗi rõ** khi không
/// biết — xem [`crate::runner`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Step(pub BTreeMap<String, serde_yaml::Value>);

impl Step {
    /// Tên bước, tức khóa duy nhất của ánh xạ.
    pub fn name(&self) -> Option<&str> {
        if self.0.len() == 1 {
            self.0.keys().next().map(String::as_str)
        } else {
            None
        }
    }

    /// Tham số của bước.
    pub fn args(&self) -> Option<&serde_yaml::Value> {
        self.0.values().next()
    }

    /// Đọc một tham số dạng chuỗi.
    pub fn arg_str(&self, key: &str) -> Option<String> {
        self.args()?
            .get(key)
            .and_then(|v| v.as_str().map(str::to_owned))
    }

    /// Đọc một tham số dạng số nguyên.
    pub fn arg_int(&self, key: &str) -> Option<i64> {
        self.args()?.get(key).and_then(serde_yaml::Value::as_i64)
    }
}

/// Một khẳng định trong `then`. Cùng hình dạng với [`Step`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Assertion(pub BTreeMap<String, serde_yaml::Value>);

impl Assertion {
    /// Tên khẳng định.
    pub fn name(&self) -> Option<&str> {
        if self.0.len() == 1 {
            self.0.keys().next().map(String::as_str)
        } else {
            None
        }
    }

    /// Tham số.
    pub fn args(&self) -> Option<&serde_yaml::Value> {
        self.0.values().next()
    }

    /// Đọc tham số chuỗi.
    pub fn arg_str(&self, key: &str) -> Option<String> {
        self.args()?
            .get(key)
            .and_then(|v| v.as_str().map(str::to_owned))
    }

    /// Đọc tham số danh sách chuỗi.
    pub fn arg_list(&self, key: &str) -> Option<Vec<String>> {
        let v = self.args()?;
        let seq = if let Some(inner) = v.get(key) {
            inner.as_sequence()?.clone()
        } else {
            v.as_sequence()?.clone()
        };
        Some(
            seq.into_iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect(),
        )
    }
}

impl Scenario {
    /// Đọc từ YAML.
    pub fn from_yaml(s: &str) -> Result<Scenario, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    /// Kiểm tra hình dạng **trước khi chạy**.
    ///
    /// Bắt lỗi ở đây thay vì lúc chạy, vì một kịch bản sai cấu trúc sẽ "chạy"
    /// và cho kết quả xanh — quy tắc 2 của `§P7.3`.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut loi = Vec::new();

        if self.scenario.is_empty() {
            loi.push("`scenario` không được rỗng".to_owned());
        }
        if self.worldseed.is_empty() {
            loi.push("`worldseed` không được rỗng".to_owned());
        }

        for (alias, b) in &self.bind {
            if !alias.starts_with('@') {
                loi.push(format!("alias `{alias}` phải bắt đầu bằng `@`"));
            }
            if b.order.is_empty() {
                loi.push(format!("`bind.{alias}.order` không được rỗng"));
            } else if b.order.last().map(String::as_str) != Some("id asc") {
                loi.push(format!(
                    "`bind.{alias}.order` phải kết thúc bằng `id asc` để phá hòa. \
                     Hiện là `{}`. Thiếu vế này thì hai lần chạy có thể chọn hai thực thể \
                     khác nhau và kịch bản trở nên chập chờn (§P7.3, quy tắc 1)",
                    b.order.last().unwrap_or(&String::new())
                ));
            }
            match b.select.as_str() {
                "first" => {}
                "nth" => {
                    if b.n.is_none() {
                        loi.push(format!("`bind.{alias}` dùng `select: nth` nhưng thiếu `n`"));
                    } else if b.n == Some(0) {
                        loi.push(format!("`bind.{alias}.n` đếm từ 1, không phải từ 0"));
                    }
                }
                khac => loi.push(format!(
                    "`bind.{alias}.select` = `{khac}`, chỉ nhận `first` hoặc `nth`"
                )),
            }
            if let Some(w) = &b.within {
                if !self.bind.contains_key(w) {
                    loi.push(format!(
                        "`bind.{alias}.in` trỏ tới `{w}` nhưng alias đó không được định nghĩa"
                    ));
                }
                if w == alias {
                    loi.push(format!("`bind.{alias}.in` trỏ tới chính nó"));
                }
            }
        }

        for (nhan, ds) in [("given", &self.given), ("when", &self.when)] {
            for (i, s) in ds.iter().enumerate() {
                if s.name().is_none() {
                    loi.push(format!(
                        "`{nhan}[{i}]` phải là một ánh xạ có đúng một khóa là tên bước"
                    ));
                }
            }
        }
        for (i, a) in self.then.iter().enumerate() {
            if a.name().is_none() {
                loi.push(format!(
                    "`then[{i}]` phải là một ánh xạ có đúng một khóa là tên khẳng định"
                ));
            }
        }

        if self.then.is_empty() {
            loi.push(
                "`then` rỗng: kịch bản không khẳng định gì thì luôn xanh và không có giá trị"
                    .to_owned(),
            );
        }

        if loi.is_empty() {
            Ok(())
        } else {
            Err(loi)
        }
    }
}
