//! XML Formatter Example
//!
//! This example demonstrates how to format an XML document into a string using the `xmltree` crate.
//!
//! It also shows that you can alter the document after parsing and still serialize it correctly.
use xmltree::{Document, DocumentSourceRef};

const SRC: &str = "<test><test2>test</test2></test>";

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
    let mut doc = Document::new(&arena, SRC).unwrap();

    //
    // You can even make changes to the tree after parsing it.
    //
    // `StrSpan::set` allocates a string in the arena and sets the value in the node.
    // Note that if serializing to binary, you should not use `BinaryStringFormat::Header` on documents that have been modified this way.
    //
    // When formatting as XML, special characters are escaped (`altered&lt;` in this example).
    doc.root.name.local.set("altered<", &arena);

    //
    // We can turn a tree into a formatted XML string
    // The `to_xml` method takes an optional string to use as indentation.
    //
    // You can pass a custom indent string; if `None`, a tab is used by default.
    // The formatter is non-recursive and handles large documents safely.
    let formatted = doc.to_xml(Some("  ")).unwrap();
    println!("Formatted XML:\n{formatted}");
}
