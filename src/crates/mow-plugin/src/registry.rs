//! Sổ đăng ký: thứ tự nạp xác định, namespace bắt buộc, xung đột là lỗi.

use crate::manifest::{content_hash, PackManifest};
use mow_math::StateHash;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use thiserror::Error;

/// Lỗi của sổ đăng ký.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Manifest sai hình dạng.
    #[error("pack `{pack}` có manifest không hợp lệ:\n{}", .errors.join("\n"))]
    BadManifest {
        /// Pack.
        pack: String,
        /// Danh sách lỗi.
        errors: Vec<String>,
    },

    /// Hai pack cùng định danh.
    #[error("hai pack cùng id `{0}`")]
    DuplicatePack(String),

    /// Thiếu phụ thuộc.
    #[error("pack `{pack}` cần `{missing}` nhưng nó không được nạp")]
    MissingDependency {
        /// Pack đang xét.
        pack: String,
        /// Phụ thuộc thiếu.
        missing: String,
    },

    /// Phụ thuộc vòng.
    #[error("phụ thuộc vòng giữa các pack: {0:?}")]
    CyclicDependency(Vec<String>),

    /// Id không có namespace (`§22.29`).
    #[error("pack `{pack}` đăng ký id `{id}` không có namespace — phải là `{pack}.<tên>`")]
    MissingNamespace {
        /// Pack.
        pack: String,
        /// Id vi phạm.
        id: String,
    },

    /// Id có namespace của pack khác mà không khai báo ghi đè.
    #[error(
        "pack `{pack}` đăng ký `{id}` thuộc namespace của pack khác mà không khai báo \
         trong `overrides`"
    )]
    ForeignNamespace {
        /// Pack.
        pack: String,
        /// Id vi phạm.
        id: String,
    },

    /// Hai pack cùng định nghĩa một id mà không ai khai báo ghi đè.
    #[error(
        "xung đột: `{id}` được định nghĩa bởi cả `{first}` và `{second}`; \
         `{second}` phải khai báo nó trong `overrides` nếu đây là chủ đích"
    )]
    Conflict {
        /// Id bị tranh chấp.
        id: String,
        /// Pack định nghĩa trước.
        first: String,
        /// Pack định nghĩa sau.
        second: String,
    },

    /// Content hash của pack khác với hash ghi trong save (`§22.30`).
    #[error(
        "pack `{pack}` đã đổi: save ghi hash {expected}, trên đĩa là {actual}. \
         Từ chối nạp thay vì nạp một phần"
    )]
    HashMismatch {
        /// Pack.
        pack: String,
        /// Hash trong save.
        expected: String,
        /// Hash thật.
        actual: String,
    },

    /// Save cần một pack không có mặt.
    #[error("save cần pack `{0}` nhưng nó không được nạp")]
    PackAbsent(String),

    /// Lỗi đọc file.
    #[error("không đọc được `{path}`: {source}")]
    Io {
        /// Đường dẫn.
        path: String,
        /// Nguyên nhân.
        #[source]
        source: std::io::Error,
    },

    /// Lỗi phân tích YAML.
    #[error("không phân tích được `{path}`: {message}")]
    Parse {
        /// Đường dẫn.
        path: String,
        /// Thông báo.
        message: String,
    },
}

/// Kết quả.
pub type RegistryResult<T> = Result<T, RegistryError>;

/// Một pack đã nạp.
#[derive(Debug, Clone)]
struct LoadedPack {
    manifest: PackManifest,
    hash: StateHash,
    /// Các id nội dung mà pack này định nghĩa.
    defines: BTreeSet<String>,
}

/// Thứ tự nạp đã được quyết định.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOrder(pub Vec<String>);

/// Bộ pack ghi vào save (`§22.30`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSet {
    /// `(id, version, content hash)` theo thứ tự nạp.
    pub entries: Vec<(String, String, StateHash)>,
}

/// Sổ đăng ký.
#[derive(Debug, Default)]
pub struct Registry {
    packs: BTreeMap<String, LoadedPack>,
    /// id nội dung → pack chủ sở hữu hiện tại.
    owners: BTreeMap<String, String>,
    order: Vec<String>,
}

