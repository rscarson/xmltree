<!-- cargo-rdme start -->

# xmltree
## Zero-copy XML Parser and Writer

This crate provides a tree wrapper around `xmlparser`.  
It provides 4 main features:
- **Zero-copy Parser**: Parse and validate XML documents from a string into a tree structure
- **XML Formatter**: Format XML documents into a string with indentation and line breaks
- **Binary Serializer**: Serialize XML documents into a binary format, and back
- **Document Writer**: Create XML documents programmatically

Here are some examples of how to use the crate

### Zero-copy Parser and XML Formatter
Please see `examples/parser.rs` for a more detailed example of parsing a document
Please see `examples/formatter.rs` for a more detailed example of formatting a document

This crate uses `xmlparser` to tokenize an XML document, and then builds a tree structure around it.  
The parser is zero-copy, designed for speed, and uses no recursion.

It's main selling point is that it tracks the original span in source of all components of the tree.

Here is a simple example that parses a document from a string and prints out the resulting tree as a formatted XML string:

```rust
use xmltree::{Document, error::XmlResult};
const SRC: &str = "<test><test2>test</test2></test>";

fn main() -> XmlResult<()> {
   let doc = Document::parse_str(SRC)?;
   let formatted_xml = doc.to_xml(Some("  "))?;
   println!("{formatted_xml}");
   Ok(())
}
```

### Binary Serializer
Please see `examples/serializer.rs` for a more detailed example of serializing a document

This crate also provides a binary serializer that can serialize XML documents into a binary format, and back.  
The serializer is generalized, and can be extended to custom types, if you want to embed XML docs, or portions, into other objects.

It can load in ~5us, vs ~15us for parsing from a string (by my testing).

See the example for a more detailed breakdown of the options.

Here is a simple example that serializes a document to binary and back:
```rust
use xmltree::{Document, error::XmlResult};
const SRC: &str = "<test><test2>test</test2></test>";

fn main() -> XmlResult<()> {
    let doc = Document::parse_str(SRC)?;

    // This removes all source span information from the document
    // However, it will significantly reduce the size of the binary
    let doc = doc.to_owned();
    let bytes = doc.to_bin()?;

    Ok(())
}
```

### Document Writer
Please see `examples/writer.rs` for a more detailed example of creating a document

You can also create XML documents programmatically using the `Document` struct.

Here is a simple example that creates a document and prints it out:
```rust
use xmltree::{OwnedDocument, OwnedDeclarationNode, node::{OwnedTagNode, OwnedNodeAttribute, OwnedNode}};

let mut root = OwnedTagNode::new("root");
let mut document = OwnedDocument::new(root);
document.declaration = Some(OwnedDeclarationNode::new("1.0", Some("UTF-8"), None));

let mut node = OwnedTagNode::new("child");
let attribute = OwnedNodeAttribute::new("xm:foo", "bar");
node.attributes.push(attribute);

document.root.children.push(OwnedNode::Tag(node));

let formatted_xml = document.to_xml(None).unwrap();
println!("{formatted_xml}");
```

<!-- cargo-rdme end -->
