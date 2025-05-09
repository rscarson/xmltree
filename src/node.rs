use super::{StrSpan, dtd::DtdNode};
use crate::{
    DocumentSourceRef,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};

/// The name of a node, with an optional prefix:  
/// `prefix:local`
#[derive(Debug, Clone)]
pub struct NodeName<'src> {
    /// The prefix of the node, if any.
    pub prefix: Option<StrSpan<'src>>,

    /// The local name of the node.
    pub local: StrSpan<'src>,
}
impl<'src> NodeName<'src> {
    /// Create a new node name.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(
        arena: &'src DocumentSourceRef,
        prefix: Option<&'b str>,
        local: &'b str,
    ) -> Self {
        let prefix = prefix.map(|s| StrSpan::from_unallocated(arena, s));
        let local = StrSpan::from_unallocated(arena, local);

        NodeName { prefix, local }
    }

    pub(crate) fn new(prefix: StrSpan<'src>, local: StrSpan<'src>) -> Self {
        let prefix = if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        };

        NodeName { prefix, local }
    }
}

impl<'src> ToBinHandler<'src> for NodeName<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.prefix.write(encoder)?;
        self.local.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let prefix = Option::<StrSpan>::read(decoder)?;
        let local = StrSpan::read(decoder)?;

        Ok(NodeName { prefix, local })
    }
}
impl std::fmt::Display for NodeName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(prefix) = self.prefix {
            write!(f, "{prefix}:{local}", local = self.local)
        } else {
            write!(f, "{local}", local = self.local)
        }
    }
}
impl PartialEq for NodeName<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.prefix.map(|s| s.as_str()) == other.prefix.map(|s| s.as_str())
            && self.local.as_str() == other.local.as_str()
    }
}
impl PartialEq<&str> for NodeName<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.to_string().as_str() == *other
    }
}

/// An attribute set on a node, with a name and value:
/// `name="value"`
///
/// A node can have multiple attributes with the same name, but only the last one is used for lookups.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeAttribute<'src> {
    /// The span of the attribute in the input XML.
    pub span: StrSpan<'src>,

    /// The name of the attribute
    pub name: NodeName<'src>,

    /// The value of the attribute
    pub value: StrSpan<'src>,
}
impl<'src> NodeAttribute<'src> {
    /// Create a new attribute with the given name and value.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(
        arena: &'src DocumentSourceRef,
        prefix: Option<&'b str>,
        local: &'b str,
        value: &'b str,
    ) -> Self {
        let name = NodeName::from_unallocated(arena, prefix, local);
        let value = StrSpan::from_unallocated(arena, value);

        Self {
            span: StrSpan::default(),
            name,
            value,
        }
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
    }

    pub(crate) fn new(
        span: StrSpan<'src>,
        prefix: StrSpan<'src>,
        local: StrSpan<'src>,
        value: StrSpan<'src>,
    ) -> Self {
        let name = NodeName::new(prefix, local);
        Self { span, name, value }
    }
}
impl<'src> ToBinHandler<'src> for NodeAttribute<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.value.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let name = NodeName::read(decoder)?;
        let value = StrSpan::read(decoder)?;

        Ok(NodeAttribute { span, name, value })
    }
}

/// A node in the document tree, with a name, attributes, and children:
/// `<name attr="value">...</name>`
///
/// Duplicate attributes are allowed (searches use the last attribute with the same name).
#[derive(Debug, Clone, PartialEq)]
pub struct Node<'src> {
    /// The span of the node in the input XML.
    pub span: StrSpan<'src>,

    /// The name of the node.
    pub name: NodeName<'src>,

    /// The attributes of the node.
    pub attributes: Vec<NodeAttribute<'src>>,

    /// The children of the node.
    pub children: Vec<NodeKind<'src>>,
}
impl<'src> Node<'src> {
    /// Create a new node with the given name and attributes
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(
        arena: &'src DocumentSourceRef,
        prefix: Option<&'b str>,
        local: &'b str,
    ) -> Self {
        let prefix = prefix.map(|s| StrSpan::from_unallocated(arena, s));
        let local = StrSpan::from_unallocated(arena, local);
        let name = NodeName { prefix, local };

        Self {
            span: StrSpan::default(),
            name,
            attributes: vec![],
            children: vec![],
        }
    }

    /// Get an attribute by name.
    ///
    /// Searches the attributes in reverse order, so the last attribute with the same name is returned.
    #[must_use]
    pub fn get_attribute(&self, prefix: Option<&str>, name: &str) -> Option<&NodeAttribute<'src>> {
        self.attributes
            .iter()
            .rev()
            .find(|a| a.name.local == name && a.name.prefix.map(|s| s.as_str()) == prefix)
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
        for attr in &mut self.attributes {
            attr.strip_metadata();
        }
        for child in &mut self.children {
            child.strip_metadata();
        }
    }

    pub(crate) fn from_spans(
        span: StrSpan<'src>,
        prefix: StrSpan<'src>,
        local: StrSpan<'src>,
    ) -> Self {
        let name = NodeName::new(prefix, local);
        Self {
            span,
            name,
            attributes: vec![],
            children: vec![],
        }
    }
}
impl<'src> ToBinHandler<'src> for Node<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.attributes.write(encoder)?;
        self.children.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let name = NodeName::read(decoder)?;
        let attributes = Vec::<NodeAttribute>::read(decoder)?;
        let children = Vec::<NodeKind>::read(decoder)?;

        Ok(Node {
            span,
            name,
            attributes,
            children,
        })
    }
}

