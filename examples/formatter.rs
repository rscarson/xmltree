//! XML Formatter Example
//!
//! This example demonstrates how to format an XML document into a string using the `xmltree` crate.
//!
//! It also shows that you can alter the document after parsing and still serialize it correctly.
use xmltree::{Document, error::XmlResult};
const SRC: &str = "<test><test2>test</test2></test>";

fn main() -> XmlResult<()> {
    //
    // Parse the XML document from the source string
    //
    // If an error occurs, the error message is designed to be human-readable;
    //      | </example>
    //      = At 25:5
    //      = Unclosed tag: bookstore
    let doc = Document::parse_str(SRC)?;

    //
    // We can turn a tree into a formatted XML string
    // The `to_xml` method takes an optional string to use as indentation.
    //
    // You can pass a custom indent string; if `None`, a tab is used by default.
    // The formatter is non-recursive and handles large documents safely.
    let formatted = doc.to_xml(Some("  "))?;
    println!("Formatted XML:\n{formatted}");

    Ok(())
}
