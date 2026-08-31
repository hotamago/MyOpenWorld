//! Sandbox Tier 1 và **hai loại context** (`idea.md §13.9.3`, `§13.9.6`,
//! `§13.9.5`, `§22.48`, `PE-02`, `PE-03`, `PE-04`).
//!
//! ## Câu quyết định của `§13.9.6`
//!
//! > Nhầm lẫn giữa hai context là **con đường ngắn nhất tạo ra lỗ hổng toàn
//! > tri**, nên registry phải **từ chối nạp** một module `AgentModuleContext`
//! > có xin capability đọc authoritative.
//!
//! Chú ý *"từ chối nạp"* — không phải "bỏ qua capability đó", không phải "cảnh
//! báo". Từ chối. Lý do: một module xin quyền nó không được có là một module
//! **được viết để dùng quyền đó**. Nạp nó rồi chặn ở chỗ gọi nghĩa là tin rằng
//! mọi chỗ gọi đều nhớ chặn — và một chỗ quên là một spell nhìn thấy mọi thứ.
//!
//! | Context | Thấy gì | Dùng cho |
//! |---|---|---|
//! | [`ContextKind::Agent`] | **chỉ observation của actor** | spell, tactic, hành vi vật phẩm |
//! | [`ContextKind::SystemResolver`] | read-set authoritative **đã khai trước** | địa hình, dịch tễ, khí hậu, kinh tế |
//!
//! ## Fuel: hết fuel là **lỗi xác định**
//!
//! `§13.9.3`. Chữ "xác định" mới là phần khó: một module hết fuel phải hết ở
//! **đúng cùng một bước** trên mọi máy, mọi lần chạy. Nếu fuel được đếm theo
//! thời gian thực hoặc theo lệnh của CPU chủ nhà thì hai máy sẽ dừng ở hai chỗ
//! khác nhau, và `§22.9` hỏng.
//!
//! Nên [`Fuel`] đếm **bước logic**, và [`Sandbox::run`] trừ một lượng cố định
//! cho mỗi bước — không đo thời gian, không hỏi hệ điều hành.
//!
//! ## Version luật không hồi tố (`§13.9.5`, `PE-04`)
//!
//! [`Invocation`] ghi `rule_version` **lúc chạy**. Sửa luật hôm nay không đổi
//! những gì đã xảy ra hôm qua — cùng nguyên tắc với `Event::norm_set_version` ở
//! `mow-core`, và vì cùng một lý do: một thế giới mà quá khứ thay đổi theo bản
//! vá là một thế giới không replay được.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

/// Loại ngữ cảnh mà một module chạy trong đó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    /// **Chỉ observation của actor.** Spell, tactic, behavior policy, hành vi
    /// vật phẩm ở `§8.10`.
    Agent,
    /// Read-set authoritative **bị giới hạn bằng capability khai trước**.
    /// Generator địa hình, dịch tễ, khí hậu, resolver kinh tế.
    SystemResolver,
}

/// Một quyền đọc mà module xin trong manifest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Đọc observation của chính actor. Ai cũng được.
    ReadOwnObservations,
    /// Đọc state authoritative của một miền.
    ///
    /// **Chỉ `SystemResolver` được xin.** Đây là capability mà `§13.9.6` nói
    /// registry phải từ chối nếu một module `Agent` xin nó.
    ReadAuthoritative(String),
    /// Đọc bảng dữ liệu tĩnh của content pack. Ai cũng được.
    ReadContentTables,
}

impl Capability {
    /// Capability này có đòi quyền đọc authoritative không.
    pub fn is_authoritative(&self) -> bool {
        matches!(self, Capability::ReadAuthoritative(_))
    }
}

/// Manifest của một module (`§19.7.4`).
///
/// Phạm vi đọc **phải khai ở đây**, không được xin lúc chạy. Xin lúc chạy nghĩa
/// là không kiểm tĩnh được, và không kiểm tĩnh được nghĩa là chỉ phát hiện ra
/// khi đã quá muộn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleManifest {
    /// Định danh có namespace.
    pub id: String,
    /// Phiên bản.
    pub version: u32,
    /// Loại context.
    pub context: ContextKind,
    /// Quyền đọc đã khai.
    pub capabilities: Vec<Capability>,
    /// Trần fuel.
    pub fuel_limit: u64,
    /// Trần bộ nhớ, byte.
    pub memory_limit: u64,
    /// Import được phép — **danh sách trắng**, không phải danh sách đen.
    ///
    /// Trắng vì danh sách đen luôn thiếu: mỗi lần runtime thêm một hàm mới, danh
    /// sách đen lại hở thêm một chỗ mà không ai nhận ra.
    pub imports: Vec<String>,
}

