//! Module for compiling data structures into byte arrays and decoding them back.
//!
//! Includes support for the arena allocator to store strings and other data types.
use std::{io::Write, path::PathBuf};

/// Binary decoder for reading data from a byte stream.
///
/// Uses an arena for allocating string references.
///
/// WARNING: This structure can cause a stack-overflow for very deep trees!
/// Use only on trusted data!
pub struct Decoder<'src> {
    buf: &'src [u8],
    cursor: usize,
    src: Option<&'src str>,
}
impl<'src> Decoder<'src> {
    /// Creates a new `Decoder` instance for the the given byte stream and arena.
    #[must_use]
    pub fn new(buf: &'src [u8]) -> Self {
        Self {
            buf,
            cursor: 0,
            src: None,
        }
    }

    /// Returns the current position in the byte stream.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Creates a new `Decoder` instance for the given byte stream
    ///
    /// # Errors
    /// Fails if the buffer is empty, or the cursor would fall out of bounds.
    pub fn read(&mut self) -> Result<u8, BinDecodeError> {
        if self.cursor >= self.buf.len() {
            return Err(BinDecodeError::UnexpectedEof);
        }
        let byte = self.buf[self.cursor];
        self.cursor += 1;
        Ok(byte)
    }

    /// Reads a slice of bytes from the byte stream.
    ///
    /// # Errors
    /// Fails if the buffer is empty, or the cursor would fall out of bounds.
    pub fn read_all(&mut self, len: usize) -> Result<&'src [u8], BinDecodeError> {
        if self.cursor + len > self.buf.len() {
            return Err(BinDecodeError::UnexpectedEof);
        }
        let bytes = &self.buf[self.cursor..self.cursor + len];
        self.cursor += len;
        Ok(bytes)
    }

    /// Reads a slice of bytes from the byte stream into the provided buffer.
    ///
    /// # Errors
    /// Fails if the buffer is empty, or the cursor would fall out of bounds.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), BinDecodeError> {
        if self.cursor + buf.len() > self.buf.len() {
            return Err(BinDecodeError::UnexpectedEof);
        }
        buf.copy_from_slice(&self.buf[self.cursor..self.cursor + buf.len()]);
        self.cursor += buf.len();
        Ok(())
    }

    /// Adds a source string to the decoder.
    ///
    /// From the point this is called all &str decodes will be offsets into this,
    /// and will not store the string in the bytecode
    pub fn with_source(&mut self, source: &'src str) {
        self.src = Some(source);
    }

    /// Returns the source string if it was provided.
    #[must_use]
    pub fn source(&self) -> Option<&'src str> {
        self.src
    }
}

/// Binary encoder for writing data to a byte stream.
///
/// WARNING: This structure can cause a stack-overflow for very deep trees!
/// Use only on trusted data!
pub struct Encoder {
    buf: Vec<u8>,
    source_header_flag: bool,
}
impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}
impl Encoder {
    /// Creates a new `Encoder` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            source_header_flag: false,
        }
    }

    /// Indicates that strings should be stored as offsets into a source string.
    pub fn with_source_header(&mut self) {
        self.source_header_flag = true;
    }

    /// If true, strings should be stored as offsets into the source string.
    #[must_use]
    pub fn has_source_header(&self) -> bool {
        self.source_header_flag
    }

    /// Returns the length of the encoded data.
    #[must_use]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Returns true if the encoded data is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns the inner buffer of the encoder.
    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    /// Write bytes to the encoder.
    ///
    /// # Errors
    /// Can fail if the buffer cannot be resized.
    pub fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.buf.write_all(bytes)
    }
}

/// Binary handler trait for encoding and decoding data types.
pub trait ToBinHandler<'src>: Sized {
    /// Writes the value to the encoder.  
    ///
    /// # Errors
    /// Should return an error if the data cannot be written to the stream.
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()>;

    /// Reads the value from the decoder.
    ///
    /// # Errors
    /// Should return an error if the data is corrupted or truncated.
    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError>;
}

//
// Primitive types
impl ToBinHandler<'_> for bool {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        encoder.write_all(&[u8::from(*self)])?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'_>) -> Result<Self, BinDecodeError> {
        let b = decoder.read()?;
        Ok(b != 0)
    }
}
impl ToBinHandler<'_> for u8 {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        encoder.write_all(&self.to_le_bytes())?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'_>) -> Result<Self, BinDecodeError> {
        decoder.read()
    }
}
impl ToBinHandler<'_> for usize {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        encoder.write_all(&self.to_le_bytes())?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'_>) -> Result<Self, BinDecodeError> {
        let mut bytes = [0u8; 8];
        decoder.read_exact(&mut bytes)?;
        Ok(usize::from_le_bytes(bytes))
    }
}
impl<'src> ToBinHandler<'src> for &'src str {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.len().write(encoder)?;
        encoder.write_all(self.as_bytes())?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let len = usize::read(decoder)?;
        let bytes = decoder.read_all(len)?;
        let s = std::str::from_utf8(bytes).map_err(|_| BinDecodeError::InvalidUtf8)?;
        Ok(s)
    }
}

