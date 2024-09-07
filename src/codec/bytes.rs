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
            Self::DecoderError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytesCodecError {}

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

            let size = match buf.len() {
                0 => return Ok(None),
                n if n > N => N,
                n => n,
            };

            let item = heapless::Vec::from_slice(&buf[..size]).expect("unreachable");
            let frame = Frame::new(size, item);

            Ok(Some(frame))
        }
    }
};

#[cfg(all(test, feature = "futures", feature = "tokio"))]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::{decode::framed_read::FramedRead, test::init_tracing, tokio::AsyncReadCompat};

    async fn from_slice<const I: usize, const O: usize>() {
        let read =
            &mut b"jh asjdk hbjsjuwjal kadjjsadhjiuwqens nd yxxcjajsdiaskdn asjdasdiouqw essd"
                .as_ref();
        let read_copy = &read[..];

        let read = AsyncReadCompat::new(read);

        let codec = BytesCodec::<O>;
        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(codec, read, buf);
        let byte_chunks: Vec<_> = framed_read.collect().await;

        let bytes = byte_chunks
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(bytes, read_copy);
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

        let read = AsyncReadCompat::new(read);

        let codec = BytesCodec::<O>;
        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(codec, read, buf);
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
