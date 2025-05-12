use crate::{
    StrSpan,
    error::{ErrorContext, XmlError, XmlErrorKind, XmlResult},
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};
use xmlparser::{Token, Tokenizer};

/// Representation of the [ExternalID](https://www.w3.org/TR/xml/#NT-ExternalID) value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExternalId<'src> {
    /// External ID containing a system identifier.
    System(StrSpan<'src>),

    /// External ID containing a public identifier and a system identifier.
    Public(StrSpan<'src>, StrSpan<'src>),
}
impl<'src> ExternalId<'src> {
    pub(crate) fn new_system(s: impl Into<StrSpan<'src>>) -> Self {
        ExternalId::System(s.into())
    }

    pub(crate) fn new_public<T: Into<StrSpan<'src>>>(p: T, s: T) -> Self {
        ExternalId::Public(p.into(), s.into())
    }

    /// Returns an owned version of the external ID, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedExternalId {
        match self {
            ExternalId::System(system) => OwnedExternalId::System(system.text().to_string()),
            ExternalId::Public(public, system) => {
                OwnedExternalId::Public(public.text().to_string(), system.text().to_string())
            }
        }
    }
}
impl<'src> From<xmlparser::ExternalId<'src>> for ExternalId<'src> {
    fn from(external_id: xmlparser::ExternalId<'src>) -> Self {
        match external_id {
            xmlparser::ExternalId::System(system) => ExternalId::System(system.into()),
            xmlparser::ExternalId::Public(public, system) => {
                ExternalId::Public(public.into(), system.into())
            }
        }
    }
}

/// An owned version of the external ID, with no span metadata. See [`ExternalId`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum OwnedExternalId {
    /// External ID containing a system identifier.
    System(String),

    /// External ID containing a public identifier and a system identifier.
    Public(String, String),
}
impl OwnedExternalId {
    /// Create a new external ID with the given system identifier.
    #[must_use]
    pub fn new_system(system: impl Into<String>) -> Self {
        OwnedExternalId::System(system.into())
    }

    /// Create a new external ID with the given public and system identifiers.
    #[must_use]
    pub fn new_public(public: impl Into<String>, system: impl Into<String>) -> Self {
        OwnedExternalId::Public(public.into(), system.into())
    }

    pub(crate) fn borrowed(&self) -> ExternalId {
        match self {
            OwnedExternalId::System(system) => ExternalId::new_system(system.as_str()),
            OwnedExternalId::Public(public, system) => {
                ExternalId::new_public(public.as_str(), system.as_str())
            }
        }
    }
}

impl<'src> ToBinHandler<'src> for ExternalId<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        let kind: u8 = match self {
            ExternalId::System(_) => 0,
            ExternalId::Public(_, _) => 1,
        };
        kind.write(encoder)?;
        match self {
            ExternalId::System(system) => system.write(encoder)?,
            ExternalId::Public(public, system) => {
                public.write(encoder)?;
                system.write(encoder)?;
            }
        }
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let kind = u8::read(decoder)?;
        match kind {
            0 => {
                let system = StrSpan::read(decoder)?;
                Ok(ExternalId::System(system))
            }
            1 => {
                let public = StrSpan::read(decoder)?;
                let system = StrSpan::read(decoder)?;
                Ok(ExternalId::Public(public, system))
            }
            _ => Err(BinDecodeError::InvalidEnumVariant),
        }
    }
}

/// Representation of the [EntityDef](https://www.w3.org/TR/xml/#NT-EntityDef) value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EntityDefinition<'src> {
    /// Entity containing a value.
    EntityValue(StrSpan<'src>),

    /// Entity containing an external ID.
    ExternalId(ExternalId<'src>),
}
impl<'src> EntityDefinition<'src> {
    pub(crate) fn new_entity_value(s: impl Into<StrSpan<'src>>) -> Self {
        EntityDefinition::EntityValue(s.into())
    }

    pub(crate) fn new_external_id(external_id: ExternalId<'src>) -> Self {
        EntityDefinition::ExternalId(external_id)
    }

    /// Returns an owned version of the entity definition, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedEntityDefinition {
        match self {
            EntityDefinition::EntityValue(value) => {
                OwnedEntityDefinition::EntityValue(value.text().to_string())
            }
            EntityDefinition::ExternalId(external_id) => {
                OwnedEntityDefinition::ExternalId(external_id.to_owned())
            }
        }
    }
}
impl<'src> From<xmlparser::EntityDefinition<'src>> for EntityDefinition<'src> {
    fn from(entity_definition: xmlparser::EntityDefinition<'src>) -> Self {
        match entity_definition {
            xmlparser::EntityDefinition::EntityValue(value) => {
                EntityDefinition::EntityValue(value.into())
            }
            xmlparser::EntityDefinition::ExternalId(external_id) => {
                EntityDefinition::ExternalId(external_id.into())
            }
        }
    }
}
impl<'src> ToBinHandler<'src> for EntityDefinition<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        let kind: u8 = match self {
            EntityDefinition::EntityValue(_) => 0,
            EntityDefinition::ExternalId(_) => 1,
        };
        kind.write(encoder)?;
        match self {
            EntityDefinition::EntityValue(value) => value.write(encoder)?,
            EntityDefinition::ExternalId(external_id) => {
                external_id.write(encoder)?;
            }
        }
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let kind = u8::read(decoder)?;
        match kind {
            0 => {
                let value = StrSpan::read(decoder)?;
                Ok(EntityDefinition::EntityValue(value))
            }
            1 => {
                let external_id = ExternalId::read(decoder)?;
                Ok(EntityDefinition::ExternalId(external_id))
            }
            _ => Err(BinDecodeError::InvalidEnumVariant),
        }
    }
}

