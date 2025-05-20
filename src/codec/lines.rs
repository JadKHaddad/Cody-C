//! Lines codecs for encoding and decoding line bytes.

use core::convert::Infallible;

use heapless::Vec;

use crate::{
    decode::{Decoder, DecoderOwned},
    encode::Encoder,
};

/// A codec that decodes bytes into a line and encodes a line into bytes.
///
/// # Note
///
/// This codec tracks progress using an internal state of the underlying buffer, and it must not be used across multiple framing sessions.
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

/// Error returned by [`LinesCodec::encode`].
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

impl core::error::Error for LinesEncodeError {}

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

/// An owned [`LinesCodec`].
///
/// # Note
///
/// This codec tracks progress using an internal state of the underlying buffer, and it must not be used across multiple framing sessions.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LinesCodecOwned<const N: usize> {
    inner: LinesCodec,
}

impl<const N: usize> LinesCodecOwned<N> {
    /// Creates a new [`LinesCodecOwned`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: LinesCodec::new(),
        }
    }
}

impl<const N: usize> From<LinesCodec> for LinesCodecOwned<N> {
    fn from(inner: LinesCodec) -> Self {
        Self { inner }
    }
}

/// Error returned by [`LinesCodecOwned::decode_owned`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LinesCodecOwnedError {
    /// The buffer is too small to fit the decoded bytes.
    BufferTooSmall,
}

impl core::fmt::Display for LinesCodecOwnedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LinesCodecOwnedError::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

impl core::error::Error for LinesCodecOwnedError {}

impl<const N: usize> DecoderOwned for LinesCodecOwned<N> {
    type Item = Vec<u8, N>;
    type Error = LinesCodecOwnedError;

    fn decode_owned(&mut self, src: &mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        match Decoder::decode(&mut self.inner, src) {
            Ok(Some((bytes, size))) => {
                let item =
                    Vec::from_slice(bytes).map_err(|_| LinesCodecOwnedError::BufferTooSmall)?;
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

#[cfg(test)]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::{SinkExt, StreamExt, pin_mut};
    use tokio::io::AsyncWriteExt;

    use crate::{
        ReadError,
        tests::{framed_read, init_tracing, sink_stream},
    };

    use super::*;

    #[tokio::test]
    async fn framed_read() {
        init_tracing();

        let items: &[&[u8]] = &[
            b"Hel",
            b"lo\n",
            b"Hell",
            b"o, world!\n",
            b"H",
            b"ei\r\n",
            b"sup",
            b"\n",
            b"Hey\r",
            b"\n",
            b"How ",
            b"are y",
        ];

        let decoder = LinesCodec::new();

        let expected: &[&[u8]] = &[];
        framed_read!(items, expected, decoder, 1, BufferTooSmall);
        framed_read!(items, expected, decoder, 1, 1, BufferTooSmall);
        framed_read!(items, expected, decoder, 1, 2, BufferTooSmall);
        framed_read!(items, expected, decoder, 1, 4, BufferTooSmall);

        framed_read!(items, expected, decoder, 2, BufferTooSmall);
        framed_read!(items, expected, decoder, 2, 1, BufferTooSmall);
        framed_read!(items, expected, decoder, 2, 2, BufferTooSmall);
        framed_read!(items, expected, decoder, 2, 4, BufferTooSmall);

        framed_read!(items, expected, decoder, 4, BufferTooSmall);
        framed_read!(items, expected, decoder, 4, 1, BufferTooSmall);
        framed_read!(items, expected, decoder, 4, 2, BufferTooSmall);
        framed_read!(items, expected, decoder, 4, 4, BufferTooSmall);

        let expected: &[&[u8]] = &[b"Hello"];
        framed_read!(items, expected, decoder, 8, BufferTooSmall);

        let expected: &[&[u8]] = &[b"Hello", b"Hello, world!", b"Hei", b"sup", b"Hey"];
        framed_read!(items, expected, decoder, 16, BytesRemainingOnStream);
        framed_read!(items, expected, decoder, 16, 1, BytesRemainingOnStream);
        framed_read!(items, expected, decoder, 16, 2, BytesRemainingOnStream);
        framed_read!(items, expected, decoder, 16, 4, BytesRemainingOnStream);

        framed_read!(items, expected, decoder);
    }

    #[tokio::test]
    async fn sink_stream() {
        init_tracing();

        let items: Vec<heapless::Vec<u8, 32>> = std::vec![
            heapless::Vec::from_slice(b"Hello").unwrap(),
            heapless::Vec::from_slice(b"Hello, world!").unwrap(),
            heapless::Vec::from_slice(b"Hei").unwrap(),
            heapless::Vec::from_slice(b"sup").unwrap(),
            heapless::Vec::from_slice(b"Hey").unwrap(),
        ];

        let decoder = LinesCodecOwned::<32>::new();
        let encoder = LinesCodecOwned::<32>::new();

        sink_stream!(encoder, decoder, items);
    }
}
