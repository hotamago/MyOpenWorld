//! Bộ chạy kịch bản.

use crate::model::{Assertion, Binding, Scenario, Step};
use crate::predicate::{Bindings, Predicate, Val};
use mow_core::invariant::Cost;
use mow_core::{Command, EntityId, InvariantRunner, Sim, Value};
use mow_math::StateHash;
use std::collections::BTreeMap;
use thiserror::Error;

/// Lỗi khi chạy kịch bản.
#[derive(Debug, Error)]
pub enum RunError {
    /// Kịch bản sai cấu trúc.
    #[error("kịch bản `{name}` không hợp lệ:\n{}", .errors.iter().map(|e| format!("  {e}")).collect::<Vec<_>>().join("\n"))]
    Invalid {
        /// Tên kịch bản.
        name: String,
        /// Danh sách lỗi.
        errors: Vec<String>,
    },

    /// Không dựng được thế giới.
    #[error("không dựng được thế giới từ worldseed `{0}`")]
    NoWorld(String),

    /// Bộ chọn không khớp ai (`§P7.3`, quy tắc 2).
    #[error(
        "alias `{alias}` không khớp đối tượng nào (kind={kind}, tag={tag:?}). \
         Không khớp là LỖI, không phải bỏ qua — một kịch bản xanh vì bộ chọn \
         không tìm thấy ai là loại kết quả sai tệ nhất"
    )]
    NoMatch {
        /// Alias.
        alias: String,
        /// Loại đang tìm.
        kind: String,
        /// Tag đang lọc.
        tag: Option<String>,
    },

    /// Bước không có handler.
    #[error("bước `{step}` không có handler. Các loại command đã đăng ký: {available}")]
    UnknownStep {
        /// Tên bước.
        step: String,
        /// Danh sách handler đã có.
        available: String,
    },

    /// Bước thất bại.
    #[error("bước `{step}` thất bại: {source}")]
    StepFailed {
        /// Tên bước.
        step: String,
        /// Nguyên nhân.
        #[source]
        source: mow_core::Failure,
    },

    /// Vị từ sai cú pháp.
    #[error("{0}")]
    BadPredicate(#[from] crate::predicate::ParseError),

    /// `run_until` chạy hết hạn mà vị từ chưa bao giờ đúng.
    #[error("`run_until` chạy hết {ticks} tick mà vị từ `{predicate}` chưa bao giờ đúng")]
    Timeout {
        /// Vị từ.
        predicate: String,
        /// Số tick đã chạy.
        ticks: u64,
    },
}

/// Kết quả một khẳng định.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertResult {
    /// Tên khẳng định.
    pub name: String,
    /// Đạt hay không.
    pub passed: bool,
    /// Chi tiết khi trượt.
    pub detail: String,
}

/// Báo cáo một lần chạy.
#[derive(Debug, Clone)]
pub struct Report {
    /// Tên kịch bản.
    pub scenario: String,
    /// Alias nào trỏ tới id nào.
    ///
    /// `§P7.3` quy tắc 3: ghi lại để khi kịch bản đỏ thì đọc log là biết nó
    /// đang nói về ai. Không có phần này, một kịch bản đỏ chỉ nói "Aren không
    /// trộm bánh" mà không cho biết Aren là thực thể nào trong hàng trăm.
    pub bindings: BTreeMap<String, EntityId>,
    /// Kết quả từng khẳng định.
    pub assertions: Vec<AssertResult>,
    /// Số tick đã chạy.
    pub ticks: u64,
    /// Hash state cuối.
    pub state_hash: StateHash,
}

impl Report {
    /// Toàn bộ khẳng định đều đạt.
    pub fn passed(&self) -> bool {
        self.assertions.iter().all(|a| a.passed)
    }
}

impl core::fmt::Display for Report {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "{} — {} ({} tick, hash {})",
            self.scenario,
            if self.passed() { "ĐẠT" } else { "TRƯỢT" },
            self.ticks,
            self.state_hash.short()
        )?;
        if !self.bindings.is_empty() {
            writeln!(f, "  ràng buộc:")?;
            for (a, id) in &self.bindings {
                writeln!(f, "    {a} → {id}")?;
            }
        }
        for a in &self.assertions {
            if a.passed {
                writeln!(f, "    ✓ {}", a.name)?;
            } else {
                writeln!(f, "    ✗ {} — {}", a.name, a.detail)?;
            }
        }
        Ok(())
    }
}

