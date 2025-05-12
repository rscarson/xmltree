use crate::{
    NamedElement, StrSpan,
    error::{ErrorContext, XmlError, XmlErrorKind, XmlResult},
    node::{
        CdataNode, DtdNode, Node, NodeAttribute, NodeName, OwnedNode, OwnedTagNode,
        ProcessingInstructionNode, TagNode, TextNode,
    },
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};
use xmlparser::{ElementEnd, Token};

#[derive(PartialEq, Debug)]
enum ParserState {
    Prolog,
    TagAttributes,
    TagChildren,
    Epilog,
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
    src: Option<&'src str>,

    declaration: Option<DeclarationNode<'src>>,
    prolog: Vec<Node<'src>>,
    root: TagNode<'src>,
    epilog: Vec<Node<'src>>,
}
impl<'src> Document<'src> {
    const HEADER_SOURCED: &'static [u8] = b"XML1";
    const HEADER_UNSOURCED: &'static [u8] = b"XML2";

    /// Creates a new document from the given source string.
    ///
    /// Source string must live at least as long as the document.  
    ///
    /// # Errors
    /// Returns errors if the XML is invalid
    ///
    /// # Example
    /// ```rust
    /// use xmltree::Document;
    /// let src = "<test><test2>test</test2></test>";
    ///
    /// let doc = Document::parse_str(src).unwrap();
    /// assert_eq!(doc.root().name(), "test");
    /// ```
    pub fn parse_str(source: &'src str) -> XmlResult<Self> {
        Self::parse(source)
    }

