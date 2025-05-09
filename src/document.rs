use crate::{
    CdataNode, DocumentSourceRef, NamedElement, NodeName, StrSpan,
    dtd::DtdNode,
    error::{ErrorContext, XmlError, XmlErrorKind, XmlResult},
    node::{Node, NodeAttribute, NodeKind, ProcessingInstructionNode, TextNode},
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};
use std::io::{Read, Write};
use xmlparser::{ElementEnd, Token};

/// Determines how strings are stored in the binary format.
///
/// Inline strings are stored as a byte-stream with a length.  
/// It does not require a source string, but is slower to encode and decode.
///
/// Header strings are stored as offsets into a source string in the header.  
/// It is faster to encode and decode, but requires a document be unmodified after parsing from a string.  
/// This is the default format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BinaryStringFormat {
    /// Strings are stored inline, as a byte-stream with a length
    Inline,

    /// Strings are stored as offsets into a source string in the header
    #[default]
    Header,
}

/// An XML document that has been parsed into a tree. It is deliberately flexible with invalid XML.  
/// All strings for components in the tree are references to the source string, stored in a bump allocated arena.
///
/// The root of the tree contains:
/// - The XML declaration node, if present
/// - Any items before the root node (DTD, comments, processing instructions, etc)
/// - The root node itself, which is a tree of nodes
/// - Any items after the root node (comments, processing instructions, etc) - This is not valid XML, but will be parsed
///
/// Other things to note:
/// - Names of nodes and properties can have a namespace prefix
/// - Node attributes can have duplicates, but `Node::get_attribute` will only return the last one defined
/// - The parser will not attempt to recover from invalid closing tags, or unclosed tags.
#[derive(Debug, Clone, PartialEq)]
pub struct Document<'src> {
    /// The <?xml declaration node.
    pub declaration: Option<DeclarationNode<'src>>,

    /// Nodes occurring before the root node.  
    /// This includes the XML declaration, comments, and processing instructions.
    pub prolog: Vec<NodeKind<'src>>,

    /// The root of the tree.
    pub root: Node<'src>,

    /// Nodes occurring after the root node.  
    /// This includes comments and processing instructions.
    ///
    /// Note that strictly speaking, this is not valid XML
    pub epilog: Vec<NodeKind<'src>>,
}
impl<'src> Document<'src> {
    /// Creates a new document from the given source string. The string will be allocated in the given arena.
    ///
    /// # Errors
    /// Returns errors if the XML is invalid
    ///
    /// # Example
    /// ```rust
    /// use xmltree::{Document, DocumentSourceRef};
    /// let src = "<test><test2>test</test2></test>";
    ///
    /// let arena = DocumentSourceRef::default();
    /// let doc = Document::new(&arena, src).unwrap();
    /// assert_eq!(doc.root.name, "test");
    /// ```
    pub fn new(arena: &'src DocumentSourceRef, source: &str) -> XmlResult<Self> {
        Self::parse(arena, source)
    }

