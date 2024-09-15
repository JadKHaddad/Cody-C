//! A bytes codec for encoding and decoding bytes with a length field (4 bytes).

#[cfg(all(
    feature = "logging",
    any(feature = "log", feature = "defmt", feature = "tracing")
))]
use crate::logging::formatter::Formatter;
use crate::{
    decode::{
        decoder::Decoder,
        frame::Frame,
        maybe_decoded::{FrameSize, MaybeDecoded},
    },
    encode::encoder::Encoder,
};

/// A codec that decodes and encodes a sequence of bytes starting with a length field (4 bytes).
///
/// `N` is the maximum number of bytes that a frame can contain.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LengthDelimitedCodec<const N: usize>;

/// An error that can occur when decoding a length delimited sequence of bytes into a sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LengthDelimitedDecodeError {
    /// The decoded sequesnce of bytes is too large to fit into the output buffer.
    OutputBufferTooSmall,
    /// The received frame size is smaller than the minimum frame size (4 bytes).
    InvalidFrameSize,
}

impl core::fmt::Display for LengthDelimitedDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
            Self::InvalidFrameSize => write!(f, "Invalid frame size"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LengthDelimitedDecodeError {}

/// An error that can occur when encoding a sequence of bytes into a length delimited sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LengthDelimitedEncodeError {
    /// The input buffer is too small to fit the encoded sequesnce of bytes.
    InputBufferTooSmall,
}

impl core::fmt::Display for LengthDelimitedEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputBufferTooSmall => write!(f, "Input buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LengthDelimitedEncodeError {}

impl<const N: usize> LengthDelimitedCodec<N> {
    /// Creates a new [`LengthDelimitedCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Encodes a slice of bytes into a destination buffer.
    pub fn encode_slice(
        &self,
        item: &[u8],
        dst: &mut [u8],
    ) -> Result<usize, LengthDelimitedEncodeError> {
        let item_size = item.len();
        let frame_size = item_size + 4;

        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let item = Formatter(item);
            tracing::debug!(?item, %item_size, %frame_size, available=%dst.len(), "Encoding Frame");
        }

        if dst.len() < frame_size {
            return Err(LengthDelimitedEncodeError::InputBufferTooSmall);
        }

        let frame_size_bytes = (frame_size as u32).to_be_bytes();
        dst[..4].copy_from_slice(&frame_size_bytes);
        dst[4..frame_size].copy_from_slice(item);

        Ok(frame_size)
    }
}

impl<const N: usize> Default for LengthDelimitedCodec<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Decoder for LengthDelimitedCodec<N> {
    type Item = heapless::Vec<u8, N>;
    type Error = LengthDelimitedDecodeError;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let src = Formatter(src);
            tracing::debug!(?src, "Decoding");
        }

        if src.len() < 4 {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::debug!("Not enough bytes to read frame size");

            return Ok(MaybeDecoded::None(FrameSize::Unknown));
        }

        let frame_size = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        #[cfg(all(feature = "logging", feature = "tracing"))]
        tracing::debug!(frame_size, "Frame size");

        if src.len() < frame_size {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::debug!("Not enough bytes to read frame");

            return Ok(MaybeDecoded::None(FrameSize::Known(frame_size)));
        }

        if frame_size < 4 {
            return Err(LengthDelimitedDecodeError::InvalidFrameSize);
        }

        let frame_buf = &src[4..frame_size];

        let item = heapless::Vec::from_slice(frame_buf)
            .map_err(|_| LengthDelimitedDecodeError::OutputBufferTooSmall)?;

        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            tracing::debug!(frame=?frame_buf, consuming=%frame_size, "Decoded frame");
        }

        Ok(MaybeDecoded::Frame(Frame::new(frame_size, item)))
    }
}

impl<const N: usize> Encoder<heapless::Vec<u8, N>> for LengthDelimitedCodec<N> {
    type Error = LengthDelimitedEncodeError;

    fn encode(&mut self, item: heapless::Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_slice(&item, dst)
    }
}

#[cfg(test)]
mod test;