    /// Returns the original source string of the document, if it was provided.
    #[must_use]
    pub fn source(&self) -> Option<&'src str> {
        self.src
    }

    /// Returns the XML declaration node, if it was provided.
    #[must_use]
    pub fn declaration(&self) -> Option<&DeclarationNode<'src>> {
        self.declaration.as_ref()
    }

    /// Returns the prolog of the document, which is everything between the declaration and root.
    /// This includes comments, DTDs, and processing instructions.
    #[must_use]
    pub fn prolog(&self) -> &[Node<'src>] {
        &self.prolog
    }

    /// Returns the root node of the document.
    #[must_use]
    pub fn root(&self) -> &TagNode<'src> {
        &self.root
    }

    /// Returns the epilog of the document, which is everything after the root.  
    /// Technically this is not valid XML, but it is parsed anyway.
    #[must_use]
    pub fn epilog(&self) -> &[Node<'src>] {
        &self.epilog
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
    /// use xmltree::Document;
    ///
    /// let src = "<test><test2>test</test2></test>";
    /// let doc = Document::parse_str(src).unwrap();
    ///
    /// let bin = doc.to_bin().unwrap();
    /// println!("Binary size: {:.2}kB", bin.len() as f64 / 1024.0);
    /// ```
    pub fn to_bin(&self) -> std::io::Result<Vec<u8>> {
        let mut encoder = Encoder::new();
        self.write(&mut encoder)?;
        Ok(encoder.into_inner())
    }

    /// Read a document from a flat binary format.
    ///
    /// # Errors
    /// Returns errors if the decoding fails
    ///
    /// # Example
    /// ```rust
    /// use xmltree::Document;
    /// const DOC: &[u8] = include_bytes!("../examples/example.bin");
    ///
    /// let doc = Document::from_bin(DOC).unwrap();
    ///
    /// assert_eq!(doc.root().name(), "bookstore");
    /// ```
    pub fn from_bin(data: &'src [u8]) -> Result<Self, BinDecodeError> {
        let mut decoder = Decoder::new(data);
        let document = Self::read(&mut decoder)?;
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
    /// use xmltree::{Document};
    /// const SRC: &str = "<test><test2>test</test2></test>";
    ///
    /// let doc = Document::parse_str(SRC).unwrap();
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
    pub fn to_xml_with_writer<W: std::io::Write>(
        &self,
        writer: &mut W,
        tab_char: Option<&str>,
    ) -> std::io::Result<()> {
        crate::to_xml::write_xml(writer, self, tab_char)
    }

    /// Returns an owned version of this document, with no source span information.
    pub fn to_owned(&self) -> OwnedDocument {
        OwnedDocument {
            declaration: self.declaration.as_ref().map(DeclarationNode::to_owned),
            prolog: self.prolog.iter().map(Node::to_owned).collect(),
            root: self.root.to_owned(),
            epilog: self.epilog.iter().map(Node::to_owned).collect(),
        }
    }

    #[expect(clippy::too_many_lines, reason = "State machine; what did you expect")]
    fn parse(src: &'src str) -> XmlResult<Self> {
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
                        let last: TagNode = stack.pop().unwrap();
                        bail!(
                            src,
                            last.span(),
                            XmlErrorKind::UnclosedTag(last.name().to_string())
                        );
                    }
                };

                return Ok(Self {
                    src: Some(src),
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
                        stack.push(TagNode::new(maybe_empty(prefix), local).with_span(span));
                        state = ParserState::TagAttributes;
                    }

                    Token::Comment { text, .. } => prolog.push(Node::Comment(text.into())),

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

                        declaration = Some(
                            DeclarationNode::new(version, encoding, standalone).with_span(span),
                        );
                    }

                    Token::ProcessingInstruction {
                        target,
                        content,
                        span,
                    } => {
                        let node = ProcessingInstructionNode::new(span, target, content);
                        prolog.push(Node::ProcessingInstruction(node));
                    }

                    Token::EmptyDtd { .. } | Token::DtdStart { .. } => {
                        let node = DtdNode::parse(next, &mut tokenizer, src)?;
                        prolog.push(Node::DocumentType(node));
                    }

                    Token::Cdata { text, span } => {
                        let node = CdataNode::new(span, text);
                        prolog.push(Node::Cdata(node));
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
                        let attr =
                            NodeAttribute::new(maybe_empty(prefix), local, value).with_span(span);
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(
                                src,
                                span,
                                msg = "Bug; Cannot apply attribute; stack is empty!"
                            );
                        };

                        node.push_attribute(attr);
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

                        node.push_child(Node::Comment(text.into()));
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

                        node.extend_span(&next.span().into(), src);

                        let Some(parent) = stack.last_mut() else {
                            state = ParserState::Epilog;
                            stack.push(node);
                            continue;
                        };

                        parent.push_child(Node::Child(node));
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
                        stack.push(TagNode::new(maybe_empty(prefix), local).with_span(span));
                        state = ParserState::TagAttributes;
                    }

                    Token::Cdata { text, span } => {
                        let cnode = CdataNode::new(span, text);
                        let Some(node) = stack.last_mut() else {
                            let span = next.span();
                            bail!(src, span, msg = "Bug; Cannot apply cdata; stack is empty!");
                        };

                        node.push_child(Node::Cdata(cnode));
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

                        let text = StrSpan::new(text, start);
                        let span = next.span();
                        let text = TextNode::new(span, text);
                        node.push_child(Node::Text(text));
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

                        node.push_child(Node::Comment(text.into()));
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

                        let pi = ProcessingInstructionNode::new(span, target, content);
                        node.push_child(Node::ProcessingInstruction(pi));
                    }

                    Token::ElementEnd {
                        end: ElementEnd::Close(prefix, local),
                        ..
                    } => {
                        let Some(mut node) = stack.pop() else {
                            let span = next.span();
                            bail!(src, span, msg = "Bug; Cannot close tag; stack is empty!");
                        };

                        node.extend_span(&next.span().into(), src);

                        let name = NodeName::new(maybe_empty(prefix), local);
                        if node.name() != &name {
                            let span = next.span();
                            bail!(
                                src,
                                span,
                                XmlErrorKind::UnclosedTag(node.name().to_string())
                            );
                        }

                        state = ParserState::TagChildren;
                        if let Some(parent) = stack.last_mut() {
                            parent.push_child(Node::Child(node));
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
                    Token::Comment { text, .. } => epilog.push(Node::Comment(text.into())),

                    Token::Cdata { text, span } => {
                        let node = CdataNode::new(span, text);
                        epilog.push(Node::Cdata(node));
                    }

                    Token::ProcessingInstruction {
                        target,
                        content,
                        span,
                    } => {
                        let node = ProcessingInstructionNode::new(span, target, content);
                        epilog.push(Node::ProcessingInstruction(node));
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
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        if let Some(src) = self.src {
            encoder.write_all(Self::HEADER_SOURCED)?;
            encoder.with_source_header();
            src.write(encoder)?;
        } else {
            encoder.write_all(Self::HEADER_UNSOURCED)?;
        }

        self.declaration.write(encoder)?;
        self.prolog.write(encoder)?;
        self.root.write(encoder)?;
        self.epilog.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let header = decoder.read_all(4)?;
        let src = match header {
            Self::HEADER_SOURCED => {
                let src = <&str>::read(decoder)?;
                decoder.with_source(src);
                Some(src)
            }
            Self::HEADER_UNSOURCED => None,
            _ => {
                return Err(BinDecodeError::InvalidHeader);
            }
        };

        let declaration = Option::<DeclarationNode>::read(decoder)?;
        let prolog = Vec::<Node>::read(decoder)?;
        let root = TagNode::read(decoder)?;
        let epilog = Vec::<Node>::read(decoder)?;

        Ok(Self {
            src,
            declaration,
            prolog,
            root,
            epilog,
        })
    }
}

/// An owned version of the XML document, with no source span information. See [`Document`].
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedDocument {
    /// The XML declaration node, if present.
    pub declaration: Option<OwnedDeclarationNode>,

    /// The prolog of the document, which is everything between the declaration and root.
    /// This includes comments, DTDs, and processing instructions.
    pub prolog: Vec<OwnedNode>,

    /// The root node of the document.
    pub root: OwnedTagNode,

    /// The epilog of the document, which is everything after the root.  
    /// Technically this is not valid XML, but it is parsed anyway.
    pub epilog: Vec<OwnedNode>,
}
impl OwnedDocument {
    /// Create a new document from the given root node.
    ///
    /// # Example
    /// ```rust
    /// use xmltree::{OwnedDocument, node::OwnedTagNode};
    ///
    /// let root = OwnedTagNode::new("root");
    /// let doc = OwnedDocument::new(root);
    /// assert_eq!(doc.root.name, "root");
    /// ```
    pub fn new(root: impl Into<OwnedTagNode>) -> Self {
        Self {
            declaration: None,
            prolog: vec![],
            root: root.into(),
            epilog: vec![],
        }
    }

    pub(crate) fn borrowed(&self) -> Document<'_> {
        Document {
            src: None,
            declaration: self
                .declaration
                .as_ref()
                .map(OwnedDeclarationNode::borrowed),
            prolog: self.prolog.iter().map(OwnedNode::borrowed).collect(),
            root: self.root.borrowed(),
            epilog: self.epilog.iter().map(OwnedNode::borrowed).collect(),
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
    /// use xmltree::Document;
    ///
    /// let src = "<test><test2>test</test2></test>";
    /// let doc = Document::parse_str(src).unwrap();
    ///
    /// let bin = doc.to_bin().unwrap();
    /// println!("Binary size: {:.2}kB", bin.len() as f64 / 1024.0);
    /// ```
    pub fn to_bin(&self) -> std::io::Result<Vec<u8>> {
        let mut encoder = Encoder::new();
        self.write(&mut encoder)?;
        Ok(encoder.into_inner())
    }

    /// Read a document from a flat binary format.
    ///
    /// # Errors
    /// Returns errors if the decoding fails
    ///
    /// # Example
    /// ```rust
    /// use xmltree::{Document};
    /// const DOC: &[u8] = include_bytes!("../examples/example.bin");
    ///
    /// let doc = Document::from_bin(DOC).unwrap();
    /// assert_eq!(doc.root().name(), "bookstore");
    /// ```
    pub fn from_bin(data: &[u8]) -> Result<Self, BinDecodeError> {
        let mut decoder = Decoder::new(data);
        let document = Self::read(&mut decoder)?;
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
    /// use xmltree::{Document};
    /// const SRC: &str = "<test><test2>test</test2></test>";
    ///
    /// let doc = Document::parse_str(SRC).unwrap();
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
    pub fn to_xml_with_writer<W: std::io::Write>(
        &self,
        writer: &mut W,
        tab_char: Option<&str>,
    ) -> std::io::Result<()> {
        let doc = self.borrowed();
        crate::to_xml::write_xml(writer, &doc, tab_char)
    }
}
impl<'src> ToBinHandler<'src> for OwnedDocument {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let document = Document::read(decoder)?;
        Ok(document.to_owned())
    }
}

/// The XML declaration node.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclarationNode<'src> {
    span: StrSpan<'src>,
    version: StrSpan<'src>,
    encoding: Option<StrSpan<'src>>,
    standalone: Option<bool>,
}
impl<'src> DeclarationNode<'src> {
    pub(crate) fn new<T: Into<StrSpan<'src>>>(
        version: T,
        encoding: Option<T>,
        standalone: Option<bool>,
    ) -> Self {
        Self {
            span: StrSpan::default(),
            version: version.into(),
            encoding: encoding.map(Into::into),
            standalone,
        }
    }

    pub(crate) fn with_span(mut self, span: impl Into<StrSpan<'src>>) -> Self {
        self.span = span.into();
        self
    }

    /// Returns the span of the declaration in the original source.
    #[must_use]
    pub fn span(&self) -> &StrSpan<'src> {
        &self.span
    }

    /// Returns the version of the XML declaration.
    #[must_use]
    pub fn version(&self) -> &StrSpan<'src> {
        &self.version
    }

    /// Returns the encoding of the XML declaration, if present.
    #[must_use]
    pub fn encoding(&self) -> Option<&StrSpan<'src>> {
        self.encoding.as_ref()
    }

    /// Returns the standalone attribute of the XML declaration, if present.
    #[must_use]
    pub fn standalone(&self) -> Option<bool> {
        self.standalone
    }

    pub(crate) fn to_owned(&self) -> OwnedDeclarationNode {
        OwnedDeclarationNode {
            version: self.version.text().to_string(),
            encoding: self.encoding.as_ref().map(|s| s.text().to_string()),
            standalone: self.standalone,
        }
    }
}

