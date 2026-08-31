//! Worldseed và lockfile (`idea.md §7.6`, `§7.6.6`).
//!
//! Worldseed là *ý định*: "một thung lũng ôn đới, ba làng, trình độ đồ sắt".
//! Lockfile là *kết quả đã chốt*: đúng những pack nào, phiên bản nào, content
//! hash nào, và seed số học nào đã được rút ra.
//!
//! Tách hai thứ này vì chúng có tuổi thọ khác nhau. Worldseed là thứ người ta
//! chia sẻ, sửa, fork. Lockfile là thứ khiến một world **mở lại được** — nó ghi
//! rằng thế giới này đã sinh ra dưới đúng những điều kiện nào, và nếu điều kiện
//! đó không còn thì phải nói ra chứ không phải lặng lẽ sinh ra một thế giới
//! khác mang cùng tên.

use mow_math::{CanonicalHash, StateHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Một worldseed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Worldseed {
    /// Định danh, ví dụ `gaia:temperate_valley`.
    pub id: String,
    /// Phiên bản.
    #[serde(default = "mot")]
    pub version: u32,
    /// Mô tả cho người đọc.
    #[serde(default)]
    pub description: String,

    /// Profile sinh thế giới.
    pub generation_profile: String,
    /// Seed số học. `None` nghĩa là rút từ `id` — để cùng một worldseed luôn
    /// cho cùng một thế giới trừ khi người dùng cố ý đổi.
    #[serde(default)]
    pub seed: Option<u64>,

    /// Pack cần nạp, theo thứ tự.
    #[serde(default = "chi_core")]
    pub packs: Vec<String>,

    /// Kịch bản khởi tạo: danh sách command chạy tại tick 0.
    ///
    /// `§22.28` cấm mọi đường ghi thẳng state vào save. Genesis **không** phải
    /// ngoại lệ: nó là một chuỗi command đi qua đúng transaction handler như
    /// mọi hành động khác. Nhờ vậy một world mới tạo có nhật ký sự kiện đầy đủ
    /// từ tick 0, và chuỗi nhân quả truy được về tận lúc khai sinh.
    #[serde(default)]
    pub genesis: Vec<GenesisStep>,

    /// Thực thể có tên, cho kịch bản test tham chiếu thẳng (`§P7.3` quy tắc 4).
    #[serde(default)]
    pub named_entities: BTreeMap<String, String>,
}

fn mot() -> u32 {
    1
}
fn chi_core() -> Vec<String> {
    vec!["core".to_owned()]
}

/// Một bước genesis: một command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenesisStep {
    /// Loại command.
    pub command: String,
    /// Tham số.
    #[serde(default)]
    pub args: BTreeMap<String, serde_yaml::Value>,
    /// Tên gán cho thực thể mà bước này tạo ra, nếu có.
    ///
    /// Cho phép các bước sau tham chiếu tới nó mà không cần biết id — id sinh
    /// ra từ genesis và sẽ đổi khi worldseed đổi.
    #[serde(default)]
    pub name: Option<String>,
}

impl Worldseed {
    /// Đọc từ YAML.
    pub fn from_yaml(s: &str) -> Result<Worldseed, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    /// Seed số học đã giải.
    ///
    /// Rút từ `id` khi không khai báo tường minh, nên hai người tải cùng một
    /// worldseed nhận cùng một thế giới.
    pub fn resolved_seed(&self) -> u64 {
        if let Some(s) = self.seed {
            return s;
        }
        let mut h = StateHasher::with_domain("mow.worldseed.v1");
        h.write_str(&self.id);
        h.write_u64(u64::from(self.version));
        u64::from_le_bytes(h.finish().0[..8].try_into().expect("32 byte đủ 8"))
    }

    /// Kiểm tra hình dạng.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut loi = Vec::new();
        if self.id.is_empty() {
            loi.push("`id` không được rỗng".to_owned());
        }
        if self.generation_profile.is_empty() {
            loi.push("`generation_profile` không được rỗng".to_owned());
        }
        if self.packs.is_empty() {
            loi.push("`packs` phải có ít nhất một pack".to_owned());
        }

