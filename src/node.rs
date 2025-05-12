//! The types of nodes and related data that can appear in an XML document.
use super::StrSpan;
use crate::to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler};

mod name;
pub use name::*;

mod text;
pub use text::*;

mod tag;
pub use tag::*;

mod pi;
pub use pi::*;

mod cdata;
pub use cdata::*;

mod dtd;
pub use dtd::*;

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
pub enum Node<'src> {
    /// A tag node.
    Child(TagNode<'src>),

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
impl Node<'_> {
    /// Returns an owned version of the node, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedNode {
        match self {
            Self::Child(node) => OwnedNode::Tag(node.to_owned()),
            Self::Text(node) => OwnedNode::Text(node.to_owned()),
            Self::Comment(span) => OwnedNode::Comment(span.text().to_string()),
            Self::ProcessingInstruction(node) => OwnedNode::ProcessingInstruction(node.to_owned()),
            Self::DocumentType(node) => OwnedNode::DocumentType(node.to_owned()),
            Self::Cdata(node) => OwnedNode::Cdata(node.to_owned()),
        }
    }
}

impl<'src> ToBinHandler<'src> for Node<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        let kind: u8 = match self {
            Self::Child(_) => 0,
            Self::Text(_) => 1,
            Self::Comment(_) => 2,
            Self::ProcessingInstruction(_) => 3,
            Self::DocumentType(_) => 4,
            Self::Cdata(_) => 5,
        };
        kind.write(encoder)?;
        match self {
            Self::Child(node) => node.write(encoder)?,
            Self::Text(node) => node.write(encoder)?,
            Self::Comment(span) => span.write(encoder)?,
            Self::ProcessingInstruction(node) => node.write(encoder)?,
            Self::DocumentType(node) => node.write(encoder)?,
            Self::Cdata(node) => node.write(encoder)?,
        }
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let kind = u8::read(decoder)?;
        let node = match kind {
            0 => Node::Child(TagNode::read(decoder)?),
            1 => Node::Text(TextNode::read(decoder)?),
            2 => Node::Comment(StrSpan::read(decoder)?),
            3 => Node::ProcessingInstruction(ProcessingInstructionNode::read(decoder)?),
            4 => Node::DocumentType(DtdNode::read(decoder)?),
            5 => Node::Cdata(CdataNode::read(decoder)?),
            _ => return Err(BinDecodeError::InvalidEnumVariant),
        };

        Ok(node)
    }
}

/// An owned version of a node in the document tree. See [`Node`].
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedNode {
    /// A tag node.
    Tag(OwnedTagNode),

    /// A text node.
    Text(OwnedTextNode),

    /// A comment node.
    Comment(String),

    /// A processing instruction node.
    ProcessingInstruction(OwnedProcessingInstructionNode),

    /// A DTD node.
    DocumentType(OwnedDtdNode),

    /// A CDATA node.
    Cdata(OwnedCdataNode),
}
impl OwnedNode {
    pub(crate) fn borrowed(&self) -> Node<'_> {
        match self {
            Self::Tag(node) => Node::Child(node.borrowed()),
            Self::Text(node) => Node::Text(node.borrowed()),
            Self::Comment(text) => Node::Comment(StrSpan::from(text.as_str())),
            Self::ProcessingInstruction(node) => Node::ProcessingInstruction(node.borrowed()),
            Self::DocumentType(node) => Node::DocumentType(node.borrowed()),
            Self::Cdata(node) => Node::Cdata(node.borrowed()),
        }
    }
}
impl<'src> ToBinHandler<'src> for OwnedNode {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = Node::read(decoder)?;
        Ok(node.to_owned())
    }
}