impl<'src> ToBinHandler<'src> for DeclarationNode<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.version.write(encoder)?;
        self.encoding.write(encoder)?;
        self.standalone.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
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

/// Owned version of the XML declaration node, with no span metadata. See [`DeclarationNode`].
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedDeclarationNode {
    /// The version of the XML declaration.
    pub version: String,

    /// The encoding of the XML declaration, if present.
    pub encoding: Option<String>,

    /// The standalone attribute of the XML declaration, if present.
    pub standalone: Option<bool>,
}
impl OwnedDeclarationNode {
    /// Create a new XML declaration node.
    pub fn new(
        version: impl Into<String>,
        encoding: Option<impl Into<String>>,
        standalone: Option<bool>,
    ) -> Self {
        Self {
            version: version.into(),
            encoding: encoding.map(Into::into),
            standalone,
        }
    }

    pub(crate) fn borrowed(&self) -> DeclarationNode<'_> {
        DeclarationNode::new(
            self.version.as_str(),
            self.encoding.as_deref(),
            self.standalone,
        )
    }
}
impl<'src> ToBinHandler<'src> for OwnedDeclarationNode {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = DeclarationNode::read(decoder)?;
        Ok(node.to_owned())
    }
}

fn maybe_empty(s: xmlparser::StrSpan) -> Option<xmlparser::StrSpan<'_>> {
    if s.is_empty() { None } else { Some(s) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin() {
        let src = "<test><test2>test</test2></test>";
        let doc = Document::parse_str(src).unwrap();
        let doc2 = doc.to_owned();
        let borrowed_bin = doc.to_bin().unwrap();
        let owned_bin = doc2.to_bin().unwrap();

        // Print the owned bin to a file
        std::fs::write("owned.bin", &owned_bin).unwrap();

        // Borrowed -> borrowed
        let borrowed_doc = Document::from_bin(&borrowed_bin).unwrap();
        assert_eq!(borrowed_doc, doc);

        // Borrowed -> owned
        let owned_doc = OwnedDocument::from_bin(&borrowed_bin).unwrap();
        assert_eq!(owned_doc, doc2);

        // Owned -> borrowed
        let borrowed_doc = Document::from_bin(&owned_bin).unwrap();
        assert_eq!(
            borrowed_doc.to_owned().borrowed(),
            doc.to_owned().borrowed()
        );

        // Owned -> owned
        let owned_doc = OwnedDocument::from_bin(&owned_bin).unwrap();
        assert_eq!(owned_doc, doc2);
    }
}
