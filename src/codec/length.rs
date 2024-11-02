//! Length codec for encoding and decoding bytes with a length prefix.

use core::convert::Infallible;

use heapless::Vec;

use crate::{trace, Decoder, DecoderOwned, Encoder};

/// The size of the length prefix in bytes.
pub const SIZE_OF_LENGTH: usize = core::mem::size_of::<u32>();

/// A codec that decodes a sequence of bytes with a length prefix into a sequence of bytes and encodes a sequence of bytes into a sequence of bytes with a length prefix.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LengthCodec;

impl LengthCodec {
    /// Creates a new [`LengthCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self {}
    }
}

impl<'buf> Decoder<'buf> for LengthCodec {
    type Item = &'buf [u8];
    type Error = Infallible;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        if src.len() < SIZE_OF_LENGTH {
            return Ok(None);
        }

        let len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
        let size = len + SIZE_OF_LENGTH;

        trace!("size: {}", size);

        if src.len() < size {
            return Ok(None);
        }

        let item = (&src[SIZE_OF_LENGTH..size], size);

        Ok(Some(item))
    }
}

/// An error that can occur when encoding a sequence of bytes into a sequence of bytes with a length prefix.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LengthEncodeError {
    /// The input buffer is too small to fit the encoded line.
    BufferTooSmall,
    /// The payload size is greater than u32::MAX.
    PayloadTooLarge,
}

impl core::fmt::Display for LengthEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::PayloadTooLarge => write!(f, "Payload too large"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LengthEncodeError {}

impl Encoder<&[u8]> for LengthCodec {
    type Error = LengthEncodeError;

    fn encode(&mut self, item: &[u8], dst: &mut [u8]) -> Result<usize, Self::Error> {
        if item.len() > u32::MAX as usize {
            return Err(LengthEncodeError::PayloadTooLarge);
        }

        let size = item.len() + SIZE_OF_LENGTH;

        trace!("size: {}", size);

        if dst.len() < size {
            return Err(LengthEncodeError::BufferTooSmall);
        }

        dst[0..SIZE_OF_LENGTH].copy_from_slice(&(item.len() as u32).to_be_bytes());
        dst[SIZE_OF_LENGTH..size].copy_from_slice(item);

        Ok(size)
    }
}

/// An owned [`LengthCodec`].
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LengthCodecOwned<const N: usize> {
    inner: LengthCodec,
}

impl<const N: usize> LengthCodecOwned<N> {
    /// Creates a new [`LengthCodecOwned`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: LengthCodec::new(),
        }
    }
}

impl<const N: usize> From<LengthCodec> for LengthCodecOwned<N> {
    fn from(inner: LengthCodec) -> Self {
        Self { inner }
    }
}

impl<const N: usize> DecoderOwned for LengthCodecOwned<N> {
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

impl<const N: usize> Encoder<Vec<u8, N>> for LengthCodecOwned<N> {
    type Error = LengthEncodeError;

    fn encode(&mut self, item: Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        Encoder::encode(&mut self.inner, &item, dst)
    }
}