    /// Creates a new document, with a 1.0 declaration, and a root node with the given name.  
    /// The strings will be allocated in the given arena.
    ///
    /// Important note: The strings allocated will live for the lifetime of the arena, even if replaced in the document!
    ///
    /// # Panics
    /// The function will panic if the arena cannot allocate the strings.  
    /// For a non-panicking version, use `DocumentSourceRef::try_alloc`
    ///
    /// # Example
    /// ```rust
    /// use xmltree::{Document, DocumentSourceRef, Node, NodeAttribute, NodeKind};
    ///
    /// let arena = DocumentSourceRef::default();
    /// let mut document = Document::new_empty(&arena, "root");
    ///
    /// let mut node = Node::from_unallocated(&arena, None, "child");
    ///
    /// let attribute = NodeAttribute::from_unallocated(&arena, Some("xm"), "name", "foo");
    /// node.attributes.push(attribute);
    ///
    /// document.root.children.push(NodeKind::Child(node));
    /// ```
    pub fn new_empty<'b>(arena: &'src DocumentSourceRef, root_name: &'b str) -> Self {
        Self {
            declaration: Some(DeclarationNode::from_unallocated(arena, "1.0")),
            prolog: vec![],
            root: Node::from_unallocated(arena, None, root_name),
            epilog: vec![],
        }
    }

    /// Write this document as a flat binary format.
    ///
    /// If src is provided, it will be written as a header before the document.  
    /// All strings will be stored as references to the source string, making deserialization faster.
    ///
    /// However, if you have modified the document after parsing and provide a source string, deserialization will fail.
    ///
    /// # Errors
    /// Returns errors if the encoding fails
    ///
    /// # Example
    /// ```rust
    /// use xmltree::{Document, DocumentSourceRef};
    ///
    /// let arena = DocumentSourceRef::default();
    /// let src = "<test><test2>test</test2></test>";
    /// let doc = Document::new(&arena, src).unwrap();
    ///
    /// let bin = doc.to_bin(Some(src)).unwrap();
    /// println!("Binary size: {:.2}kB", bin.len() as f64 / 1024.0);
    /// ```
    pub fn to_bin(&self, src: Option<&'src str>) -> std::io::Result<Vec<u8>> {
        let mut buffer = vec![];
        let mut encoder = Encoder::new(&mut buffer);

        if let Some(src) = src {
            encoder = encoder.with_source(src);
            encoder.write(&src)?;
        }

        encoder.write(self)?;
        Ok(buffer)
    }

    /// Read a document from a flat binary format.
    ///
    /// If `BinaryStringFormat::Header` is used, then it is assumed the document has a header with the complete source string,  
    /// created with `Document::to_bin(Some(src))`.
    ///
    /// # Errors
    /// Returns errors if the decoding fails
    ///
    /// # Example
    /// ```rust
    /// use xmltree::{Document, DocumentSourceRef, BinaryStringFormat};
    /// const DOC: &[u8] = include_bytes!("../examples/example.bin");
    ///
    /// let arena = DocumentSourceRef::default();
    /// let doc = Document::from_bin(DOC, BinaryStringFormat::Header, &arena).unwrap();
    ///
    /// assert_eq!(doc.root.name, "bookstore");
    /// ```
    pub fn from_bin<'dec>(
        data: &'dec [u8],
        string_format: BinaryStringFormat,
        arena: &'src DocumentSourceRef,
    ) -> Result<Self, BinDecodeError> {
        let mut decoder = Decoder::new(data, arena);

        if BinaryStringFormat::Header == string_format {
            let src = decoder.read::<&str>()?;
            let src = arena.try_alloc(src).map_err(BinDecodeError::Allocation)?;
            decoder = decoder.with_source(src);
        }

        let document = decoder.read()?;
        Ok(document)
    }

    /// Create a formatted XML string from this document.
    ///
    /// This is mostly used to format the document, or to get a source string for a programatically created document.
    ///
    /// `tab_char` is used to indent the XML. If `None`, a tab is used.
    ///
    /// # Errors
    /// Can fail if a string in the document cannot be entity encoded.
    ///
    /// # Example
    /// ```rust
    /// use xmltree::{Document, DocumentSourceRef};
    /// const SRC: &str = "<test><test2>test</test2></test>";
    ///
    /// let arena = DocumentSourceRef::default();
    /// let doc = Document::new(&arena, SRC).unwrap();
    ///
    /// let formatted = doc.to_xml(Some("    ")).unwrap();
    /// println!("Formatted XML:\n{formatted}");
    /// /*
    /// <test>
    ///     <test2>
    ///         test
    ///     </test2>
    /// </test>
    ///  */
    /// ```
    pub fn to_xml(&self, tab_char: Option<&str>) -> std::io::Result<String> {
        let mut buffer = vec![];
        self.to_xml_with_writer(&mut buffer, tab_char)?;

        let buffer = String::from_utf8(buffer).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to convert to UTF-8: {e}"),
            )
        })?;
        Ok(buffer)
    }

    /// Write this document as a formatted XML string using the given writer.
    ///
    /// See [`Document::to_xml`] for more details.
    ///
    /// # Errors
    /// Can fail if a string in the document cannot be entity encoded.
    pub fn to_xml_with_writer<W: Write>(
        &self,
        writer: &mut W,
        tab_char: Option<&str>,
    ) -> std::io::Result<()> {
        crate::to_xml::write_xml(writer, self, tab_char)
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.declaration
            .as_mut()
            .map(DeclarationNode::strip_metadata);
        for item in &mut self.prolog {
            item.strip_metadata();
        }
        self.root.strip_metadata();
        for item in &mut self.epilog {
            item.strip_metadata();
        }
    }

    #[expect(clippy::too_many_lines, reason = "State machine; what did you expect")]
    fn parse<'b>(arena: &'src DocumentSourceRef, src: &'b str) -> XmlResult<Self> {
        let src: &'src _ = arena.try_alloc(src).map_err(|e| {
            XmlError::new(
                XmlErrorKind::Allocation(e),
                ErrorContext::new("", StrSpan::default()),
            )
        })?;
        let src = arena.alloc(src);

        let mut tokenizer = xmlparser::Tokenizer::from(src);

        let mut state = ParserState::Prolog;
        let mut stack = vec![];

        let mut prolog = vec![];
        let mut epilog = vec![];
        let mut declaration = None;

        loop {
            // Get the next token
            let Some(next) = tokenizer.next() else {
                let root = match stack.len() {
                    0 => bail!(src, XmlErrorKind::UnexpectedEof),
                    1 => stack.pop().unwrap(),
                    _ => {
                        let last: Node = stack.pop().unwrap();
                        bail!(
                            src,
                            last.span,
                            XmlErrorKind::UnclosedTag(last.name.to_string())
                        );
                    }
                };

                return Ok(Self {
                    declaration,
                    prolog,
                    root,
                    epilog,
                });
            };
            let next = match next {
                Ok(token) => token,
                Err(e) => {
                    bail!(src, XmlErrorKind::Xml(e));
                }
            };

            //
            // At this point, next is significant.
            match state {
                ParserState::Prolog => match next {
                    Token::ElementStart {
                        prefix,
                        local,
                        span,
                    } => {
                        stack.push(Node::from_spans(span.into(), prefix.into(), local.into()));
                        state = ParserState::TagAttributes;
                    }

                    Token::Comment { text, .. } => prolog.push(NodeKind::Comment(text.into())),

                    Token::Declaration {
                        version,
                        encoding,
                        standalone,
                        span,
                    } => {
                        if !prolog.is_empty() {
                            let span = next.span();
                            bail!(src, span, XmlErrorKind::DeclarationNotFirst);
                        }

                        declaration = Some(DeclarationNode::new(
                            span.into(),
                            version.into(),
                            encoding.map(Into::into),
                            standalone,
                        ));
                    }

                    Token::ProcessingInstruction {
                        target,
                        content,
                        span,
                    } => {
                        let node = ProcessingInstructionNode::new(
                            span.into(),
                            target.into(),
                            content.map(Into::into),
                        );
                        prolog.push(NodeKind::ProcessingInstruction(node));
                    }

                    Token::EmptyDtd { .. } | Token::DtdStart { .. } => {
                        let node = DtdNode::parse(next, &mut tokenizer, src)?;
                        prolog.push(NodeKind::DocumentType(node));
                    }

                    Token::Cdata { text, span } => {
                        let node = CdataNode::new(span.into(), text.into());
                        prolog.push(NodeKind::Cdata(node));
                    }

                    _ => {
                        let span = next.span();
                        bail!(
                            src,
                            span,
                            msg = "Unexpected {} in prolog section",
                            next.name()
                        );
                    }
                },

                ParserState::TagAttributes => match next {
                    Token::Attribute {
                        prefix,
                        local,
                        value,
                        span,
                        ..
                    } => {
                        let attr = NodeAttribute::new(
                            span.into(),
                            prefix.into(),
                            local.into(),
                            value.into(),
                        );
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(
                                src,
                                span,
                                msg = "Bug; Cannot apply attribute; stack is empty!"
                            );
                        };

                        node.attributes.push(attr);
                    }

                    Token::Comment { text, .. } => {
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(
                                src,
                                span,
                                msg = "Bug; Cannot apply comment; stack is empty!"
                            );
                        };

                        node.children.push(NodeKind::Comment(text.into()));
                    }

                    Token::ElementEnd {
                        end: ElementEnd::Open,
                        ..
                    } => {
                        state = ParserState::TagChildren;
                    }

                    Token::ElementEnd {
                        end: ElementEnd::Empty,
                        ..
                    } => {
                        let Some(mut node) = stack.pop() else {
                            let span = next.span();
                            bail!(src, span, msg = "Bug; Cannot close tag; stack is empty!");
                        };

                        node.span.extend(&next.span().into(), src);

                        let Some(parent) = stack.last_mut() else {
                            state = ParserState::Epilog;
                            stack.push(node);
                            continue;
                        };

                        parent.children.push(NodeKind::Child(node));
                        state = ParserState::TagChildren;
                    }

                    Token::Text { .. } => {
                        // ignore
                    }

                    _ => {
                        let span = next.span();
                        bail!(
                            src,
                            span,
                            msg = "Unexpected {} in tag attributes",
                            next.name()
                        );
                    }
                },

                ParserState::TagChildren => match next {
                    Token::ElementStart {
                        prefix,
                        local,
                        span,
                        ..
                    } => {
                        stack.push(Node::from_spans(span.into(), prefix.into(), local.into()));
                        state = ParserState::TagAttributes;
                    }

                    Token::Cdata { text, span } => {
                        let cnode = CdataNode::new(span.into(), text.into());
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(src, span, msg = "Bug; Cannot apply cdata; stack is empty!");
                        };

                        node.children.push(NodeKind::Cdata(cnode));
                    }

                    Token::Text { text, .. } => {
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(src, span, msg = "Bug; Cannot apply text; stack is empty!");
                        };

                        // Translate the reference to a source reference
                        let start = text.start();
                        let text = src[start..text.end()].trim();
                        if text.is_empty() {
                            continue;
                        }

                        let text = StrSpan { text, start };
                        let span = next.span();
                        let text = TextNode::new(span.into(), text);
                        node.children.push(NodeKind::Text(text));
                    }

                    Token::Comment { text, .. } => {
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(
                                src,
                                span,
                                msg = "Bug; Cannot apply comment; stack is empty!"
                            );
                        };

                        node.children.push(NodeKind::Comment(text.into()));
                    }

                    Token::ProcessingInstruction {
                        target,
                        content,
                        span,
                    } => {
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(
                                src,
                                span,
                                msg = "Bug; Cannot apply processing instruction; stack is empty!"
                            );
                        };

                        let pi = ProcessingInstructionNode::new(
                            span.into(),
                            target.into(),
                            content.map(Into::into),
                        );
                        node.children.push(NodeKind::ProcessingInstruction(pi));
                    }

                    Token::ElementEnd {
                        end: ElementEnd::Close(prefix, local),
                        ..
                    } => {
                        let Some(mut node) = stack.pop() else {
                            let span = next.span();
                            bail!(src, span, msg = "Bug; Cannot close tag; stack is empty!");
                        };

                        node.span.extend(&next.span().into(), src);

                        let name = NodeName::new(prefix.into(), local.into());
                        if node.name != name {
                            let span = next.span();

                            println!("`{:?}` != `{:?}`", node.name, name);
                            bail!(src, span, XmlErrorKind::UnclosedTag(node.name.to_string()));
                        }

                        state = ParserState::TagChildren;
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(NodeKind::Child(node));
                        } else {
                            state = ParserState::Epilog;
                            stack.push(node);
                            continue;
                        }
                    }

                    _ => {
                        let span = next.span();
                        bail!(src, span, msg = "Unexpected {} inside tag", next.name());
                    }
                },

                ParserState::Epilog => match next {
                    Token::Comment { text, .. } => epilog.push(NodeKind::Comment(text.into())),

                    Token::Cdata { text, span } => {
                        let node = CdataNode::new(span.into(), text.into());
                        epilog.push(NodeKind::Cdata(node));
                    }

                    Token::ProcessingInstruction {
                        target,
                        content,
                        span,
                    } => {
                        let node = ProcessingInstructionNode::new(
                            span.into(),
                            target.into(),
                            content.map(Into::into),
                        );
                        epilog.push(NodeKind::ProcessingInstruction(node));
                    }

                    _ => {
                        let span = next.span();
                        bail!(src, span, msg = "Unexpected {} in after root", next.name());
                    }
                },
            }
        }
    }
}

