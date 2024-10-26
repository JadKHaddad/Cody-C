//! Decoder trait definition.

/// A decoder that decodes a frame from a buffer.
pub trait Decoder<'buf> {
    /// The type of item that this decoder decodes.
    type Item;
    /// The type of error that this decoder returns.
    type Error;

    /// Decodes a frame from the provided buffer.
    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error>;

    /// Decodes a frame from the provided buffer at the end of the stream.
    fn decode_eof(
        &mut self,
        src: &'buf mut [u8],
    ) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        self.decode(src)
    }
}

impl<'buf, D> Decoder<'buf> for &mut D
where
    D: Decoder<'buf>,
{
    type Item = D::Item;
    type Error = D::Error;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        (*self).decode(src)
    }

    fn decode_eof(
        &mut self,
        src: &'buf mut [u8],
    ) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        (*self).decode_eof(src)
    }
}

/// A decoder that decodes an owned frame from a buffer.
pub trait DecoderOwned {
    /// The type of item that this decoder decodes.
    type Item;
    /// The type of error that this decoder returns.
    type Error;

    /// Decodes a frame from the provided buffer.
    fn decode_owned(&mut self, src: &mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error>;

    /// Decodes a frame from the provided buffer at the end of the stream.
    fn decode_eof_owned(
        &mut self,
        src: &mut [u8],
    ) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        self.decode_owned(src)
    }
}

impl<D> DecoderOwned for &mut D
where
    D: DecoderOwned,
{
    type Item = D::Item;
    type Error = D::Error;

    fn decode_owned(&mut self, src: &mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        (*self).decode_owned(src)
    }

    fn decode_eof_owned(
        &mut self,
        src: &mut [u8],
    ) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        (*self).decode_eof_owned(src)
    }
}
