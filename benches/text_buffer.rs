use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use roxanne::editor::TextBuffer;

fn build_text(lines: usize, line_len: usize) -> String {
    let line = "a".repeat(line_len);
    std::iter::repeat(line)
        .take(lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn bench_buffer_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer_build");
    for lines in [1_000usize, 10_000, 50_000] {
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

criterion_group!(benches, bench_buffer_construction, bench_insert_delete);
criterion_main!(benches);