/// A non-empty span of text inside a node of the document tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode<'src> {
    /// The span of the text node in the input XML.
    pub span: StrSpan<'src>,

    /// The text of the node (trimmed)
    pub text: StrSpan<'src>,
}
impl<'src> TextNode<'src> {
    /// Create a new text node.
    pub(crate) fn new(span: StrSpan<'src>, text: StrSpan<'src>) -> Self {
        Self { span, text }
    }

    /// Create a new node.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(arena: &'src DocumentSourceRef, text: &'b str) -> Self {
        let text = StrSpan::from_unallocated(arena, text);
        Self {
            span: StrSpan::default(),
            text,
        }
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
    }
}
impl<'src> ToBinHandler<'src> for TextNode<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.text.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let text = StrSpan::read(decoder)?;

        Ok(TextNode { span, text })
    }
}

/// A processing instruction node:  
/// `<?target content?>`
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessingInstructionNode<'src> {
    /// The span of the processing instruction node in the input XML.
    pub span: StrSpan<'src>,

    /// The target of the processing instruction.
    pub target: StrSpan<'src>,

    /// The content of the processing instruction.
    pub content: Option<StrSpan<'src>>,
}
impl<'src> ProcessingInstructionNode<'src> {
    /// Create a new processing instruction node.
    pub(crate) fn new(
        span: StrSpan<'src>,
        target: StrSpan<'src>,
        content: Option<StrSpan<'src>>,
    ) -> Self {
        Self {
            span,
            target,
            content,
        }
    }

    /// Create a new node.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(
        arena: &'src DocumentSourceRef,
        target: &'b str,
        content: Option<&'b str>,
    ) -> Self {
        let target = StrSpan::from_unallocated(arena, target);
        let content = content.map(|s| StrSpan::from_unallocated(arena, s));

        Self {
            span: StrSpan::default(),
            target,
            content,
        }
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
    }
}
impl<'src> ToBinHandler<'src> for ProcessingInstructionNode<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.target.write(encoder)?;
        self.content.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let target = StrSpan::read(decoder)?;
        let content = Option::<StrSpan>::read(decoder)?;

        Ok(ProcessingInstructionNode {
            span,
            target,
            content,
        })
    }
}

/// A CDATA node:  
/// `<![CDATA[content]]>`
#[derive(Debug, Clone, PartialEq)]
pub struct CdataNode<'src> {
    /// The span of the CDATA node in the input XML.
    pub span: StrSpan<'src>,

    /// The content of the CDATA node.
    pub content: StrSpan<'src>,
}
impl<'src> CdataNode<'src> {
    /// Create a new CDATA node.
    pub(crate) fn new(span: StrSpan<'src>, content: StrSpan<'src>) -> Self {
        Self { span, content }
    }

    /// Create a new node.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(arena: &'src DocumentSourceRef, content: &'b str) -> Self {
        let content = StrSpan::from_unallocated(arena, content);

        Self {
            span: StrSpan::default(),
            content,
        }
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
    }
}
impl<'src> ToBinHandler<'src> for CdataNode<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.content.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let content = StrSpan::read(decoder)?;

        Ok(CdataNode { span, content })
    }
}

/// A node in the document tree. Can be any of:
/// - `Child` - a tag node
/// - `Text` - a text node
/// - `Comment` - a comment node
/// - `ProcessingInstruction` - a processing instruction node
/// - `DocumentType` - a DTD node
/// - `Cdata` - a CDATA node
///
/// Prolog and epilog of a document can contain any of these except for child nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind<'src> {
    /// A tag node.
    Child(Node<'src>),

    /// A text node.
    Text(TextNode<'src>),

    /// A comment node.
    Comment(StrSpan<'src>),

    /// A processing instruction node.
    ProcessingInstruction(ProcessingInstructionNode<'src>),

    /// A DTD node.
    DocumentType(DtdNode<'src>),

    /// A CDATA node.
    Cdata(CdataNode<'src>),
}
impl NodeKind<'_> {
    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        match self {
            NodeKind::Child(node) => node.strip_metadata(),
            NodeKind::Text(node) => node.strip_metadata(),
            NodeKind::Comment(_) => (),
            NodeKind::ProcessingInstruction(node) => node.strip_metadata(),
            NodeKind::DocumentType(node) => node.strip_metadata(),
            NodeKind::Cdata(node) => node.strip_metadata(),
        }
    }
}

impl<'src> ToBinHandler<'src> for NodeKind<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        let kind: u8 = match self {
            NodeKind::Child(_) => 0,
            NodeKind::Text(_) => 1,
            NodeKind::Comment(_) => 2,
            NodeKind::ProcessingInstruction(_) => 3,
            NodeKind::DocumentType(_) => 4,
            NodeKind::Cdata(_) => 5,
        };
        kind.write(encoder)?;
        match self {
            NodeKind::Child(node) => node.write(encoder)?,
            NodeKind::Text(node) => node.write(encoder)?,
            NodeKind::Comment(span) => span.write(encoder)?,
            NodeKind::ProcessingInstruction(node) => node.write(encoder)?,
            NodeKind::DocumentType(node) => node.write(encoder)?,
            NodeKind::Cdata(node) => node.write(encoder)?,
        }
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let kind = u8::read(decoder)?;
        let node = match kind {
            0 => NodeKind::Child(Node::read(decoder)?),
            1 => NodeKind::Text(TextNode::read(decoder)?),
            2 => NodeKind::Comment(StrSpan::read(decoder)?),
            3 => NodeKind::ProcessingInstruction(ProcessingInstructionNode::read(decoder)?),
            4 => NodeKind::DocumentType(DtdNode::read(decoder)?),
            5 => NodeKind::Cdata(CdataNode::read(decoder)?),
            _ => return Err(BinDecodeError::InvalidEnumVariant),
        };

        Ok(node)
    }
}
