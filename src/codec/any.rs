//! Any delimiter codecs for encoding and decoding bytes.

use core::convert::Infallible;

use heapless::Vec;

use crate::{
    decode::{Decoder, DecoderOwned},
    encode::Encoder,
};

/// A codec that decodes bytes ending with a `delimiter` into bytes and encodes bytes into bytes ending with a `delimiter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AnyDelimiterCodec<'a> {
    /// The delimiter to search for.
    delimiter: &'a [u8],
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

impl<'a> AnyDelimiterCodec<'a> {
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
}

impl<'buf> Decoder<'buf> for AnyDelimiterCodec<'_> {
    type Item = &'buf [u8];
    type Error = Infallible;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        if src.len() < self.delimiter.len() {
            return Ok(None);
        }

        match self.delimiter.last() {
            None => {
                let bytes = &src[..self.seen + 1];
                let item = (bytes, self.seen + 1);

                Ok(Some(item))
            }
            Some(last_byte) => {
                while self.seen < src.len() {
                    if src[self.seen] == *last_byte {
                        let src_delimiter =
                            &src[self.seen + 1 - self.delimiter.len()..self.seen + 1];

                        if src_delimiter == self.delimiter {
                            let bytes = &src[..self.seen + 1 - self.delimiter.len()];
                            let item = (bytes, self.seen + 1);

                            self.seen = 0;

                            return Ok(Some(item));
                        }
                    }

                    self.seen += 1;
                }

                Ok(None)
            }
        }
    }
}

/// Error returned by [`AnyDelimiterCodec::encode`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AnyDelimiterEncodeError {
    /// The input buffer is too small to fit the encoded bytes.
    BufferTooSmall,
}

impl core::fmt::Display for AnyDelimiterEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AnyDelimiterEncodeError::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

impl core::error::Error for AnyDelimiterEncodeError {}

impl Encoder<&[u8]> for AnyDelimiterCodec<'_> {
    type Error = AnyDelimiterEncodeError;

    fn encode(&mut self, item: &[u8], dst: &mut [u8]) -> Result<usize, Self::Error> {
        let size = item.len() + self.delimiter.len();

        if dst.len() < size {
            return Err(AnyDelimiterEncodeError::BufferTooSmall);
        }

        dst[..item.len()].copy_from_slice(item);
        dst[item.len()..size].copy_from_slice(self.delimiter);

        Ok(size)
    }
}

/// An owned [`AnyDelimiterCodec`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AnyDelimiterCodecOwned<'a, const N: usize> {
    inner: AnyDelimiterCodec<'a>,
}

impl<'a, const N: usize> AnyDelimiterCodecOwned<'a, N> {
    /// Creates a new [`AnyDelimiterCodecOwned`] with the given `delimiter`.
    #[inline]
    pub const fn new(delimiter: &'a [u8]) -> Self {
        Self {
            inner: AnyDelimiterCodec::new(delimiter),
        }
    }

    /// Returns the delimiter to search for.
    #[inline]
    pub const fn delimiter(&self) -> &'a [u8] {
        self.inner.delimiter
    }

    /// Returns the number of bytes of the slice that have been seen so far.
    #[inline]
    pub const fn seen(&self) -> usize {
        self.inner.seen
    }

    /// Clears the number of bytes of the slice that have been seen so far.
    #[inline]
    pub fn clear(&mut self) {
        self.inner.seen = 0;
    }
}

impl<'a, const N: usize> From<AnyDelimiterCodec<'a>> for AnyDelimiterCodecOwned<'a, N> {
    fn from(inner: AnyDelimiterCodec<'a>) -> Self {
        Self { inner }
    }
}

impl<const N: usize> DecoderOwned for AnyDelimiterCodecOwned<'_, N> {
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

impl<const N: usize> Encoder<Vec<u8, N>> for AnyDelimiterCodecOwned<'_, N> {
    type Error = AnyDelimiterEncodeError;

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
        FramedRead, FramedWrite, ReadError,
        logging::error,
        tests::{framed_read, init_tracing, sink_stream},
    };

    use super::*;

    #[tokio::test]
    async fn framed_read() {
        init_tracing();

        // cspell: disable
        let items: &[&[u8]] = &[
            b"jh asjd##ppppppppppppppp##",
            b"k hb##jsjuwjal kadj##jsadhjiu##w",
            b"##jal kadjjsadhjiuwqens ##",
            b"nd ",
            b"yxxcjajsdi##askdn as",
            b"jdasd##iouqw es",
            b"sd##k",
        ];
        // cspell: enable

        let decoder = AnyDelimiterCodec::new(b"##");

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

        // cspell: disable

        let expected: &[&[u8]] = &[b"jh asjd"];
        framed_read!(items, expected, decoder, 16, BufferTooSmall);

        let expected: &[&[u8]] = &[
            b"jh asjd",
            b"ppppppppppppppp",
            b"k hb",
            b"jsjuwjal kadj",
            b"jsadhjiu",
            b"w",
            b"jal kadjjsadhjiuwqens ",
            b"nd yxxcjajsdi",
            b"askdn asjdasd",
            b"iouqw essd",
        ];

        // cspell: enable

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
            heapless::Vec::from_slice(b"Hei").unwrap(),
            heapless::Vec::from_slice(b"sup").unwrap(),
            heapless::Vec::from_slice(b"Hey").unwrap(),
        ];

        let decoder = AnyDelimiterCodecOwned::<32>::new(b"###");
        let encoder = AnyDelimiterCodecOwned::<32>::new(b"###");

        sink_stream!(encoder, decoder, items);
    }
}
