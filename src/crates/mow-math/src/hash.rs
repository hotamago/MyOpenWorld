//! Hash canonical cho state (`idea.md §19.6`, `plan.md §P10.2`).
//!
//! Hai lần chạy cùng một thế giới phải cho cùng một chuỗi 32 byte, kể cả khi
//! chạy trên máy khác, số luồng khác, hay bản Rust khác. Ba thứ phá điều đó và
//! ba thứ đó đều bị chặn ở đây:
//!
//! 1. **Thứ tự lặp không xác định.** `DefaultHasher` của std còn không hứa ổn
//!    định giữa các phiên bản, và `HashMap` thì đổi thứ tự mỗi lần chạy. Nên ở
//!    đây có [`StateHasher::hash_set`] bắt buộc sắp xếp trước khi trộn.
//! 2. **Nhập nhằng ranh giới.** `("ab", "c")` và `("a", "bc")` phải cho hai hash
//!    khác nhau; mọi thứ có độ dài thay đổi đều được ghi kèm độ dài.
//! 3. **Endianness.** Mọi số ghi ở little-endian tường minh.

use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Giá trị hash 32 byte.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct StateHash(pub [u8; 32]);

impl StateHash {
    /// Hash của trạng thái rỗng.
    pub const ZERO: StateHash = StateHash([0u8; 32]);

    /// Dạng hex thường, 64 ký tự. Đây là dạng dùng trong log và repro bundle.
    pub fn to_hex(self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Bốn byte đầu ở dạng hex — đủ để đọc trong log khi bisect theo tick.
    pub fn short(self) -> String {
        self.to_hex()[..8].to_owned()
    }

    /// Đọc từ hex. Trả `None` nếu không phải đúng 64 ký tự hex.
    pub fn from_hex(s: &str) -> Option<StateHash> {
        if s.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks_exact(2).enumerate() {
            let hi = (chunk[0] as char).to_digit(16)? as u8;
            let lo = (chunk[1] as char).to_digit(16)? as u8;
            out[i] = (hi << 4) | lo;
        }
        Some(StateHash(out))
    }
}

impl core::fmt::Debug for StateHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "StateHash({})", self.short())
    }
}

impl core::fmt::Display for StateHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Thẻ kiểu, ghi trước mỗi giá trị.
///
/// Không có thẻ thì `1u64` và `Some(1u32)` có thể cho cùng chuỗi byte, và hai
/// state khác nhau sẽ cho cùng hash. Đó đúng là loại lỗi mà harness determinism
/// không bao giờ bắt được, vì nó làm hai thứ khác nhau *trông giống nhau*.
#[repr(u8)]
enum Tag {
    U64 = 1,
    I64 = 2,
    Bytes = 3,
    Str = 4,
    SeqBegin = 5,
    SeqEnd = 6,
    None = 7,
    Some = 8,
    Bool = 9,
    U128 = 10,
    I128 = 11,
    Hash = 12,
}

