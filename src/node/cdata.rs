use crate::{
    StrSpan,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};

/// A CDATA node:  
/// `<![CDATA[content]]>`
#[derive(Debug, Clone, PartialEq)]
pub struct CdataNode<'src> {
    span: StrSpan<'src>,
    content: StrSpan<'src>,
}
impl<'src> CdataNode<'src> {
    /// Create a new CDATA node.
    pub(crate) fn new<T: Into<StrSpan<'src>>>(span: T, content: T) -> Self {
        Self {
            span: span.into(),
            content: content.into(),
        }
    }

    /// Returns the span of the node in the original source.
    #[must_use]
    pub fn span(&self) -> &StrSpan<'src> {
        &self.span
    }

    /// Returns the content of the CDATA node.
    #[must_use]
    pub fn content(&self) -> &StrSpan<'src> {
        &self.content
    }

    /// Returns an owned version of the CDATA node, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedCdataNode {
        OwnedCdataNode {
            content: self.content.text().to_string(),
        }
    }
}
impl<'src> ToBinHandler<'src> for CdataNode<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.content.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let content = StrSpan::read(decoder)?;

        Ok(CdataNode { span, content })
    }
}

/// An owned version of a CDATA node, with no span metadata. See [`CdataNode`].
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedCdataNode {
    /// The inner content of the CDATA node.
    pub content: String,
}
impl OwnedCdataNode {
    /// Create a new CDATA node.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    pub(crate) fn borrowed(&self) -> CdataNode<'_> {
        CdataNode::new("", self.content.as_str())
    }
}
impl<'src> ToBinHandler<'src> for OwnedCdataNode {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = CdataNode::read(decoder)?;
        Ok(Self::new(node.content.text()))
    }
}