/// An owned version of the entity definition, with no span metadata. See [`EntityDefinition`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum OwnedEntityDefinition {
    /// Entity containing a value.
    EntityValue(String),

    /// Entity containing an external ID.
    ExternalId(OwnedExternalId),
}
impl OwnedEntityDefinition {
    /// Create a new entity definition with the given value.
    #[must_use]
    pub fn new_entity_value(value: impl Into<String>) -> Self {
        OwnedEntityDefinition::EntityValue(value.into())
    }

    /// Create a new entity definition with the given external ID.
    #[must_use]
    pub fn new_external_id(external_id: OwnedExternalId) -> Self {
        OwnedEntityDefinition::ExternalId(external_id)
    }

    pub(crate) fn borrowed(&self) -> EntityDefinition {
        match self {
            OwnedEntityDefinition::EntityValue(value) => {
                EntityDefinition::new_entity_value(value.as_str())
            }
            OwnedEntityDefinition::ExternalId(external_id) => {
                EntityDefinition::new_external_id(external_id.borrowed())
            }
        }
    }
}

/// An entity declaration in a DTD.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DtdEntity<'src> {
    /// The span of the entity declaration in the source XML.
    pub span: StrSpan<'src>,

    /// The name of the entity.
    pub name: StrSpan<'src>,

    /// The definition of the entity.
    pub definition: EntityDefinition<'src>,
}
impl<'src> DtdEntity<'src> {
    pub(crate) fn new<T: Into<StrSpan<'src>>>(
        span: T,
        name: T,
        definition: EntityDefinition<'src>,
    ) -> Self {
        Self {
            span: span.into(),
            name: name.into(),
            definition,
        }
    }

    /// Returns an owned version of the entity, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedDtdEntity {
        OwnedDtdEntity {
            name: self.name.text().to_string(),
            definition: self.definition.to_owned(),
        }
    }
}
impl<'src> ToBinHandler<'src> for DtdEntity<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.definition.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let name = StrSpan::read(decoder)?;
        let definition = EntityDefinition::read(decoder)?;

        Ok(DtdEntity {
            span,
            name,
            definition,
        })
    }
}

/// An owned version of the DTD entity, with no span metadata. See [`DtdEntity`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct OwnedDtdEntity {
    /// The name of the entity.
    pub name: String,

    /// The definition of the entity.
    pub definition: OwnedEntityDefinition,
}
impl OwnedDtdEntity {
    /// Create a new DTD entity.
    pub fn new(name: impl Into<String>, definition: OwnedEntityDefinition) -> Self {
        Self {
            name: name.into(),
            definition,
        }
    }

    pub(crate) fn borrowed(&self) -> DtdEntity<'_> {
        DtdEntity::new("", self.name.as_str(), self.definition.borrowed())
    }
}
impl<'src> ToBinHandler<'src> for OwnedDtdEntity {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.borrowed().write(encoder)
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let entity = DtdEntity::read(decoder)?;
        Ok(entity.to_owned())
    }
}

