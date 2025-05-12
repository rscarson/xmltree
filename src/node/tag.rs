use super::{Node, NodeName, OwnedNode, OwnedNodeName};
use crate::{
    StrSpan,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};

/// A node in the document tree, with a name, attributes, and children:
/// `<name attr="value">...</name>`
///
/// Duplicate attributes are allowed (searches use the last attribute with the same name).
#[derive(Debug, Clone, PartialEq)]
pub struct TagNode<'src> {
    span: StrSpan<'src>,
    name: NodeName<'src>,
    attributes: Vec<NodeAttribute<'src>>,
    children: Vec<Node<'src>>,
}
impl<'src> TagNode<'src> {
    pub(crate) fn new<T: Into<StrSpan<'src>>>(prefix: Option<T>, local: T) -> Self {
        Self {
            span: StrSpan::default(),
            name: NodeName::new(prefix, local),
            attributes: vec![],
            children: vec![],
        }
    }

    pub(crate) fn with_span(mut self, span: impl Into<StrSpan<'src>>) -> Self {
        self.span = span.into();
        self
    }

    pub(crate) fn push_child(&mut self, child: Node<'src>) {
        self.children.push(child);
    }

    pub(crate) fn push_attribute(&mut self, attribute: NodeAttribute<'src>) {
        self.attributes.push(attribute);
    }

    pub(crate) fn extend_span(&mut self, span: &StrSpan<'src>, src: &'src str) {
        self.span.extend(span, src);
    }

    /// Get an attribute by name.
    ///
    /// Searches the attributes in reverse order, so the last attribute with the same name is returned.
    #[must_use]
    pub fn get_attribute(&self, prefix: Option<&str>, name: &str) -> Option<&NodeAttribute<'src>> {
        self.attributes
            .iter()
            .rev()
            .find(|a| a.name.equals(prefix, name))
    }

    /// Get the span of the node in the original source.
    #[must_use]
    pub fn span(&self) -> &StrSpan<'src> {
        &self.span
    }

    /// Get the name of the node.
    #[must_use]
    pub fn name(&self) -> &NodeName<'src> {
        &self.name
    }

    /// Get the attributes of the node.
    #[must_use]
    pub fn attributes(&self) -> &[NodeAttribute<'src>] {
        &self.attributes
    }

    /// Get the children of the node.
    #[must_use]
    pub fn children(&self) -> &[Node<'src>] {
        &self.children
    }

    /// Get an owned version of the tag node, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedTagNode {
        OwnedTagNode {
            name: self.name.to_owned(),
            attributes: self
                .attributes
                .iter()
                .map(NodeAttribute::to_owned)
                .collect(),
            children: self.children.iter().map(Node::to_owned).collect(),
        }
    }
}
impl<'src> ToBinHandler<'src> for TagNode<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.attributes.write(encoder)?;
        self.children.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let name = NodeName::read(decoder)?;
        let attributes = Vec::<NodeAttribute>::read(decoder)?;
        let children = Vec::<Node>::read(decoder)?;

        Ok(TagNode {
            span,
            name,
            attributes,
            children,
        })
    }
}

/// An owned version of a tag node, with no span metadata. See [`TagNode`].
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedTagNode {
    /// The name of the node.
    pub name: OwnedNodeName,

    /// The attributes of the node.
    pub attributes: Vec<OwnedNodeAttribute>,

    /// The children of the node.
    pub children: Vec<OwnedNode>,
}
impl OwnedTagNode {
    /// Create a new tag node.
    #[must_use]
    pub fn new(name: impl Into<OwnedNodeName>) -> Self {
        Self {
            name: name.into(),
            attributes: vec![],
            children: vec![],
        }
    }

    /// Get an attribute by name.
    ///
    /// Searches the attributes in reverse order, so the last attribute with the same name is returned.
    #[must_use]
    pub fn get_attribute(&self, prefix: Option<&str>, name: &str) -> Option<&OwnedNodeAttribute> {
        self.attributes
            .iter()
            .rev()
            .find(|a| a.name.equals(prefix, name))
    }

    /// Get an attribute by name.
    ///
    /// Searches the attributes in reverse order, so the last attribute with the same name is returned.
    #[must_use]
    pub fn get_attribute_mut(
        &mut self,
        prefix: Option<&str>,
        name: &str,
    ) -> Option<&mut OwnedNodeAttribute> {
        self.attributes
            .iter_mut()
            .rev()
            .find(|a| a.name.equals(prefix, name))
    }

    pub(crate) fn borrowed(&self) -> TagNode<'_> {
        TagNode {
            span: StrSpan::default(),
            name: self.name.borrowed(),
            attributes: self.attributes.iter().map(|a| a.borrowed()).collect(),
            children: self.children.iter().map(|c| c.borrowed()).collect(),
        }
    }
}
impl<'src> ToBinHandler<'src> for OwnedTagNode {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = TagNode::read(decoder)?;
        Ok(node.to_owned())
    }
}

/// An attribute set on a node, with a name and value:
/// `name="value"`
///
/// A node can have multiple attributes with the same name, but only the last one is used for lookups.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeAttribute<'src> {
    span: StrSpan<'src>,
    name: NodeName<'src>,
    value: StrSpan<'src>,
}
impl<'src> NodeAttribute<'src> {
    pub(crate) fn new<T: Into<StrSpan<'src>>>(prefix: Option<T>, local: T, value: T) -> Self {
        Self {
            span: StrSpan::default(),
            name: NodeName::new(prefix, local),
            value: value.into(),
        }
    }

    pub(crate) fn with_span(mut self, span: impl Into<StrSpan<'src>>) -> Self {
        self.span = span.into();
        self
    }

    /// Returns the name of the attribute.
    #[must_use]
    pub fn name(&self) -> &NodeName<'src> {
        &self.name
    }

    /// Returns the value of the attribute.
    #[must_use]
    pub fn value(&self) -> &StrSpan<'src> {
        &self.value
    }

    /// Returns the span of the attribute in the original source
    #[must_use]
    pub fn span(&self) -> &StrSpan<'src> {
        &self.span
    }

    /// Returns an owned version of the attribute, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedNodeAttribute {
        OwnedNodeAttribute {
            name: self.name.to_owned(),
            value: self.value.text().to_string(),
        }
    }
}
impl<'src> ToBinHandler<'src> for NodeAttribute<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.value.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let name = NodeName::read(decoder)?;
        let value = StrSpan::read(decoder)?;

        Ok(NodeAttribute { span, name, value })
    }
}

/// Owned version of a node attribute, with no span metadata. See [`NodeAttribute`].
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedNodeAttribute {
    /// The name of the attribute.
    pub name: OwnedNodeName,

    /// The value of the attribute.
    pub value: String,
}
impl OwnedNodeAttribute {
    /// Create a new node attribute.
    pub fn new(name: impl Into<OwnedNodeName>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub(crate) fn borrowed(&self) -> NodeAttribute<'_> {
        NodeAttribute {
            span: StrSpan::default(),
            name: self.name.borrowed(),
            value: self.value.as_str().into(),
        }
    }
}
impl<'src> ToBinHandler<'src> for OwnedNodeAttribute {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = NodeAttribute::read(decoder)?;
        Ok(node.to_owned())
    }
}
