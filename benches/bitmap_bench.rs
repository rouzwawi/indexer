//! Placeholder benchmarks that avoid using WahEncoder/WahDecoder.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// Simple reversible transform to stand in for encode/decode work.
fn placeholder_encode(input: &[u8]) -> Vec<u8> {
    input.iter().map(|b| b.reverse_bits()).collect()
}

fn placeholder_decode(input: &[u8]) -> Vec<u8> {
    // Reverse again to "decode"
    input.iter().map(|b| b.reverse_bits()).collect()
}

fn benchmark_placeholder_encode(c: &mut Criterion) {
    let test_data = vec![0xAA; 1024]; // 1 KiB of data
    c.bench_function("placeholder_encode_1kib", |b| {
        b.iter(|| black_box(placeholder_encode(&test_data)))
    });
}

fn benchmark_placeholder_decode(c: &mut Criterion) {
    let test_data = vec![0xAA; 1024];
    let encoded = placeholder_encode(&test_data);
    c.bench_function("placeholder_decode_1kib", |b| {
        b.iter(|| black_box(placeholder_decode(&encoded)))
    });
}

criterion_group!(
    benches,
    benchmark_placeholder_encode,
    benchmark_placeholder_decode
);
criterion_main!(benches);
