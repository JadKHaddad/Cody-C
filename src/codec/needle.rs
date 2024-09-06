use crate::decode::{
    decoder::{Decoder, Error as DecoderError},
    frame::Frame,
};

/// A codec that searches for a needle in a haystack.
///
/// Decodes the hyastack into a sequence of bytes that comes before the needle, discarding the needle.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NeedleCodec<'a, const N: usize> {
    /// The needle to search for.
    needle: &'a [u8],
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NeedleCodecError {
    /// The decoded sequesnce of bytes is too large to fit into the return buffer.
    OutputBufferTooSmall,
    DecoderError(DecoderError),
}

impl From<DecoderError> for NeedleCodecError {
    fn from(err: DecoderError) -> Self {
        Self::DecoderError(err)
    }
}

impl core::fmt::Display for NeedleCodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
            Self::DecoderError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NeedleCodecError {}

impl<'a, const N: usize> NeedleCodec<'a, N> {
    /// Creates a new [`NeedleCodec`] with the given needle.
    #[inline]
    pub const fn new(needle: &'a [u8]) -> Self {
        Self { needle, seen: 0 }
    }

    /// Returns the needle.
    #[inline]
    pub const fn needle(&self) -> &'a [u8] {
        self.needle
    }

    /// Returns the number of bytes of the slice that have been seen so far.
    #[inline]
    pub const fn seen(&self) -> usize {
        self.seen
    }
}

const _: () = {
    #[cfg(all(
        feature = "logging",
        any(feature = "log", feature = "defmt", feature = "tracing")
    ))]
    use crate::logging::formatter::Formatter;

    impl<'a, const N: usize> Decoder for NeedleCodec<'a, N> {
        type Item = heapless::Vec<u8, N>;
        type Error = NeedleCodecError;

        fn decode(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                let buf = Formatter(buf);
                tracing::debug!(needle=?self.needle, seen=%self.seen, buf=?buf, "Decoding");
            }

            while self.seen < buf.len() {
                if buf[self.seen..].starts_with(self.needle) {
                    let item = heapless::Vec::from_slice(&buf[..self.seen])
                        .map_err(|_| NeedleCodecError::OutputBufferTooSmall)?;

                    let frame = Frame::new(self.seen + self.needle.len(), item);

                    self.seen = 0;

                    return Ok(Some(frame));
                }

                self.seen += 1;
            }

            Ok(None)
        }
    }
};
