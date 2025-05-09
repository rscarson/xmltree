//! XML formatting module
//!
//! Use [`Document::to_xml`] unless you need to write the XML to a file or other writer.
use crate::{Document, EntityDefinition, ExternalId, Node, NodeKind, NodeName};
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
    if let Some(declaration) = &document.declaration {
        let version = encode_entities(declaration.version.as_str())?;
        writer.write_all(format!(r#"<?xml version="{version}""#).as_bytes())?;

        if let Some(encoding) = &declaration.encoding {
            let encoding = encode_entities(encoding.as_str())?;
            writer.write_all(format!(r#" encoding="{encoding}""#).as_bytes())?;
        }

        if let Some(standalone) = &declaration.standalone {
            let standalone = standalone.to_string();
            writer.write_all(format!(r#" standalone="{standalone}""#).as_bytes())?;
        }

        writer.write_all(b" ?>\n")?;
    }

    //
    // Write the prolog section
    for item in &document.prolog {
        write_node(writer, item, tab_char, 0)?;
    }

    //
    // Write the root node
    let mut stack = vec![(NodeTask::OpenNode(&document.root), 0)];
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
                if let NodeKind::Child(node) = node_kind {
                    stack.push((NodeTask::OpenNode(node), depth));
                } else {
                    write_node(writer, node_kind, tab_char, depth)?;
                }
            }

            NodeTask::OpenNode(node) => {
                let name = encode_entities(&node.name.to_string())?;
                writer.write_all(format!("{tab}<{name}").as_bytes())?;

                for attr in &node.attributes {
                    let attr_name = encode_entities(&attr.name.to_string())?;
                    let attr_value = encode_entities(attr.value.as_str())?;
                    writer.write_all(format!(r#" {attr_name}="{attr_value}""#).as_bytes())?;
                }

                if node.children.is_empty() {
                    writer.write_all(b" />\n")?;
                    continue;
                }

                writer.write_all(b">\n")?;
                stack.push((NodeTask::Close(&node.name), depth));
                for child in node.children.iter().rev() {
                    stack.push((NodeTask::OpenKind(child), depth + 1));
                }
            }
        }
    }

    //
    // Write the epilog section
    // Not valud XML but, can exist
    for item in &document.epilog {
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
    node: &NodeKind<'_>,
    tab_char: &str,
    depth: u8,
) -> std::io::Result<()> {
    let tab = tab_char.repeat(depth as usize);

    match node {
        NodeKind::Comment(str_span) => {
            let comment = encode_entities(str_span.as_str())?;
            writer.write_all(format!("{tab}<!--{comment}-->\n").as_bytes())?;
        }

        NodeKind::Text(text_node) => {
            let text = encode_entities(text_node.text.as_str())?;
            writer.write_all(format!("{tab}{text}\n").as_bytes())?;
        }

        NodeKind::ProcessingInstruction(processing_instruction_node) => {
            let target = encode_entities(processing_instruction_node.target.as_str())?;
            writer.write_all(format!("{tab}<?{target}").as_bytes())?;

            if let Some(content) = &processing_instruction_node.content {
                let content = encode_entities(content.as_str())?;
                writer.write_all(format!(" {content}").as_bytes())?;
            }

            writer.write_all(b"?>\n")?;
        }

        NodeKind::DocumentType(dtd_node) => {
            let name = encode_entities(dtd_node.name.as_str())?;
            writer.write_all(format!("{tab}<!DOCTYPE {name}").as_bytes())?;

            if let Some(external_id) = &dtd_node.external_id {
                match external_id {
                    ExternalId::Public(name, value) => {
                        let name = encode_entities(name.as_str())?;
                        let value = encode_entities(value.as_str())?;
                        writer.write_all(format!(r#" PUBLIC "{name}" "{value}""#).as_bytes())?;
                    }
                    ExternalId::System(value) => {
                        let value = encode_entities(value.as_str())?;
                        writer.write_all(format!(r#" SYSTEM "{value}""#).as_bytes())?;
                    }
                }
            }

            if !dtd_node.entities.is_empty() {
                writer.write_all(b" [\n")?;
                for entity in &dtd_node.entities {
                    let tab = tab_char.repeat((depth + 1) as usize);

                    let entity_name = encode_entities(entity.name.as_str())?;
                    writer.write_all(format!("{tab}<!ENTITY {entity_name}").as_bytes())?;

                    match &entity.definition {
                        EntityDefinition::EntityValue(value) => {
                            let value = encode_entities(value.as_str())?;
                            writer.write_all(format!(r#" "{value}""#).as_bytes())?;
                        }

                        EntityDefinition::ExternalId(ExternalId::System(value)) => {
                            let value = encode_entities(value.as_str())?;
                            writer.write_all(format!(r#" SYSTEM "{value}""#).as_bytes())?;
                        }

                        EntityDefinition::ExternalId(ExternalId::Public(name, value)) => {
                            let name = encode_entities(name.as_str())?;
                            let value = encode_entities(value.as_str())?;
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

        NodeKind::Cdata(cdata_node) => {
            let cdata = encode_entities(cdata_node.content.as_str())?;
            writer.write_all(format!("{tab}<![CDATA[{cdata}]]>\n").as_bytes())?;
        }

        NodeKind::Child(_) => (),
    }

    Ok(())
}

enum NodeTask<'src> {
    OpenNode(&'src Node<'src>),
    OpenKind(&'src NodeKind<'src>),
    Close(&'src NodeName<'src>),
}

#[cfg(test)]
mod tests {
    use crate::{NodeAttribute, StrSpan, document::DeclarationNode};

    use super::*;

    #[test]
    fn test_write_xml_with_declaration() {
        let mut output = Vec::new();
        let document = Document {
            declaration: Some(DeclarationNode {
                span: StrSpan::default(),
                version: "1.0".into(),
                encoding: Some("UTF-8".into()),
                standalone: Some(true),
            }),
            prolog: vec![],
            root: Node {
                span: StrSpan::default(),
                name: NodeName {
                    local: "root".into(),
                    prefix: None,
                },
                attributes: vec![],
                children: vec![],
            },
            epilog: vec![],
        };

        write_xml(&mut output, &document, None).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains(r#"<?xml version="1.0" encoding="UTF-8" standalone="true" ?>"#));
    }

    #[test]
    fn test_write_xml_with_root_node() {
        let mut output = Vec::new();
        let document = Document {
            declaration: None,
            prolog: vec![],
            root: Node {
                span: StrSpan::default(),
                name: NodeName {
                    local: "root".into(),
                    prefix: None,
                },
                attributes: vec![],
                children: vec![],
            },
            epilog: vec![],
        };

        write_xml(&mut output, &document, None).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("<root />"));
    }

    #[test]
    fn test_write_xml_with_attributes() {
        let mut output = Vec::new();
        let document = Document {
            declaration: None,
            prolog: vec![],
            root: Node {
                span: StrSpan::default(),
                name: NodeName {
                    prefix: None,
                    local: "root".into(),
                },
                attributes: vec![
                    NodeAttribute {
                        span: StrSpan::default(),
                        name: NodeName {
                            prefix: None,
                            local: "id".into(),
                        },
                        value: "123".into(),
                    },
                    NodeAttribute {
                        span: StrSpan::default(),
                        name: NodeName {
                            prefix: Some("xm".into()),
                            local: "class".into(),
                        },
                        value: "test".into(),
                    },
                ],
                children: vec![],
            },
            epilog: vec![],
        };

        write_xml(&mut output, &document, None).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains(r#"<root id="123" xm:class="test" />"#));
    }

    #[test]
    fn test_write_xml_with_nested_nodes() {
        let mut output = Vec::new();
        let document = Document {
            declaration: None,
            prolog: vec![],
            root: Node {
                span: StrSpan::default(),
                name: NodeName {
                    prefix: None,
                    local: "root".into(),
                },
                attributes: vec![],
                children: vec![NodeKind::Child(Node {
                    span: StrSpan::default(),
                    name: NodeName {
                        prefix: None,
                        local: "child".into(),
                    },
                    attributes: vec![],
                    children: vec![],
                })],
            },
            epilog: vec![],
        };

        write_xml(&mut output, &document, None).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("<root>\n\t<child />\n</root>"));
    }

    #[test]
    fn test_write_xml_with_prolog_and_epilog() {
        let mut output = Vec::new();
        let document = Document {
            declaration: None,
            prolog: vec![NodeKind::Comment("Prolog comment".into())],
            root: Node {
                span: StrSpan::default(),
                name: NodeName {
                    prefix: None,
                    local: "root".into(),
                },
                attributes: vec![],
                children: vec![],
            },
            epilog: vec![NodeKind::Comment("Epilog comment".into())],
        };

        write_xml(&mut output, &document, None).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("<!--Prolog comment-->"));
        assert!(result.contains("<!--Epilog comment-->"));
    }
}
