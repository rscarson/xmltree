//! XML Serializer Example
//!
//! This example demonstrates how to serialize an XML document to a binary format and back using the `xmltree` crate.
//!
//! Note that the serializer is generalized, and can be extended to custom types, if you want to embed XML docs, or portions, into other objects
use xmltree::{Document, error::XmlResult};

const BYTES: &[u8] = include_bytes!("example.bin");

fn main() -> XmlResult<()> {
    //
    // Parse the XML document from a flat binary format
    // This is considerably faster than parsing from a string, with a small size overhead.
    //
    // For this example document, the binary format is ~3kB, vs ~2kB for the XML string.
    // But can be parsed in ~5us, vs ~15us for the XML string (by my testing).
    //
    // The tradeoff is that it should only be used on XML documents you trust, due to the use of recursion in the parser.
    let document = Document::from_bin(BYTES)?;

    //
    // You can also serialize any document back to binary format:
    // Here we convert to an owned document first, which strips all source span information.
    //
    // The result is a far smaller binary, that does not sacrifice any speed at deserialization time.
    let bytes = document.to_owned().to_bin()?;
    println!(
        "Owned document binary size: {:.2}kB",
        bytes.len() as f64 / 1024.0
    );
    println!(
        "Parsed document binary size: {:.2}kB",
        BYTES.len() as f64 / 1024.0
    );

    Ok(())
}
