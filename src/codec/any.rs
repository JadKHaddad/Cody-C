//! An any delimiter codec for encoding and decoding bytes.

use core::convert::Infallible;

use heapless::Vec;

use crate::{
    decode::{Decoder, DecoderOwned},
    encode::Encoder,
};

/// A codec that decodes a sequence of bytes ending with a `delimiter` into a sequence of bytes and encodes a sequence of bytes into a sequence of bytes ending with a `delimiter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AnyDelimiterCodec<'a> {
    /// The delimiter to search for.
    delimiter: &'a [u8],
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

impl<'a> AnyDelimiterCodec<'a> {
    /// Creates a new [`AnyDelimiterCodec`] with the given `delimiter`.
    #[inline]
    pub const fn new(delimiter: &'a [u8]) -> Self {
        Self { delimiter, seen: 0 }
    }

    /// Returns the delimiter to search for.
    #[inline]
    pub const fn delimiter(&self) -> &'a [u8] {
        self.delimiter
    }

    /// Returns the number of bytes of the slice that have been seen so far.
    #[inline]
    pub const fn seen(&self) -> usize {
        self.seen
    }

    /// Clears the number of bytes of the slice that have been seen so far.
    #[inline]
    pub fn clear(&mut self) {
        self.seen = 0;
    }
}

impl<'buf> Decoder<'buf> for AnyDelimiterCodec<'_> {
    type Item = &'buf [u8];
    type Error = Infallible;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        if src.len() < self.delimiter.len() {
            return Ok(None);
        }

        match self.delimiter.last() {
            None => {
                let bytes = &src[..self.seen + 1];
                let item = (bytes, self.seen + 1);

                Ok(Some(item))
            }
            Some(last_byte) => {
                while self.seen < src.len() {
                    if src[self.seen] == *last_byte {
                        let src_delimiter =
                            &src[self.seen + 1 - self.delimiter.len()..self.seen + 1];

                        if src_delimiter == self.delimiter {
                            let bytes = &src[..self.seen + 1 - self.delimiter.len()];
                            let item = (bytes, self.seen + 1);

                            self.seen = 0;

                            return Ok(Some(item));
                        }
                    }

                    self.seen += 1;
                }

                Ok(None)
            }
        }
    }
}

/// An error that can occur when encoding a sequence of bytes into a sequence of bytes ending with a `delimiter`.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AnyDelimiterEncodeError {
    /// The input buffer is too small to fit the encoded sequesnce of bytes.
    BufferTooSmall,
}

impl core::fmt::Display for AnyDelimiterEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AnyDelimiterEncodeError::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AnyDelimiterEncodeError {}

impl Encoder<&[u8]> for AnyDelimiterCodec<'_> {
    type Error = AnyDelimiterEncodeError;

    fn encode(&mut self, item: &[u8], dst: &mut [u8]) -> Result<usize, Self::Error> {
        let size = item.len() + self.delimiter.len();

        if dst.len() < size {
            return Err(AnyDelimiterEncodeError::BufferTooSmall);
        }

        dst[..item.len()].copy_from_slice(item);
        dst[item.len()..size].copy_from_slice(self.delimiter);

        Ok(size)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AnyDelimiterCodecOwned<'a, const N: usize> {
    inner: AnyDelimiterCodec<'a>,
}

impl<'a, const N: usize> AnyDelimiterCodecOwned<'a, N> {
    #[inline]
    pub const fn new(delimiter: &'a [u8]) -> Self {
        Self {
            inner: AnyDelimiterCodec::new(delimiter),
        }
    }

    #[inline]
    pub const fn delimiter(&self) -> &'a [u8] {
        self.inner.delimiter
    }

    #[inline]
    pub const fn seen(&self) -> usize {
        self.inner.seen
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.seen = 0;
    }
}

impl<'a, const N: usize> From<AnyDelimiterCodec<'a>> for AnyDelimiterCodecOwned<'a, N> {
    fn from(inner: AnyDelimiterCodec<'a>) -> Self {
        Self { inner }
    }
}

impl<'a, const N: usize> DecoderOwned for AnyDelimiterCodecOwned<'a, N> {
    type Item = Vec<u8, N>;
    type Error = ();

    fn decode_owned(&mut self, src: &mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        match Decoder::decode(&mut self.inner, src) {
            Ok(Some((bytes, size))) => {
                let item = Vec::from_slice(bytes)?;
                Ok(Some((item, size)))
            }
            Ok(None) => Ok(None),
            Err(_) => unreachable!(),
        }
    }
}

impl<'a, const N: usize> Encoder<Vec<u8, N>> for AnyDelimiterCodecOwned<'a, N> {
    type Error = AnyDelimiterEncodeError;

    fn encode(&mut self, item: Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        Encoder::encode(&mut self.inner, &item, dst)
    }
}
