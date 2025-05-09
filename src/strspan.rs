use crate::{
    DocumentSourceRef,
    to_bin::{BinDecodeError, Decoder, Encoder, ToBinHandler},
};

/// A span of a string in the input XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StrSpan<'a> {
    /// The string slice.
    pub text: &'a str,

    /// The start position of the span in the input XML.
    pub start: usize,
}
impl<'a> StrSpan<'a> {
    /// Create a new span from the given string slice.  
    /// The strings will be allocated in the given arena.
    ///
    /// # Panics
    /// Panics if the arena cannot allocate the strings.  
    /// For a non-panicking version, use `DocumentSourceRef::try_alloc`.
    #[must_use]
    pub fn from_unallocated<'b>(arena: &'a DocumentSourceRef, text: &'b str) -> Self {
        let text = arena.alloc(text);
        StrSpan { text, start: 0 }
    }

    /// Create a span at the end of the string.
    #[must_use]
    pub fn end(str: &str) -> Self {
        let len = str.len();
        StrSpan {
            text: "",
            start: if len == 0 { 0 } else { len - 1 },
        }
    }

    /// Extend the range of this span to include the other span.
    pub fn extend(&mut self, other: &StrSpan<'a>, src: &'a str) {
        let start = self.start.min(other.start);
        let end = (self.start + self.len()).max(other.start + other.len());

        self.text = &src[start..end];
        self.start = start;
    }

    /// Returns the length of the span.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Return true if len == 0
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the start position of the span in the input XML.
    #[inline]
    #[must_use]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns this span as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &'a str {
        self.text
    }

    /// Calculates the row and column of the span in the input XML.
    ///
    /// Warning: This is an expensive operation, and should be used for error reporting only.
    #[must_use]
    pub fn position(&self, source: &str) -> (usize, usize) {
        Self::position_in_text(self.start, source)
    }

    /// Sets the internal text of the span to the given string slice.  
    /// The strings will be allocated in the given arena.
    ///
    /// # Panics
    /// Panics if the arena cannot allocate the strings.  
    /// For a non-panicking version, use `DocumentSourceRef::try_alloc`.
    pub fn set(&mut self, text: impl AsRef<str>, arena: &'a DocumentSourceRef) {
        self.text = arena.alloc(text);
    }

    pub(crate) fn position_in_text(start: usize, source: &str) -> (usize, usize) {
        let mut row = 1;
        let mut col = 1;
        for (i, c) in source.char_indices() {
            if i == start {
                break;
            }
            if c == '\n' {
                row += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (row, col)
    }
}

impl<'src> ToBinHandler<'src> for StrSpan<'src> {
    fn write<W: std::io::Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.start.write(encoder)?;

        if encoder.source().is_some() {
            //
            // If a source is provided, we can get away with just an offset/len pair
            self.text.len().write(encoder)?;
        } else {
            self.text.write(encoder)?;
        }

        Ok(())
    }

    fn read<R: std::io::Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let start = usize::read(decoder)?;

        let text = if let Some(src) = decoder.source() {
            let len = usize::read(decoder)?;
            &src[start..start + len]
        } else {
            <&str>::read(decoder)?
        };

        Ok(StrSpan { text, start })
    }
}

//
// We need our own type since StrSpan does not expose any internals
impl<'a> From<xmlparser::StrSpan<'a>> for StrSpan<'a> {
    #[inline]
    fn from(span: xmlparser::StrSpan<'a>) -> StrSpan<'a> {
        StrSpan {
            text: span.as_str(),
            start: span.start(),
        }
    }
}

impl<'a> From<&'a str> for StrSpan<'a> {
    #[inline]
    fn from(text: &'a str) -> Self {
        StrSpan { text, start: 0 }
    }
}

impl AsRef<str> for StrSpan<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.text
    }
}
impl std::fmt::Display for StrSpan<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
impl PartialEq<str> for StrSpan<'_> {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}
impl PartialEq<&str> for StrSpan<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}
impl PartialEq<StrSpan<'_>> for str {
    fn eq(&self, other: &StrSpan<'_>) -> bool {
        self == other.text
    }
}
impl PartialEq<StrSpan<'_>> for &str {
    fn eq(&self, other: &StrSpan<'_>) -> bool {
        *self == other.text
    }
}

