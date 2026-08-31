//! Yuu tạo nội dung: World Architect, Species Foundry, Law Forge
//! (`idea.md §15.1`–`§15.3`, `§9.6`, `PF-06`).
//!
//! ## Ba module, một nguyên tắc chung
//!
//! `§15.1`: *"Một persona **Yuu** thống nhất giao tiếp với người chơi, nhưng
//! bên trong các module **không chia sẻ quyền tùy tiện**"*.
//!
//! Nên ba module ở đây không dùng chung một kiểu `YuuContext` toàn năng. Mỗi
//! cái nhận đúng thứ nó cần và trả về đúng thứ nó được phép tạo — và không cái
//! nào trả về một thay đổi state đã commit.
//!
//! ## Species Foundry: viability check là bắt buộc
//!
//! `§9.5.4` viết thẳng:
//!
//! > Mọi cá thể lai tạo bằng phép vẫn phải qua **kiểm tra viability** của
//! > `§9.6`. **Yuu không được phép tạo ra một sinh vật không thở được rồi để nó
//! > chết ngay.**
//!
//! Câu cuối là một yêu cầu về **thời điểm**: kiểm phải xảy ra **trước** khi
//! loài vào registry, không phải sau khi cá thể đầu tiên được sinh ra. Nên
//! [`SpeciesFoundry::forge`] trả `Err` chứ không trả một loài kèm cảnh báo.
//!
//! ## Law Forge: sandbox trước, đăng ký sau
//!
//! `§15.3` cấm chạy code do LLM sinh trực tiếp. [`LawForge::forge`] nhận một
//! `Rule` **đã phân tích** của `mow-magic`, chạy nó trong sandbox với đầu vào
//! thử, và chỉ trả về nếu nó dừng và ra đúng đơn vị đã khai.

use mow_magic::dsl::{Quantity, Rule, RuleError, Unit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

// ─────────────────────────── World Architect ───────────────────────────

/// Một template world mà World Architect dựng (`§15.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldTemplate {
    /// Định danh.
    pub id: String,
    /// Profile sinh địa hình.
    pub generation_profile: String,
    /// Dải điều kiện của world: khí quyển, nhiệt độ, mana.
    pub conditions: Conditions,
    /// Luật nền mà world này bật.
    pub law_profile: Vec<String>,
}

/// Dải điều kiện môi trường của một world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conditions {
    /// Khí quyển, `(min, max)`.
    pub atmosphere: (i32, i32),
    /// Nhiệt độ, phần trăm độ C, `(min, max)`.
    pub temperature: (i32, i32),
    /// Mật độ mana, `(min, max)`.
    pub mana: (i32, i32),
}

impl Conditions {
    /// Dải này có hợp lệ không: cận dưới không vượt cận trên.
    pub fn well_formed(&self) -> bool {
        self.atmosphere.0 <= self.atmosphere.1
            && self.temperature.0 <= self.temperature.1
            && self.mana.0 <= self.mana.1
    }
}

// ─────────────────────────── Species Foundry ───────────────────────────

/// Một loài mà Species Foundry đề xuất (`§15.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesDraft {
    /// Định danh.
    pub id: String,
    /// Dải điều kiện loài chịu được.
    pub tolerances: Conditions,
    /// Nhu cầu năng lượng mỗi ngày, kcal.
    pub kcal_per_day: i64,
    /// Nguồn thức ăn.
    pub food_sources: Vec<String>,
    /// Tuổi thọ, năm.
    pub lifespan_years: u32,
    /// Tuổi trưởng thành, năm.
    pub adult_at_years: u32,
}

