//! Bộ gen **nén** (`idea.md §9.5.2`).
//!
//! > Suy ra từ cha mẹ + seed tái tổ hợp + đột biến, **không lưu genome đầy đủ**.
//!
//! ## Vì sao nén
//!
//! Một genome đầy đủ 20 000 locus là 20 KB mỗi cá thể. Một thế giới có một
//! triệu sinh vật qua hai trăm năm mô phỏng: đó là hàng terabyte cho một thứ
//! mà gần như không ai đọc trực tiếp.
//!
//! Nhưng lý do thật sự sâu hơn tiết kiệm chỗ. Genome đầy đủ là **dữ liệu**, và
//! dữ liệu phải được sao chép khi fork nhánh, gửi qua mạng, đưa vào state hash.
//! Genome nén là một **hàm**: `(cha, mẹ, seed) → kiểu hình`. Hàm thì fork miễn
//! phí, gửi trong 24 byte, và luôn cho cùng kết quả.
//!
//! Cái giá: không thể sửa một gen đơn lẻ của một cá thể đã sinh ra. Đó là cái
//! giá đúng — trong thế giới thật cũng không sửa được, và mọi cơ chế muốn đổi
//! kiểu hình đều đi qua effect chứ không qua gen.
//!
//! ## Chỉ thừa kế đặc tính cơ bản
//!
//! `§9.5.2` giới hạn rõ. Di truyền định lượng đầy đủ — `h²` theo từng tính
//! trạng, tương tác gen×môi trường, hệ số cận huyết — thuộc `PD-22`, vì nó là
//! toán nhiều tính trạng và không cần thiết cho một khu định cư tối thiểu.

use mow_math::{CanonicalHash, DetRng, Prob, RngStreams, StateHasher, WorldSeed};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Bộ gen nén. **24 byte**, bất kể loài phức tạp thế nào.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genome {
    /// Seed của cá thể. Mọi tính trạng suy ra từ đây.
    pub genotype_seed: u64,
    /// Loài, để biết dùng bảng tính trạng nào.
    pub species: u32,
    /// Số thế hệ tính từ genesis. Dùng cho `PD-22` (cận huyết) và cho biên niên sử.
    pub generation: u32,
    /// Seed của cha mẹ, gộp lại. Cho phép truy huyết thống mà không lưu cây.
    pub lineage: u64,
}

impl Genome {
    /// Bộ gen gốc, sinh từ genesis.
    pub fn founder(seed: u64, species: u32) -> Genome {
        Genome {
            genotype_seed: seed,
            species,
            generation: 0,
            lineage: seed,
        }
    }

    /// Lai hai bộ gen.
    ///
    /// `recomb_seed` phải là hàm của `(cha, mẹ, tick sinh)` chứ không phải một
    /// số rút từ một bộ đếm — nếu không, thứ tự các ca sinh trong một tick sẽ
    /// quyết định con nào giống ai, và thứ tự đó không phải một phần của thế
    /// giới.
    pub fn breed(a: Genome, b: Genome, recomb_seed: u64) -> Genome {
        let mut h = StateHasher::with_domain("mow.genome.recomb.v1");
        h.write_u64(a.genotype_seed);
        h.write_u64(b.genotype_seed);
        h.write_u64(recomb_seed);
        let genotype_seed = u64::from_le_bytes(h.finish().0[..8].try_into().expect("32 ≥ 8"));

        let mut hl = StateHasher::with_domain("mow.genome.lineage.v1");
        hl.write_u64(a.lineage);
        hl.write_u64(b.lineage);
        let lineage = u64::from_le_bytes(hl.finish().0[..8].try_into().expect("32 ≥ 8"));

        Genome {
            genotype_seed,
            species: a.species,
            generation: a.generation.max(b.generation).saturating_add(1),
            lineage,
        }
    }

    /// Áp đột biến.
    ///
    /// `rate` là xác suất **mỗi locus**, và đây chính là chỗ mà miền
    /// [`Prob`] phải tồn tại: `2.1e-8` lưu vào Q16.16 sẽ thành 0 và đột biến
    /// biến mất khỏi thế giới. Xem `mow-math` để biết phép đo.
    ///
    /// Số locus không được mô hình hóa riêng lẻ; ta chỉ hỏi "có ít nhất một đột
    /// biến không" trên toàn bộ genome, rồi trộn lại seed nếu có.
    pub fn mutate(self, rate: Prob, loci: u64, rng: &mut DetRng) -> Genome {
        let co_dot_bien = rate.at_least_once_in(loci).sample(rng);
        if !co_dot_bien {
            return self;
        }
        let mut h = StateHasher::with_domain("mow.genome.mutate.v1");
        h.write_u64(self.genotype_seed);
        h.write_u64(rng.gen::<u64>());
        Genome {
            genotype_seed: u64::from_le_bytes(h.finish().0[..8].try_into().expect("32 ≥ 8")),
            ..self
        }
    }

    /// Một tính trạng cơ bản, thang `[0, 1000]`.
    ///
    /// Hàm thuần của `(genotype_seed, tên tính trạng)`. Nghĩa là hai anh em ruột
    /// khác nhau vì `genotype_seed` khác nhau, còn cùng một cá thể luôn có cùng
    /// chiều cao — kể cả sau khi save và nạp lại, kể cả trên máy khác.
    pub fn trait_value(self, name: &str) -> i64 {
        let mut h = StateHasher::with_domain("mow.genome.trait.v1");
        h.write_u64(self.genotype_seed);
        h.write_u64(u64::from(self.species));
        h.write_str(name);
        let v = u32::from_le_bytes(h.finish().0[..4].try_into().expect("32 ≥ 4"));
        i64::from(v % 1001)
    }

    /// Độ tương đồng huyết thống thô với một bộ gen khác, thang `[0, 64]`.
    ///
    /// Đếm số bit trùng của `lineage`. Đây là **xấp xỉ**, đủ để phát hiện họ
    /// hàng gần cho `PD-22`, và cố ý không chính xác hơn: hệ số cận huyết thật
    /// cần cây phả hệ, và cây phả hệ là thứ mà genome nén đánh đổi đi.
    pub fn lineage_similarity(self, other: Genome) -> u32 {
        (!(self.lineage ^ other.lineage)).count_ones()
    }
}

impl CanonicalHash for Genome {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.genotype_seed);
        h.write_u64(u64::from(self.species));
        h.write_u64(u64::from(self.generation));
        h.write_u64(self.lineage);
    }
}

/// Sinh seed tái tổ hợp cho một ca sinh.
///
/// Hàm thuần của `(cha, mẹ, tick)`, nên nó không phụ thuộc thứ tự xử lý các ca
/// sinh trong cùng một tick.
pub fn recombination_seed(world: WorldSeed, a: Genome, b: Genome, birth_tick: u64) -> u64 {
    let streams = RngStreams::new(world);
    let mut rng = streams.stream_at(
        mow_math::rng::streams::LIFE_RECOMBINATION,
        &[a.genotype_seed, b.genotype_seed, birth_tick],
    );
    rng.gen()
}
