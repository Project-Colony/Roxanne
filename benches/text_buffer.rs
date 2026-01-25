use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use roxanne::editor::TextBuffer;
use std::time::{Duration, Instant};

fn build_text(lines: usize, line_len: usize) -> String {
    let line = "a".repeat(line_len);
    std::iter::repeat(line)
        .take(lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn bench_buffer_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer_build");
    for lines in [1_000usize, 10_000, 50_000, 100_000, 200_000] {
        let payload = build_text(lines, 80);
        group.throughput(Throughput::Bytes(payload.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(lines), &payload, |b, text| {
            b.iter(|| TextBuffer::from(text));
        });
    }
    group.finish();
}

fn bench_insert_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer_edit");
    let payload = build_text(10_000, 120);
    group.throughput(Throughput::Bytes(payload.len() as u64));

    group.bench_function("insert_middle", |b| {
        b.iter(|| {
            let mut buffer = TextBuffer::from(&payload);
            let midpoint = buffer.position_from_index(payload.len() / 2);
            buffer.insert(midpoint, "INSERTED");
        });
    });

    group.bench_function("delete_span", |b| {
        b.iter(|| {
            let mut buffer = TextBuffer::from(&payload);
            let start = buffer.position_from_index(payload.len() / 3);
            let end = buffer.position_from_index(payload.len() / 3 + 128);
            buffer.delete_range(start, end);
        });
    });

    group.finish();
}

fn bench_repeated_edits(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer_repeated_edits");
    let payload = build_text(50_000, 120);
    group.throughput(Throughput::Bytes(payload.len() as u64));

    group.bench_function("insert_delete_burst", |b| {
        b.iter_batched(
            || TextBuffer::from(&payload),
            |mut buffer| {
                for step in 0..200usize {
                    let insert_at = buffer.position_from_index((step * 64) % payload.len());
                    buffer.insert(insert_at, "BATCH");
                    let start_index = (step * 96) % payload.len();
                    let end_index = ((step * 96) + 32) % payload.len();
                    let (start_index, end_index) = if start_index <= end_index {
                        (start_index, end_index)
                    } else {
                        (end_index, start_index)
                    };
                    let delete_start = buffer.position_from_index(start_index);
                    let delete_end = buffer.position_from_index(end_index);
                    buffer.delete_range(delete_start, delete_end);
                }
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("cursor_walk", |b| {
        b.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                let buffer = TextBuffer::from(&payload);
                let mut cursor = buffer.position_from_index(0);
                let begin = Instant::now();
                for offset in (0..payload.len()).step_by(256) {
                    cursor = buffer.position_from_index(offset);
                }
                elapsed += begin.elapsed();
                drop(cursor);
            }
            elapsed
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_buffer_construction,
    bench_insert_delete,
    bench_repeated_edits
);
criterion_main!(benches);