/// Vì sao một loài không sống nổi (`§9.6`).
///
/// Mỗi biến thể là một cách một loài **chết ngay sau khi được tạo**, và mỗi
/// cái đã từng là một sinh vật ai đó tạo ra rồi không hiểu vì sao nó chết.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Inviable {
    /// Dải chịu đựng của loài không giao với điều kiện world nào.
    #[error("loài `{species}` không sống được ở world `{world}`: {axis}")]
    CannotSurvive {
        /// Loài nào.
        species: String,
        /// World nào.
        world: String,
        /// Trục nào không giao.
        axis: &'static str,
    },
    /// Dải chịu đựng tự mâu thuẫn.
    #[error("loài `{species}` có dải chịu đựng sai hình dạng: cận dưới vượt cận trên")]
    MalformedTolerance {
        /// Loài nào.
        species: String,
    },
    /// Không có nguồn thức ăn.
    #[error("loài `{species}` cần {kcal} kcal/ngày nhưng không khai nguồn thức ăn nào")]
    NoFoodSource {
        /// Loài nào.
        species: String,
        /// Cần bao nhiêu.
        kcal: i64,
    },
    /// Trưởng thành sau khi chết.
    #[error(
        "loài `{species}` trưởng thành ở tuổi {adult} nhưng chỉ sống {lifespan} năm — \
         không cá thể nào kịp sinh sản"
    )]
    MaturesAfterDeath {
        /// Loài nào.
        species: String,
        /// Tuổi trưởng thành.
        adult: u32,
        /// Tuổi thọ.
        lifespan: u32,
    },
    /// Nhu cầu năng lượng phi lý.
    #[error("loài `{species}` cần {kcal} kcal/ngày — không hệ sinh thái nào nuôi nổi")]
    ImpossibleMetabolism {
        /// Loài nào.
        species: String,
        /// Cần bao nhiêu.
        kcal: i64,
    },
}

/// Trần năng lượng mà một hệ sinh thái nuôi nổi cho một cá thể, kcal/ngày.
///
/// Cỡ một con voi lớn ăn cả ngày. Trên mức này thì loài đó phải ăn liên tục
/// nhiều hơn số giờ có trong ngày, và nó chết vì đói dù thức ăn đầy xung quanh.
pub const TRAN_KCAL_MOI_NGAY: i64 = 60_000;

/// Species Foundry (`§15.1`).
#[derive(Debug, Clone, Default)]
pub struct SpeciesFoundry;

impl SpeciesFoundry {
    /// Kiểm và tạo một loài — **kiểm trước khi vào registry** (`§9.6`).
    ///
    /// Trả về **mọi** lỗi cùng lúc: người thiết kế loài cần sửa một lần.
    pub fn forge(
        draft: &SpeciesDraft,
        worlds: &[WorldTemplate],
    ) -> Result<SpeciesDraft, Vec<Inviable>> {
        let mut loi = Vec::new();

        if !draft.tolerances.well_formed() {
            loi.push(Inviable::MalformedTolerance {
                species: draft.id.clone(),
            });
        }
        if draft.kcal_per_day > 0 && draft.food_sources.is_empty() {
            loi.push(Inviable::NoFoodSource {
                species: draft.id.clone(),
                kcal: draft.kcal_per_day,
            });
        }
        if draft.kcal_per_day > TRAN_KCAL_MOI_NGAY {
            loi.push(Inviable::ImpossibleMetabolism {
                species: draft.id.clone(),
                kcal: draft.kcal_per_day,
            });
        }
        if draft.adult_at_years >= draft.lifespan_years {
            loi.push(Inviable::MaturesAfterDeath {
                species: draft.id.clone(),
                adult: draft.adult_at_years,
                lifespan: draft.lifespan_years,
            });
        }

        // Loài phải sống được ở **ít nhất một** world. Không world nào thì nó
        // là một sinh vật không có chỗ nào để tồn tại.
        if !worlds.is_empty() {
            let song_duoc = worlds
                .iter()
                .any(|w| khop(&draft.tolerances, &w.conditions).is_none());
            if !song_duoc {
                // Báo theo world đầu tiên, kèm trục cụ thể — "không sống được ở
                // đâu cả" là một câu không sửa được.
                let w = &worlds[0];
                loi.push(Inviable::CannotSurvive {
                    species: draft.id.clone(),
                    world: w.id.clone(),
                    axis: khop(&draft.tolerances, &w.conditions).unwrap_or("không rõ"),
                });
            }
        }

        if loi.is_empty() {
            Ok(draft.clone())
        } else {
            Err(loi)
        }
    }
}

