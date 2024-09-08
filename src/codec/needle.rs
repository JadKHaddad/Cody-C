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
pub enum NeedleDecoderError {
    /// The decoded sequesnce of bytes is too large to fit into the return buffer.
    OutputBufferTooSmall,
    DecoderError(DecoderError),
}

impl From<DecoderError> for NeedleDecoderError {
    fn from(err: DecoderError) -> Self {
        Self::DecoderError(err)
    }
}

impl core::fmt::Display for NeedleDecoderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
            Self::DecoderError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NeedleDecoderError {}

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
        type Error = NeedleDecoderError;

        fn decode(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                let buf = Formatter(buf);
                tracing::debug!(needle=?self.needle, seen=%self.seen, buf=?buf, "Decoding");
            }

            while self.seen < buf.len() {
                if buf[self.seen..].starts_with(self.needle) {
                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    {
                        {
                            let buf = Formatter(&buf[..self.seen + self.needle.len()]);
                            tracing::debug!(sequence=?buf, "Found");
                        }

                        let buf = Formatter(&buf[..self.seen]);
                        let consuming = self.seen + self.needle.len();
                        tracing::debug!(frame=?buf, %consuming, "Framing");
                    }

                    let item = heapless::Vec::from_slice(&buf[..self.seen])
                        .map_err(|_| NeedleDecoderError::OutputBufferTooSmall)?;

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

#[cfg(all(test, feature = "futures", feature = "tokio"))]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::{decode::framed_read::FramedRead, test::init_tracing, tokio::AsyncReadCompat};

    async fn one_from_slice<const I: usize, const O: usize>() {
        let read: &[u8] = b"1##";
        let result = std::vec![heapless::Vec::<_, O>::from_slice(b"1").unwrap(),];

        let codec = NeedleCodec::<O>::new(b"##");
        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(read, codec, buf);
        let items: Vec<_> = framed_read
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(items, result);
    }

    async fn three_from_slice<const I: usize, const O: usize>() {
        let read: &[u8] = b"1##2##3##";
        let result = std::vec![
            heapless::Vec::<_, O>::from_slice(b"1").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"2").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"3").unwrap(),
        ];

        let codec = NeedleCodec::<O>::new(b"##");
        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(read, codec, buf);
        let items: Vec<_> = framed_read
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(items, result);
    }

    async fn from_slow_reader<const I: usize, const O: usize>() {
        let chunks = std::vec![
            Vec::from(b"jh asjd##"),
            Vec::from(b"k hb##jsjuwjal kadj##jsadhjiu##w"),
            Vec::from(b"##jal kadjjsadhjiuwqens ##"),
            Vec::from(b"nd "),
            Vec::from(b"yxxcjajsdi##askdn as"),
            Vec::from(b"jdasd##iouqw es"),
            Vec::from(b"sd##"),
        ];

        let result = std::vec![
            heapless::Vec::<_, O>::from_slice(b"jh asjd").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"k hb").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"jsjuwjal kadj").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"jsadhjiu").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"w").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"jal kadjjsadhjiuwqens ").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"nd yxxcjajsdi").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"askdn asjdasd").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"iouqw essd").unwrap(),
        ];

        let (read, mut write) = tokio::io::duplex(1024);

        tokio::spawn(async move {
            for chunk in chunks {
                write.write_all(&chunk).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let read = AsyncReadCompat::new(read);

        let codec = NeedleCodec::<O>::new(b"##");
        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(read, codec, buf);
        let byte_chunks: Vec<_> = framed_read.collect().await;

        let bytes: Vec<_> = byte_chunks.into_iter().flatten().collect::<Vec<_>>();

        assert_eq!(bytes, result);
    }

    #[tokio::test]
    async fn one_item_one_stroke() {
        init_tracing();

        one_from_slice::<5, 3>().await;
    }

    #[tokio::test]
    async fn three_items_one_stroke() {
        init_tracing();

        three_from_slice::<9, 5>().await;
    }

    #[tokio::test]
    async fn three_items_many_strokes() {
        init_tracing();

        // Input buffer will refill 3 times.
        three_from_slice::<3, 5>().await;
    }

    #[tokio::test]
    async fn from_slow_reader_small_buffer() {
        init_tracing();

        from_slow_reader::<32, 24>().await;
    }

    #[tokio::test]
    async fn from_slow_reader_large_buffer() {
        init_tracing();

        from_slow_reader::<1024, 24>().await;
    }
}
