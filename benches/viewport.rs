use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use roxanne::editor::{TextBuffer, ViewportCache};
use std::time::{Duration, Instant};

fn build_text(lines: usize, line_len: usize) -> String {
    let line = "a".repeat(line_len);
    std::iter::repeat(line)
        .take(lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn bench_viewport_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("viewport_cache_update");
    let height = 60usize;

    for lines in [10_000usize, 50_000] {
        let payload = build_text(lines, 120);
        let buffer = TextBuffer::from(&payload);
        let max_start = lines.saturating_sub(height).max(1);
        group.throughput(Throughput::Elements(height as u64));
        group.bench_with_input(BenchmarkId::from_parameter(lines), &buffer, |b, buffer| {
            b.iter_custom(|iters| {
                let mut cache = ViewportCache::new();
                let mut start_line = 0usize;
                let mut elapsed = Duration::ZERO;

                for _ in 0..iters {
                    let begin = Instant::now();
                    cache.update(buffer, start_line, height);
                    elapsed += begin.elapsed();
                    start_line = (start_line + 1) % max_start;
                }

                elapsed
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_viewport_update);
criterion_main!(benches);