        let mut da_dat: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for (i, g) in self.genesis.iter().enumerate() {
            if g.command.is_empty() {
                loi.push(format!("`genesis[{i}].command` không được rỗng"));
            }
            if let Some(n) = &g.name {
                if !da_dat.insert(n.as_str()) {
                    loi.push(format!("`genesis[{i}].name` = `{n}` bị trùng"));
                }
            }
            // Tham chiếu tới tên phải trỏ tới thứ đã được tạo **trước đó**.
            // Không kiểm điều này thì một worldseed sẽ nạp được nhưng đổ vỡ ở
            // giữa chừng genesis, để lại một thế giới hỏng một nửa.
            for (k, v) in &g.args {
                if let Some(s) = v.as_str() {
                    if let Some(ten) = s.strip_prefix('$') {
                        if !da_dat.contains(ten) {
                            loi.push(format!(
                                "`genesis[{i}].args.{k}` trỏ tới `${ten}` nhưng chưa có bước nào \
                                 đặt tên đó trước đây"
                            ));
                        }
                    }
                }
            }
        }

        if loi.is_empty() {
            Ok(())
        } else {
            Err(loi)
        }
    }
}

impl CanonicalHash for Worldseed {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_u64(u64::from(self.version));
        h.write_str(&self.generation_profile);
        h.write_u64(self.resolved_seed());
        h.write_seq(self.packs.iter(), |hh, p| {
            hh.write_str(p);
        });
        h.write_seq(self.genesis.iter(), |hh, g| {
            hh.write_str(&g.command);
            hh.write_option(g.name.as_deref(), |h3, n| {
                h3.write_str(n);
            });
            // `BTreeMap` nên thứ tự khóa xác định.
            hh.write_seq(g.args.iter(), |h3, (k, v)| {
                h3.write_str(k);
                h3.write_str(&serde_yaml::to_string(v).unwrap_or_default());
            });
        });
    }
}

/// Lockfile: mọi thứ đã giải, đủ để mở lại đúng thế giới đó (`§7.6.6`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lockfile {
    /// Worldseed nào.
    pub worldseed_id: String,
    /// Phiên bản worldseed.
    pub worldseed_version: u32,
    /// Hash của chính worldseed, để phát hiện nó bị sửa sau khi world đã tạo.
    pub worldseed_hash: StateHash,
    /// Seed số học đã giải.
    pub resolved_seed: u64,
    /// Profile và phiên bản của nó.
    pub generation_profile: String,
    /// Hash ảnh chụp profile.
    pub profile_hash: StateHash,
    /// Pack set: `id → (version, content hash)`, theo thứ tự nạp.
    pub packs: Vec<(String, String, StateHash)>,
}

impl Lockfile {
    /// So với môi trường hiện tại.
    ///
    /// Lệch thì **từ chối load**, không load một phần (`§22.30`). Một world nạp
    /// một nửa sẽ tham chiếu tới những định nghĩa không tồn tại, và nó hỏng rải
    /// rác — khó truy hơn nhiều so với một lỗi rõ ràng lúc mở file.
    pub fn verify(
        &self,
        worldseed: &Worldseed,
        profile_hash: StateHash,
        packs: &[(String, String, StateHash)],
    ) -> Result<(), Vec<String>> {
        let mut lech = Vec::new();

        let h = {
            let mut hh = StateHasher::with_domain("mow.worldseed.hash.v1");
            worldseed.canonical_hash(&mut hh);
            hh.finish()
        };
        if h != self.worldseed_hash {
            lech.push(format!(
                "worldseed `{}` đã đổi kể từ khi world được tạo (lock {}, hiện tại {})",
                self.worldseed_id,
                self.worldseed_hash.short(),
                h.short()
            ));
        }
        if profile_hash != self.profile_hash {
            lech.push(format!(
                "generation profile `{}` đã đổi (lock {}, hiện tại {})",
                self.generation_profile,
                self.profile_hash.short(),
                profile_hash.short()
            ));
        }

        let hien: BTreeMap<&str, (&str, StateHash)> = packs
            .iter()
            .map(|(a, b, c)| (a.as_str(), (b.as_str(), *c)))
            .collect();
        for (id, ver, hash) in &self.packs {
            match hien.get(id.as_str()) {
                None => lech.push(format!("thiếu pack `{id}` (lock cần bản {ver})")),
                Some((_, h2)) if h2 != hash => lech.push(format!(
                    "pack `{id}` đã đổi (lock {}, hiện tại {})",
                    hash.short(),
                    h2.short()
                )),
                Some(_) => {}
            }
        }
        for (id, _, _) in packs {
            if !self.packs.iter().any(|(p, _, _)| p == id) {
                lech.push(format!(
                    "pack `{id}` được nạp thêm so với lockfile — nó có thể đăng ký luật mới"
                ));
            }
        }

        if lech.is_empty() {
            Ok(())
        } else {
            Err(lech)
        }
    }
}
