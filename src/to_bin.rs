//! Module for compiling data structures into byte arrays and decoding them back.
//!
//! Includes support for the arena allocator to store strings and other data types.
use super::DocumentSourceRef;
use std::{
    io::{Read, Write},
    path::PathBuf,
};

/// Binary decoder for reading data from a byte stream.
///
/// Uses an arena for allocating string references.
///
/// WARNING: This structure can cause a stack-overflow for very deep trees!
/// Use only on trusted data!
pub struct Decoder<'src, R: Read> {
    arena: &'src DocumentSourceRef,
    src: Option<&'src str>,
    reader: R,
}
impl<'src, R: Read> Decoder<'src, R> {
    /// Creates a new `Decoder` instance for the the given byte stream and arena.
    pub fn new(reader: R, arena: &'src DocumentSourceRef) -> Self {
        Self {
            arena,
            reader,
            src: None,
        }
    }

    /// Adds a source string to the decoder.
    ///
    /// From the point this is called all &str decodes will be offsets into this,
    /// and will not store the string in the bytecode
    #[must_use]
    pub fn with_source(mut self, source: &'src str) -> Self {
        self.src = Some(source);
        self
    }

    /// Returns the source string if it was provided.
    pub fn source(&self) -> Option<&'src str> {
        self.src
    }

    /// Allocates a string reference in the arena from the given source string.
    ///
    /// # Errors
    /// Returns an error if the allocation fails.
    pub fn alloc(&self, source: impl AsRef<str>) -> Result<&'src str, BinDecodeError> {
        self.arena
            .try_alloc(source.as_ref())
            .map_err(BinDecodeError::Allocation)
    }

    /// Returns a mutable reference to the byte stream reader.
    pub fn reader(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Read a value of type `T` from the byte stream.
    ///
    /// # Errors
    /// Returns an error if the data is corrupted or truncated.
    pub fn read<T>(&mut self) -> Result<T, BinDecodeError>
    where
        T: ToBinHandler<'src>,
    {
        T::read(self)
    }
}

/// Binary encoder for writing data to a byte stream.
///
/// WARNING: This structure can cause a stack-overflow for very deep trees!
/// Use only on trusted data!
pub struct Encoder<'src, W: Write> {
    src: Option<&'src str>,
    writer: W,
}
impl<'src, W: Write> Encoder<'src, W> {
    /// Creates a new `Encoder` instance for the given byte stream.
    pub fn new(writer: W) -> Self {
        Self { writer, src: None }
    }

    /// Adds a source string to the encoder.
    ///
    /// From the point this is called all &str decodes will be offsets into this,
    /// and will not store the string in the bytecode
    #[must_use]
    pub fn with_source(mut self, source: &'src str) -> Self {
        self.src = Some(source);
        self
    }

    /// Returns the source string if it was provided.
    pub fn source(&self) -> Option<&'src str> {
        self.src
    }

    /// Returns a mutable reference to the byte stream writer.
    pub fn writer(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Writes a value of type `T` to the byte stream.
    ///
    /// # Errors
    /// Returns an error if the data cannot be written to the stream.
    pub fn write<T>(&mut self, value: &T) -> std::io::Result<()>
    where
        T: ToBinHandler<'src>,
    {
        value.write(self)
    }
}

/// Binary handler trait for encoding and decoding data types.
pub trait ToBinHandler<'src>: Sized {
    /// Writes the value to the encoder.  
    ///
    /// # Errors
    /// Should return an error if the data cannot be written to the stream.
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()>;

    /// Reads the value from the decoder.
    ///
    /// # Errors
    /// Should return an error if the data is corrupted or truncated.
    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError>;
}

