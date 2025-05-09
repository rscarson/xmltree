/// An add-only arena for storing source strings.
///
/// This contains a bump allocator that will own strings for the lifetime of the arena.  
/// It is used to manage the 'src lifetime of XML documents.
///
/// This structure is needed for most usecases of this crate, and must live at least as long as the documents attached to it.
pub struct DocumentSourceRef(bumpalo::Bump);
impl DocumentSourceRef {
    /// Creates a new arena for storing source strings.
    #[must_use]
    pub fn new() -> Self {
        Self(bumpalo::Bump::new())
    }

    /// Copies a string into the arena and returns a reference to it.  
    /// The resulting string will live for the lifetime of the arena.
    ///
    /// # Panics
    /// Will panic if memory allocation fails. Use `try_alloc` for a non-panicking version.
    pub fn alloc(&self, source: impl AsRef<str>) -> &'_ str {
        self.0.alloc_str(source.as_ref())
    }

    /// Copies a string into the arena and returns a reference to it.  
    /// The resulting string will live for the lifetime of the arena.
    ///
    /// # Errors
    /// Will return an error if memory allocation fails.
    pub fn try_alloc(&self, source: impl AsRef<str>) -> Result<&'_ str, bumpalo::AllocErr> {
        self.0.try_alloc_str(source.as_ref()).map(|s| &*s)
    }

    /// Returns the number of bytes allocated in the arena.  
    /// May include some padding bytes, so the size may be larger than the sum of the lengths of all strings in the arena.
    ///
    /// Does not include metadata for the strings, only the actual string data.
    pub fn size(&self) -> usize {
        self.0.allocated_bytes()
    }
}
impl Default for DocumentSourceRef {
    fn default() -> Self {
        Self::new()
    }
}
