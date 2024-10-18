//! Lines codecs for encoding and decoding line bytes.

use core::convert::Infallible;

use heapless::Vec;

use crate::{
    decode::{Decoder, DecoderOwned},
    encode::Encoder,
};

/// A codec that decodes a sequence of bytes into a line and encodes a line into a sequence of bytes.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LinesCodec {
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

impl LinesCodec {
    /// Creates a new [`LinesCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self { seen: 0 }
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

impl<'buf> Decoder<'buf> for LinesCodec {
    type Item = &'buf [u8];
    type Error = Infallible;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        while self.seen < src.len() {
            if src[self.seen] == b'\n' {
                let line_bytes = match &src[..self.seen].last() {
                    Some(b'\r') => &src[..self.seen - 1],
                    _ => &src[..self.seen],
                };

                let item = (line_bytes, self.seen + 1);

                self.seen = 0;

                return Ok(Some(item));
            }

            self.seen += 1;
        }

        Ok(None)
    }
}

/// An error that can occur when encoding a line into a sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LinesEncodeError {
    /// The input buffer is too small to fit the encoded line.
    BufferTooSmall,
}

impl core::fmt::Display for LinesEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LinesEncodeError {}

impl Encoder<&[u8]> for LinesCodec {
    type Error = LinesEncodeError;

    fn encode(&mut self, item: &[u8], dst: &mut [u8]) -> Result<usize, Self::Error> {
        let size = item.len() + 2;

        if dst.len() < size {
            return Err(LinesEncodeError::BufferTooSmall);
        }

        dst[..item.len()].copy_from_slice(item);
        dst[item.len()..size].copy_from_slice(b"\r\n");

        Ok(size)
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LinesCodecOwned<const N: usize> {
    inner: LinesCodec,
}

impl<const N: usize> LinesCodecOwned<N> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: LinesCodec::new(),
        }
    }

    #[inline]
    pub const fn seen(&self) -> usize {
        self.inner.seen()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<const N: usize> From<LinesCodec> for LinesCodecOwned<N> {
    fn from(inner: LinesCodec) -> Self {
        Self { inner }
    }
}

impl<const N: usize> DecoderOwned for LinesCodecOwned<N> {
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

impl<const N: usize> Encoder<Vec<u8, N>> for LinesCodecOwned<N> {
    type Error = LinesEncodeError;

    fn encode(&mut self, item: Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        Encoder::encode(&mut self.inner, &item, dst)
    }
}
