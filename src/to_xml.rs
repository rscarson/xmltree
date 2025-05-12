//! XML formatting module
//!
//! Use [`Document::to_xml`] unless you need to write the XML to a file or other writer.
use crate::Document;
use crate::node::{EntityDefinition, ExternalId, Node, NodeName, TagNode};
use htmlentity::entity::ICodedDataTrait;
use htmlentity::entity::{CharacterSet, EncodeType, encode};

const TAB: &str = "\t";

/// Flatten a document as a formatted XML string using the given writer.
///
/// # Errors
/// This function will return an error if the writer fails to write the XML string.
pub fn write_xml(
    writer: &mut dyn std::io::Write,
    document: &Document,
    tab_char: Option<&str>,
) -> std::io::Result<()> {
    let tab_char = tab_char.unwrap_or(TAB);

    //
    // Write the XML declaration
    if let Some(declaration) = &document.declaration() {
        let version = encode_entities(declaration.version().text())?;
        writer.write_all(format!(r#"<?xml version="{version}""#).as_bytes())?;

        if let Some(encoding) = &declaration.encoding() {
            let encoding = encode_entities(encoding.text())?;
            writer.write_all(format!(r#" encoding="{encoding}""#).as_bytes())?;
        }

        if let Some(standalone) = &declaration.standalone() {
            let standalone = standalone.to_string();
            writer.write_all(format!(r#" standalone="{standalone}""#).as_bytes())?;
        }

        writer.write_all(b" ?>\n")?;
    }

    //
    // Write the prolog section
    for item in document.prolog() {
        write_node(writer, item, tab_char, 0)?;
    }

    //
    // Write the root node
    let mut stack = vec![(NodeTask::OpenNode(document.root()), 0)];
    loop {
        let Some((task, depth)) = stack.pop() else {
            break;
        };
        let tab = tab_char.repeat(depth as usize);

        match task {
            NodeTask::Close(node_name) => {
                let name = encode_entities(&node_name.to_string())?;
                writer.write_all(format!("{tab}</{name}>\n").as_bytes())?;
            }

            NodeTask::OpenKind(node_kind) => {
                if let Node::Child(node) = node_kind {
                    stack.push((NodeTask::OpenNode(node), depth));
                } else {
                    write_node(writer, node_kind, tab_char, depth)?;
                }
            }

            NodeTask::OpenNode(node) => {
                let name = encode_entities(&node.name().to_string())?;
                writer.write_all(format!("{tab}<{name}").as_bytes())?;

                for attr in node.attributes() {
                    let attr_name = encode_entities(&attr.name().to_string())?;
                    let attr_value = encode_entities(attr.value().text())?;
                    writer.write_all(format!(r#" {attr_name}="{attr_value}""#).as_bytes())?;
                }

                if node.children().is_empty() {
                    writer.write_all(b" />\n")?;
                    continue;
                }

                writer.write_all(b">\n")?;
                stack.push((NodeTask::Close(node.name()), depth));
                for child in node.children().iter().rev() {
                    stack.push((NodeTask::OpenKind(child), depth + 1));
                }
            }
        }
    }

    //
    // Write the epilog section
    // Not valud XML but, can exist
    for item in document.epilog() {
        write_node(writer, item, tab_char, 0)?;
    }

    Ok(())
}

fn encode_entities(input: &str) -> std::io::Result<String> {
    encode(
        input.as_bytes(),
        &EncodeType::NamedOrHex,
        &CharacterSet::Html,
    )
    .to_string()
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

fn write_node(
    writer: &mut dyn std::io::Write,
    node: &Node<'_>,
    tab_char: &str,
    depth: u8,
) -> std::io::Result<()> {
    let tab = tab_char.repeat(depth as usize);

    match node {
        Node::Comment(str_span) => {
            let comment = encode_entities(str_span.text())?;
            writer.write_all(format!("{tab}<!--{comment}-->\n").as_bytes())?;
        }

        Node::Text(text_node) => {
            let text = encode_entities(text_node.text().text())?;
            writer.write_all(format!("{tab}{text}\n").as_bytes())?;
        }

        Node::ProcessingInstruction(processing_instruction_node) => {
            let target = encode_entities(processing_instruction_node.target().text())?;
            writer.write_all(format!("{tab}<?{target}").as_bytes())?;

            if let Some(content) = &processing_instruction_node.content() {
                let content = encode_entities(content.text())?;
                writer.write_all(format!(" {content}").as_bytes())?;
            }

            writer.write_all(b"?>\n")?;
        }

        Node::DocumentType(dtd_node) => {
            let name = encode_entities(dtd_node.name().text())?;
            writer.write_all(format!("{tab}<!DOCTYPE {name}").as_bytes())?;

            if let Some(external_id) = &dtd_node.external_id() {
                match external_id {
                    ExternalId::Public(name, value) => {
                        let name = encode_entities(name.text())?;
                        let value = encode_entities(value.text())?;
                        writer.write_all(format!(r#" PUBLIC "{name}" "{value}""#).as_bytes())?;
                    }
                    ExternalId::System(value) => {
                        let value = encode_entities(value.text())?;
                        writer.write_all(format!(r#" SYSTEM "{value}""#).as_bytes())?;
                    }
                }
            }

            if !dtd_node.entities().is_empty() {
                writer.write_all(b" [\n")?;
                for entity in dtd_node.entities() {
                    let tab = tab_char.repeat((depth + 1) as usize);

                    let entity_name = encode_entities(entity.name.text())?;
                    writer.write_all(format!("{tab}<!ENTITY {entity_name}").as_bytes())?;

                    match &entity.definition {
                        EntityDefinition::EntityValue(value) => {
                            let value = encode_entities(value.text())?;
                            writer.write_all(format!(r#" "{value}""#).as_bytes())?;
                        }

                        EntityDefinition::ExternalId(ExternalId::System(value)) => {
                            let value = encode_entities(value.text())?;
                            writer.write_all(format!(r#" SYSTEM "{value}""#).as_bytes())?;
                        }

                        EntityDefinition::ExternalId(ExternalId::Public(name, value)) => {
                            let name = encode_entities(name.text())?;
                            let value = encode_entities(value.text())?;
                            writer
                                .write_all(format!(r#" PUBLIC "{name}" "{value}""#).as_bytes())?;
                        }
                    }

                    writer.write_all(b">\n")?;
                }
                writer.write_all(b"]")?;
            }

            writer.write_all(b">\n")?;
        }

        Node::Cdata(cdata_node) => {
            let cdata = encode_entities(cdata_node.content().text())?;
            writer.write_all(format!("{tab}<![CDATA[{cdata}]]>\n").as_bytes())?;
        }

        Node::Child(_) => (),
    }

    Ok(())
}

enum NodeTask<'src> {
    OpenNode(&'src TagNode<'src>),
    OpenKind(&'src Node<'src>),
    Close(&'src NodeName<'src>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_xml_with_declaration() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?><root />"#;
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert!(xml2.contains(r#"<?xml version="1.0" encoding="UTF-8" standalone="true" ?>"#));
    }
    #[test]
    fn test_write_xml_without_declaration() {
        let xml = "<root />\n";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert_eq!(xml, xml2);
    }

    #[test]
    fn test_write_xml_with_nested_elements() {
        let xml = "<root><child><subchild /></child></root>";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert_eq!(
            xml2,
            "<root>\n\t<child>\n\t\t<subchild />\n\t</child>\n</root>\n"
        );
    }

    #[test]
    fn test_write_xml_with_attributes() {
        let xml = "<root attr1=\"value1\" attr2=\"value2\" />\n";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert_eq!(xml, xml2);
    }

    #[test]
    fn test_write_xml_with_text_content() {
        let xml = "<root>Some text content</root>";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert_eq!(xml2, "<root>\n\tSome text content\n</root>\n");
    }

    #[test]
    fn test_write_xml_with_cdata() {
        let xml = "<root><![CDATA[Some <CDATA> content]]></root>";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert!(xml2.contains("<![CDATA[Some &lt;CDATA&gt; content]]>"));
    }

    #[test]
    fn test_write_xml_with_comment() {
        let xml = "<root><!-- This is a comment --></root>";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert!(xml2.contains("<!-- This is a comment -->"));
    }

    #[test]
    fn test_write_xml_with_processing_instruction() {
        let xml = "<root><?pi target?></root>";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert!(xml2.contains("<?pi target?>"));
    }

    #[test]
    fn test_write_xml_with_doctype() {
        let xml = "<!DOCTYPE root><root />";
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert!(xml2.contains("<!DOCTYPE root>"));
    }

    #[test]
    fn test_write_xml_with_entities() {
        let xml = r#"<!DOCTYPE root [<!ENTITY example "example value">]><root>&example;</root>"#;
        let document = Document::parse_str(xml).unwrap();
        let xml2 = document.to_xml(None).unwrap();
        assert_eq!(
            xml2,
            "<!DOCTYPE root [\n\t<!ENTITY example \"example value\">\n]>\n<root>\n\t&amp;example;\n</root>\n"
        );
    }
}
