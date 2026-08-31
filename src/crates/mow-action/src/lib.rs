//! # `mow-action` — hanh dong
//!
//! Bon he thong, va mot chu de chung: **ket qua phai den tu thu quan sat duoc
//! trong the gioi**.
//!
//! - [`registry`] — dieu kien tien quyet tinh tu **state**, khong tu loi khai
//!   cua LLM hay tu mot chuoi trong YAML (`§22.5`).
//! - [`timeline`] — ba pha `wind_up -> impact -> recovery`; `wind_up` quan sat
//!   duoc, nen phong thu co y nghia (`§10.8`).
//! - [`resolve`] — tranh chap giai bang **diem phan dinh quan sat duoc**;
//!   `EntityId` chi pha hoa o buoc cuoi (`§22.43`).
//! - [`tactical`] — tam yeu to chien truong, moi yeu to tao ra mot quyet dinh.
//!
//! [`consent`] la ngoai le cua moi thu khac trong du an nay: mot rang buoc ma
//! **khong plugin, khong override, khong tham so nao** noi ra duoc (`§22.26`).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]

pub mod consent;
pub mod perception;
pub mod registry;
pub mod resolve;
pub mod tactical;
pub mod timeline;
pub mod utility;

pub use consent::{ConsentCapacity, ConsentDenial, DenialReason, IntimacyRegistry};
pub use perception::{observe, CognitionContext, Conditions, Observation, Sense, Senses};
pub use registry::{ActionDef, ActionRegistry, Precondition};
pub use resolve::{resolve_all, Contention, LossReason, Outcome, Tier};
pub use tactical::{
    assess, clamp_move_speed, friendly_fire_risk, hit_chance, zone_of_control, Assessment, Cover,
    Engagement, Facing, Footing,
};
pub use timeline::{
    cognitive_latency, movement_rate, next_cognition_tick, Phase, PhaseDurations, Scheduled,
    Speeds, Timeline,
};
pub use utility::{
    villager_brain, Brain, Candidate, Consideration, Layer, Reflex, RoutineSlot, Scorer,
};
