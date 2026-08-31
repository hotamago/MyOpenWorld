//! # `mow-worldgen` — sinh thế giới lười và xác định
//!
//! Một ô nguyên thủy là **hàm thuần** của `(world_seed, profile snapshot, x, y, z)`
//! (`idea.md §7.2`). Không có trạng thái, không có thứ tự, không có "đã sinh
//! chưa". Hệ quả trực tiếp: mở một chunk ở tick 10 hay tick 10 triệu đều cho
//! cùng địa hình, và save chỉ cần lưu **seed cộng delta** thay vì hàng tỉ ô.
//!
//! ## Vì sao sông không đứt ở biên chunk
//!
//! Cách sai và rất phổ biến: sinh từng chunk rồi cố khâu mép lại. Nó không bao
//! giờ khớp hoàn toàn, và lỗi biểu hiện thành những vết nứt chỉ lộ ra khi người
//! chơi đi dọc theo một con sông.
//!
//! Cách ở đây (`§7.4`): **đặc trưng lớn được quyết định ở lưới thô hơn, chunk
//! chỉ lấy mẫu kết quả**. Một dãy núi tồn tại ở lưới 1024 ô; hai chunk kề nhau
//! hỏi cùng lưới đó và nhận cùng câu trả lời. Không có gì để khâu, vì không có
//! gì bị cắt.
//!
//! Với sông thì mạnh hơn nữa: mỗi ô lưu vực thô có một **outlet xác định**
//! (`[`hydrology`]`), và dòng chảy cục bộ nối vào outlet đó. Nên hướng chảy ở
//! hai bên biên chunk luôn nhất quán mà không cần chunk nào biết chunk kia tồn
//! tại.
//!
//! ## Mười bước của `§7.3`
//!
//! | Bước | Ở đâu | Trạng thái |
//! |---|---|---|
//! | 1. Khung vũ trụ | [`profile::GenerationProfile`] | xong |
//! | 2. Địa chất vĩ mô | [`macro_fields`] | xong |
//! | 3. Độ cao nhiều tần số | [`elevation`] | xong |
//! | 4. Thủy văn phân cấp | [`hydrology`] | xong |
//! | 5. Khí hậu | [`climate`] | xong |
//! | 6. Địa tầng | [`strata`] | xong |
//! | 7. Biome | [`biome`] | xong |
//! | 8. Sinh thái ban đầu | `mow-life` | `PE-12` |
//! | 9. Di tích và anomaly | `mow-scenario` | `PF-05` |
//! | 10. Delta lịch sử | tiền sử chạy bằng aggregate | `PF-05` |
//!
//! Ba bước cuối **không** thuộc crate này, và đó là chủ đích: chúng cần thời
//! gian trôi qua, còn bảy bước đầu là hàm thuần của tọa độ. Trộn chúng lại sẽ
//! làm mất tính "mở chunk lúc nào cũng như nhau".

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod biome;
pub mod climate;
pub mod elevation;
pub mod hydrology;
pub mod macro_fields;
pub mod noise;
pub mod profile;
pub mod strata;

pub use biome::Biome;
pub use climate::Climate;
pub use elevation::Elevation;
pub use hydrology::{Basin, Flow};
pub use profile::{EdgePolicy, GenerationProfile, Topology};
pub use strata::{Material, Strata};

use mow_math::{CanonicalHash, StateHasher, WorldSeed};

/// Một ô nguyên thủy đã sinh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCell {
    /// Độ cao và các trường liên quan.
    pub elevation: Elevation,
    /// Khí hậu tại ô.
    pub climate: Climate,
    /// Dòng chảy.
    pub flow: Flow,
    /// Địa tầng theo chiều sâu.
    pub strata: Strata,
    /// Quần xã sinh vật.
    pub biome: Biome,
}

impl CanonicalHash for BaseCell {
    fn canonical_hash(&self, h: &mut StateHasher) {
        self.elevation.canonical_hash(h);
        self.climate.canonical_hash(h);
        self.flow.canonical_hash(h);
        self.strata.canonical_hash(h);
        self.biome.canonical_hash(h);
    }
}

/// Bộ sinh thế giới.
///
/// Không có trạng thái nào ngoài seed và profile — cố ý. Nếu có cache bên
/// trong, thì thứ tự truy vấn sẽ ảnh hưởng kết quả khi cache bị đuổi, và đó là
/// một lớp lỗi không tái hiện được.
#[derive(Debug, Clone)]
pub struct Worldgen {
    seed: WorldSeed,
    profile: GenerationProfile,
}

/// Lỗi khi sinh.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GenError {
    /// Tọa độ nằm ngoài thế giới có biên.
    #[error("({x}, {y}) nằm ngoài biên thế giới")]
    OutOfBounds {
        /// Tọa độ `x`.
        x: i64,
        /// Tọa độ `y`.
        y: i64,
    },
}

impl Worldgen {
    /// Dựng.
    pub fn new(seed: WorldSeed, profile: GenerationProfile) -> Worldgen {
        Worldgen { seed, profile }
    }

    /// Profile đang dùng.
    pub fn profile(&self) -> &GenerationProfile {
        &self.profile
    }

    /// Seed.
    pub fn seed(&self) -> WorldSeed {
        self.seed
    }

    /// Sinh một ô nguyên thủy.
    ///
    /// Hàm thuần: cùng đầu vào luôn cho cùng đầu ra, không phụ thuộc thứ tự
    /// gọi, số luồng, hay việc ô lân cận đã được sinh hay chưa.
    pub fn base_cell(&self, x: i64, y: i64) -> Result<BaseCell, GenError> {
        if !self.profile.topology.contains(x, y) {
            return match self.profile.edge_policy {
                EdgePolicy::Reject => Err(GenError::OutOfBounds { x, y }),
                EdgePolicy::Clamp => {
                    let (cx, cy) = kep_vao_bien(&self.profile.topology, x, y);
                    self.base_cell(cx, cy)
                }
            };
        }

        let (x, y) = self.profile.topology.canonicalize(x, y);
        let s = self.seed.0;

        let elevation = elevation::sample(s, &self.profile, x, y);
        let climate = climate::sample(s, &self.profile, x, y, &elevation);
        let flow = hydrology::sample(s, &self.profile, x, y, &elevation, &climate);
        let strata = strata::sample(s, &self.profile, x, y, &elevation);
        let biome = biome::classify(&elevation, &climate, &flow, &strata);

        Ok(BaseCell {
            elevation,
            climate,
            flow,
            strata,
            biome,
        })
    }
}

fn kep_vao_bien(t: &Topology, x: i64, y: i64) -> (i64, i64) {
    match t {
        Topology::BoundedBox { half_x, half_y } => {
            (x.clamp(-half_x, *half_x), y.clamp(-half_y, *half_y))
        }
        _ => (x, y),
    }
}
