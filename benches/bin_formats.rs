use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use xmltree::{Document, OwnedDocument};

const SRC: &str = include_str!("../examples/example.xml");

fn borrowed_doc(src: &[u8]) {
    let _ = Document::from_bin(src).unwrap();
}

fn owned_doc(src: &[u8]) {
    let _ = OwnedDocument::from_bin(src).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let document = Document::parse_str(SRC).unwrap();
    let borrowed_bin = document.to_bin().unwrap();
    let owned_bin = document.to_owned().to_bin().unwrap();

    c.bench_function("borrowed -> borrowed", |b| {
        b.iter(|| borrowed_doc(black_box(&borrowed_bin)))
    });

    c.bench_function("borrowed -> owned", |b| {
        b.iter(|| owned_doc(black_box(&borrowed_bin)))
    });

    c.bench_function("owned -> borrowed", |b| {
        b.iter(|| borrowed_doc(black_box(&owned_bin)))
    });

    c.bench_function("owned -> owned", |b| {
        b.iter(|| owned_doc(black_box(&owned_bin)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
