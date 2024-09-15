//! An any delimiter codec for encoding and decoding bytes.

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

/// A codec that decodes a sequence of bytes ending with a `delimiter` into a sequence of bytes and encodes a sequence of bytes into a sequence of bytes ending with a `delimiter`.
///
/// `N` is the maximum number of bytes that a frame can contain.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AnyDelimiterCodec<'a, const N: usize> {
    /// The delimiter to search for.
    delimiter: &'a [u8],
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

/// An error that can occur when decoding a sequence of bytes ending with a `delimiter` into a sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AnyDelimiterDecodeError {
    /// The decoded sequesnce of bytes is too large to fit into the output buffer.
    OutputBufferTooSmall,
}

impl core::fmt::Display for AnyDelimiterDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AnyDelimiterDecodeError {}

/// An error that can occur when encoding a sequence of bytes into a sequence of bytes ending with a `delimiter`.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AnyDelimiterEncodeError {
    /// The input buffer is too small to fit the encoded sequesnce of bytes.
    InputBufferTooSmall,
}

impl core::fmt::Display for AnyDelimiterEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputBufferTooSmall => write!(f, "Input buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AnyDelimiterEncodeError {}

impl<'a, const N: usize> AnyDelimiterCodec<'a, N> {
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

    /// Encodes a slice of bytes into a destination buffer.
    pub fn encode_slice(
        &self,
        item: &[u8],
        dst: &mut [u8],
    ) -> Result<usize, AnyDelimiterEncodeError> {
        let size = item.len() + self.delimiter.len();

        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let item = Formatter(item);
            tracing::debug!(frame=?item, item_size=%size, available=%dst.len(), "Encoding Frame");
        }

        if dst.len() < size {
            return Err(AnyDelimiterEncodeError::InputBufferTooSmall);
        }

        dst[..item.len()].copy_from_slice(item);
        dst[item.len()..size].copy_from_slice(self.delimiter);

        Ok(size)
    }
}

impl<'a, const N: usize> Decoder for AnyDelimiterCodec<'a, N> {
    type Item = heapless::Vec<u8, N>;
    type Error = AnyDelimiterDecodeError;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let src = Formatter(src);
            let delimiter = Formatter(self.delimiter);
            tracing::debug!(?delimiter, seen=%self.seen, ?src, "Decoding");
        }

        if src.len() < self.delimiter.len() {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::debug!("Not enough bytes to read frame size");

            return Ok(MaybeDecoded::None(FrameSize::Unknown));
        }

        match self.delimiter.last() {
            None => {
                let item = heapless::Vec::from_slice(&src[..self.seen + 1])
                    .map_err(|_| AnyDelimiterDecodeError::OutputBufferTooSmall)?;

                let frame = Frame::new(self.seen + 1, item);

                Ok(MaybeDecoded::Frame(frame))
            }
            Some(last_byte) => {
                while self.seen < src.len() {
                    if src[self.seen] == *last_byte {
                        let src_delimiter =
                            &src[self.seen + 1 - self.delimiter.len()..self.seen + 1];

                        if src_delimiter == self.delimiter {
                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            {
                                {
                                    let src = Formatter(&src[..self.seen + 1]);
                                    tracing::debug!(sequence=?src, "Found");
                                }

                                let src = Formatter(&src[..self.seen + 1 - self.delimiter.len()]);
                                let consuming = self.seen + 1;
                                tracing::debug!(frame=?src, %consuming, "Decoding frame");
                            }

                            let item = heapless::Vec::from_slice(
                                &src[..self.seen + 1 - self.delimiter.len()],
                            )
                            .map_err(|_| AnyDelimiterDecodeError::OutputBufferTooSmall)?;

                            let frame = Frame::new(self.seen + 1, item);

                            self.seen = 0;

                            return Ok(MaybeDecoded::Frame(frame));
                        }
                    }

                    self.seen += 1;
                }

                Ok(MaybeDecoded::None(FrameSize::Unknown))
            }
        }
    }
}

impl<'a, const N: usize> Encoder<heapless::Vec<u8, N>> for AnyDelimiterCodec<'a, N> {
    type Error = AnyDelimiterEncodeError;

    fn encode(&mut self, item: heapless::Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_slice(&item, dst)
    }
}

#[cfg(test)]
mod test;
