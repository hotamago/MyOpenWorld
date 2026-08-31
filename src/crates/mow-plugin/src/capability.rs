//! Quyền của content pack theo capability (`idea.md §19.7`, `§22.29`, `PF-01`).
//!
//! ## Vì sao một pack cần quyền, khi nó chỉ là dữ liệu
//!
//! Vì nó không chỉ là dữ liệu. Một pack khai được luật, module WASM, prompt và
//! generator — và ba trong bốn thứ đó **chạy**. Câu hỏi *"pack này được đụng
//! vào cái gì"* vì thế là một câu hỏi an ninh thật, không phải một thủ tục.
//!
//! ## Khai trước, kiểm lúc nạp
//!
//! Cùng nguyên tắc với `ModuleManifest` ở `mow-magic`: quyền **khai trong
//! manifest**, kiểm **lúc nạp**, và một pack xin quyền nó không có thì **từ
//! chối nạp** chứ không bỏ qua phần vi phạm. Nạp một phần là cách hỏng tệ hơn
//! không nạp: một số định nghĩa có, một số không, và thế giới tham chiếu tới
//! những thứ không tồn tại.
//!
//! ## Mặc định là **không có quyền gì**
//!
//! Một pack không khai `capabilities` chỉ khai được dữ liệu tĩnh: vật phẩm,
//! công thức, bảng tra. Đó là đại đa số pack, và chúng an toàn theo cấu trúc
//! chứ không phải nhờ ai đó đã đọc qua.
//!
//! | Capability | Cho phép | Vì sao phải xin riêng |
//! |---|---|---|
//! | [`Capability::DefineContent`] | vật phẩm, công thức, bảng | mặc định, ai cũng có |
//! | [`Capability::DefineLaw`] | luật DSL Tier 0 | luật đổi kết quả mô phỏng |
//! | [`Capability::DefineModule`] | module WASM | code chạy trong sandbox |
//! | [`Capability::DefinePrompt`] | prompt persona | chạm vào đường LLM |
//! | [`Capability::DefineGenerator`] | generator địa hình/loài | đổi `base` của thế giới |
//! | [`Capability::OverrideForeign`] | ghi đè id pack khác | đổi hành vi pack khác |

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Một quyền mà pack xin trong manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Khai dữ liệu tĩnh: vật phẩm, công thức, bảng tra. **Mặc định.**
    DefineContent,
    /// Khai luật DSL Tier 0.
    DefineLaw,
    /// Khai module WASM.
    DefineModule,
    /// Khai prompt và persona.
    DefinePrompt,
    /// Khai generator địa hình, khí hậu, loài.
    DefineGenerator,
    /// Ghi đè id thuộc namespace của pack khác.
    OverrideForeign,
}

impl Capability {
    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::DefineContent => "define_content",
            Capability::DefineLaw => "define_law",
            Capability::DefineModule => "define_module",
            Capability::DefinePrompt => "define_prompt",
            Capability::DefineGenerator => "define_generator",
            Capability::OverrideForeign => "override_foreign",
        }
    }

    /// Quyền này có đổi được kết quả mô phỏng không.
    ///
    /// Dùng cho UI: một pack chỉ thêm vật phẩm và một pack viết lại luật vật lý
    /// **phải hiện khác nhau** trước khi người dùng bấm cài.
    pub fn affects_simulation(self) -> bool {
        !matches!(self, Capability::DefineContent)
    }

    /// Câu cảnh báo cho người cài.
    pub fn warning(self) -> &'static str {
        match self {
            Capability::DefineContent => "thêm dữ liệu tĩnh — không đổi cách thế giới vận hành",
            Capability::DefineLaw => "viết luật mới: đổi kết quả mô phỏng",
            Capability::DefineModule => "chạy code trong sandbox: có fuel và trần bộ nhớ",
            Capability::DefinePrompt => "chạm vào đường LLM: đổi cách nhân vật nghĩ",
            Capability::DefineGenerator => "đổi generator: hai world cùng seed sẽ khác nhau",
            Capability::OverrideForeign => "ghi đè nội dung của pack khác",
        }
    }
}

