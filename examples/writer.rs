//! XML Writer Example
//!
//! This example creates a new XML document with a root node and a child node, and adds an attribute to the root node.  
//! It demonstrates how to use the `xmltree` crate to create XML documents programmatically.
use xmltree::{
    OwnedDeclarationNode, OwnedDocument,
    error::XmlResult,
    node::{OwnedNodeAttribute, OwnedTagNode},
};

fn main() -> XmlResult<()> {
    //
    // While this crate is mostly a parser, it can also be used to create XML documents.
    let mut root = OwnedTagNode::new("root");
    root.attributes
        .push(OwnedNodeAttribute::new("xm:foo", "bar"));
    let mut document = OwnedDocument::new(root);
    document.declaration = Some(OwnedDeclarationNode::new("1.0", Some("UTF-8"), None));

    //
    // And viola! A valid XML document.
    println!("{}", document.to_xml(Some("  "))?);

    //
    // The result of `to_bin` is a far more compact document, since no source-metadata is stored.
    let bin = document.to_bin()?;
    println!("Binary size: {:.2}kB", bin.len() as f64 / 1024.0);

    Ok(())
}
