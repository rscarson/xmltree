//! Error handling for XML parsing
use crate::to_bin::BinDecodeError;

use super::{StrSpan, StringSpan};
use std::path::PathBuf;

/// A result type for XML parsing, which can be either a successful value or an error.
pub type XmlResult<T> = std::result::Result<T, XmlError>;

/// An error that occurred while parsing a document.
#[derive(Debug)]
pub struct XmlError {
    /// The context of the error
    pub context: Box<ErrorContext>,

    /// The kind of error that occurred while parsing a document
    pub kind: XmlErrorKind,
}
impl XmlError {
    /// Creates a new `XmlError`
    #[must_use]
    pub fn new(kind: XmlErrorKind, context: ErrorContext) -> Self {
        Self {
            context: Box::new(context),
            kind,
        }
    }

    /// Adds a path to the error context.
    #[must_use]
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.context.path = Some(path);
        self
    }
}
impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.context)?;

        for line in self.kind.to_string().lines() {
            writeln!(f, "= {line}")?;
        }
        Ok(())
    }
}
impl From<BinDecodeError> for XmlError {
    fn from(err: BinDecodeError) -> Self {
        Self::new(
            XmlErrorKind::Decode(err),
            ErrorContext::new("", StrSpan::default()),
        )
    }
}
impl From<std::io::Error> for XmlError {
    fn from(err: std::io::Error) -> Self {
        Self::new(
            XmlErrorKind::Io(err),
            ErrorContext::new("", StrSpan::default()),
        )
    }
}

/// The kind of error that occurred while parsing a document.
#[derive(Debug, thiserror::Error)]
pub enum XmlErrorKind {
    /// Another error occurred while parsing the document
    #[error("{0}")]
    Custom(String),

    /// The XML declaration was not first
    #[error("The <?xml> declaration must appear at the start of the document")]
    DeclarationNotFirst,

    /// A tag in the document was not closed properly
    #[error("Unclosed tag: {0}")]
    UnclosedTag(String),

    /// File ended unexpectedly
    #[error("End of file reached unexpectedly")]
    UnexpectedEof,

    /// XML parsing failed
    #[from(xmlparser::Error)]
    #[error("XML parser error: {0}")]
    Xml(xmlparser::Error),

    /// IO error occurred while reading a file
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error occurred while parsing binary
    #[error("Invalid bytecode: {0}")]
    Decode(#[from] BinDecodeError),
}

/// Context describing the error location in the source code.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The path to the file that was parsed, if available.
    pub path: Option<PathBuf>,

    /// Full source code of the file that was parsed, for row/col calculation.
    pub source: String,

    /// Position of the error in the source code.
    pub span: StringSpan,
}
impl ErrorContext {
    /// Creates a new `ErrorContext` with the given source, and span.
    #[must_use]
    pub fn new(source: &str, span: StrSpan) -> Self {
        Self {
            path: None,
            source: source.to_string(),
            span: span.into(),
        }
    }

    /// Creates a new `ErrorContext` with the given path, source, and span.
    #[must_use]
    pub fn with_path(path: PathBuf, source: &str, span: StrSpan) -> Self {
        Self {
            path: Some(path),
            source: source.to_string(),
            span: span.into(),
        }
    }

    /// Returns the row and column of the error in the source code.
    #[must_use]
    pub fn position(&self) -> (usize, usize) {
        self.span.position(&self.source)
    }
}
impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path.as_ref().map(|p| p.display());

        let span = self.span.as_ref();
        let line = span.split('\n').next().unwrap_or(span);

        let (row, col) = self.span.position(&self.source);

        if !line.is_empty() {
            writeln!(f, "| {line}")?;
        }

        if self.span.start() > 0 {
            write!(f, "= At ")?;

            if let Some(path) = path {
                write!(f, "{path}:")?;
            }

            writeln!(f, "{row}:{col}")?;
        } else if let Some(path) = path {
            writeln!(f, "= In {path}")?;
        }
        Ok(())
    }
}