impl<'src> ToBinHandler<'src> for Document<'src> {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.declaration.write(encoder)?;
        self.prolog.write(encoder)?;
        self.root.write(encoder)?;
        self.epilog.write(encoder)?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let declaration = Option::<DeclarationNode>::read(decoder)?;
        let prolog = Vec::<NodeKind>::read(decoder)?;
        let root = Node::read(decoder)?;
        let epilog = Vec::<NodeKind>::read(decoder)?;

        Ok(Self {
            declaration,
            prolog,
            root,
            epilog,
        })
    }
}

#[derive(PartialEq, Debug)]
enum ParserState {
    Prolog,
    TagAttributes,
    TagChildren,
    Epilog,
}

/// The XML declaration node.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclarationNode<'src> {
    /// The span of the declaration node in the input XML.
    pub span: StrSpan<'src>,

    /// The version of the XML declaration.
    pub version: StrSpan<'src>,

    /// The encoding of the XML declaration.
    pub encoding: Option<StrSpan<'src>>,

    /// The standalone attribute of the XML declaration.
    pub standalone: Option<bool>,
}
impl<'src> DeclarationNode<'src> {
    /// Create a new declaration node.
    pub(crate) fn new(
        span: StrSpan<'src>,
        version: StrSpan<'src>,
        encoding: Option<StrSpan<'src>>,
        standalone: Option<bool>,
    ) -> Self {
        Self {
            span,
            version,
            encoding,
            standalone,
        }
    }

