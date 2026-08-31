//! # `mow-life` — co the, nhu cau, di truyen, lao hoa
//!
//! Bon he thong, va chung deu chia mot nguyen tac: **trang thai la mot ham cua
//! thoi gian, khong phai mot bien duoc cap nhat moi tick**.
//!
//! - [`homeostasis`] — nhu cau suy ra bang tich phan dong (`§9.7`, `§22.24`).
//! - [`genome`] — bo gen la 24 byte cong mot ham, khong phai 20 KB du lieu (`§9.5.2`).
//! - [`senescence`] — lao hoa Gompertz hoac khong dang ke (`§9.5.6`).
//! - [`body`] — thuong tich theo bo phan; `vitality` chi la chi so suy ra (`§9.4`).
//!
//! Nguyen tac chung do khong phai vi toi uu hoa som. No la dieu kien de LOD
//! hoat dong: mot thuc the o muc `Far` khong chay vong lap nao, va khi quay lai
//! `Active` no phai o dung trang thai ma no le ra phai o. Voi ham cua thoi gian
//! thi dieu do mien phi; voi bien cap nhat moi tick thi no la mot bai toan dong
//! bo khong co loi giai sach.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod barrier;
pub mod body;
pub mod genome;
pub mod homeostasis;
pub mod quantgen;
pub mod senescence;
pub mod speciation;

pub use barrier::{
    coordination_cost, inapplicable_systems, time_gap, Axis, Barriers, Habitat, Lifespan, Range,
    Reproductive, Senses, SocialStructure, Territorial, TimeGap, Unteachable, BRIDGES, NGUONG_CHAT,
};
pub use body::{BodyPart, BodyPlan, Injury, InjuryKind, Tissue};
pub use genome::{recombination_seed, Genome};
pub use homeostasis::{Homeostasis, Need, Threshold, SCALE};
pub use senescence::{senescence_effect, LifeStage, LifeStages, SenescenceModel};
pub use speciation::{
    secondary_contact, Divergence, IsolatedPopulation, SecondaryContact, SpeciationRoute,
    CAP_DU_DE_VO_SINH,
};
