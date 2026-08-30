//! Repro bundle (`plan.md §P7.6`).
//!
//! Một thư mục **tự chứa**: chụp nó xong thì chạy lại được trên máy khác, sáu
//! tháng sau, mà không cần gì ngoài chính nó cộng đúng bản engine.
//!
//! Chữ "tự chứa" là phần khó và cũng là phần hay bị bỏ sót. Một bundle chỉ ghi
//! "worldseed: test:tiny_village" thì phụ thuộc vào việc worldseed đó *hôm nay*
//! còn giống *hôm chụp* — và nó sẽ không giống, vì content pack tiến hóa. Nên
//! bundle ghi kèm **content hash của toàn bộ pack set**, và [`ReproBundle::verify`]
//! từ chối chạy nếu chúng lệch. Thà báo "không tái hiện được vì pack đã đổi"
//! còn hơn chạy ra một kết quả khác rồi kết luận sai rằng bug đã tự hết.
//!
//! Bundle có thể chứa **nội dung nhạy cảm do người chơi tạo** (`§P10.6`), nên
//! nó nằm cục bộ theo mặc định; chia sẻ phải là hành động tường minh.

use mow_math::StateHash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Lỗi khi chụp hoặc chạy bundle.
#[derive(Debug, Error)]
pub enum ReproError {
    /// Lỗi đọc/ghi.
    #[error("lỗi tệp `{path}`: {source}")]
    Io {
        /// Đường dẫn.
        path: String,
        /// Nguyên nhân.
        #[source]
        source: std::io::Error,
    },
    /// Lỗi mã hóa/giải mã manifest.
    #[error("manifest hỏng: {0}")]
    Manifest(String),
    /// Môi trường hiện tại không khớp bundle.
    #[error("không tái hiện được: {0}")]
    Mismatch(String),
}

/// Kết quả.
pub type ReproResult<T> = Result<T, ReproError>;

/// Manifest của bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Định danh bundle, dùng làm tên thư mục.
    pub id: String,
    /// Git sha của engine lúc chụp.
    pub git_sha: String,
    /// Phiên bản engine.
    pub engine_version: String,
    /// Thời điểm chụp, ISO-8601.
    pub captured_at: String,
    /// Worldseed.
    pub worldseed: String,
    /// Hash của cấu hình có ảnh hưởng mô phỏng.
    pub config_hash: StateHash,
    /// Pack set: `id → (version, content hash)`.
    ///
    /// **Đây là trường làm bundle tự chứa.** Không có nó, một bundle chụp hôm
    /// nay sẽ chạy ra kết quả khác sau khi content pack tiến hóa, và ta sẽ kết
    /// luận sai rằng bug đã tự hết.
    pub packs: BTreeMap<String, (String, StateHash)>,
    /// Tick đầu cửa sổ tái hiện — điểm mà `snapshot.bin` mô tả.
    pub from_tick: u64,
    /// Tick xảy ra lỗi.
    pub to_tick: u64,
    /// State hash mong đợi tại `to_tick`.
    ///
    /// Chạy lại bundle mà ra hash này nghĩa là tái hiện thành công. Ra hash
    /// khác nghĩa là **môi trường đã đổi**, không phải bug đã hết.
    pub expected_hash: StateHash,
    /// Mô tả một dòng về lỗi.
    pub symptom: String,
}

/// Một bundle trên đĩa.
#[derive(Debug, Clone)]
pub struct ReproBundle {
    /// Thư mục gốc.
    pub root: PathBuf,
    /// Manifest.
    pub manifest: Manifest,
}

const TEN_MANIFEST: &str = "manifest.json";
const TEN_SNAPSHOT: &str = "snapshot.bin";
const TEN_EVENTS: &str = "events.log";

fn io<T>(path: &Path, r: std::io::Result<T>) -> ReproResult<T> {
    r.map_err(|source| ReproError::Io {
        path: path.display().to_string(),
        source,
    })
}