impl<'src> ToBinHandler<'src> for String {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.len().write(encoder)?;
        encoder.write_all(self.as_bytes())?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let bytes = Vec::<u8>::read(decoder)?;
        let str = String::from_utf8(bytes).map_err(|_| BinDecodeError::InvalidUtf8)?;
        Ok(str)
    }
}
impl<'src> ToBinHandler<'src> for PathBuf {
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        let path = self.to_string_lossy();
        path.len().write(encoder)?;
        encoder.write_all(path.as_bytes())?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let bytes = Vec::<u8>::read(decoder)?;
        let path = String::from_utf8(bytes).map_err(|_| BinDecodeError::InvalidUtf8)?;
        Ok(PathBuf::from(path))
    }
}

//
// Compound types
impl<'src, T> ToBinHandler<'src> for Vec<T>
where
    T: ToBinHandler<'src>,
{
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.len().write(encoder)?;
        for item in self {
            item.write(encoder)?;
        }
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let len = usize::read(decoder)?;
        let mut vec = vec![];
        vec.try_reserve(len)?;
        for _ in 0..len {
            let item = T::read(decoder)?;
            vec.push(item);
        }
        Ok(vec)
    }
}
impl<'src, T> ToBinHandler<'src> for Option<T>
where
    T: ToBinHandler<'src>,
{
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        match self {
            Some(item) => {
                1u8.write(encoder)?;
                item.write(encoder)?;
            }
            None => {
                0u8.write(encoder)?;
            }
        }
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let has_value = u8::read(decoder)?;
        if has_value != 0 {
            let value = T::read(decoder)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}
impl<'src, S, T> ToBinHandler<'src> for (S, T)
where
    S: ToBinHandler<'src>,
    T: ToBinHandler<'src>,
{
    fn write(&self, encoder: &mut Encoder) -> std::io::Result<()> {
        self.0.write(encoder)?;
        self.1.write(encoder)?;
        Ok(())
    }

    fn read(decoder: &mut Decoder<'src>) -> Result<Self, BinDecodeError> {
        let first = S::read(decoder)?;
        let second = T::read(decoder)?;
        Ok((first, second))
    }
}

/// Error occurred while decoding binary data.
#[derive(Debug, thiserror::Error)]
pub enum BinDecodeError {
    /// Data ran out before the expected length was reached.
    #[error("End of file; expected more data")]
    UnexpectedEof,

    /// Corrupted UTF-8 string.
    #[error("Invalid UTF-8 string")]
    InvalidUtf8,

    /// Variant code is not valid for the enum.
    #[error("Invalid enum variant")]
    InvalidEnumVariant,

    /// IO error while reading or writing data.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error occurred while trying to reserve memory in a vector.
    #[error("Memory allocation error: {0}")]
    TryReserveError(#[from] std::collections::TryReserveError),

    /// Error occurred while trying to read the header from the stream.
    #[error("Data did not have a valid header")]
    InvalidHeader,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_encoding_decoding() {
        let mut encoder = Encoder::new();
        true.write(&mut encoder).unwrap();
        false.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert!(bool::read(&mut decoder).unwrap());
        assert!(!bool::read(&mut decoder).unwrap());
    }

    #[test]
    fn test_u8_encoding_decoding() {
        let mut encoder = Encoder::new();
        42u8.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert_eq!(u8::read(&mut decoder).unwrap(), 42u8);
    }

    #[test]
    fn test_usize_encoding_decoding() {
        let mut encoder = Encoder::new();
        12345usize.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert_eq!(usize::read(&mut decoder).unwrap(), 12345usize);
    }

    #[test]
    fn test_string_encoding_decoding() {
        let mut encoder = Encoder::new();
        let input = String::from("Hello, world!");
        input.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert_eq!(String::read(&mut decoder).unwrap(), input);
    }

    #[test]
    fn test_vec_encoding_decoding() {
        let mut encoder = Encoder::new();
        let input = vec![1u8, 2, 3, 4, 5];
        input.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert_eq!(Vec::<u8>::read(&mut decoder).unwrap(), input);
    }

    #[test]
    fn test_option_encoding_decoding() {
        let mut encoder = Encoder::new();
        let some_value: Option<u8> = Some(42);
        let none_value: Option<u8> = None;
        some_value.write(&mut encoder).unwrap();
        none_value.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert_eq!(Option::<u8>::read(&mut decoder).unwrap(), some_value);
        assert_eq!(Option::<u8>::read(&mut decoder).unwrap(), none_value);
    }

    #[test]
    fn test_tuple_encoding_decoding() {
        let mut encoder = Encoder::new();
        let input = (42u8, String::from("Hello"));
        input.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert_eq!(<(u8, String)>::read(&mut decoder).unwrap(), input);
    }

    #[test]
    fn test_pathbuf_encoding_decoding() {
        let mut encoder = Encoder::new();
        let input = PathBuf::from("/some/path");
        input.write(&mut encoder).unwrap();

        let buffer = encoder.into_inner();
        let mut decoder = Decoder::new(buffer.as_slice());
        assert_eq!(PathBuf::read(&mut decoder).unwrap(), input);
    }
}
