//! Bytes codecs for encoding and decoding bytes.

use core::convert::Infallible;

use heapless::Vec;

use crate::{
    decode::{Decoder, DecoderOwned},
    encode::Encoder,
};

/// A codec that decodes a sequence of bytes into a sequence of bytes and encodes a sequence of bytes into a sequence of bytes.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BytesCodec {}

impl BytesCodec {
    /// Creates a new [`BytesCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self {}
    }
}

impl<'buf> Decoder<'buf> for BytesCodec {
    type Item = &'buf [u8];
    type Error = Infallible;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        Ok(Some((src, src.len())))
    }
}

/// An error that can occur when encoding a sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BytesEncodeError {
    /// The input buffer is too small to fit the sequence of bytes.
    BufferTooSmall,
}

impl core::fmt::Display for BytesEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytesEncodeError {}

impl Encoder<&[u8]> for BytesCodec {
    type Error = BytesEncodeError;

    fn encode(&mut self, item: &[u8], dst: &mut [u8]) -> Result<usize, Self::Error> {
        let size = item.len();

        if dst.len() < size {
            return Err(BytesEncodeError::BufferTooSmall);
        }

        dst[..item.len()].copy_from_slice(item);

        Ok(size)
    }
}

/// An owned [`BytesCodec`].
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BytesCodecOwned<const N: usize> {
    inner: BytesCodec,
}

impl<const N: usize> BytesCodecOwned<N> {
    /// Creates a new [`BytesCodecOwned`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: BytesCodec::new(),
        }
    }
}

impl<const N: usize> From<BytesCodec> for BytesCodecOwned<N> {
    fn from(inner: BytesCodec) -> Self {
        Self { inner }
    }
}

impl<const N: usize> DecoderOwned for BytesCodecOwned<N> {
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

impl<const N: usize> Encoder<Vec<u8, N>> for BytesCodecOwned<N> {
    type Error = BytesEncodeError;

    fn encode(&mut self, item: Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        Encoder::encode(&mut self.inner, &item, dst)
    }
}