impl ReproBundle {
    /// Chụp một bundle.
    ///
    /// `snapshot` là state tại `from_tick`; `events` là nhật ký từ đó tới
    /// `to_tick`. Cả hai là byte đục với module này.
    pub fn capture(
        dir: impl AsRef<Path>,
        manifest: Manifest,
        snapshot: &[u8],
        events: &[u8],
    ) -> ReproResult<ReproBundle> {
        let root = dir.as_ref().join(&manifest.id);
        io(&root, std::fs::create_dir_all(&root))?;

        let mp = root.join(TEN_MANIFEST);
        let json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| ReproError::Manifest(e.to_string()))?;
        io(&mp, std::fs::write(&mp, json))?;

        let sp = root.join(TEN_SNAPSHOT);
        io(&sp, std::fs::write(&sp, snapshot))?;

        let ep = root.join(TEN_EVENTS);
        io(&ep, std::fs::write(&ep, events))?;

        Ok(ReproBundle { root, manifest })
    }

    /// Mở một bundle đã có.
    pub fn open(root: impl AsRef<Path>) -> ReproResult<ReproBundle> {
        let root = root.as_ref().to_path_buf();
        let mp = root.join(TEN_MANIFEST);
        let text = io(&mp, std::fs::read_to_string(&mp))?;
        let manifest: Manifest =
            serde_json::from_str(&text).map_err(|e| ReproError::Manifest(e.to_string()))?;
        Ok(ReproBundle { root, manifest })
    }

    /// Ảnh chụp.
    pub fn snapshot(&self) -> ReproResult<Vec<u8>> {
        let p = self.root.join(TEN_SNAPSHOT);
        io(&p, std::fs::read(&p))
    }

    /// Nhật ký sự kiện.
    pub fn events(&self) -> ReproResult<Vec<u8>> {
        let p = self.root.join(TEN_EVENTS);
        io(&p, std::fs::read(&p))
    }

    /// Kiểm tra môi trường hiện tại có tái hiện được bundle này không.
    ///
    /// Gọi **trước** khi chạy. Chạy rồi mới phát hiện pack đã đổi là quá muộn:
    /// lúc đó ta đã có một kết quả, và một kết quả sai luôn thuyết phục hơn là
    /// không có kết quả nào.
    pub fn verify(
        &self,
        packs_hien_tai: &BTreeMap<String, (String, StateHash)>,
        config_hash: StateHash,
    ) -> ReproResult<()> {
        let mut lech = Vec::new();

        if config_hash != self.manifest.config_hash {
            lech.push(format!(
                "cấu hình đã đổi (bundle {}, hiện tại {})",
                self.manifest.config_hash.short(),
                config_hash.short()
            ));
        }

        for (id, (ver, hash)) in &self.manifest.packs {
            match packs_hien_tai.get(id) {
                None => lech.push(format!("thiếu pack `{id}` (bundle cần bản {ver})")),
                Some((v2, h2)) if h2 != hash => lech.push(format!(
                    "pack `{id}` đã đổi: bundle {} ({}), hiện tại {} ({})",
                    ver,
                    hash.short(),
                    v2,
                    h2.short()
                )),
                Some(_) => {}
            }
        }

        // Pack **thừa** cũng là lệch: một pack mới nạp có thể đăng ký luật mới
        // và đổi kết quả, kể cả khi mọi pack cũ vẫn nguyên.
        for id in packs_hien_tai.keys() {
            if !self.manifest.packs.contains_key(id) {
                lech.push(format!(
                    "pack `{id}` được nạp thêm so với lúc chụp — nó có thể đăng ký luật mới"
                ));
            }
        }

        if lech.is_empty() {
            Ok(())
        } else {
            Err(ReproError::Mismatch(format!(
                "{}\nBug có thể vẫn còn; đây là môi trường đã đổi, không phải bằng chứng đã sửa.",
                lech.join("\n")
            )))
        }
    }

    /// Đối chiếu hash chạy lại được với hash mong đợi.
    pub fn check_result(&self, actual: StateHash) -> ReproResult<()> {
        if actual == self.manifest.expected_hash {
            Ok(())
        } else {
            Err(ReproError::Mismatch(format!(
                "chạy lại cho hash {} nhưng bundle ghi {}",
                actual.short(),
                self.manifest.expected_hash.short()
            )))
        }
    }
}
