use crate::{
    StrSpan,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};

/// A non-empty span of text inside a node of the document tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode<'src> {
    /// The span of the text node in the input XML.
    span: StrSpan<'src>,

    /// The text of the node (trimmed)
    text: StrSpan<'src>,
}
impl<'src> TextNode<'src> {
    /// Create a new text node.
    pub(crate) fn new(span: impl Into<StrSpan<'src>>, text: impl Into<StrSpan<'src>>) -> Self {
        Self {
            span: span.into(),
            text: text.into(),
        }
    }

    /// Returns the span of the node in the original source.
    #[must_use]
    pub fn span(&self) -> &StrSpan<'src> {
        &self.span
    }

    /// Returns the text of the node.
    /// The text is trimmed of leading and trailing whitespace.
    #[must_use]
    pub fn text(&self) -> &StrSpan<'src> {
        &self.text
    }

    /// Returns an owned version of the text node, with no span metadata
    #[must_use]
    pub fn to_owned(&self) -> OwnedTextNode {
        OwnedTextNode {
            text: self.text.text().to_string(),
        }
    }
}
impl<'src> ToBinHandler<'src> for TextNode<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.text.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let text = StrSpan::read(decoder)?;

        Ok(Self { span, text })
    }
}

/// A non-empty span of text inside a node of the document tree.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedTextNode {
    /// The inner text of the node.
    pub text: String,
}
impl OwnedTextNode {
    /// Create a new text node.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub(crate) fn borrowed(&self) -> TextNode<'_> {
        TextNode::new("", self.text.as_str())
    }
}
impl<'src> ToBinHandler<'src> for OwnedTextNode {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = TextNode::read(decoder)?;
        Ok(node.to_owned())
    }
}
