//! A bytes codec for encoding and decoding bytes.

use core::convert::Infallible;

use crate::{
    decode::{
        decoder::Decoder,
        frame::Frame,
        maybe_decoded::{FrameSize, MaybeDecoded},
    },
    encode::encoder::Encoder,
};

/// A codec that decodes a sequence of bytes as it comes in and encodes a sequence of bytes into a sequence of bytes.
///
/// `N` is the maximum number of bytes that a frame can contain.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BytesCodec<const N: usize>;

/// An error that can occur when encoding a sequence of bytes into a sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BytesEncodeError {
    /// The input buffer is too small to fit the encoded bytes.
    InputBufferTooSmall,
}

impl core::fmt::Display for BytesEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputBufferTooSmall => write!(f, "Input buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytesEncodeError {}

impl<const N: usize> BytesCodec<N> {
    /// Creates a new [`BytesCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Encodes a slice of bytes into a destination buffer.
    pub fn encode_slice(&self, item: &[u8], dst: &mut [u8]) -> Result<usize, BytesEncodeError> {
        let size = item.len();

        if dst.len() < size {
            return Err(BytesEncodeError::InputBufferTooSmall);
        }

        dst[..size].copy_from_slice(item);

        Ok(size)
    }
}

impl<const N: usize> Decoder for BytesCodec<N> {
    type Item = heapless::Vec<u8, N>;
    type Error = Infallible;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        let size = match src.len() {
            0 => return Ok(MaybeDecoded::None(FrameSize::Unknown)),
            n if n > N => N,
            n => n,
        };

        let item = heapless::Vec::from_slice(&src[..size]).expect("unreachable");
        let frame = Frame::new(size, item);

        Ok(MaybeDecoded::Frame(frame))
    }
}

impl<const N: usize> Encoder<heapless::Vec<u8, N>> for BytesCodec<N> {
    type Error = BytesEncodeError;

    fn encode(&mut self, item: heapless::Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_slice(&item, dst)
    }
}

impl<const N: usize> Default for BytesCodec<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test;