//
// Primitive types
impl ToBinHandler<'_> for bool {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        encoder.writer.write_all(&[u8::from(*self)])?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'_, R>) -> Result<Self, BinDecodeError> {
        let mut bytes = [0u8; 1];
        decoder
            .reader
            .read_exact(&mut bytes)
            .map_err(|_| BinDecodeError::UnexpectedEof)?;
        Ok(bytes[0] != 0)
    }
}
impl ToBinHandler<'_> for u8 {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        encoder.writer.write_all(&self.to_le_bytes())?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'_, R>) -> Result<Self, BinDecodeError> {
        let mut bytes = [0u8; 1];
        decoder
            .reader
            .read_exact(&mut bytes)
            .map_err(|_| BinDecodeError::UnexpectedEof)?;
        Ok(u8::from_le_bytes(bytes))
    }
}
impl ToBinHandler<'_> for usize {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        encoder.writer.write_all(&self.to_le_bytes())?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'_, R>) -> Result<Self, BinDecodeError> {
        let mut bytes = [0u8; 8];
        decoder.reader.read_exact(&mut bytes)?;
        Ok(usize::from_le_bytes(bytes))
    }
}
impl<'src> ToBinHandler<'src> for &'src str {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.len().write(encoder)?;
        encoder.writer.write_all(self.as_bytes())?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let len = usize::read(decoder)?;
        let mut bytes = vec![0u8; len];
        decoder.reader.read_exact(&mut bytes)?;

        let str = String::from_utf8(bytes).map_err(|_| BinDecodeError::InvalidUtf8)?;
        let str = decoder.alloc(str)?;
        Ok(str)
    }
}

impl<'src> ToBinHandler<'src> for String {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.len().write(encoder)?;
        encoder.writer.write_all(self.as_bytes())?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let bytes = Vec::<u8>::read(decoder)?;
        let str = String::from_utf8(bytes).map_err(|_| BinDecodeError::InvalidUtf8)?;
        Ok(str)
    }
}
impl<'src> ToBinHandler<'src> for PathBuf {
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        let path = self.to_string_lossy();
        path.len().write(encoder)?;
        encoder.writer.write_all(path.as_bytes())?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
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
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.len().write(encoder)?;
        for item in self {
            item.write(encoder)?;
        }
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
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
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
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

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
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
    fn write<W: Write>(&self, encoder: &mut Encoder<W>) -> std::io::Result<()> {
        self.0.write(encoder)?;
        self.1.write(encoder)?;
        Ok(())
    }

    fn read<R: Read>(decoder: &mut Decoder<'src, R>) -> Result<Self, BinDecodeError> {
        let first = decoder.read()?;
        let second = decoder.read()?;
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

    /// Error occurred while allocating memory
    #[error("Memory allocation error: {0}")]
    Allocation(bumpalo::AllocErr),

    /// Error occurred while trying to reserve memory in a vector.
    #[error("Memory allocation error: {0}")]
    TryReserveError(#[from] std::collections::TryReserveError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        true.write(&mut encoder).unwrap();
        false.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert!(bool::read(&mut decoder).unwrap());
        assert!(!bool::read(&mut decoder).unwrap());
    }

    #[test]
    fn test_u8_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        42u8.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert_eq!(u8::read(&mut decoder).unwrap(), 42u8);
    }

    #[test]
    fn test_usize_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        12345usize.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert_eq!(usize::read(&mut decoder).unwrap(), 12345usize);
    }

    #[test]
    fn test_string_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        let input = String::from("Hello, world!");
        input.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert_eq!(String::read(&mut decoder).unwrap(), input);
    }

    #[test]
    fn test_vec_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        let input = vec![1u8, 2, 3, 4, 5];
        input.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert_eq!(Vec::<u8>::read(&mut decoder).unwrap(), input);
    }

    #[test]
    fn test_option_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        let some_value: Option<u8> = Some(42);
        let none_value: Option<u8> = None;
        some_value.write(&mut encoder).unwrap();
        none_value.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert_eq!(Option::<u8>::read(&mut decoder).unwrap(), some_value);
        assert_eq!(Option::<u8>::read(&mut decoder).unwrap(), none_value);
    }

    #[test]
    fn test_tuple_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        let input = (42u8, String::from("Hello"));
        input.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert_eq!(<(u8, String)>::read(&mut decoder).unwrap(), input);
    }

    #[test]
    fn test_pathbuf_encoding_decoding() {
        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(&mut buffer);
        let input = PathBuf::from("/some/path");
        input.write(&mut encoder).unwrap();

        let arena = DocumentSourceRef::new();
        let mut decoder = Decoder::new(buffer.as_slice(), &arena);
        assert_eq!(PathBuf::read(&mut decoder).unwrap(), input);
    }
}