impl Registry {
    /// Sổ rỗng.
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Nạp một pack từ thư mục.
    ///
    /// Đọc `pack.yaml`, băm toàn bộ nội dung, đăng ký. `content/core` đi qua
    /// **đúng hàm này**, không có đường tắt.
    pub fn add_from_dir(&mut self, dir: impl AsRef<Path>) -> RegistryResult<()> {
        let dir = dir.as_ref();
        let mp = dir.join("pack.yaml");
        let text = std::fs::read_to_string(&mp).map_err(|e| RegistryError::Io {
            path: mp.display().to_string(),
            source: e,
        })?;
        let manifest = PackManifest::from_yaml(&text).map_err(|e| RegistryError::Parse {
            path: mp.display().to_string(),
            message: e.to_string(),
        })?;

        let files = doc_cay_thu_muc(dir)?;
        self.add(manifest, &files)
    }

    /// Nạp một pack từ manifest và nội dung đã có sẵn trong bộ nhớ.
    pub fn add(
        &mut self,
        manifest: PackManifest,
        files: &BTreeMap<String, Vec<u8>>,
    ) -> RegistryResult<()> {
        manifest
            .validate()
            .map_err(|errors| RegistryError::BadManifest {
                pack: manifest.id.clone(),
                errors,
            })?;

        if self.packs.contains_key(&manifest.id) {
            return Err(RegistryError::DuplicatePack(manifest.id));
        }

        let hash = content_hash(&manifest, files);
        let id = manifest.id.clone();
        self.packs.insert(
            id.clone(),
            LoadedPack {
                manifest,
                hash,
                defines: BTreeSet::new(),
            },
        );
        Ok(())
    }

    /// Khai báo một id nội dung do pack định nghĩa.
    ///
    /// Đây là chỗ `§22.29` được thi hành, và cả ba nhánh đều là lỗi thật:
    /// thiếu namespace, mượn namespace của người khác, và xung đột không khai báo.
    pub fn define(&mut self, pack: &str, id: &str) -> RegistryResult<()> {
        let Some(p) = self.packs.get(pack) else {
            return Err(RegistryError::PackAbsent(pack.to_owned()));
        };

        let Some(ns) = id.split('.').next().filter(|n| !n.is_empty()) else {
            return Err(RegistryError::MissingNamespace {
                pack: pack.to_owned(),
                id: id.to_owned(),
            });
        };
        if !id.contains('.') {
            return Err(RegistryError::MissingNamespace {
                pack: pack.to_owned(),
                id: id.to_owned(),
            });
        }

        let la_ghi_de = p.manifest.overrides.iter().any(|o| o == id);
        if ns != pack && !la_ghi_de {
            return Err(RegistryError::ForeignNamespace {
                pack: pack.to_owned(),
                id: id.to_owned(),
            });
        }

        if let Some(chu_cu) = self.owners.get(id) {
            if chu_cu != pack && !la_ghi_de {
                return Err(RegistryError::Conflict {
                    id: id.to_owned(),
                    first: chu_cu.clone(),
                    second: pack.to_owned(),
                });
            }
        }

        self.owners.insert(id.to_owned(), pack.to_owned());
        self.packs
            .get_mut(pack)
            .expect("vừa kiểm tra ở trên")
            .defines
            .insert(id.to_owned());
        Ok(())
    }

    /// Ai đang sở hữu một id nội dung.
    pub fn owner_of(&self, id: &str) -> Option<&str> {
        self.owners.get(id).map(String::as_str)
    }

    /// Quyết định thứ tự nạp: sắp xếp tô-pô, phá hòa bằng id.
    ///
    /// Phá hòa bằng id chứ không phải bằng thứ tự thêm vào. Hai pack độc lập
    /// nhau thì thứ tự giữa chúng không ảnh hưởng ngữ nghĩa, nhưng nó **ảnh
    /// hưởng content hash của bộ pack**, và hash đó nằm trong save. Nếu thứ tự
    /// phụ thuộc vào thứ tự thư mục trên đĩa, cùng một tập pack sẽ cho hash
    /// khác nhau trên Windows và Linux, và save sẽ không chuyển máy được.
    pub fn resolve_order(&mut self) -> RegistryResult<LoadOrder> {
        for (id, p) in &self.packs {
            for r in &p.manifest.requires {
                if !self.packs.contains_key(&r.id) {
                    return Err(RegistryError::MissingDependency {
                        pack: id.clone(),
                        missing: r.id.clone(),
                    });
                }
            }
        }

        let mut xong: Vec<String> = Vec::new();
        let mut da_xong: BTreeSet<String> = BTreeSet::new();
        let mut dang_xet: BTreeSet<String> = BTreeSet::new();

        // Duyệt theo `BTreeMap`, tức theo id tăng dần — đây là vế phá hòa.
        let ds: Vec<String> = self.packs.keys().cloned().collect();
        for id in ds {
            self.tham(&id, &mut xong, &mut da_xong, &mut dang_xet)?;
        }

        self.order.clone_from(&xong);
        Ok(LoadOrder(xong))
    }

