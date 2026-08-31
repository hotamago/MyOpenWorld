//! Dòng ngẫu nhiên **có tên** (`idea.md §19.6`).
//!
//! Một bộ sinh dùng chung cho cả thế giới nghe có vẻ vô hại cho tới khi bạn
//! thêm một hệ thống mới. Nếu thời tiết và đột biến rút số từ cùng một dòng,
//! thì thêm một lời gọi vào code thời tiết sẽ **đổi toàn bộ lịch sử di truyền**
//! của thế giới. Mọi save cũ replay ra kết quả khác, và không có gì trong log
//! chỉ ra nguyên nhân.
//!
//! Cách chữa là mỗi hệ thống rút từ một dòng riêng, dẫn xuất từ seed gốc bằng
//! một hàm thuần của **tên dòng**:
//!
//! ```text
//! stream_seed = BLAKE3("mow.rng.v1" ‖ world_seed ‖ name ‖ toa_do_logic)
//! ```
//!
//! Nhờ vậy hai tính chất cùng đúng: thêm hệ thống mới không đụng vào hệ thống
//! cũ, và cùng một sự kiện ở cùng một tick luôn rút cùng một số dù thứ tự xử lý
//! trong tick có đổi.

use crate::hash::{StateHash, StateHasher};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Nhãn phiên bản của sơ đồ dẫn xuất. Đổi giá trị này là đổi toàn bộ dòng
/// ngẫu nhiên của mọi thế giới, nên nó phải đi kèm một bước migration.
const DERIVATION_DOMAIN: &str = "mow.rng.v1";

/// Bộ sinh dùng trên đường commit.
///
/// `ChaCha8Rng` chứ không phải bộ sinh mặc định của `rand`: `ChaCha8Rng` hứa
/// **ổn định giữa các phiên bản** — cùng seed cho cùng dãy byte, mãi mãi. Bộ
/// sinh mặc định không hứa điều đó, và một lần nâng phụ thuộc sẽ âm thầm làm
/// mọi replay sai.
pub type DetRng = ChaCha8Rng;

/// Seed gốc của một thế giới.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldSeed(pub u64);

/// Tên dòng ngẫu nhiên.
///
/// Quy ước đặt tên: `<miền>.<hệ thống>.<mục đích>`, ví dụ `life.genome.mutation`
/// hay `worldgen.terrain.ridge`. Tên là **một phần của hợp đồng**: đổi tên dòng
/// đổi kết quả, nên đổi tên phải đi kèm migration giống như đổi schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamName(pub &'static str);

/// Tập dòng ngẫu nhiên của một thế giới.
#[derive(Debug, Clone, Copy)]
pub struct RngStreams {
    seed: WorldSeed,
}

impl RngStreams {
    /// Dựng từ seed thế giới.
    pub fn new(seed: WorldSeed) -> RngStreams {
        RngStreams { seed }
    }

    /// Seed gốc.
    pub fn seed(self) -> WorldSeed {
        self.seed
    }

    /// Dòng thuần theo tên — dùng cho thứ chỉ xảy ra một lần, như worldgen.
    pub fn stream(self, name: StreamName) -> DetRng {
        self.derive(name, &[])
    }

    /// Dòng theo tên và **tọa độ logic**.
    ///
    /// Tọa độ logic là bất cứ thứ gì định danh sự kiện một cách ổn định: `(tick,
    /// entity_id)`, `(chunk_x, chunk_y)`, `(parent_a, parent_b, birth_tick)`.
    /// Vì seed dẫn xuất từ chính tọa độ đó chứ không từ một bộ đếm, kết quả
    /// **không phụ thuộc thứ tự** các sự kiện được xử lý trong tick — và đó là
    /// điều kiện để job song song chỉ tạo proposal rồi commit tuần tự
    /// (`plan.md §P10.3`) mà vẫn ra cùng một thế giới.
    pub fn stream_at(self, name: StreamName, coords: &[u64]) -> DetRng {
        self.derive(name, coords)
    }

    fn derive(self, name: StreamName, coords: &[u64]) -> DetRng {
        let mut h = StateHasher::with_domain(DERIVATION_DOMAIN);
        h.write_u64(self.seed.0);
        h.write_str(name.0);
        h.write_seq(coords.iter().copied(), |hh, c| {
            hh.write_u64(c);
        });
        let StateHash(bytes) = h.finish();
        ChaCha8Rng::from_seed(bytes)
    }
}

/// Tên dòng chuẩn của engine.
///
/// Gom vào một chỗ để hai hệ thống không vô tình đặt trùng tên — trùng tên là
/// tương quan ngầm giữa hai hệ thống lẽ ra độc lập, và nó biểu hiện thành
/// những mẫu hình kỳ lạ mà không ai truy được nguồn.
pub mod streams {
    use super::StreamName;

    /// Địa hình cơ bản.
    pub const WORLDGEN_TERRAIN: StreamName = StreamName("worldgen.terrain");
    /// Thủy văn: sông, lưu vực.
    pub const WORLDGEN_HYDRO: StreamName = StreamName("worldgen.hydrology");
    /// Phân bố quần xã sinh vật.
    pub const WORLDGEN_BIOME: StreamName = StreamName("worldgen.biome");
    /// Đặt tài nguyên.
    pub const WORLDGEN_RESOURCE: StreamName = StreamName("worldgen.resource");

    /// Tái tổ hợp khi sinh sản.
    pub const LIFE_RECOMBINATION: StreamName = StreamName("life.genome.recombination");
    /// Đột biến điểm.
    pub const LIFE_MUTATION: StreamName = StreamName("life.genome.mutation");
    /// Tử vong theo đường cong lão hóa.
    pub const LIFE_MORTALITY: StreamName = StreamName("life.senescence.mortality");
    /// Lây bệnh theo tiếp xúc.
    pub const LIFE_INFECTION: StreamName = StreamName("life.disease.infection");

    /// Kết quả hành động có yếu tố may rủi.
    pub const ACTION_OUTCOME: StreamName = StreamName("action.outcome");
    /// Chất lượng chế tác.
    pub const CRAFT_QUALITY: StreamName = StreamName("items.craft.quality");
    /// Hao mòn.
    pub const ITEM_WEAR: StreamName = StreamName("items.wear");

    /// Phép phản chủ.
    pub const MAGIC_BACKFIRE: StreamName = StreamName("magic.backfire");
    /// Ban phát thiên phú.
    pub const MAGIC_TALENT: StreamName = StreamName("magic.talent.grant");

    /// Nhân chứng có thấy hay không.
    pub const SOCIAL_WITNESS: StreamName = StreamName("society.crime.witness");
    /// Nhiễu khi tin đồn lan.
    pub const SOCIAL_RUMOR: StreamName = StreamName("society.message.drift");
    /// Lấy mẫu tính cách khi một người được sinh ra.
    ///
    /// Tách khỏi [`LIFE_RECOMBINATION`] có chủ đích: tính cách và gen là hai
    /// thứ khác nhau, và dùng chung dòng sẽ buộc chúng tương quan ngầm — anh
    /// em ruột sẽ có tính cách giống nhau vì một lý do thuần kỹ thuật.
    pub const SOCIAL_PERSONALITY: StreamName = StreamName("society.personality.sample");

    /// Chọn storylet khi salience bằng nhau.
    pub const DIRECTOR_TIEBREAK: StreamName = StreamName("director.salience.tiebreak");
}
