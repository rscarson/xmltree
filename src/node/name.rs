use crate::{
    StrSpan,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};

/// The name of a node, with an optional prefix:  
/// `prefix:local`
#[derive(Debug, Clone)]
pub struct NodeName<'src> {
    prefix: Option<StrSpan<'src>>,
    local: StrSpan<'src>,
}
impl<'src> NodeName<'src> {
    pub(crate) fn new<T: Into<StrSpan<'src>>>(prefix: Option<T>, local: T) -> Self {
        Self {
            prefix: prefix.map(Into::into),
            local: local.into(),
        }
    }

    /// Compare the name with a prefix and local name.
    #[must_use]
    pub fn equals(&self, prefix: Option<&str>, local: &str) -> bool {
        self.prefix.map(|s| s.text()) == prefix && self.local.text() == local
    }

    /// Returns the prefix of the node name.
    #[must_use]
    pub fn prefix(&self) -> Option<&StrSpan<'src>> {
        self.prefix.as_ref()
    }

    /// Returns the local portion of the node name.
    #[must_use]
    pub fn local(&self) -> &StrSpan<'src> {
        &self.local
    }

    /// Get an owned version of the node name, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedNodeName {
        OwnedNodeName {
            prefix: self.prefix.as_ref().map(|s| s.text().to_string()),
            local: self.local.text().to_string(),
        }
    }
}
impl<'src> ToBinHandler<'src> for NodeName<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.prefix.write(encoder)?;
        self.local.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
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
        self.prefix.map(|s| s.text()) == other.prefix.map(|s| s.text())
            && self.local.text() == other.local.text()
    }
}
impl PartialEq<&str> for NodeName<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.to_string().as_str() == *other
    }
}
impl PartialEq<str> for NodeName<'_> {
    fn eq(&self, other: &str) -> bool {
        self.to_string().as_str() == other
    }
}

/// Owned version of a node name, with no span metadata. See [`NodeName`].
#[derive(Debug, Clone)]
pub struct OwnedNodeName {
    /// The prefix of the node name.
    pub prefix: Option<String>,

    /// The local portion of the node name.
    pub local: String,
}
impl OwnedNodeName {
    /// Create a new node name.
    pub fn new<T: Into<String>>(prefix: Option<T>, local: T) -> Self {
        Self {
            prefix: prefix.map(Into::into),
            local: local.into(),
        }
    }

    /// Compare the name with a prefix and local name.
    #[must_use]
    pub fn equals(&self, prefix: Option<&str>, local: &str) -> bool {
        self.prefix.as_deref() == prefix && self.local.as_str() == local
    }

    pub(crate) fn borrowed(&self) -> NodeName<'_> {
        NodeName::new(self.prefix.as_deref(), self.local.as_str())
    }
}

impl<'src> ToBinHandler<'src> for OwnedNodeName {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let node = NodeName::read(decoder)?;
        Ok(node.to_owned())
    }
}
impl std::fmt::Display for OwnedNodeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(prefix) = &self.prefix {
            write!(f, "{prefix}:{local}", local = self.local)
        } else {
            write!(f, "{local}", local = self.local)
        }
    }
}
impl PartialEq for OwnedNodeName {
    fn eq(&self, other: &Self) -> bool {
        self.prefix.as_deref() == other.prefix.as_deref()
            && self.local.as_str() == other.local.as_str()
    }
}
impl PartialEq<&str> for OwnedNodeName {
    fn eq(&self, other: &&str) -> bool {
        self.to_string().as_str() == *other
    }
}
impl PartialEq<str> for OwnedNodeName {
    fn eq(&self, other: &str) -> bool {
        self.to_string().as_str() == other
    }
}
impl PartialEq<NodeName<'_>> for OwnedNodeName {
    fn eq(&self, other: &NodeName<'_>) -> bool {
        self.prefix.as_deref() == other.prefix.map(|s| s.text())
            && self.local.as_str() == other.local.text()
    }
}

impl From<&str> for OwnedNodeName {
    fn from(name: &str) -> Self {
        let parts: Vec<&str> = name.split(':').collect();
        match parts.as_slice() {
            [local] => OwnedNodeName::new(None, *local),
            [prefix, local] => OwnedNodeName::new(Some(*prefix), *local),
            _ => panic!("Invalid node name format"),
        }
    }
}
impl From<String> for OwnedNodeName {
    fn from(name: String) -> Self {
        OwnedNodeName::from(name.as_str())
    }
}