    fn tham(
        &self,
        id: &str,
        xong: &mut Vec<String>,
        da_xong: &mut BTreeSet<String>,
        dang_xet: &mut BTreeSet<String>,
    ) -> RegistryResult<()> {
        if da_xong.contains(id) {
            return Ok(());
        }
        if !dang_xet.insert(id.to_owned()) {
            let mut v: Vec<String> = dang_xet.iter().cloned().collect();
            v.sort();
            return Err(RegistryError::CyclicDependency(v));
        }
        let p = self.packs.get(id).expect("đã kiểm tra tồn tại");
        let mut deps: Vec<&str> = p.manifest.requires.iter().map(|r| r.id.as_str()).collect();
        deps.sort_unstable();
        for d in deps {
            self.tham(d, xong, da_xong, dang_xet)?;
        }
        dang_xet.remove(id);
        da_xong.insert(id.to_owned());
        xong.push(id.to_owned());
        Ok(())
    }

    /// Bộ pack để ghi vào save.
    pub fn pack_set(&self) -> PackSet {
        let ds = if self.order.is_empty() {
            self.packs.keys().cloned().collect::<Vec<_>>()
        } else {
            self.order.clone()
        };
        PackSet {
            entries: ds
                .into_iter()
                .filter_map(|id| {
                    self.packs
                        .get(&id)
                        .map(|p| (id, p.manifest.version.clone(), p.hash))
                })
                .collect(),
        }
    }

    /// Đối chiếu với bộ pack ghi trong save (`§22.30`).
    ///
    /// Lệch thì **từ chối load**, không load một phần. Load một phần nghĩa là
    /// một số định nghĩa có còn một số không, và thế giới sẽ tham chiếu tới
    /// những thứ không tồn tại — hỏng theo cách rải rác và khó truy hơn nhiều
    /// so với một lỗi rõ ràng lúc mở file.
    pub fn verify_against(&self, saved: &PackSet) -> RegistryResult<()> {
        for (id, _ver, hash) in &saved.entries {
            let Some(p) = self.packs.get(id) else {
                return Err(RegistryError::PackAbsent(id.clone()));
            };
            if p.hash != *hash {
                return Err(RegistryError::HashMismatch {
                    pack: id.clone(),
                    expected: hash.short(),
                    actual: p.hash.short(),
                });
            }
        }
        Ok(())
    }

    /// Số pack đã nạp.
    pub fn len(&self) -> usize {
        self.packs.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    /// Manifest của một pack.
    pub fn manifest(&self, id: &str) -> Option<&PackManifest> {
        self.packs.get(id).map(|p| &p.manifest)
    }

    /// Content hash của một pack.
    pub fn hash_of(&self, id: &str) -> Option<StateHash> {
        self.packs.get(id).map(|p| p.hash)
    }
}

/// Đọc toàn bộ cây thư mục thành ánh xạ đường dẫn tương đối → nội dung.
fn doc_cay_thu_muc(root: &Path) -> RegistryResult<BTreeMap<String, Vec<u8>>> {
    let mut ra = BTreeMap::new();
    let mut hang_doi = vec![root.to_path_buf()];
    while let Some(d) = hang_doi.pop() {
        let entries = std::fs::read_dir(&d).map_err(|e| RegistryError::Io {
            path: d.display().to_string(),
            source: e,
        })?;
        for e in entries {
            let e = e.map_err(|e| RegistryError::Io {
                path: d.display().to_string(),
                source: e,
            })?;
            let p = e.path();
            if p.is_dir() {
                hang_doi.push(p);
            } else {
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = std::fs::read(&p).map_err(|err| RegistryError::Io {
                    path: p.display().to_string(),
                    source: err,
                })?;
                ra.insert(rel, bytes);
            }
        }
    }
    Ok(ra)
}