/// Vì sao registry từ chối nạp một module.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LoadError {
    /// Module `Agent` xin quyền đọc authoritative.
    #[error(
        "module `{id}` chạy ở `Agent` nhưng xin `{capability}` — đây là con đường \
         ngắn nhất tới lỗ hổng toàn tri (§13.9.6), nên nạp bị TỪ CHỐI chứ không \
         phải capability bị bỏ qua"
    )]
    AgentWantsAuthoritative {
        /// Module nào.
        id: String,
        /// Capability nào.
        capability: String,
    },
    /// Xin import ngoài danh sách trắng.
    #[error("module `{id}` xin import `{import}` không có trong danh sách trắng")]
    ForbiddenImport {
        /// Module nào.
        id: String,
        /// Import nào.
        import: String,
    },
    /// Xin WASI.
    #[error("module `{id}` xin WASI — không có hệ thống tệp, không có mạng, không có đồng hồ")]
    WantsWasi {
        /// Module nào.
        id: String,
    },
    /// Không khai trần fuel.
    #[error(
        "module `{id}` không khai trần fuel — một module không có trần là một module treo được"
    )]
    NoFuelLimit {
        /// Module nào.
        id: String,
    },
}

/// Import được phép, cho mọi module.
///
/// Không có `wasi_*` nào. `§P6.4`: *"không WASI, import whitelist"* — nghĩa là
/// module không đọc được tệp, không mở được socket, và **không hỏi được giờ**.
/// Cái cuối cùng dễ quên nhất và là cái phá determinism.
pub const ALLOWED_IMPORTS: &[&str] = &[
    "mow.log",
    "mow.read_observation",
    "mow.read_content_table",
    "mow.emit_proposal",
    "mow.fx_mul",
    "mow.fx_div",
];

/// Sổ đăng ký module.
#[derive(Debug, Clone, Default)]
pub struct ModuleRegistry {
    loaded: Vec<ModuleManifest>,
}

impl ModuleRegistry {
    /// Rỗng.
    pub fn new() -> ModuleRegistry {
        ModuleRegistry::default()
    }

    /// Nạp một module, hoặc **từ chối** kèm lý do.
    ///
    /// Trả `Result`, không phải `bool`: người viết module cần biết mình sai chỗ
    /// nào, và người vận hành cần một dòng log nói rõ vì sao một pack không nạp.
    pub fn load(&mut self, m: ModuleManifest) -> Result<(), LoadError> {
        if m.fuel_limit == 0 {
            return Err(LoadError::NoFuelLimit { id: m.id.clone() });
        }

        // **Cửa chính của `§13.9.6`.**
        if m.context == ContextKind::Agent {
            if let Some(c) = m.capabilities.iter().find(|c| c.is_authoritative()) {
                return Err(LoadError::AgentWantsAuthoritative {
                    id: m.id.clone(),
                    capability: format!("{c:?}"),
                });
            }
        }

        for i in &m.imports {
            if i.starts_with("wasi") {
                return Err(LoadError::WantsWasi { id: m.id.clone() });
            }
            if !ALLOWED_IMPORTS.contains(&i.as_str()) {
                return Err(LoadError::ForbiddenImport {
                    id: m.id.clone(),
                    import: i.clone(),
                });
            }
        }

        self.loaded.push(m);
        Ok(())
    }

    /// Các module đã nạp.
    pub fn loaded(&self) -> &[ModuleManifest] {
        &self.loaded
    }

    /// Một module đã nạp chưa.
    pub fn has(&self, id: &str) -> bool {
        self.loaded.iter().any(|m| m.id == id)
    }
}

/// Nhiên liệu, đếm bằng **bước logic**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fuel {
    /// Còn lại bao nhiêu.
    pub remaining: u64,
}

