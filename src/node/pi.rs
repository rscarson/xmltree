use crate::{
    StrSpan,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};

/// A processing instruction node:  
/// `<?target content?>`
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessingInstructionNode<'src> {
    span: StrSpan<'src>,
    target: StrSpan<'src>,
    content: Option<StrSpan<'src>>,
}
impl<'src> ProcessingInstructionNode<'src> {
    /// Create a new processing instruction node.
    pub(crate) fn new<T: Into<StrSpan<'src>>>(span: T, target: T, content: Option<T>) -> Self {
        Self {
            span: span.into(),
            target: target.into(),
            content: content.map(Into::into),
        }
    }

    /// Returns the span of the node in the original source.
    #[must_use]
    pub fn span(&self) -> &StrSpan<'src> {
        &self.span
    }

    /// Returns the target of the processing instruction.
    #[must_use]
    pub fn target(&self) -> &StrSpan<'src> {
        &self.target
    }

    /// Returns the content of the processing instruction.
    #[must_use]
    pub fn content(&self) -> Option<&StrSpan<'src>> {
        self.content.as_ref()
    }

    /// Returns an owned version of the processing instruction node, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedProcessingInstructionNode {
        OwnedProcessingInstructionNode {
            target: self.target.text().to_string(),
            content: self.content.as_ref().map(|s| s.text().to_string()),
        }
    }
}
impl<'src> ToBinHandler<'src> for ProcessingInstructionNode<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.target.write(encoder)?;
        self.content.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
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

/// An owned version of a processing instruction node, with no span metadata. See [`ProcessingInstructionNode`].
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedProcessingInstructionNode {
    /// The target of the processing instruction.
    pub target: String,

    /// The content of the processing instruction.
    pub content: Option<String>,
}
impl OwnedProcessingInstructionNode {
    /// Create a new processing instruction node.
    #[must_use]
    pub fn new(target: String, content: Option<String>) -> Self {
        Self { target, content }
    }

    pub(crate) fn borrowed(&self) -> ProcessingInstructionNode<'_> {
        ProcessingInstructionNode::new("", self.target.as_str(), self.content.as_deref())
    }
}
impl<'src> ToBinHandler<'src> for OwnedProcessingInstructionNode {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = ProcessingInstructionNode::read(decoder)?;
        Ok(node.to_owned())
    }
}
