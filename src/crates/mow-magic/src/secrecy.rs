//! Bí mật vật phẩm không được vào prompt (`idea.md §8.10.3`, `§22.40`, `PE-06`).
//!
//! > **Rò một lần là bug nghiêm trọng.**
//!
//! Đây là bất biến khắc nghiệt nhất trong cả tài liệu, và lý do nằm ở chỗ nó
//! **không tự khỏi**: một bí mật đã lọt vào một prompt thì đã ra khỏi máy, đã có
//! thể nằm trong log của nhà cung cấp, và không có cách nào rút lại. Mọi bất
//! biến khác cho phép sửa rồi chạy tiếp; cái này thì không.
//!
//! ## Hai lớp, và lớp thứ hai **không thay thế** lớp thứ nhất
//!
//! 1. **View lọc theo người quan sát.** Thứ không được đưa vào view thì không
//!    lọt được vào prompt. Đây là lớp thật sự bảo vệ.
//! 2. **Auditor quét mọi prompt đã gửi.** Đây là lưới bắt lỗi cài đặt của lớp 1.
//!
//! Cùng cấu trúc với `§P6.2` quy tắc 3–4 ở prompt registry, và vì cùng một lý
//! do: quét chuỗi bắt được khẩu quyết và tên riêng — những thứ tồn tại dưới dạng
//! chuỗi cố định — nhưng **không bao giờ** bắt được rò rỉ ngữ nghĩa kiểu *"cây
//! trượng này phản ứng với người mang dòng máu hoàng gia"*.
//!
//! Nên [`audit_prompt`] là lưới cuối, và nó **ném** thay vì cảnh báo: một cảnh
//! báo trong log là thứ không ai đọc, và một bí mật đã gửi đi thì không rút lại
//! được.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

/// Một bí mật của vật phẩm.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Secret {
    /// Vật phẩm nào.
    pub item: u64,
    /// Loại: `command_word`, `true_name`, `curse`, `hidden_enchant`.
    pub kind: String,
    /// Nội dung — **thứ không bao giờ được ra khỏi máy** trừ khi người xem biết.
    pub content: String,
}

/// Ai biết bí mật nào.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SecretRegistry {
    secrets: Vec<Secret>,
    /// `(người, item, kind)` — ai đã biết gì.
    known_by: BTreeSet<(u64, u64, String)>,
}

impl SecretRegistry {
    /// Rỗng.
    pub fn new() -> SecretRegistry {
        SecretRegistry::default()
    }

    /// Ghi nhận một bí mật.
    pub fn add(&mut self, s: Secret) -> &mut SecretRegistry {
        self.secrets.push(s);
        self
    }

    /// Cho một người biết.
    pub fn reveal_to(&mut self, who: u64, item: u64, kind: &str) {
        self.known_by.insert((who, item, kind.to_owned()));
    }

    /// Người này có biết không.
    pub fn knows(&self, who: u64, item: u64, kind: &str) -> bool {
        self.known_by.contains(&(who, item, kind.to_owned()))
    }

    /// Mọi bí mật của một vật phẩm mà người này **không** biết.
    ///
    /// Đây là danh sách mà Auditor quét prompt để tìm.
    pub fn hidden_from(&self, who: u64, item: u64) -> Vec<&Secret> {
        self.secrets
            .iter()
            .filter(|s| s.item == item && !self.knows(who, s.item, &s.kind))
            .collect()
    }

    /// **Lớp 1**: dựng view của một vật phẩm cho một người xem.
    ///
    /// Chỉ những bí mật họ đã biết mới có mặt. Đây là thứ đi vào prompt, và nó
    /// **không có chỗ** cho những gì họ chưa biết — không phải "có chỗ nhưng để
    /// trống".
    pub fn view_for(&self, who: u64, item: u64) -> ItemView {
        ItemView {
            item,
            known_secrets: self
                .secrets
                .iter()
                .filter(|s| s.item == item && self.knows(who, s.item, &s.kind))
                .cloned()
                .collect(),
        }
    }
}

/// Vật phẩm, nhìn từ một người cụ thể.
///
/// **Đây là kiểu duy nhất được đưa vào prompt.** `Secret` thô thì không.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemView {
    /// Vật phẩm nào.
    pub item: u64,
    /// Chỉ những bí mật người xem đã biết.
    pub known_secrets: Vec<Secret>,
}

/// Một lần rò rỉ đã bị bắt.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "RÒ BÍ MẬT: prompt gửi cho thực thể {viewer} chứa `{kind}` của vật phẩm \
     {item} — người này chưa biết nó. §8.10.3 gọi đây là bug NGHIÊM TRỌNG, vì \
     một bí mật đã gửi đi thì không rút lại được."
)]
pub struct Leak {
    /// Prompt gửi cho ai.
    pub viewer: u64,
    /// Vật phẩm nào.
    pub item: u64,
    /// Loại bí mật.
    pub kind: String,
}

/// **Lớp 2**: quét một prompt đã dựng, tìm bí mật không được phép.
///
/// Trả `Err` chứ không log cảnh báo. `§22.40`: rò một lần là bug nghiêm trọng,
/// và cách duy nhất để một bug nghiêm trọng không bị bỏ qua là làm nó dừng
/// chương trình ở môi trường phát triển.
///
/// **Không thay thế lớp 1.** Nó chỉ bắt được bí mật xuất hiện *nguyên văn*.
/// Một prompt viết *"cây trượng phản ứng với dòng máu hoàng gia"* rò đúng cái
/// `attunement` mà không chứa chuỗi nào — và hàm này sẽ nói prompt đó sạch.
pub fn audit_prompt(
    prompt: &str,
    viewer: u64,
    items_mentioned: &[u64],
    reg: &SecretRegistry,
) -> Result<(), Vec<Leak>> {
    let mut ro = Vec::new();
    for item in items_mentioned {
        for s in reg.hidden_from(viewer, *item) {
            if prompt.contains(&s.content) {
                ro.push(Leak {
                    viewer,
                    item: *item,
                    kind: s.kind.clone(),
                });
            }
        }
    }
    if ro.is_empty() {
        Ok(())
    } else {
        Err(ro)
    }
}

/// Quét **cả một phiên**: mọi prompt đã gửi.
///
/// `§8.10.3` nói Auditor quét *"mọi prompt đã gửi"* — số nhiều và toàn bộ. Một
/// lần quét ngẫu nhiên không đủ: rò rỉ hiếm là rò rỉ khó tái hiện, và rò rỉ khó
/// tái hiện là rò rỉ sẽ sống sót tới bản phát hành.
pub fn audit_session(
    prompts: &[(u64, String, Vec<u64>)],
    reg: &SecretRegistry,
) -> Result<(), Vec<Leak>> {
    let mut tat_ca = Vec::new();
    for (viewer, p, items) in prompts {
        if let Err(mut ro) = audit_prompt(p, *viewer, items, reg) {
            tat_ca.append(&mut ro);
        }
    }
    if tat_ca.is_empty() {
        Ok(())
    } else {
        Err(tat_ca)
    }
}