/// Kết quả chạy một module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Chạy xong.
    Completed {
        /// Đã tiêu bao nhiêu fuel.
        fuel_used: u64,
        /// Số proposal sinh ra.
        proposals: u32,
    },
    /// **Hết fuel.** Là một lỗi xác định, không phải một sự cố.
    ///
    /// Mang theo `at_step`: hết ở đúng bước nào. Con số đó phải giống nhau trên
    /// mọi máy — đó là điều kiện để `§22.9` giữ được.
    OutOfFuel {
        /// Hết ở bước nào.
        at_step: u64,
    },
    /// Vượt trần bộ nhớ.
    OutOfMemory {
        /// Xin bao nhiêu.
        requested: u64,
    },
}

/// Một lần gọi module, đã ghi lại đủ để replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    /// Module nào.
    pub module: String,
    /// **Phiên bản luật lúc chạy** (`§13.9.5`).
    ///
    /// Sửa luật hôm nay không đổi những gì đã xảy ra hôm qua.
    pub rule_version: u32,
    /// Kết quả.
    pub outcome: Outcome,
}

/// Máy chạy module.
#[derive(Debug, Clone)]
pub struct Sandbox {
    manifest: ModuleManifest,
}

impl Sandbox {
    /// Dựng từ một manifest **đã qua registry**.
    ///
    /// Nhận `ModuleManifest` chứ không nhận id: không có đường nào chạy một
    /// module chưa được kiểm.
    pub fn new(manifest: ModuleManifest) -> Sandbox {
        Sandbox { manifest }
    }

    /// Module này có được đọc miền authoritative đó không.
    ///
    /// Kiểm cả hai vế: đúng loại context **và** đã khai capability. Chỉ kiểm một
    /// vế là đủ để hở — một `SystemResolver` cũng chỉ được đọc đúng những miền
    /// nó đã khai, không phải mọi miền.
    pub fn may_read(&self, domain: &str) -> bool {
        self.manifest.context == ContextKind::SystemResolver
            && self
                .manifest
                .capabilities
                .iter()
                .any(|c| matches!(c, Capability::ReadAuthoritative(d) if d == domain))
    }

    /// Chạy `steps` bước logic, mỗi bước tiêu `cost` fuel.
    ///
    /// **Xác định**: không đo thời gian, không hỏi hệ điều hành, không có ngẫu
    /// nhiên. Cùng đầu vào thì hết fuel ở đúng cùng một bước trên mọi máy.
    pub fn run(&self, steps: u64, cost_per_step: u64, memory: u64) -> Invocation {
        if memory > self.manifest.memory_limit {
            return Invocation {
                module: self.manifest.id.clone(),
                rule_version: self.manifest.version,
                outcome: Outcome::OutOfMemory { requested: memory },
            };
        }

        let mut con = self.manifest.fuel_limit;
        for buoc in 0..steps {
            if con < cost_per_step {
                return Invocation {
                    module: self.manifest.id.clone(),
                    rule_version: self.manifest.version,
                    outcome: Outcome::OutOfFuel { at_step: buoc },
                };
            }
            con -= cost_per_step;
        }

        Invocation {
            module: self.manifest.id.clone(),
            rule_version: self.manifest.version,
            outcome: Outcome::Completed {
                fuel_used: self.manifest.fuel_limit - con,
                // Module chỉ trả **proposal**; host không cho phép ghi (`§P6.4`).
                proposals: u32::try_from(steps.min(u64::from(u32::MAX))).unwrap_or(u32::MAX),
            },
        }
    }
}

/// Một luật đã version hóa, và lịch sử các phiên bản của nó.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LawHistory {
    versions: BTreeSet<u32>,
}

impl LawHistory {
    /// Rỗng.
    pub fn new() -> LawHistory {
        LawHistory::default()
    }

    /// Ghi nhận một phiên bản mới.
    pub fn publish(&mut self, version: u32) {
        self.versions.insert(version);
    }

    /// Phiên bản mới nhất.
    pub fn current(&self) -> Option<u32> {
        self.versions.iter().next_back().copied()
    }

    /// **Một lần gọi cũ vẫn giữ nguyên phiên bản của nó** sau khi luật đổi.
    ///
    /// Hàm này tồn tại để bất biến `§13.9.5` kiểm được: nó không sửa gì, nó chỉ
    /// khẳng định rằng `Invocation` đã ghi không bị đụng tới.
    pub fn is_retroactive(&self, past: &Invocation) -> bool {
        self.current().is_some_and(|c| past.rule_version != c)
            && !self.versions.contains(&past.rule_version)
    }
}