/// Trục nào không giao giữa hai dải; `None` nghĩa là sống được.
fn khop(loai: &Conditions, world: &Conditions) -> Option<&'static str> {
    let giao = |a: (i32, i32), b: (i32, i32)| a.0.max(b.0) <= a.1.min(b.1);
    if !giao(loai.atmosphere, world.atmosphere) {
        return Some("khí quyển");
    }
    if !giao(loai.temperature, world.temperature) {
        return Some("nhiệt độ");
    }
    if !giao(loai.mana, world.mana) {
        return Some("mật độ mana");
    }
    None
}

// ─────────────────────────── Law Forge ───────────────────────────

/// Vì sao một luật không qua được Law Forge.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ForgeError {
    /// Kiểm tĩnh trượt.
    #[error("luật `{rule}` không qua kiểm tĩnh, {} lỗi:\n{}", .errors.len(),
            .errors.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    StaticCheck {
        /// Luật nào.
        rule: String,
        /// Lỗi.
        errors: Vec<RuleError>,
    },
    /// Chạy thử trượt.
    #[error("luật `{rule}` không chạy được với đầu vào thử: {source}")]
    TrialRun {
        /// Luật nào.
        rule: String,
        /// Lỗi.
        #[source]
        source: RuleError,
    },
    /// Không có đầu vào thử nào.
    ///
    /// Một luật chưa từng chạy thử là một luật chưa ai biết nó làm gì — và
    /// đăng ký nó là đưa vào thế giới một thứ chỉ kiểm được bằng cú pháp.
    #[error(
        "luật `{rule}` không có đầu vào thử nào — một luật chưa từng chạy là một \
         luật chưa ai biết nó làm gì"
    )]
    NoTrialInput {
        /// Luật nào.
        rule: String,
    },
}

/// Một luật đã qua Law Forge, kèm bằng chứng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgedLaw {
    /// Luật.
    pub rule: Rule,
    /// Kết quả các lần chạy thử — **bằng chứng**, không phải một cờ "đã kiểm".
    pub trials: Vec<(BTreeMap<String, Quantity>, BTreeMap<String, Quantity>)>,
}

impl ForgedLaw {
    /// Số lần đã chạy thử.
    pub fn trial_count(&self) -> usize {
        self.trials.len()
    }

    /// Mọi lần chạy có ra đúng đơn vị đã khai không.
    pub fn units_consistent(&self) -> bool {
        self.trials.iter().all(|(_, ra)| {
            self.rule
                .output_units
                .iter()
                .all(|(k, u)| ra.get(k).is_some_and(|q| q.unit == *u))
        })
    }
}

/// Law Forge (`§15.1`, `§15.3`).
#[derive(Debug, Clone, Default)]
pub struct LawForge;

impl LawForge {
    /// Kiểm tĩnh, chạy thử trong sandbox, rồi mới trả về.
    ///
    /// Ba bước theo đúng thứ tự đó. Đảo thứ tự — chạy trước rồi kiểm — nghĩa
    /// là một luật sai đơn vị đã kịp chạy một lần, và ở một hệ thống mà luật
    /// có tác dụng phụ thì một lần là đủ.
    pub fn forge(
        rule: &Rule,
        trial_inputs: &[BTreeMap<String, Quantity>],
    ) -> Result<ForgedLaw, ForgeError> {
        let loi = rule.validate();
        if !loi.is_empty() {
            return Err(ForgeError::StaticCheck {
                rule: rule.rule_id.clone(),
                errors: loi,
            });
        }
        if trial_inputs.is_empty() {
            return Err(ForgeError::NoTrialInput {
                rule: rule.rule_id.clone(),
            });
        }

        let mut trials = Vec::new();
        for vao in trial_inputs {
            let ra = rule.run(vao).map_err(|e| ForgeError::TrialRun {
                rule: rule.rule_id.clone(),
                source: e,
            })?;
            trials.push((vao.clone(), ra));
        }
        Ok(ForgedLaw {
            rule: rule.clone(),
            trials,
        })
    }
}

/// Đơn vị mà một luật khai là đầu ra — tiện cho chỗ gọi dựng đầu vào thử.
pub fn declared_outputs(rule: &Rule) -> Vec<(&str, Unit)> {
    rule.output_units
        .iter()
        .map(|(k, u)| (k.as_str(), *u))
        .collect()
}
