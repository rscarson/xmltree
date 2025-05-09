//! XML Parser Example
//!
//! This example demonstrates how to parse an XML document using the `xmltree` crate.
//!
//! It shows how to create a bump allocator for string references, parse the XML document, and access its attributes and locations in the source string.
use xmltree::{Document, DocumentSourceRef};

const DOCUMENT: &str = include_str!("example.xml");

fn main() {
    //
    // Arena for allocating string references.
    // This is a bump allocator that will hold the source string
    let arena = DocumentSourceRef::default();

    //
    // Parse the XML document from the source string
    //
    // If an error occurs, the error message is designed to be human-readable;
    //      | </example>
    //      = At 25:5
    //      = Unclosed tag: bookstore
    //
    // The parser uses no recursion, so it can handle deeply nested documents without stack overflow.
    // It's also zero-copy and designed for speed.
    let document = match Document::new(&arena, DOCUMENT) {
        Ok(doc) => doc,
        Err(e) => panic!("Error parsing XML document:\n{e}"),
    };

    //
    // Every single part of the tree maintains a reference to the source!
    //
    // Also note that if a node has duplicate attributes, the last one is used,
    // But all of them are stored in the tree.
    if let Some(name) = document.root.get_attribute(None, "name") {
        println!(
            "The bookstore name is defined at byte offset {}",
            name.span.start
        );

        // We can also use the source to get the exact location
        let (row, col) = name.span.position(DOCUMENT);
        println!("The bookstore name is at row {}, column {}", row, col);
    }
}
