use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use xmltree::{BinaryStringFormat, Document, DocumentSourceRef};

const SRC: &str = include_str!("../examples/example.xml");
const BIN: &[u8] = include_bytes!("../examples/example.bin");

fn parse_xml(src: &str, arena: &DocumentSourceRef) {
    let _ = Document::new(arena, src).unwrap();
}

fn parse_bin(src: &[u8], arena: &DocumentSourceRef) {
    let _ = Document::from_bin(src, BinaryStringFormat::Header, arena).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let arena = DocumentSourceRef::default();

    c.bench_function("parse_xml", |b| {
        b.iter(|| parse_xml(black_box(SRC), black_box(&arena)))
    });

    c.bench_function("parse_bin", |b| {
        b.iter(|| parse_bin(black_box(BIN), black_box(&arena)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
