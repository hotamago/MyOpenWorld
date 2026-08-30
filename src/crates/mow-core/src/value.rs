//! [`Value`] — cây dữ liệu canonical cho payload của command và event.
//!
//! Vì sao không dùng thẳng `serde_json::Value`: nó có biến thể `Number` chứa
//! `f64`. Một content pack viết `speed: 1.5` sẽ lặng lẽ tạo ra một số thực
//! trên đường commit, và không lint nào bắt được vì lỗi nằm trong *dữ liệu*
//! chứ không nằm trong *mã*.
//!
//! `Value` không có biến thể số thực. Không phải "không khuyến khích" — là
//! **không tồn tại**, nên viết `1.5` trong content sẽ hỏng lúc parse, ở đúng
//! chỗ có tên file và số dòng.
//!
//! Map dùng [`BTreeMap`] chứ không phải `HashMap`, nên thứ tự duyệt là thứ tự
//! khóa và giống nhau ở mọi lần chạy (`plan.md §P10.3`).

use mow_math::{CanonicalHash, Fx, Prob, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Giá trị canonical.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum Value {
    /// Không có giá trị.
    Null,
    /// Đúng hoặc sai.
    Bool(bool),
    /// Số nguyên có dấu. **Đây là biến thể số duy nhất mang thang tự do.**
    Int(i64),
    /// Số nguyên không dấu, cho những thang cần đủ 64 bit như [`Prob`].
    Uint(u64),
    /// Q16.16 ở dạng thô.
    Fixed(Fx),
    /// Chuỗi.
    Text(String),
    /// Chuỗi byte, dùng cho hash và blob nhỏ.
    Bytes(Vec<u8>),
    /// Dãy có thứ tự.
    List(Vec<Value>),
    /// Ánh xạ có thứ tự khóa.
    Map(BTreeMap<String, Value>),
}

impl Value {
    /// Ánh xạ rỗng.
    pub fn map() -> Value {
        Value::Map(BTreeMap::new())
    }

    /// Dựng ánh xạ từ danh sách cặp.
    pub fn from_pairs<I, K>(pairs: I) -> Value
    where
        I: IntoIterator<Item = (K, Value)>,
        K: Into<String>,
    {
        Value::Map(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// Lấy trường của một ánh xạ.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(m) => m.get(key),
            _ => None,
        }
    }

    /// Lấy trường và đòi nó là số nguyên.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(Value::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Lấy trường và đòi nó là số nguyên không dấu.
    pub fn get_uint(&self, key: &str) -> Option<u64> {
        match self.get(key) {
            Some(Value::Uint(v)) => Some(*v),
            _ => None,
        }
    }

    /// Lấy trường và đòi nó là chuỗi.
    pub fn get_text(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Value::Text(v)) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Lấy trường và đòi nó là Q16.16.
    pub fn get_fixed(&self, key: &str) -> Option<Fx> {
        match self.get(key) {
            Some(Value::Fixed(v)) => Some(*v),
            _ => None,
        }
    }

    /// Lấy trường và đòi nó là xác suất.
    pub fn get_prob(&self, key: &str) -> Option<Prob> {
        self.get_uint(key).map(Prob::from_raw)
    }

    /// Đặt một trường; chỉ có tác dụng trên ánh xạ.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        if let Value::Map(m) = self {
            m.insert(key.into(), value);
        }
        self
    }

    /// Tên biến thể, dùng trong thông báo lỗi.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Uint(_) => "uint",
            Value::Fixed(_) => "fixed",
            Value::Text(_) => "text",
            Value::Bytes(_) => "bytes",
            Value::List(_) => "list",
            Value::Map(_) => "map",
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Value {
        Value::Bool(v)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Value {
        Value::Int(v)
    }
}
impl From<u64> for Value {
    fn from(v: u64) -> Value {
        Value::Uint(v)
    }
}
impl From<Fx> for Value {
    fn from(v: Fx) -> Value {
        Value::Fixed(v)
    }
}
impl From<Prob> for Value {
    fn from(v: Prob) -> Value {
        Value::Uint(v.raw())
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Value {
        Value::Text(v.to_owned())
    }
}
impl From<String> for Value {
    fn from(v: String) -> Value {
        Value::Text(v)
    }
}
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Value {
        Value::List(v.into_iter().map(Into::into).collect())
    }
}

impl CanonicalHash for Value {
    fn canonical_hash(&self, h: &mut StateHasher) {
        // Thẻ biến thể đi trước: nếu không, `Int(1)` và `Uint(1)` cho cùng hash
        // và hai state khác nhau sẽ trông giống nhau với harness determinism.
        h.write_str(self.type_name());
        match self {
            Value::Null => {}
            Value::Bool(v) => {
                h.write_bool(*v);
            }
            Value::Int(v) => {
                h.write_i64(*v);
            }
            Value::Uint(v) => {
                h.write_u64(*v);
            }
            Value::Fixed(v) => {
                h.write_i64(v.raw());
            }
            Value::Text(v) => {
                h.write_str(v);
            }
            Value::Bytes(v) => {
                h.write_bytes(v);
            }
            Value::List(v) => {
                h.write_seq(v.iter(), |hh, item| item.canonical_hash(hh));
            }
            Value::Map(m) => {
                // `BTreeMap` đã sắp theo khóa, nên đây là dãy có thứ tự xác
                // định và không cần `write_set`.
                h.write_seq(m.iter(), |hh, (k, v)| {
                    hh.write_str(k);
                    v.canonical_hash(hh);
                });
            }
        }
    }
}

/// Dựng nhanh một [`Value::Map`] trong test và trong mã dựng command.
///
/// ```
/// use mow_core::{val, Value};
/// let v = val! { "kind" => "move", "dx" => 1i64 };
/// assert_eq!(v.get_text("kind"), Some("move"));
/// ```
#[macro_export]
macro_rules! val {
    ($($k:expr => $v:expr),* $(,)?) => {{
        let mut m = std::collections::BTreeMap::new();
        $( m.insert(String::from($k), $crate::Value::from($v)); )*
        $crate::Value::Map(m)
    }};
}