/// Dựng thế giới từ một worldseed.
///
/// Tách thành trait vì `PA-04` sẽ thay hiện thực bằng worldgen thật, còn Giai
/// đoạn 0 chỉ cần vài thế giới nhỏ viết tay để harness tự kiểm được chính nó.
pub trait WorldFactory {
    /// Dựng.
    fn build(&self, worldseed: &str, overrides: &BTreeMap<String, String>) -> Option<Sim>;
}

/// Chạy một kịch bản.
pub fn run(sc: &Scenario, factory: &dyn WorldFactory) -> Result<Report, RunError> {
    sc.validate().map_err(|errors| RunError::Invalid {
        name: sc.scenario.clone(),
        errors,
    })?;

    let mut sim = factory
        .build(&sc.worldseed, &sc.seed_overrides)
        .ok_or_else(|| RunError::NoWorld(sc.worldseed.clone()))?;

    // ── bind: một lần, sau genesis ──────────────────────────────────────────
    let aliases = rang_buoc(&sim, &sc.bind)?;
    let alias_ids: BTreeMap<String, u64> =
        aliases.iter().map(|(k, v)| (k.clone(), v.get())).collect();

    let tick_dau = sim.clock().local().0;

    // ── given ───────────────────────────────────────────────────────────────
    for s in &sc.given {
        chay_buoc(&mut sim, s, &alias_ids)?;
    }

    // ── when ────────────────────────────────────────────────────────────────
    for s in &sc.when {
        match s.name() {
            Some("run_until") => chay_den_khi(&mut sim, s, &alias_ids)?,
            Some("step") => {
                let n = s.arg_int("ticks").unwrap_or(1).max(0) as u64;
                tien(&mut sim, n)?;
            }
            _ => chay_buoc(&mut sim, s, &alias_ids)?,
        }
    }

    // ── then ────────────────────────────────────────────────────────────────
    let assertions = sc
        .then
        .iter()
        .map(|a| danh_gia(&sim, a, &aliases))
        .collect();

    Ok(Report {
        scenario: sc.scenario.clone(),
        bindings: aliases,
        assertions,
        ticks: sim.clock().local().0.saturating_sub(tick_dau),
        state_hash: sim.state_hash(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Ràng buộc alias
// ─────────────────────────────────────────────────────────────────────────────

fn rang_buoc(
    sim: &Sim,
    binds: &BTreeMap<String, Binding>,
) -> Result<BTreeMap<String, EntityId>, RunError> {
    let mut ra: BTreeMap<String, EntityId> = BTreeMap::new();

    // Duyệt theo thứ tự alias (`BTreeMap`), nhưng alias có `in` phải chờ alias
    // nó trỏ tới. Lặp cho tới khi không tiến thêm được nữa; đứng yên nghĩa là
    // có phụ thuộc vòng, và đó là lỗi soạn thảo chứ không phải lý do để treo.
    while ra.len() < binds.len() {
        let san_sang: Vec<(&String, &Binding)> = binds
            .iter()
            .filter(|(a, b)| {
                !ra.contains_key(*a) && b.within.as_ref().is_none_or(|w| ra.contains_key(w))
            })
            .collect();

        if san_sang.is_empty() {
            let ket: Vec<&str> = binds
                .keys()
                .filter(|a| !ra.contains_key(*a))
                .map(String::as_str)
                .collect();
            return Err(RunError::Invalid {
                name: "bind".to_owned(),
                errors: vec![format!("phụ thuộc `in` vòng giữa các alias: {ket:?}")],
            });
        }

        for (alias, b) in san_sang {
            let trong = b.within.as_ref().and_then(|w| ra.get(w).copied());
            let id = chon(sim, alias, b, trong)?;
            ra.insert(alias.clone(), id);
        }
    }
    Ok(ra)
}

fn chon(
    sim: &Sim,
    alias: &str,
    b: &Binding,
    trong: Option<EntityId>,
) -> Result<EntityId, RunError> {
    // Ứng viên: đúng `kind`, có `tag` nếu khai báo, thuộc `in` nếu khai báo.
    let mut ung_vien: Vec<EntityId> = sim
        .store()
        .ids()
        .filter(|id| sim.store().attr_text(*id, "core.kind") == Some(b.kind.as_str()))
        .filter(|id| {
            b.tag.as_ref().is_none_or(|t| {
                sim.store()
                    .attrs(*id)
                    .is_some_and(|a| a.contains_key(&format!("tag.{t}")))
            })
        })
        .filter(|id| {
            trong.is_none_or(|w| sim.store().attr_int(*id, "core.within") == Some(w.get() as i64))
        })
        .collect();

    // Sắp xếp theo `order`. Vế cuối luôn là `id asc` (đã kiểm ở `validate`),
    // nên thứ tự là toàn phần và kết quả không chập chờn.
    ung_vien.sort_by(|a, b2| so_sanh_theo(sim, *a, *b2, &b.order));

    let chi_so = match b.select.as_str() {
        "nth" => b.n.unwrap_or(1).saturating_sub(1),
        _ => 0,
    };

    ung_vien
        .get(chi_so)
        .copied()
        .ok_or_else(|| RunError::NoMatch {
            alias: alias.to_owned(),
            kind: b.kind.clone(),
            tag: b.tag.clone(),
        })
}

fn so_sanh_theo(sim: &Sim, a: EntityId, b: EntityId, order: &[String]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    for khoa in order {
        let (truong, giam) = match khoa.rsplit_once(' ') {
            Some((t, "desc")) => (t, true),
            Some((t, _)) => (t, false),
            None => (khoa.as_str(), false),
        };
        let ord = if truong == "id" {
            a.get().cmp(&b.get())
        } else {
            let va = sim.store().attr_int(a, &format!("core.{truong}"));
            let vb = sim.store().attr_int(b, &format!("core.{truong}"));
            va.cmp(&vb)
        };
        let ord = if giam { ord.reverse() } else { ord };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

// ─────────────────────────────────────────────────────────────────────────────
// Chạy bước
// ─────────────────────────────────────────────────────────────────────────────

fn chay_buoc(sim: &mut Sim, s: &Step, aliases: &BTreeMap<String, u64>) -> Result<(), RunError> {
    let ten = s.name().unwrap_or_default().to_owned();

    // Tên bước là loại command. Thử nguyên văn trước, rồi thử với namespace
    // `core.` — để kịch bản viết `set_need` thay vì `core.set_need`, nhưng vẫn
    // gọi được command của pack khác bằng tên đầy đủ.
    let kind = if sim.handlers().get(&ten).is_some() {
        ten.clone()
    } else if sim.handlers().get(&format!("core.{ten}")).is_some() {
        format!("core.{ten}")
    } else {
        return Err(RunError::UnknownStep {
            step: ten.clone(),
            available: sim.handlers().kinds().collect::<Vec<_>>().join(", "),
        });
    };

    let payload = doi_gia_tri(
        s.args().cloned().unwrap_or(serde_yaml::Value::Null),
        aliases,
    );
    let cmd = Command::new(&kind, sim.world_id(), payload);
    sim.apply(&cmd)
        .map(|_| ())
        .map_err(|source| RunError::StepFailed { step: ten, source })
}

/// Chuyển YAML sang [`Value`], thay alias `@x` bằng id đã ràng buộc.
fn doi_gia_tri(v: serde_yaml::Value, aliases: &BTreeMap<String, u64>) -> Value {
    match v {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::Uint(u)
            } else {
                // Số thực trong kịch bản là lỗi soạn thảo — §P10.2 cấm chúng
                // trên đường commit. Giữ nguyên văn để thông báo lỗi của handler
                // chỉ đúng vào giá trị, thay vì âm thầm cắt về số nguyên.
                Value::Text(n.to_string())
            }
        }
        serde_yaml::Value::String(s) => {
            if let Some(id) = aliases.get(&s) {
                Value::Uint(*id)
            } else {
                Value::Text(s)
            }
        }
        serde_yaml::Value::Sequence(xs) => {
            Value::List(xs.into_iter().map(|x| doi_gia_tri(x, aliases)).collect())
        }
        serde_yaml::Value::Mapping(m) => Value::Map(
            m.into_iter()
                .filter_map(|(k, v)| k.as_str().map(|k| (k.to_owned(), doi_gia_tri(v, aliases))))
                .collect(),
        ),
        serde_yaml::Value::Tagged(t) => doi_gia_tri(t.value, aliases),
    }
}

fn tien(sim: &mut Sim, n: u64) -> Result<(), RunError> {
    sim.advance(n).map_err(|source| RunError::StepFailed {
        step: "step".to_owned(),
        source,
    })
}

fn chay_den_khi(sim: &mut Sim, s: &Step, aliases: &BTreeMap<String, u64>) -> Result<(), RunError> {
    let bieu_thuc = s.arg_str("predicate").unwrap_or_default();
    let p = Predicate::parse(&bieu_thuc)?;
    // `max_ticks` hoặc `max_days`; ngày quy ra tick bằng 24 giờ × 60 phút.
    let tran = s
        .arg_int("max_ticks")
        .or_else(|| s.arg_int("max_days").map(|d| d * 1_440))
        .unwrap_or(10_000)
        .max(0) as u64;

    for da_chay in 0..=tran {
        if p.eval(&ngu_canh(sim), aliases) {
            return Ok(());
        }
        if da_chay < tran {
            tien(sim, 1)?;
        }
    }
    Err(RunError::Timeout {
        predicate: bieu_thuc,
        ticks: tran,
    })
}

/// Ngữ cảnh đánh giá vị từ.
fn ngu_canh(sim: &Sim) -> Bindings {
    let mut ctx = Bindings::new();
    ctx.insert("tick".to_owned(), Val::Int(sim.clock().local().0 as i64));
    ctx.insert("event.count".to_owned(), Val::Int(sim.log().len() as i64));
    if let Some(last) = sim.log().iter().last() {
        ctx.insert("event.kind".to_owned(), Val::Text(last.kind.0.clone()));
        if let Some(a) = last.actor {
            ctx.insert("event.actor".to_owned(), Val::Int(a.get() as i64));
        }
        if let Some(s) = last.subject {
            ctx.insert("event.subject".to_owned(), Val::Int(s.get() as i64));
        }
    }
    ctx
}

// ─────────────────────────────────────────────────────────────────────────────
// Khẳng định
// ─────────────────────────────────────────────────────────────────────────────

fn dat(name: &str) -> AssertResult {
    AssertResult {
        name: name.to_owned(),
        passed: true,
        detail: String::new(),
    }
}

fn truot(name: &str, detail: impl Into<String>) -> AssertResult {
    AssertResult {
        name: name.to_owned(),
        passed: false,
        detail: detail.into(),
    }
}

// Dài vì nó **liệt kê**: một nhánh cho mỗi động từ. Chia nhỏ ra sẽ làm
// danh sách khó đọc hơn chứ không dễ hơn — người đọc phải nhảy qua lại
// để biết có bao nhiêu động từ, và đó chính là câu hỏi hay được hỏi nhất.
#[allow(clippy::too_many_lines)]
fn danh_gia(sim: &Sim, a: &Assertion, aliases: &BTreeMap<String, EntityId>) -> AssertResult {
    let ten = a.name().unwrap_or("?").to_owned();
    match ten.as_str() {
        "assert_event_exists" => {
            let kind = a.arg_str("kind").unwrap_or_default();
            if sim.log().iter().any(|e| e.kind.0 == kind) {
                dat(&ten)
            } else {
                let da_co: Vec<&str> = {
                    let mut v: Vec<&str> = sim.log().iter().map(|e| e.kind.0.as_str()).collect();
                    v.sort_unstable();
                    v.dedup();
                    v
                };
                truot(
                    &ten,
                    format!("không có sự kiện `{kind}`; nhật ký có: {da_co:?}"),
                )
            }
        }

        "assert_event_absent" => {
            let kind = a.arg_str("kind").unwrap_or_default();
            if sim.log().iter().any(|e| e.kind.0 == kind) {
                truot(
                    &ten,
                    format!("sự kiện `{kind}` đã xảy ra nhưng lẽ ra không"),
                )
            } else {
                dat(&ten)
            }
        }

        "assert_attr" => {
            let alias = a.arg_str("entity").unwrap_or_default();
            let key = a.arg_str("key").unwrap_or_default();
            let Some(id) = aliases.get(&alias) else {
                return truot(&ten, format!("alias `{alias}` chưa được ràng buộc"));
            };
            let thuc_te = sim.store().attr(*id, &key);
            let mong_doi = a.args().and_then(|v| v.get("equals").cloned());
            match (thuc_te, mong_doi) {
                (Some(Value::Int(v)), Some(m)) if m.as_i64() == Some(*v) => dat(&ten),
                (Some(Value::Text(v)), Some(m)) if m.as_str() == Some(v.as_str()) => dat(&ten),
                (t, m) => truot(&ten, format!("{alias}.{key} = {t:?}, mong đợi {m:?}")),
            }
        }

        "assert_invariants" => {
            let muon: Vec<String> = a.arg_list("ids").unwrap_or_default();
            let rep = sim.check(&InvariantRunner::standard(Cost::Expensive));
            let vi_pham: Vec<&mow_core::Violation> = rep
                .violations
                .iter()
                .filter(|v| muon.is_empty() || muon.iter().any(|m| m == v.id))
                .collect();

            // Đòi một bất biến không tồn tại là lỗi soạn thảo, và phải bắt được
            // — nếu không, gõ nhầm `INV-22-4` thành `INV-22-04` sẽ làm khẳng
            // định luôn xanh mà chẳng kiểm gì.
            let khong_biet: Vec<&String> = muon
                .iter()
                .filter(|m| !rep.checked.contains(&m.as_str()))
                .collect();
            if !khong_biet.is_empty() {
                return truot(
                    &ten,
                    format!("không có bất biến nào tên {khong_biet:?} — kiểm tra lại cách viết id"),
                );
            }
            if vi_pham.is_empty() {
                dat(&ten)
            } else {
                truot(
                    &ten,
                    vi_pham
                        .iter()
                        .map(|v| format!("{}: {}", v.id, v.detail))
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            }
        }

        "assert_cause_chain_contains" => {
            let Some(cuoi) = sim.last_event() else {
                return truot(&ten, "nhật ký rỗng");
            };
            let can: Vec<String> = a.arg_list("nodes").unwrap_or_default();
            let chuoi: Vec<&str> = sim
                .log()
                .cause_chain(cuoi, 256)
                .iter()
                .map(|e| e.kind.0.as_str())
                .collect();
            let thieu: Vec<&String> = can
                .iter()
                .filter(|c| !chuoi.contains(&c.as_str()))
                .collect();
            if thieu.is_empty() {
                dat(&ten)
            } else {
                truot(
                    &ten,
                    format!("chuỗi nhân quả thiếu {thieu:?}; có {chuoi:?}"),
                )
            }
        }

        "assert_entity_count" => {
            let mong = a.args().and_then(serde_yaml::Value::as_i64).unwrap_or(-1);
            let thuc = sim.store().len() as i64;
            if thuc == mong {
                dat(&ten)
            } else {
                truot(&ten, format!("có {thuc} thực thể, mong đợi {mong}"))
            }
        }

        "assert_no_orphan_entities" => {
            // Thực thể không có `core.kind` là thứ được tạo ra mà không đi qua
            // đường khởi tạo bình thường — dấu hiệu của một handler bỏ sót.
            let mo_coi: Vec<EntityId> = sim
                .store()
                .ids()
                .filter(|id| sim.store().attr(*id, "core.kind").is_none())
                .collect();
            if mo_coi.is_empty() {
                dat(&ten)
            } else {
                truot(&ten, format!("thực thể không có `core.kind`: {mo_coi:?}"))
            }
        }

        khac => truot(
            &ten,
            format!("không biết khẳng định `{khac}` — kiểm tra lại tên"),
        ),
    }
}
