//! Định danh và **khóa ổn định** (`idea.md §19.6`).
//!
//! Có hai loại định danh trong hệ thống này và trộn chúng là một lỗi tinh vi:
//!
//! - **Định danh** ([`EntityId`], [`WorldId`], [`BranchId`]) trả lời "đây là ai".
//!   Nó sinh ra từ genesis và ổn định suốt đời một thực thể.
//! - **Khóa ổn định** ([`StableKey`]) trả lời "khi hai việc xảy ra cùng lúc thì
//!   xử lý cái nào trước". Nó là **thứ tự toàn phần** dùng để sắp xếp trước khi
//!   commit, để kết quả không phụ thuộc thứ tự mà job song song trả về.
//!
//! `idea.md §22.43` nói rõ một điều dễ làm sai: `EntityId` được dùng để **sắp
//! xếp ổn định**, không phải để **quyết định thắng thua**. Nếu hai người cùng
//! với tay lấy một quả táo và người có id nhỏ hơn luôn thắng, thì id đã trở
//! thành một thuộc tính của thế giới — và nó là thuộc tính vô hình, không ai
//! chơi game mà đoán được.

use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Sinh một newtype định danh trên `u64`.
macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident, $prefix:literal) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);

        impl $name {
            #[doc = "Giá trị dành riêng nghĩa là \"không có\"."]
            pub const NONE: $name = $name(0);

            #[doc = "Dựng."]
            #[inline]
            pub const fn new(v: u64) -> $name { $name(v) }

            #[doc = "Giá trị thô."]
            #[inline]
            pub const fn get(self) -> u64 { self.0 }

            #[doc = "Có phải giá trị \"không có\" không."]
            #[inline]
            pub const fn is_none(self) -> bool { self.0 == 0 }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl CanonicalHash for $name {
            fn canonical_hash(&self, h: &mut StateHasher) {
                h.write_str($prefix);
                h.write_u64(self.0);
            }
        }
    };
}

id_type!(
    /// Định danh một thực thể. Ổn định suốt đời, kể cả qua cổng sang world khác.
    EntityId, "e#"
);
id_type!(
    /// Định danh một thế giới trong đa vũ trụ.
    WorldId, "w#"
);
id_type!(
    /// Định danh một nhánh lịch sử (`§4.4`).
    BranchId, "b#"
);
id_type!(
    /// Định danh một content pack đã nạp.
    PackId, "p#"
);

/// Khóa ổn định: thứ tự toàn phần dùng để sắp xếp trước khi commit.
///
/// Ba trường theo đúng thứ tự ưu tiên ở `plan.md §P6.5`:
///
/// 1. `priority` — tầng giải quyết. Hai việc khác tầng thì tầng quyết định,
///    và đây là chỗ *luật của thế giới* được diễn đạt.
/// 2. `discriminator` — dữ liệu phân định trong cùng tầng, ví dụ tốc độ ra đòn.
///    Đây cũng là luật, và cũng quan sát được từ trong thế giới.
/// 3. `entity` — **chỉ để phá hòa**, khi hai việc giống hệt nhau về mọi mặt
///    luật quan tâm. Nếu hai trường trên đã phân định thì trường này không bao
///    giờ được đọc tới.
///
/// Property test ở `PB-10` chứng minh rằng đảo `EntityId` của mọi thực thể
/// không đổi kết quả — nghĩa là trường thứ ba thật sự chỉ phá hòa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StableKey {
    /// Tầng giải quyết; nhỏ hơn chạy trước.
    pub priority: i32,
    /// Phân định trong tầng; nhỏ hơn chạy trước.
    pub discriminator: i64,
    /// Phá hòa cuối cùng.
    pub entity: EntityId,
}

impl StableKey {
    /// Dựng.
    pub const fn new(priority: i32, discriminator: i64, entity: EntityId) -> StableKey {
        StableKey {
            priority,
            discriminator,
            entity,
        }
    }

    /// Khóa chỉ phá hòa theo thực thể — dùng khi luật không phân định gì thêm.
    pub const fn plain(entity: EntityId) -> StableKey {
        StableKey {
            priority: 0,
            discriminator: 0,
            entity,
        }
    }
}

impl CanonicalHash for StableKey {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_i64(i64::from(self.priority));
        h.write_i64(self.discriminator);
        self.entity.canonical_hash(h);
    }
}

/// Bộ cấp phát định danh, xác định và tuần tự.
///
/// Không dùng UUID ngẫu nhiên cho `EntityId`: id phải là hàm của lịch sử để
/// replay cùng một event log cho cùng những thực thể. UUID xuất hiện ở biên
/// ngoài (branch, request) nơi không có yêu cầu replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdAllocator {
    next: u64,
}

impl Default for IdAllocator {
    fn default() -> Self {
        // Bắt đầu từ 1 vì 0 dành cho `NONE`.
        IdAllocator { next: 1 }
    }
}

impl IdAllocator {
    /// Bộ cấp phát mới.
    pub fn new() -> IdAllocator {
        IdAllocator::default()
    }

    /// Khôi phục từ snapshot. Dùng khi nạp save.
    pub fn resume_from(next: u64) -> IdAllocator {
        IdAllocator { next: next.max(1) }
    }

    /// Id kế tiếp sẽ cấp. Phần của state, nên nằm trong state hash.
    pub fn peek(&self) -> u64 {
        self.next
    }

    /// Cấp một `EntityId` mới.
    pub fn next_entity(&mut self) -> EntityId {
        let id = EntityId(self.next);
        self.next += 1;
        id
    }
}

impl CanonicalHash for IdAllocator {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.next);
    }
}