/// Loại nội dung mà một id thuộc về, suy ra từ thư mục chứa nó.
///
/// Suy từ đường dẫn chứ không từ một trường khai trong file: một pack khai
/// `kind: content` cho một file luật thì lời khai đó là thứ ta đang muốn kiểm,
/// nên không dùng nó làm căn cứ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    /// `content/`, `items/`, `recipes/`, `tables/`.
    Content,
    /// `laws/`.
    Law,
    /// `modules/`.
    Module,
    /// `prompts/`.
    Prompt,
    /// `generators/`.
    Generator,
}

impl ContentKind {
    /// Loại này cần quyền nào.
    pub fn requires(self) -> Capability {
        match self {
            ContentKind::Content => Capability::DefineContent,
            ContentKind::Law => Capability::DefineLaw,
            ContentKind::Module => Capability::DefineModule,
            ContentKind::Prompt => Capability::DefinePrompt,
            ContentKind::Generator => Capability::DefineGenerator,
        }
    }

    /// Suy loại từ đường dẫn tương đối trong pack.
    ///
    /// Không nhận ra thì trả [`ContentKind::Content`] — mức quyền **thấp
    /// nhất**. Mặc định phải nghiêng về ít quyền: một thư mục lạ được coi là
    /// dữ liệu tĩnh sẽ bị chặn nếu nó thật sự chứa luật, còn mặc định ngược
    /// lại thì một thư mục lạ chứa luật sẽ lọt.
    pub fn from_path(path: &str) -> ContentKind {
        let p = path.replace('\\', "/");
        let dau = p.split('/').next().unwrap_or("");
        match dau {
            "laws" => ContentKind::Law,
            "modules" => ContentKind::Module,
            "prompts" => ContentKind::Prompt,
            "generators" => ContentKind::Generator,
            _ => ContentKind::Content,
        }
    }
}

/// Một vi phạm quyền, tìm thấy lúc nạp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Pack nào.
    pub pack: String,
    /// File nào.
    pub path: String,
    /// Cần quyền gì.
    pub needs: Capability,
}

impl core::fmt::Display for Violation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "pack `{}` có `{}` nhưng không xin `{}` trong manifest",
            self.pack,
            self.path,
            self.needs.as_str()
        )
    }
}

/// Quyền đã cấp cho một pack.
///
/// [`Grants::default`] là **chỉ dữ liệu tĩnh** — mặc định ít quyền nhất.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grants {
    granted: BTreeSet<Capability>,
}

impl Default for Grants {
    fn default() -> Grants {
        Grants {
            granted: BTreeSet::from([Capability::DefineContent]),
        }
    }
}

impl Grants {
    /// Từ danh sách khai trong manifest.
    ///
    /// `DefineContent` **luôn** có: mọi pack đều khai được dữ liệu tĩnh, và
    /// bắt khai nó sẽ khiến 90% manifest phải viết thêm một dòng vô nghĩa.
    pub fn from_declared(declared: &[Capability]) -> Grants {
        let mut g = Grants::default();
        g.granted.extend(declared.iter().copied());
        g
    }

    /// Có quyền này không.
    pub fn has(&self, c: Capability) -> bool {
        self.granted.contains(&c)
    }

    /// Danh sách quyền, thứ tự ổn định.
    pub fn list(&self) -> Vec<Capability> {
        self.granted.iter().copied().collect()
    }

    /// Những quyền **đổi được kết quả mô phỏng** — thứ phải hiện lên trước khi
    /// người dùng bấm cài.
    pub fn risky(&self) -> Vec<Capability> {
        self.granted
            .iter()
            .copied()
            .filter(|c| c.affects_simulation())
            .collect()
    }

    /// Kiểm mọi file của pack: file nào cần quyền pack chưa xin.
    ///
    /// Trả về **toàn bộ** vi phạm chứ không dừng ở cái đầu: người viết pack cần
    /// sửa một lần, không phải chạy lại năm lần để phát hiện năm lỗi.
    pub fn audit<'a>(
        &self,
        pack: &str,
        files: impl IntoIterator<Item = &'a String>,
    ) -> Vec<Violation> {
        files
            .into_iter()
            .filter_map(|path| {
                let can = ContentKind::from_path(path).requires();
                (!self.has(can)).then(|| Violation {
                    pack: pack.to_owned(),
                    path: path.clone(),
                    needs: can,
                })
            })
            .collect()
    }
}