/// Bộ trộn hash canonical.
#[derive(Clone)]
pub struct StateHasher {
    inner: Hasher,
}

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHasher {
    /// Bộ trộn rỗng.
    pub fn new() -> StateHasher {
        StateHasher {
            inner: Hasher::new(),
        }
    }

    /// Bộ trộn gắn với một domain, để hash của hai loại đối tượng khác nhau
    /// không bao giờ va nhau dù nội dung giống hệt.
    pub fn with_domain(domain: &str) -> StateHasher {
        let mut h = StateHasher::new();
        h.write_str(domain);
        h
    }

    fn tag(&mut self, t: Tag) {
        self.inner.update(&[t as u8]);
    }

    /// Trộn một `u64`.
    pub fn write_u64(&mut self, v: u64) -> &mut Self {
        self.tag(Tag::U64);
        self.inner.update(&v.to_le_bytes());
        self
    }

    /// Trộn một `i64`.
    pub fn write_i64(&mut self, v: i64) -> &mut Self {
        self.tag(Tag::I64);
        self.inner.update(&v.to_le_bytes());
        self
    }

    /// Trộn một `u128`.
    pub fn write_u128(&mut self, v: u128) -> &mut Self {
        self.tag(Tag::U128);
        self.inner.update(&v.to_le_bytes());
        self
    }

    /// Trộn một `i128`.
    pub fn write_i128(&mut self, v: i128) -> &mut Self {
        self.tag(Tag::I128);
        self.inner.update(&v.to_le_bytes());
        self
    }

    /// Trộn một bool.
    pub fn write_bool(&mut self, v: bool) -> &mut Self {
        self.tag(Tag::Bool);
        self.inner.update(&[u8::from(v)]);
        self
    }

    /// Trộn chuỗi byte kèm độ dài.
    pub fn write_bytes(&mut self, v: &[u8]) -> &mut Self {
        self.tag(Tag::Bytes);
        self.inner.update(&(v.len() as u64).to_le_bytes());
        self.inner.update(v);
        self
    }

    /// Trộn chuỗi UTF-8 kèm độ dài.
    pub fn write_str(&mut self, v: &str) -> &mut Self {
        self.tag(Tag::Str);
        self.inner.update(&(v.len() as u64).to_le_bytes());
        self.inner.update(v.as_bytes());
        self
    }

    /// Trộn một hash đã tính sẵn.
    pub fn write_hash(&mut self, v: StateHash) -> &mut Self {
        self.tag(Tag::Hash);
        self.inner.update(&v.0);
        self
    }

    /// Trộn một `Option`.
    pub fn write_option<T, F>(&mut self, v: Option<T>, f: F) -> &mut Self
    where
        F: FnOnce(&mut StateHasher, T),
    {
        match v {
            None => {
                self.tag(Tag::None);
            }
            Some(inner) => {
                self.tag(Tag::Some);
                f(self, inner);
            }
        }
        self
    }

    /// Trộn một dãy **có thứ tự**. Thứ tự là một phần của giá trị.
    pub fn write_seq<T, I, F>(&mut self, items: I, mut f: F) -> &mut Self
    where
        I: IntoIterator<Item = T>,
        F: FnMut(&mut StateHasher, T),
    {
        self.tag(Tag::SeqBegin);
        let mut n = 0u64;
        for item in items {
            f(self, item);
            n += 1;
        }
        self.inner.update(&n.to_le_bytes());
        self.tag(Tag::SeqEnd);
        self
    }

    /// Trộn một tập **không có thứ tự nội tại**, bằng cách hash từng phần tử rồi
    /// sắp xếp các hash.
    ///
    /// Đây là chỗ duy nhất được phép đưa một `HashMap` vào state hash, và nó
    /// hoạt động vì thứ tự bị loại bỏ hoàn toàn trước khi trộn. Nhớ rằng phần
    /// tử trùng nhau vẫn được đếm — đây là multiset, không phải set.
    pub fn write_set<T, I, F>(&mut self, items: I, mut f: F) -> &mut Self
    where
        I: IntoIterator<Item = T>,
        F: FnMut(&mut StateHasher, T),
    {
        let mut hashes: Vec<[u8; 32]> = Vec::new();
        for item in items {
            let mut sub = StateHasher::new();
            f(&mut sub, item);
            hashes.push(sub.inner.finalize().into());
        }
        hashes.sort_unstable();
        self.tag(Tag::SeqBegin);
        for h in &hashes {
            self.inner.update(h);
        }
        self.inner.update(&(hashes.len() as u64).to_le_bytes());
        self.tag(Tag::SeqEnd);
        self
    }

    /// Chốt và trả hash.
    pub fn finish(&self) -> StateHash {
        StateHash(self.inner.finalize().into())
    }
}

/// Đối tượng có hash canonical.
///
/// Cố ý **không** cài `CanonicalHash` cho `f32`/`f64` — kể cả bằng cách hash
/// bit pattern. Nếu một số thực lọt vào state thì lỗi phải nổ lúc biên dịch,
/// không phải trở thành một hash ổn định của một giá trị không ổn định.
pub trait CanonicalHash {
    /// Trộn `self` vào bộ trộn.
    fn canonical_hash(&self, h: &mut StateHasher);

    /// Hash độc lập của riêng `self`.
    fn state_hash(&self) -> StateHash {
        let mut h = StateHasher::new();
        self.canonical_hash(&mut h);
        h.finish()
    }
}

macro_rules! impl_int {
    ($($t:ty => $m:ident),* $(,)?) => {
        $(impl CanonicalHash for $t {
            fn canonical_hash(&self, h: &mut StateHasher) { h.$m(*self as _); }
        })*
    };
}
impl_int!(u8 => write_u64, u16 => write_u64, u32 => write_u64, u64 => write_u64);
impl_int!(i8 => write_i64, i16 => write_i64, i32 => write_i64, i64 => write_i64);
impl_int!(u128 => write_u128, i128 => write_i128);

impl CanonicalHash for bool {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_bool(*self);
    }
}

impl CanonicalHash for str {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self);
    }
}

impl CanonicalHash for String {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self);
    }
}

impl<T: CanonicalHash> CanonicalHash for Option<T> {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_option(self.as_ref(), |hh, v| v.canonical_hash(hh));
    }
}

impl<T: CanonicalHash> CanonicalHash for Vec<T> {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.iter(), |hh, v| v.canonical_hash(hh));
    }
}

impl<T: CanonicalHash> CanonicalHash for [T] {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.iter(), |hh, v| v.canonical_hash(hh));
    }
}

impl CanonicalHash for crate::fixed::Fx {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.raw());
    }
}

impl CanonicalHash for crate::fixed::Unit {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.get().raw());
    }
}

impl CanonicalHash for crate::prob::Prob {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.raw());
    }
}

impl CanonicalHash for crate::rate::Rate {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.num());
        h.write_i64(self.den());
    }
}

impl CanonicalHash for crate::coord::WorldPos {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.x);
        h.write_i64(self.y);
        h.write_i64(self.z);
    }
}

impl CanonicalHash for crate::coord::ChunkPos {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(self.cx);
        h.write_i64(self.cy);
        h.write_i64(self.cz);
    }
}
