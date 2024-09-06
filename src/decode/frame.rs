#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Frame<T> {
    /// Number of bytes consumed from the buffer
    ///
    /// # Note
    /// Number of bytes needed to create the [`Frame::item`] may be less than the number of bytes consumed from the `buffer` but never more.
    pub(super) size: usize,
    /// The decoded item
    pub(super) item: T,
}

impl<T> Frame<T> {
    #[inline]
    pub const fn new(size: usize, item: T) -> Self {
        Self { size, item }
    }

    #[inline]
    pub const fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub const fn item(&self) -> &T {
        &self.item
    }

    pub fn into_item(self) -> T {
        self.item
    }
}
