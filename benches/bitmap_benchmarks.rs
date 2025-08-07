use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bitmap_indexer::{BitmapIndex};
use tempfile::TempDir;

fn benchmark_bitmap_creation(c: &mut Criterion) {
    c.bench_function("bitmap_creation", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let index = BitmapIndex::open(temp_dir.path());
            black_box(index)
        })
    });
}

fn benchmark_wah_compression(c: &mut Criterion) {
    use bitmap_indexer::wah::WahEncoder;
    
    c.bench_function("wah_encode_random", |b| {
        let data = vec![0xAA, 0x55, 0xFF, 0x00]; // Pattern data
        b.iter(|| {
            let mut encoder = WahEncoder::new();
            for &byte in &data {
                encoder.append_bits(&[byte], 8).unwrap();
            }
            let words = encoder.finish().unwrap();
            black_box(words)
        })
    });

    c.bench_function("wah_encode_fill", |b| {
        b.iter(|| {
            let mut encoder = WahEncoder::new();
            encoder.fill(true, 10000).unwrap();
            let words = encoder.finish().unwrap();
            black_box(words)
        })
    });
}

criterion_group!(benches, benchmark_bitmap_creation, benchmark_wah_compression);
criterion_main!(benches);