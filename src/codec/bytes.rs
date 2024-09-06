use crate::decode::{
    decoder::{Decoder, Error as DecoderError},
    frame::Frame,
};

/// A codec that spits out bytes as they come in.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BytesCodec<const N: usize>;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BytesCodecError {
    /// The decoded sequesnce of bytes is too large to fit into the return buffer.
    OutputBufferTooSmall,
    DecoderError(DecoderError),
}

impl From<DecoderError> for BytesCodecError {
    fn from(err: DecoderError) -> Self {
        Self::DecoderError(err)
    }
}

impl core::fmt::Display for BytesCodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
            Self::DecoderError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NeedleCodecError {}

const _: () = {
    #[cfg(all(
        feature = "logging",
        any(feature = "log", feature = "defmt", feature = "tracing")
    ))]
    use crate::logging::formatter::Formatter;

    impl<const N: usize> Decoder for BytesCodec<N> {
        type Item = heapless::Vec<u8, N>;
        type Error = BytesCodecError;

        fn decode(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                let buf = Formatter(buf);
                tracing::debug!(buf=?buf, "Decoding");
            }

            let item = heapless::Vec::from_slice(buf)
                .map_err(|_| BytesCodecError::OutputBufferTooSmall)?;

            let frame = Frame::new(buf.len(), item);

            Ok(Some(frame))
        }
    }
};