    /// Create a new declaration node from strings not referencing a source document.  
    /// The strings will be allocated in the given arena.
    ///
    /// # Panics
    /// Panics if the arena cannot allocate the strings.  
    /// For a non-panicking version, use `DocumentSourceRef::try_alloc`.
    pub fn from_unallocated<'b>(arena: &'src DocumentSourceRef, version: &'b str) -> Self {
        let version = arena.alloc(version);
        Self {
            span: StrSpan::default(),
            version: StrSpan::from_unallocated(arena, version),
            encoding: None,
            standalone: None,
        }
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
    }
}

impl<'src> ToBinHandler<'src> for DeclarationNode<'src> {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.version.write(encoder)?;
        self.encoding.write(encoder)?;
        self.standalone.write(encoder)?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let version = StrSpan::read(decoder)?;
        let encoding = Option::<StrSpan>::read(decoder)?;
        let standalone = Option::<bool>::read(decoder)?;

        Ok(Self {
            span,
            version,
            encoding,
            standalone,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentSourceRef;

    #[test]
    fn test_new_document() {
        let src = "<test><test2>test</test2></test>";
        let arena = DocumentSourceRef::default();
        let doc = Document::new(&arena, src).unwrap();
        assert_eq!(doc.root.name, "test");
        assert_eq!(doc.root.children.len(), 1);
    }

    #[test]
    fn test_new_empty_document() {
        let arena = DocumentSourceRef::default();
        let document = Document::new_empty(&arena, "root");
        assert_eq!(document.root.name, "root");
        assert!(document.root.children.is_empty());
    }

    #[test]
    fn test_to_bin() {
        let src = "<test><test2>test</test2></test>";
        let arena = DocumentSourceRef::default();
        let doc = Document::new(&arena, src).unwrap();
        let bin = doc.to_bin(Some(src)).unwrap();
        assert!(!bin.is_empty());
    }

    #[test]
    fn test_from_bin() {
        let src = "<test><test2>test</test2></test>";
        let arena = DocumentSourceRef::default();
        let doc = Document::new(&arena, src).unwrap();
        let bin = doc.to_bin(Some(src)).unwrap();
        let decoded_doc = Document::from_bin(&bin, BinaryStringFormat::Header, &arena).unwrap();
        assert_eq!(decoded_doc.root.name, "test");
    }

    #[test]
    fn test_to_xml() {
        let src = "<test><test2>test</test2></test>";
        let arena = DocumentSourceRef::default();
        let doc = Document::new(&arena, src).unwrap();
        let formatted = doc.to_xml(Some("    ")).unwrap();
        assert!(formatted.contains("<test>"));
        assert!(formatted.contains("    <test2>\n        test\n    </test2>"));
    }

    #[test]
    fn test_parse_invalid_xml() {
        let src = "<test><test2>test</test>";
        let arena = DocumentSourceRef::default();
        let result = Document::new(&arena, src);
        assert!(result.is_err());
    }

    #[test]
    fn test_declaration_node_creation() {
        let arena = DocumentSourceRef::default();
        let declaration = DeclarationNode::from_unallocated(&arena, "1.0");
        assert_eq!(declaration.version.text, "1.0");
        assert!(declaration.encoding.is_none());
    }

    #[test]
    fn test_document_with_prolog_and_epilog() {
        let src = "<?xml version=\"1.0\"?><!-- Comment --><root></root><?pi?>";
        let arena = DocumentSourceRef::default();
        let doc = Document::new(&arena, src).unwrap();
        assert!(doc.declaration.is_some());
        assert_eq!(doc.prolog.len(), 1);
        assert_eq!(doc.epilog.len(), 1);
    }
}
