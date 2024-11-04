//! Length codec for encoding and decoding bytes with a payload length prefix.

use core::convert::Infallible;

use heapless::Vec;

use crate::{Decoder, DecoderOwned, Encoder};

/// The size of the payload length prefix in bytes.
pub const SIZE_OF_LENGTH: usize = core::mem::size_of::<u32>();

/// A codec that decodes a sequence of bytes with a payload length prefix into a sequence of bytes and encodes a sequence of bytes into a sequence of bytes with a payload length prefix.
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

        let payload_len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        let packet_len = payload_len + SIZE_OF_LENGTH;

        if src.len() < packet_len {
            return Ok(None);
        }

        let item = (&src[SIZE_OF_LENGTH..packet_len], packet_len);

        Ok(Some(item))
    }
}

/// An error that can occur when encoding a sequence of bytes into a sequence of bytes with a payload length prefix.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LengthEncodeError {
    /// The input buffer is too small to fit the encoded sequence of bytes.
    BufferTooSmall,
    /// Payload length is zero.
    ZeroPayloadLength,
    /// The payload size is greater than u32::MAX.
    PayloadTooLarge,
}

impl core::fmt::Display for LengthEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::ZeroPayloadLength => write!(f, "Zero payload length"),
            Self::PayloadTooLarge => write!(f, "Payload too large"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LengthEncodeError {}

impl Encoder<&[u8]> for LengthCodec {
    type Error = LengthEncodeError;

    fn encode(&mut self, item: &[u8], dst: &mut [u8]) -> Result<usize, Self::Error> {
        let payload_len = item.len();

        if payload_len > u32::MAX as usize {
            return Err(LengthEncodeError::PayloadTooLarge);
        }

        let packet_len = payload_len + SIZE_OF_LENGTH;

        if dst.len() < packet_len {
            return Err(LengthEncodeError::BufferTooSmall);
        }

        dst[0..SIZE_OF_LENGTH].copy_from_slice(&(item.len() as u32).to_be_bytes());
        dst[SIZE_OF_LENGTH..packet_len].copy_from_slice(item);

        Ok(packet_len)
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

#[cfg(test)]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::{pin_mut, SinkExt, StreamExt};
    use tokio::io::AsyncWriteExt;

    use crate::{
        error, framed_read, sink_stream, test::init_tracing, tokio::Compat, FramedRead,
        FramedReadError, FramedWrite,
    };

    use super::*;

    #[tokio::test]
    async fn framed_read() {
        init_tracing();

        let items: &[&[u8]] = &[
            b"\x00\x00\x00\x05Hel",
            b"lo\x00\x00",
            b"\x00\x0DHell",
            b"o, world!",
            b"\x00",
            b"\x00\x00\x03H",
            b"ei\x00\x00",
            b"\x00",
            b"\x03sup",
            b"\x00\x00",
            b"\x00\x03Hey",
            b"\x00\x00",
            b"\x00\x0CHow are y",
        ];

        let decoder = LengthCodec::new();

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
        framed_read!(items, expected, decoder, 12, BufferTooSmall);

        let expected: &[&[u8]] = &[b"Hello", b"Hello, world!", b"Hei", b"sup", b"Hey"];
        framed_read!(items, expected, decoder, 32, BytesRemainingOnStream);
        framed_read!(items, expected, decoder, 32, 1, BytesRemainingOnStream);
        framed_read!(items, expected, decoder, 32, 2, BytesRemainingOnStream);
        framed_read!(items, expected, decoder, 32, 4, BytesRemainingOnStream);

        framed_read!(items, expected, decoder);
    }

    #[tokio::test]
    async fn sink_stream() {
        init_tracing();

        let items: Vec<heapless::Vec<u8, 32>> = std::vec![
            heapless::Vec::from_slice(b"Hello").unwrap(),
            heapless::Vec::from_slice(b"Hello, world!").unwrap(),
            heapless::Vec::from_slice(b"").unwrap(),
            heapless::Vec::from_slice(b"Hei").unwrap(),
            heapless::Vec::from_slice(b"sup").unwrap(),
            heapless::Vec::from_slice(b"Hey").unwrap(),
        ];

        let decoder = LengthCodecOwned::<32>::new();
        let encoder = LengthCodecOwned::<32>::new();

        sink_stream!(encoder, decoder, items);
    }
}
