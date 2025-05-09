use super::{
    StrSpan,
    error::{ErrorContext, XmlError, XmlErrorKind, XmlResult},
};
use crate::{
    DocumentSourceRef,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};
use xmlparser::{Token, Tokenizer};

/// Representation of the [ExternalID](https://www.w3.org/TR/xml/#NT-ExternalID) value.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExternalId<'src> {
    System(StrSpan<'src>),
    Public(StrSpan<'src>, StrSpan<'src>),
}
impl<'src> ExternalId<'src> {
    /// Create a new node.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn system_from_unallocated<'b>(arena: &'src DocumentSourceRef, value: &'b str) -> Self {
        ExternalId::System(StrSpan::from_unallocated(arena, value))
    }

    /// Create a new node.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn public_from_unallocated<'b>(
        arena: &'src DocumentSourceRef,
        public: &'b str,
        system: &'b str,
    ) -> Self {
        ExternalId::Public(
            StrSpan::from_unallocated(arena, public),
            StrSpan::from_unallocated(arena, system),
        )
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

impl<'src> ToBinHandler<'src> for ExternalId<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
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

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
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
#[allow(missing_docs)]
pub enum EntityDefinition<'src> {
    EntityValue(StrSpan<'src>),
    ExternalId(ExternalId<'src>),
}
impl<'src> EntityDefinition<'src> {
    /// Create a new node.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(arena: &'src DocumentSourceRef, value: &'b str) -> Self {
        EntityDefinition::EntityValue(StrSpan::from_unallocated(arena, value))
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
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
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

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
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
    /// Create a new node.
    ///
    /// Assumes the strings are not allocated in the arena.
    pub fn from_unallocated<'b>(
        arena: &'src DocumentSourceRef,
        name: &'b str,
        definition: EntityDefinition<'src>,
    ) -> Self {
        DtdEntity {
            span: StrSpan::default(),
            name: StrSpan::from_unallocated(arena, name),
            definition,
        }
    }

    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
    }
}
impl<'src> ToBinHandler<'src> for DtdEntity<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.definition.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
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

/// The DTD node in the XML document.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DtdNode<'src> {
    /// The span of the DTD node in the source XML.
    pub span: StrSpan<'src>,

    /// The name of the DTD node.
    pub name: StrSpan<'src>,

    /// The external ID of the DTD node, if any.
    pub external_id: Option<ExternalId<'src>>,

    /// The entities declared in the DTD node.
    pub entities: Vec<DtdEntity<'src>>,
}
impl<'src> DtdNode<'src> {
    /// Removes the source span from the structure.  
    /// Used to make binary data smaller.
    pub fn strip_metadata(&mut self) {
        self.span = StrSpan::default();
        for entity in &mut self.entities {
            entity.strip_metadata();
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
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.span.write(encoder)?;
        self.name.write(encoder)?;
        self.external_id.write(encoder)?;
        self.entities.write(encoder)?;
        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
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
