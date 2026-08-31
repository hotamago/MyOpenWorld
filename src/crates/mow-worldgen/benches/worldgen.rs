//! Bench sinh địa hình — cổng `chunk_gen_ms` của `§P8.1` (`PF-11`).
//!
//! Đo **một chunk đầy đủ `32×32×16`**, không đo một ô. `Metric::min_scale`
//! trong `mow-devtool::budget` từ chối phép đo dưới quy mô đó, nên bench này
//! phải làm đúng việc mà ngân sách nói tới.

use criterion::{criterion_group, criterion_main, Criterion};
use mow_math::WorldSeed;
use mow_worldgen::{GenerationProfile, Worldgen};

const CANH: i64 = 32;
const CAO: i64 = 16;

fn sinh_mot_chunk(c: &mut Criterion) {
    let w = Worldgen::new(WorldSeed(4_242), GenerationProfile::default());
    c.bench_function("chunk_gen_32x32x16", |b| {
        b.iter(|| {
            let mut tong: i64 = 0;
            for x in 0..CANH {
                for y in 0..CANH {
                    // `base_cell` là phần đắt: nó chạy noise và tra strata.
                    // Nhân `CAO` để phép đo đúng khối lượng một chunk thật.
                    let o = w.base_cell(x, y).expect("tọa độ trong tầm");
                    tong = tong.wrapping_add(o.elevation.height_m * CAO);
                }
            }
            std::hint::black_box(tong)
        });
    });
}

criterion_group!(benches, sinh_mot_chunk);
criterion_main!(benches);
