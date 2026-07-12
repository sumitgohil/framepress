//! Criterion benchmark for the adaptive optimizer's hot path.
//!
//! Run with `cargo bench --bench optimizer`. Useful for tracking regressions
//! in candidate scoring latency as we add engines or change the scoring
//! algorithm.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use image::{ImageBuffer, Rgba};
use tinydrop_core::{default_registry, AdaptiveOptimizer, CompressionPreset};

fn bench_optimizer(c: &mut Criterion) {
    // Build a representative noisy RGBA fixture in a tempdir. Criterion
    // requires measurements to be deterministic and side-effect free, so
    // we generate the fixture once and reuse it across iterations.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bench.png");
    let img = ImageBuffer::<Rgba<u8>, _>::from_fn(512, 512, |x, y| {
        Rgba([
            (x.wrapping_mul(7)) as u8,
            (y.wrapping_mul(11)) as u8,
            ((x ^ y).wrapping_mul(13)) as u8,
            255,
        ])
    });
    img.save(&input).unwrap();

    let optimizer = AdaptiveOptimizer::new(default_registry());

    let mut group = c.benchmark_group("adaptive_optimizer");
    group.throughput(Throughput::Bytes(512 * 512 * 4));

    group.bench_function("website_preset_512x512_png", |b| {
        b.iter(|| {
            let out = dir.path().join("out.png");
            let _ = optimizer.optimize(&input, CompressionPreset::Website, &out);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_optimizer);
criterion_main!(benches);
