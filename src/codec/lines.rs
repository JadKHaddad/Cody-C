//! Lines codecs for encoding and decoding line bytes or line `string`s.

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

/// A codec that decodes a sequence of bytes into a line and encodes a line into a sequence of bytes.
///
/// `N` is the maximum number of bytes that a frame can contain.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LineBytesCodec<const N: usize> {
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

/// An error that can occur when decoding a sequence of bytes into a line.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineBytesDecodeError {
    /// The decoded line is too large to fit into the output buffer.
    OutputBufferTooSmall,
}

impl core::fmt::Display for LineBytesDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LineBytesDecodeError {}

/// An error that can occur when encoding a line into a sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineBytesEncodeError {
    /// The input buffer is too small to fit the encoded line.
    InputBufferTooSmall,
}

impl core::fmt::Display for LineBytesEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputBufferTooSmall => write!(f, "Input buffer too small"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LineBytesEncodeError {}

impl<const N: usize> Default for LineBytesCodec<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// A codec that decodes a sequence of bytes into a line `string` and encodes a line `string` into a sequence of bytes.
///
/// `N` is the maximum number of bytes that a frame can contain.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LinesCodec<const N: usize> {
    /// The inner [`LineBytesCodec`].
    inner: LineBytesCodec<N>,
}

/// An error that can occur when decoding a sequence of bytes into a line `string`.
#[derive(Debug)]
pub enum LinesDecodeError {
    /// The decoded line `string` is not valid UTF-8.
    Utf8Error(core::str::Utf8Error),
    /// The underlying [`LineBytesCodec`] encountered an error.
    LineBytesDecodeError(LineBytesDecodeError),
}

#[cfg(feature = "defmt")]
impl defmt::Format for LinesDecodeError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Utf8Error(_) => defmt::write!(f, "UTF-8 error"),
            Self::LineBytesDecodeError(err) => {
                defmt::write!(f, "Line bytes decoder error: {}", err)
            }
        }
    }
}

impl From<core::str::Utf8Error> for LinesDecodeError {
    fn from(err: core::str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

impl From<LineBytesDecodeError> for LinesDecodeError {
    fn from(err: LineBytesDecodeError) -> Self {
        Self::LineBytesDecodeError(err)
    }
}

impl core::fmt::Display for LinesDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Utf8Error(err) => write!(f, "UTF-8 error: {}", err),
            Self::LineBytesDecodeError(err) => write!(f, "Line bytes decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LinesDecodeError {}

impl<const N: usize> LineBytesCodec<N> {
    /// Creates a new [`LineBytesCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self { seen: 0 }
    }

    /// Returns the number of bytes of the slice that have been seen so far.
    #[inline]
    pub const fn seen(&self) -> usize {
        self.seen
    }

    /// Encodes a line bytes into a destination buffer.
    pub fn encode_slice(&self, item: &[u8], dst: &mut [u8]) -> Result<usize, LineBytesEncodeError> {
        let size = item.len() + 2;

        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let item = Formatter(item);
            tracing::debug!(frame=?item, item_size=%size, available=%dst.len(), "Encoding Frame");
        }

        if dst.len() < size {
            return Err(LineBytesEncodeError::InputBufferTooSmall);
        }

        dst[..item.len()].copy_from_slice(item);
        dst[item.len()..size].copy_from_slice(b"\r\n");

        Ok(size)
    }
}

impl<const N: usize> Decoder for LineBytesCodec<N> {
    type Item = heapless::Vec<u8, N>;
    type Error = LineBytesDecodeError;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let src = Formatter(src);
            tracing::debug!(seen=%self.seen, ?src, "Decoding");
        }

        while self.seen < src.len() {
            if src[self.seen] == b'\n' {
                #[cfg(all(feature = "logging", feature = "tracing"))]
                {
                    let line_bytes_with_n = &src[..self.seen + 1];
                    let src = Formatter(line_bytes_with_n);
                    tracing::debug!(line=?src, "Found");
                }

                let line_bytes_without_n = &src[..self.seen];

                let line_bytes = match line_bytes_without_n.last() {
                    Some(b'\r') => &line_bytes_without_n[..self.seen - 1],
                    _ => line_bytes_without_n,
                };

                #[cfg(all(feature = "logging", feature = "tracing"))]
                {
                    let src = Formatter(line_bytes);
                    let consuming = self.seen + 1;
                    tracing::debug!(frame=?src, %consuming, "Decoding frame");
                }

                let item = heapless::Vec::from_slice(line_bytes)
                    .map_err(|_| LineBytesDecodeError::OutputBufferTooSmall)?;

                let frame = Frame::new(self.seen + 1, item);

                self.seen = 0;

                return Ok(MaybeDecoded::Frame(frame));
            }

            self.seen += 1;
        }

        Ok(MaybeDecoded::None(FrameSize::Unknown))
    }
}

impl<const N: usize> Encoder<heapless::Vec<u8, N>> for LineBytesCodec<N> {
    type Error = LineBytesEncodeError;

    fn encode(&mut self, item: heapless::Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_slice(&item, dst)
    }
}

/// An error that can occur when encoding a line `string` into a sequence of bytes.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LinesEncodeError {
    /// The underlying [`LineBytesCodec`] encountered an error.
    LineBytesEncodeError(LineBytesEncodeError),
}

impl From<LineBytesEncodeError> for LinesEncodeError {
    fn from(err: LineBytesEncodeError) -> Self {
        Self::LineBytesEncodeError(err)
    }
}

impl core::fmt::Display for LinesEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LineBytesEncodeError(err) => write!(f, "Line bytes encoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LinesEncodeError {}

impl<const N: usize> LinesCodec<N> {
    /// Creates a new [`LinesCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: LineBytesCodec::new(),
        }
    }

    /// Returns the number of bytes of the slice that have been seen so far.
    #[inline]
    pub const fn seen(&self) -> usize {
        self.inner.seen()
    }

    /// Encodes a line `string` into a destination buffer.
    pub fn encode_str(&self, item: &str, dst: &mut [u8]) -> Result<usize, LinesEncodeError> {
        match self.inner.encode_slice(item.as_bytes(), dst) {
            Ok(size) => Ok(size),
            Err(err) => Err(LinesEncodeError::LineBytesEncodeError(err)),
        }
    }
}

impl<const N: usize> Default for LinesCodec<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Decoder for LinesCodec<N> {
    type Item = heapless::String<N>;
    type Error = LinesDecodeError;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        match self.inner.decode(src)? {
            MaybeDecoded::Frame(frame) => {
                let size = frame.size();
                let item = heapless::String::from_utf8(frame.into_item())
                    .map_err(LinesDecodeError::Utf8Error)?;

                Ok(MaybeDecoded::Frame(Frame::new(size, item)))
            }
            MaybeDecoded::None(frame_size) => Ok(MaybeDecoded::None(frame_size)),
        }
    }
}

impl<const N: usize> Encoder<heapless::String<N>> for LinesCodec<N> {
    type Error = LinesEncodeError;

    fn encode(&mut self, item: heapless::String<N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_str(&item, dst)
    }
}

#[cfg(test)]
mod test;