/// Owned variant of `StrSpan`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StringSpan {
    /// The string slice.
    pub text: String,

    /// The start position of the span in the input XML.
    pub start: usize,
}
impl StringSpan {
    /// Create a new owned span
    #[must_use]
    pub fn new(text: String, start: usize) -> Self {
        StringSpan { text, start }
    }

    /// Calculates the row and column of the span in the input XML.
    ///
    /// Warning: This is an expensive operation, and should be used for error reporting only.
    #[must_use]
    pub fn position(&self, source: &str) -> (usize, usize) {
        StrSpan::position_in_text(self.start, source)
    }

    /// Returns the length of the span.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Return true if len == 0
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the start position of the span in the input XML.
    #[inline]
    #[must_use]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns this span as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}
impl<'a> From<xmlparser::StrSpan<'a>> for StringSpan {
    #[inline]
    fn from(span: xmlparser::StrSpan<'a>) -> Self {
        Self {
            text: span.to_string(),
            start: span.start(),
        }
    }
}
impl From<StrSpan<'_>> for StringSpan {
    #[inline]
    fn from(span: StrSpan<'_>) -> StringSpan {
        StringSpan {
            text: span.text.to_string(),
            start: span.start,
        }
    }
}
impl AsRef<str> for StringSpan {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.text
    }
}
impl std::fmt::Display for StringSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
impl PartialEq<str> for StringSpan {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}
impl PartialEq<&str> for StringSpan {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}
impl PartialEq<StringSpan> for str {
    fn eq(&self, other: &StringSpan) -> bool {
        self == other.text
    }
}
impl PartialEq<StringSpan> for &str {
    fn eq(&self, other: &StringSpan) -> bool {
        *self == other.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strspan_from_unallocated() {
        let arena = DocumentSourceRef::new();
        let text = "example";
        let span = StrSpan::from_unallocated(&arena, text);
        assert_eq!(span.as_str(), text);
        assert_eq!(span.start(), 0);
    }

    #[test]
    fn test_strspan_end() {
        let span = StrSpan::end("example");
        assert_eq!(span.as_str(), "");
        assert_eq!(span.start(), 6);
    }

    #[test]
    fn test_strspan_extend() {
        let src = "example text";
        let mut span1 = StrSpan {
            text: &src[0..7],
            start: 0,
        };
        let span2 = StrSpan {
            text: &src[8..12],
            start: 8,
        };
        span1.extend(&span2, src);
        assert_eq!(span1.as_str(), "example text");
        assert_eq!(span1.start(), 0);
    }

    #[test]
    fn test_strspan_position() {
        let source = "line1\nline2\nline3";
        let span = StrSpan {
            text: "line2",
            start: 6,
        };
        assert_eq!(span.position(source), (2, 1));
    }

    #[test]
    fn test_string_span_new() {
        let span = StringSpan::new("example".to_string(), 5);
        assert_eq!(span.as_str(), "example");
        assert_eq!(span.start(), 5);
    }

    #[test]
    fn test_string_span_position() {
        let source = "line1\nline2\nline3";
        let span = StringSpan::new("line2".to_string(), 6);
        assert_eq!(span.position(source), (2, 1));
    }

    #[test]
    fn test_strspan_partial_eq() {
        let span = StrSpan {
            text: "example",
            start: 0,
        };
        assert_eq!(span, "example");
        assert_eq!("example", span);
    }

    #[test]
    fn test_string_span_partial_eq() {
        let span = StringSpan::new("example".to_string(), 0);
        assert_eq!(span, "example");
        assert_eq!("example", span);
    }

    #[test]
    fn test_strspan_is_empty() {
        let empty_span = StrSpan { text: "", start: 0 };
        let non_empty_span = StrSpan {
            text: "text",
            start: 0,
        };
        assert!(empty_span.is_empty());
        assert!(!non_empty_span.is_empty());
    }

    #[test]
    fn test_string_span_is_empty() {
        let empty_span = StringSpan::new(String::new(), 0);
        let non_empty_span = StringSpan::new("text".to_string(), 0);
        assert!(empty_span.is_empty());
        assert!(!non_empty_span.is_empty());
    }
}
