//! XML Writer Example
//!
//! This example creates a new XML document with a root node and a child node, and adds an attribute to the root node.
//!
//! It demonstrates how to use the `xmltree` crate to create XML documents programmatically.
//!
//! Note that this being possible is more or less a side-effect of how the crate works, and has some limitations:
//! - Serialization with a source header does not work, because there is no source from which to header
//! - Everything is still just `&str` references
//!     - If you use the `from_unallocated` functions, any strings will live for the lifetime of the arena, even if replaced in the document
//!     - You can also use any other string references, but the document will be tied to their lifetime
//! - Limited verification, since the document is not parsed from a string
use xmltree::{Document, DocumentSourceRef, Node, NodeAttribute, NodeKind};

fn main() {
    //
    // Arena for allocating string references.
    // This is a bump allocator that will hold the source string
    let arena = DocumentSourceRef::default();

    //
    // While this crate is mostly a parser, it can also be used to create XML documents.
    // For now everything is still arena allocated, so use with caution.
    //
    // Let's call this mode of operation experimental, to be safe.
    //
    // The only thing you -cannot- do with this document, is write it to a binary format with a source header.
    // To do that, first use `to_xml()` to get a string, then parse it back into a new document.
    let mut document = Document::new_empty(&arena, "root");

    //
    // We can add the attribute `xm:name="foo"` to the root node.
    let attribute = NodeAttribute::from_unallocated(&arena, Some("xm"), "name", "foo");
    document.root.attributes.push(attribute);

    //
    // Add whole new nodes to the document.
    let node = Node::from_unallocated(&arena, None, "child");
    document.root.children.push(NodeKind::Child(node));

    //
    // And viola! A valid XML document.
    println!("{}", document.to_xml(Some("  ")).unwrap());

    //
    // The result is a far more compact document, since no source-metadata is stored.
    //
    // Using a source string on to_bin here would not make sense, since there is no source for this document.
    let bin = document.to_bin(None).unwrap();
    println!("Binary size: {:.2}kB", bin.len() as f64 / 1024.0);
}
