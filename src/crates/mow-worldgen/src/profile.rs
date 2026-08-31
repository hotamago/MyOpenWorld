//! Generation profile — **ảnh chụp bất biến** khóa lúc tạo world (`§7.2`).
//!
//! Đây là một trong những quyết định quan trọng nhất của thiết kế, và nó dễ bị
//! làm sai theo một cách rất tự nhiên.
//!
//! Cám dỗ là để worldgen đọc "luật hiện hành". Nghe hợp lý: nếu True God đổi
//! trọng lực thì địa hình mới sinh ra nên phản ánh điều đó. Nhưng hệ quả là
//! **thời điểm mở một chunk quyết định nội dung của nó**. Hai người chơi cùng
//! một save, đi cùng một hướng, mở chunk ở hai thời điểm khác nhau — và nhận
//! hai địa hình khác nhau. Save không còn tái tạo được, và không có gì trong
//! log giải thích.
//!
//! Nên profile là **ảnh chụp**: nó chứa hằng số vật lý cần cho worldgen, được
//! khóa lúc khai sinh, và **không trỏ tới bộ luật runtime đang thay đổi**. Muốn
//! viết lại địa chất nền thì phải tạo migration có preview, bake delta, hoặc
//! fork sang profile mới (`§7.5`).

use crate::noise::Octave;
use mow_math::{CanonicalHash, Fx, StateHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Hình học của thế giới (`§7.4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topology {
    /// Mặt phẳng rộng thực dụng. Mặc định cho Gaia.
    ///
    /// **Không giả vờ là bề mặt cầu.** World cần hành tinh cầu phải dùng
    /// topology riêng; diễn giải lại tọa độ Gaia giữa chừng là cách chắc chắn
    /// làm hỏng mọi thứ đã lưu.
    InfiniteCartesian,
    /// Hộp có biên.
    BoundedBox {
        /// Nửa cạnh theo `x`.
        half_x: i64,
        /// Nửa cạnh theo `y`.
        half_y: i64,
    },
    /// Mặt xuyến: đi hết mép phải quay về mép trái.
    ToroidalXy {
        /// Chu kỳ theo `x`.
        period_x: i64,
        /// Chu kỳ theo `y`.
        period_y: i64,
    },
}

impl Topology {
    /// Đưa một tọa độ về dạng chuẩn của topology.
    ///
    /// Với [`Topology::ToroidalXy`] đây là phép quấn; với hai cái kia là hàm
    /// đồng nhất. Gọi nó **trước mọi lần lấy mẫu nhiễu**, nếu không mặt xuyến
    /// sẽ có một đường nối rõ rệt ở chỗ tọa độ nhảy.
    pub fn canonicalize(self, x: i64, y: i64) -> (i64, i64) {
        match self {
            Topology::ToroidalXy { period_x, period_y } => {
                (x.rem_euclid(period_x.max(1)), y.rem_euclid(period_y.max(1)))
            }
            _ => (x, y),
        }
    }

    /// Tọa độ có nằm trong thế giới không.
    pub fn contains(self, x: i64, y: i64) -> bool {
        match self {
            Topology::BoundedBox { half_x, half_y } => {
                x >= -half_x && x <= half_x && y >= -half_y && y <= half_y
            }
            _ => true,
        }
    }
}

impl CanonicalHash for Topology {
    fn canonical_hash(&self, h: &mut StateHasher) {
        match self {
            Topology::InfiniteCartesian => {
                h.write_str("infinite_cartesian");
            }
            Topology::BoundedBox { half_x, half_y } => {
                h.write_str("bounded_box");
                h.write_i64(*half_x);
                h.write_i64(*half_y);
            }
            Topology::ToroidalXy { period_x, period_y } => {
                h.write_str("toroidal_xy");
                h.write_i64(*period_x);
                h.write_i64(*period_y);
            }
        }
    }
}

/// Chính sách khi một hành động cố vượt biên thế giới.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgePolicy {
    /// Từ chối rõ ràng. Mặc định, và là lựa chọn đúng gần như luôn luôn.
    Reject,
    /// Kẹp vào biên.
    Clamp,
}

/// Ảnh chụp profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationProfile {
    /// Định danh profile.
    pub id: String,
    /// Phiên bản. Đổi phiên bản là đổi thế giới, nên nó cần migration (`§7.5`).
    pub version: u32,
    /// Hình học.
    pub topology: Topology,
    /// Chính sách biên.
    pub edge_policy: EdgePolicy,

    // ── Bước 1: khung vũ trụ ────────────────────────────────────────────────
    /// Mực biển, tính bằng mét.
    pub sea_level_m: i64,
    /// Trọng lực, phần nghìn của `g` Trái Đất.
    pub gravity_milli_g: i64,
    /// Cường độ mana nền, Q16.16 trong `[0,1]`.
    pub mana_density: Fx,

    // ── Bước 2–3: địa chất và độ cao ────────────────────────────────────────
    /// Bước lưới lục địa, tính bằng ô. Lớn thì lục địa rộng.
    pub continental_cell: i64,
    /// Bước lưới dãy núi.
    pub mountain_cell: i64,
    /// Bước lưới đồi.
    pub hill_cell: i64,
    /// Bước lưới chi tiết cục bộ.
    pub detail_cell: i64,
    /// Biên độ độ cao, tính bằng mét, cho mỗi tầng theo thứ tự trên.
    pub elevation_amplitudes_m: [i64; 4],

    // ── Bước 5: khí hậu ─────────────────────────────────────────────────────
    /// Bước lưới của trường khí hậu.
    ///
    /// Đây là **trường khí hậu procedural**, không phải vĩ độ thiên văn
    /// (`§7.4`). Ghi rõ ra để không ai suy diễn ngày dài đêm ngắn từ nó.
    pub climate_cell: i64,
    /// Nhiệt độ ở mực biển tại trung tâm dải ấm, mK.
    pub base_temp_mk: i64,
    /// Nhiệt giảm theo độ cao, mK mỗi 1000 m.
    pub lapse_rate_mk_per_km: i64,
}

impl Default for GenerationProfile {
    fn default() -> Self {
        GenerationProfile {
            id: "gaia".to_owned(),
            version: 1,
            topology: Topology::InfiniteCartesian,
            edge_policy: EdgePolicy::Reject,
            sea_level_m: 0,
            gravity_milli_g: 1_000,
            mana_density: Fx::from_raw(6_554), // ≈ 0.1
            continental_cell: 4_096,
            mountain_cell: 1_024,
            hill_cell: 128,
            detail_cell: 16,
            elevation_amplitudes_m: [2_400, 1_200, 220, 40],
            climate_cell: 8_192,
            base_temp_mk: 295_000, // ≈ 21.85 °C
            lapse_rate_mk_per_km: 6_500,
        }
    }
}

impl GenerationProfile {
    /// Các tầng nhiễu độ cao, theo thứ tự lớn → nhỏ.
    pub fn elevation_octaves(&self) -> [Octave; 4] {
        let cells = [
            self.continental_cell,
            self.mountain_cell,
            self.hill_cell,
            self.detail_cell,
        ];
        std::array::from_fn(|i| Octave {
            cell: cells[i].max(1),
            amplitude: Fx::from_int(self.elevation_amplitudes_m[i]).unwrap_or(Fx::ZERO),
        })
    }

    /// Hash của ảnh chụp.
    ///
    /// Nằm trong state hash và trong save. Hai world có cùng seed nhưng khác
    /// profile là hai world khác nhau, và điều đó phải nhìn thấy được ngay ở
    /// tầng hash chứ không phải sau khi đi bộ 4000 ô mới nhận ra.
    pub fn snapshot_hash(&self) -> StateHash {
        let mut h = StateHasher::with_domain("mow.genprofile.v1");
        self.canonical_hash(&mut h);
        h.finish()
    }
}

impl CanonicalHash for GenerationProfile {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(&self.id);
        h.write_u64(u64::from(self.version));
        self.topology.canonical_hash(h);
        h.write_str(match self.edge_policy {
            EdgePolicy::Reject => "reject",
            EdgePolicy::Clamp => "clamp",
        });
        h.write_i64(self.sea_level_m);
        h.write_i64(self.gravity_milli_g);
        h.write_i64(self.mana_density.raw());
        h.write_i64(self.continental_cell);
        h.write_i64(self.mountain_cell);
        h.write_i64(self.hill_cell);
        h.write_i64(self.detail_cell);
        h.write_seq(self.elevation_amplitudes_m.iter().copied(), |hh, v| {
            hh.write_i64(v);
        });
        h.write_i64(self.climate_cell);
        h.write_i64(self.base_temp_mk);
        h.write_i64(self.lapse_rate_mk_per_km);
    }
}
