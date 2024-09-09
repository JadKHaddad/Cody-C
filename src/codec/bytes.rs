#[cfg(all(
    feature = "logging",
    any(feature = "log", feature = "defmt", feature = "tracing")
))]
use crate::logging::formatter::Formatter;

use crate::{
    decode::{
        decoder::{DecodeError, Decoder},
        frame::Frame,
    },
    encode::encoder::Encoder,
};

/// A codec that spits out bytes as they come in.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BytesCodec<const N: usize>;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BytesDecodeError {
    DecodeError(DecodeError),
}

impl From<DecodeError> for BytesDecodeError {
    fn from(err: DecodeError) -> Self {
        Self::DecodeError(err)
    }
}

impl core::fmt::Display for BytesDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DecodeError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytesDecodeError {}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BytesEncodeError {
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
    pub const fn new() -> Self {
        Self
    }

    pub fn encode_slice(&self, item: &[u8], dst: &mut [u8]) -> Result<usize, BytesEncodeError> {
        let size = item.len();

        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let item = Formatter(item);
            tracing::debug!(frame=?item, item_size=%size, available=%dst.len(), "Encoding Frame");
        }

        if dst.len() < size {
            return Err(BytesEncodeError::InputBufferTooSmall);
        }

        dst[..size].copy_from_slice(item);

        Ok(size)
    }
}

impl<const N: usize> Decoder for BytesCodec<N> {
    type Item = heapless::Vec<u8, N>;
    type Error = BytesDecodeError;

    fn decode(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let src = Formatter(src);
            tracing::debug!(?src, "Decoding");
        }

        let size = match src.len() {
            0 => return Ok(None),
            n if n > N => N,
            n => n,
        };

        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let src = Formatter(&src[..size]);
            tracing::debug!(frame=?src, consuming=%size, "Decoding frame");
        }

        let item = heapless::Vec::from_slice(&src[..size]).expect("unreachable");
        let frame = Frame::new(size, item);

        Ok(Some(frame))
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

#[cfg(all(test, feature = "tokio"))]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::{decode::framed_read::FramedRead, test::init_tracing, tokio::Compat};

    async fn from_slice<const I: usize, const O: usize>() {
        let read: &[u8] =
            b"jh asjdk hbjsjuwjal kadjjsadhjiuwqens nd yxxcjajsdiaskdn asjdasdiouqw essd";
        let codec = BytesCodec::<O>;
        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(read, codec, buf);
        let byte_chunks: Vec<_> = framed_read.collect().await;

        let bytes = byte_chunks
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(bytes, read);
    }

    async fn from_slow_reader<const I: usize, const O: usize>() {
        let chunks = std::vec![
            Vec::from(b"jh asjd"),
            Vec::from(b"k hbjsjuwjal kadjjsadhjiuw"),
            Vec::from(b"jal kadjjsadhjiuwqens "),
            Vec::from(b"nd "),
            Vec::from(b"yxxcjajsdiaskdn as"),
            Vec::from(b"jdasdiouqw es"),
            Vec::from(b"sd"),
        ];

        let chunks_copy = chunks.clone();

        let (read, mut write) = tokio::io::duplex(1024);

        tokio::spawn(async move {
            for chunk in chunks {
                write.write_all(&chunk).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let read = Compat::new(read);

        let codec = BytesCodec::<O>;
        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(read, codec, buf);
        let byte_chunks: Vec<_> = framed_read.collect().await;

        let bytes = byte_chunks
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(bytes, chunks_copy.concat());
    }

    #[tokio::test]
    async fn from_slice_tiny_buffers() {
        init_tracing();

        from_slice::<1, 1>().await;
    }

    #[tokio::test]
    async fn from_slice_same_size() {
        init_tracing();

        from_slice::<5, 5>().await;
    }

    #[tokio::test]
    async fn from_slice_input_larger() {
        init_tracing();

        from_slice::<5, 3>().await;
    }

    #[tokio::test]
    async fn from_slice_output_larger() {
        init_tracing();

        from_slice::<3, 5>().await;
    }

    #[tokio::test]
    async fn from_slow_reader_tiny_buffers() {
        init_tracing();

        from_slow_reader::<1, 1>().await;
    }

    #[tokio::test]
    async fn from_slow_reader_same_size() {
        init_tracing();

        from_slow_reader::<5, 5>().await;
    }

    #[tokio::test]
    async fn from_slow_reader_input_larger() {
        init_tracing();

        from_slow_reader::<5, 3>().await;
    }

    #[tokio::test]
    async fn from_slow_reader_output_larger() {
        init_tracing();

        from_slow_reader::<3, 5>().await;
    }
}
