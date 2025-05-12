use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use xmltree::Document;

const SRC: &str = include_str!("../examples/example.xml");
const BIN: &[u8] = include_bytes!("../examples/example.bin");

fn parse_xml(src: &str) {
    let _ = Document::parse_str(src).unwrap();
}

fn parse_bin(src: &[u8]) {
    let _ = Document::from_bin(src).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("parse_xml", |b| b.iter(|| parse_xml(black_box(SRC))));

    c.bench_function("parse_bin", |b| b.iter(|| parse_bin(black_box(BIN))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