/// The DTD node in the XML document.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DtdNode<'src> {
    span: StrSpan<'src>,
    name: StrSpan<'src>,
    external_id: Option<ExternalId<'src>>,
    entities: Vec<DtdEntity<'src>>,
}
impl<'src> DtdNode<'src> {
    /// Returns the span of the DTD node in the original source.
    #[must_use]
    pub fn span(&self) -> &StrSpan<'src> {
        &self.span
    }

    /// Returns the name of the DTD node.
    #[must_use]
    pub fn name(&self) -> &StrSpan<'src> {
        &self.name
    }

    /// Returns the external ID of the DTD node, if any.
    #[must_use]
    pub fn external_id(&self) -> Option<&ExternalId<'src>> {
        self.external_id.as_ref()
    }

    /// Returns the entities declared in the DTD node.
    #[must_use]
    pub fn entities(&self) -> &[DtdEntity<'src>] {
        &self.entities
    }

    /// Returns an owned version of the DTD node, with no span metadata.
    #[must_use]
    pub fn to_owned(&self) -> OwnedDtdNode {
        OwnedDtdNode {
            name: self.name.text().to_string(),
            external_id: self.external_id.as_ref().map(ExternalId::to_owned),
            entities: self.entities.iter().map(DtdEntity::to_owned).collect(),
        }
    }

    pub(crate) fn new<T: Into<StrSpan<'src>>>(
        span: T,
        name: T,
        external_id: Option<ExternalId<'src>>,
    ) -> Self {
        Self {
            span: span.into(),
            name: name.into(),
            external_id,
            entities: Vec::new(),
        }
    }

    pub(crate) fn parse(
        start: Token<'src>,
        tokenizer: &mut Tokenizer<'src>,
        src: &'src str,
    ) -> XmlResult<Self> {
        let mut node = match start {
            Token::DtdStart {
                span,
                name,
                external_id,
            } => DtdNode {
                span: StrSpan::from(span),
                name: StrSpan::from(name),
                external_id: external_id.map(Into::into),
                entities: Vec::new(),
            },

            Token::EmptyDtd {
                name,
                external_id,
                span,
            } => {
                return Ok(DtdNode {
                    span: StrSpan::from(span),
                    name: StrSpan::from(name),
                    external_id: external_id.map(Into::into),
                    entities: Vec::new(),
                });
            }

            _ => {
                return Err(XmlError::new(
                    XmlErrorKind::Custom("Expected DTD start or empty DTD".to_string()),
                    ErrorContext::new(src, start.span().into()),
                ))?;
            }
        };

        loop {
            let token = match tokenizer.next() {
                None => {
                    return Err(XmlError::new(
                        XmlErrorKind::UnexpectedEof,
                        ErrorContext::new(src, StrSpan::end(src)),
                    ));
                }

                Some(Err(e)) => {
                    return Err(XmlError::new(
                        XmlErrorKind::Xml(e),
                        ErrorContext::new(src, StrSpan::default()),
                    ));
                }

                Some(Ok(token)) => token,
            };

            match token {
                Token::DtdEnd { span } => {
                    node.span.extend(&span.into(), src);
                    return Ok(node);
                }

                Token::EntityDeclaration {
                    name,
                    definition,
                    span,
                } => {
                    let entity = DtdEntity {
                        span: StrSpan::from(span),
                        name: StrSpan::from(name),
                        definition: definition.into(),
                    };
                    node.entities.push(entity);
                }

                _ => {
                    return Err(XmlError::new(
                        XmlErrorKind::Custom("Expected Entity or DTD end".to_string()),
                        ErrorContext::new(src, token.span().into()),
                    ));
                }
            }
        }
    }
}
impl<'src> ToBinHandler<'src> for DtdNode<'src> {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.external_id.write(encoder)?;
        self.entities.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let span = StrSpan::read(decoder)?;
        let name = StrSpan::read(decoder)?;
        let external_id = Option::<ExternalId>::read(decoder)?;
        let entities = Vec::<DtdEntity>::read(decoder)?;

        Ok(DtdNode {
            span,
            name,
            external_id,
            entities,
        })
    }
}

/// An owned version of the DTD node, with no span metadata. See [`DtdNode`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct OwnedDtdNode {
    /// The name of the DTD node.
    pub name: String,

    /// The external ID of the DTD node, if any.
    pub external_id: Option<OwnedExternalId>,

    /// The entities declared in the DTD node.
    pub entities: Vec<OwnedDtdEntity>,
}
impl OwnedDtdNode {
    /// Create a new DTD node.
    pub fn new(name: impl Into<String>, external_id: Option<OwnedExternalId>) -> Self {
        Self {
            name: name.into(),
            external_id,
            entities: Vec::new(),
        }
    }

    pub(crate) fn borrowed(&self) -> DtdNode<'_> {
        DtdNode::new(
            "",
            self.name.as_str(),
            self.external_id.as_ref().map(|e| e.borrowed()),
        )
    }
}
