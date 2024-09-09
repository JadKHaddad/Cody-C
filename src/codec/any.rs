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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AnyDelimiterCodec<'a, const N: usize> {
    /// The delimiter to search for.
    delimiter: &'a [u8],
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AnyDelimiterDecodeError {
    /// The decoded sequesnce of bytes is too large to fit into the return buffer.
    OutputBufferTooSmall,
    DecodeError(DecodeError),
}

impl From<DecodeError> for AnyDelimiterDecodeError {
    fn from(err: DecodeError) -> Self {
        Self::DecodeError(err)
    }
}

impl core::fmt::Display for AnyDelimiterDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
            Self::DecodeError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AnyDelimiterDecodeError {}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AnyDelimiterEncodeError {
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
    /// Creates a new [`AnyDelimiterCodec`] with the given AnyDelimiter.
    #[inline]
    pub const fn new(delimiter: &'a [u8]) -> Self {
        Self { delimiter, seen: 0 }
    }

    /// Returns the AnyDelimiter.
    #[inline]
    pub const fn delimiter(&self) -> &'a [u8] {
        self.delimiter
    }

    /// Returns the number of bytes of the slice that have been seen so far.
    #[inline]
    pub const fn seen(&self) -> usize {
        self.seen
    }

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

    fn decode(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let src = Formatter(src);
            tracing::debug!(AnyDelimiter=?self.delimiter, seen=%self.seen, ?src, "Decoding");
        }

        while self.seen < src.len() {
            if src[self.seen..].starts_with(self.delimiter) {
                #[cfg(all(feature = "logging", feature = "tracing"))]
                {
                    {
                        let src = Formatter(&src[..self.seen + self.delimiter.len()]);
                        tracing::debug!(sequence=?src, "Found");
                    }

                    let src = Formatter(&src[..self.seen]);
                    let consuming = self.seen + self.delimiter.len();
                    tracing::debug!(frame=?src, %consuming, "Decoding frame");
                }

                let item = heapless::Vec::from_slice(&src[..self.seen])
                    .map_err(|_| AnyDelimiterDecodeError::OutputBufferTooSmall)?;

                let frame = Frame::new(self.seen + self.delimiter.len(), item);

                self.seen = 0;

                return Ok(Some(frame));
            }

            self.seen += 1;
        }

        Ok(None)
    }
}

impl<'a, const N: usize> Encoder<heapless::Vec<u8, N>> for AnyDelimiterCodec<'a, N> {
    type Error = AnyDelimiterEncodeError;

    fn encode(&mut self, item: heapless::Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_slice(&item, dst)
    }
}

#[cfg(all(test, feature = "tokio"))]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::{SinkExt, StreamExt};
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::{
        decode::framed_read::FramedRead, encode::framed_write::FramedWrite, test::init_tracing,
        tokio::Compat,
    };

    async fn one_from_slice<const I: usize, const O: usize>() {
        let read: &[u8] = b"1##";
        let result = std::vec![heapless::Vec::<_, O>::from_slice(b"1").unwrap(),];

        let codec = AnyDelimiterCodec::<O>::new(b"##");
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

        let codec = AnyDelimiterCodec::<O>::new(b"##");
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

        let read = Compat::new(read);

        let codec = AnyDelimiterCodec::<O>::new(b"##");
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

    #[tokio::test]
    async fn sink_stream() {
        const O: usize = 24;

        init_tracing();

        let items = std::vec![
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

        let items_clone = items.clone();

        let (read, write) = tokio::io::duplex(1024);

        let handle = tokio::spawn(async move {
            let write_buf = &mut [0_u8; 1024];
            let mut framed_write = FramedWrite::new(
                Compat::new(write),
                AnyDelimiterCodec::<O>::new(b"##"),
                write_buf,
            );

            for item in items_clone {
                framed_write.send(item).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            framed_write.close().await.unwrap();
        });

        let read_buf = &mut [0_u8; 1024];
        let framed_read = FramedRead::new(
            Compat::new(read),
            AnyDelimiterCodec::<O>::new(b"##"),
            read_buf,
        );

        let collected_items: Vec<_> = framed_read
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        handle.await.unwrap();

        assert_eq!(collected_items, items);
    }
}
