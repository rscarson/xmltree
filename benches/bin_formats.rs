use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use xmltree::{BinaryStringFormat, Document, DocumentSourceRef};

const SRC: &str = include_str!("../examples/example.xml");

fn header_strings(src: &[u8], arena: &DocumentSourceRef) {
    let _ = Document::from_bin(src, BinaryStringFormat::Header, arena).unwrap();
}

fn inline_strings(src: &[u8], arena: &DocumentSourceRef) {
    let _ = Document::from_bin(src, BinaryStringFormat::Inline, arena).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let arena = DocumentSourceRef::default();
    let mut document = Document::new(&arena, SRC).unwrap();
    let header_bin = document.to_bin(Some(SRC)).unwrap();
    let inline_bin = document.to_bin(None).unwrap();

    document.strip_metadata();
    let stripped_inline = document.to_bin(None).unwrap();
    let stripped_header = document.to_bin(Some(SRC)).unwrap();

    c.bench_function("header_strings", |b| {
        b.iter(|| header_strings(black_box(&header_bin), black_box(&arena)))
    });

    c.bench_function("inline_strings", |b| {
        b.iter(|| inline_strings(black_box(&inline_bin), black_box(&arena)))
    });

    c.bench_function("stripped_bin", |b| {
        b.iter(|| inline_strings(black_box(&stripped_inline), black_box(&arena)))
    });

    c.bench_function("stripped_header", |b| {
        b.iter(|| header_strings(black_box(&stripped_header), black_box(&arena)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
