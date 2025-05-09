//! XML Serializer Example
//!
//! This example demonstrates how to serialize an XML document to a binary format and back using the `xmltree` crate.
//!
//! Note that the serializer is generalized, and can be extended to custom types, if you want to embed XML docs, or portions, into other objects
use xmltree::{BinaryStringFormat, Document, DocumentSourceRef};

const BYTES: &[u8] = include_bytes!("example.bin");

fn main() {
    //
    // Arena for allocating string references.
    // This is a bump allocator that will hold the source string
    let arena = DocumentSourceRef::default();

    //
    // Parse the XML document from a flat binary format
    // This is considerably faster than parsing from a string, with a small size overhead.
    //
    // For this example document, the binary format is ~3kB, vs ~2kB for the XML string.
    // But can be parsed in ~6us, vs ~16us for the XML string (by my testing).
    //
    // The tradeoff is that it should only be used on XML documents you trust
    // due to the use of recursion in the parser.
    //
    // The `true` here indicates that the document has a header with the complete source string.
    // - This stores all strings as offsets and is a lot faster and smaller, but can only be used on documents that are not modified after parsing.
    let mut document = Document::from_bin(BYTES, BinaryStringFormat::Header, &arena).unwrap();

    //
    // You can also serialize any document back to binary format:
    // We use None here to indicate that we should embed inline strings (instead of a shared header).
    //
    // This is more flexible, as it works on documents you have modified, but ~double the size of the output.
    // - Workaround: use `Document::to_xml()`, then re-parse to get a clean headerless version you can serialize with source
    //
    // Once again, you should only use this on documents you trust, as it uses recursion, and can overflow the stack on very large documents.
    let bytes = document.to_bin(None).unwrap();
    println!(
        "Inline-string binary size: {:.2}kB",
        bytes.len() as f64 / 1024.0
    );
    println!(
        "Header-string binary size: {:.2}kB",
        BYTES.len() as f64 / 1024.0
    );

    //
    // You can dramatically reduce the size of the binary format by using `strip_metadata`:
    // Note that you can still use `BinaryStringFormat::Header` here, but it does not make much sense, as the entire document is still included in the binary.
    document.strip_metadata();
    println!(
        "Stripped binary size: {:.2}kB",
        document.to_bin(None).unwrap().len() as f64 / 1024.0
    );

    //
    // By playing with `BinaryStringFormat` and `strip_metadata`, you can select a balance between speed and size:
    // - `BinaryStringFormat::Header` with no metadata is the fastest option, but must be used on documents that are not modified after parsing.
    // - `BinaryStringFormat::Inline` is the most flexible, but with a performance cost. Strip metadata to get the smallest size and reasonable speed.
    //
    // Here is a more detailed breakdown of the options:
    //
    // |                      | Metadata Incl.                                 | Metadata Stripped                     |
    // |----------------------|------------------------------------------------|---------------------------------------|
    // | Header strings       | Faster (~7.4 µs), mid size                     | Fastest (~6.8 µs), mid size           |
    // | Inline strings       | Slowest (~17.7 µs), largest size, but flexible | Mid speed (10.0 µs), Smallest size    |
    //
    // In this example, the binary sizes are:
    // `BinaryStringFormat::Header` - 2.82kB
    // `BinaryStringFormat::Inline` - 4.30kB
    // `BinaryStringFormat::Inline` with metadata stripped - 2.08kB`
}
