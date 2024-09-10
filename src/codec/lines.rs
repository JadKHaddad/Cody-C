#[cfg(all(
    feature = "logging",
    any(feature = "log", feature = "defmt", feature = "tracing")
))]
use crate::logging::formatter::Formatter;

use crate::{
    decode::{decoder::Decoder, frame::Frame},
    encode::encoder::Encoder,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LineBytesCodec<const N: usize> {
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineBytesDecodeError {
    /// The decoded sequesnce of bytes is too large to fit into the return buffer.
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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineBytesEncodeError {
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LinesCodec<const N: usize> {
    inner: LineBytesCodec<N>,
}

#[derive(Debug)]
pub enum LinesDecodeError {
    Utf8Error(core::str::Utf8Error),
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
            Self::DecodeError(err) => defmt::write!(f, "Decoder error: {}", err),
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

    fn decode(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
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

                return Ok(Some(frame));
            }

            self.seen += 1;
        }

        Ok(None)
    }
}

impl<const N: usize> Encoder<heapless::Vec<u8, N>> for LineBytesCodec<N> {
    type Error = LineBytesEncodeError;

    fn encode(&mut self, item: heapless::Vec<u8, N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_slice(&item, dst)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LinesEncodeError {
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

    fn decode(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        match self.inner.decode(src)? {
            Some(frame) => {
                let size = frame.size();
                let item = heapless::String::from_utf8(frame.into_item())
                    .map_err(LinesDecodeError::Utf8Error)?;

                Ok(Some(Frame::new(size, item)))
            }
            None => Ok(None),
        }
    }
}

impl<const N: usize> Encoder<heapless::String<N>> for LinesCodec<N> {
    type Error = LinesEncodeError;

    fn encode(&mut self, item: heapless::String<N>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.encode_str(&item, dst)
    }
}

#[cfg(all(test, feature = "tokio"))]
mod test {
    extern crate std;

    use core::str::FromStr;
    use std::vec::Vec;

    use futures::{SinkExt, StreamExt};
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::{
        decode::framed_read::FramedRead, encode::framed_write::FramedWrite, test::init_tracing,
        tokio::Compat,
    };

    macro_rules! collect_items {
        ($framed_read:expr) => {{
            let items: Vec<_> = $framed_read
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

            items
        }};
    }

    async fn one_from_slice<const I: usize, const O: usize>() {
        // Test with `LineBytesCodec`

        let read: &[u8] = b"1\r\n";

        let result = std::vec![heapless::Vec::<_, O>::from_slice(b"1").unwrap(),];

        let codec = LineBytesCodec::<O>::new();
        let buf = &mut [0_u8; I];
        let framed_read = FramedRead::new(read, codec, buf);

        let items = collect_items!(framed_read);

        assert_eq!(items, result);

        // Test with `LinesCodec`

        let read: &[u8] = b"1\r\n";
        let result = std::vec![heapless::String::<O>::from_str("1").unwrap(),];

        let codec = LinesCodec::<O>::new();
        let buf = &mut [0_u8; I];
        let framed_read = FramedRead::new(read, codec, buf);

        let items = collect_items!(framed_read);

        assert_eq!(items, result);
    }

    async fn four_from_slice<const I: usize, const O: usize>() {
        // Test with `LineBytesCodec`

        let read: &[u8] = b"1\r\n2\n3\n4\r\n";
        let result = std::vec![
            heapless::Vec::<_, O>::from_slice(b"1").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"2").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"3").unwrap(),
            heapless::Vec::<_, O>::from_slice(b"4").unwrap(),
        ];

        let codec = LineBytesCodec::<O>::new();
        let buf = &mut [0_u8; I];
        let framed_read = FramedRead::new(read, codec, buf);

        let items = collect_items!(framed_read);

        assert_eq!(items, result);

        // Test with `LinesCodec`

        let read: &[u8] = b"1\r\n2\n3\n4\r\n";
        let result = std::vec![
            heapless::String::<O>::from_str("1").unwrap(),
            heapless::String::<O>::from_str("2").unwrap(),
            heapless::String::<O>::from_str("3").unwrap(),
            heapless::String::<O>::from_str("4").unwrap(),
        ];

        let codec = LinesCodec::<O>::new();
        let buf = &mut [0_u8; I];
        let framed_read = FramedRead::new(read, codec, buf);

        let items = collect_items!(framed_read);

        assert_eq!(items, result);
    }

    async fn from_slow_reader<const I: usize, const O: usize>() {
        let chunks = std::vec![
            Vec::from(b"jh asjd\r\n"),
            Vec::from(b"k hb\njsjuwjal kadj\njsadhjiu\r\nw"),
            Vec::from(b"\r\njal kadjjsadhjiuwqens \n"),
            Vec::from(b"nd "),
            Vec::from(b"yxxcjajsdi\naskdn as"),
            Vec::from(b"jdasd\r\niouqw es"),
            Vec::from(b"sd\n"),
        ];

        // Test with `LineBytesCodec`

        let chunks_clone = chunks.clone();

        let result_bytes = std::vec![
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
            for chunk in chunks_clone {
                write.write_all(&chunk).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let read = Compat::new(read);
        let codec = LineBytesCodec::<O>::new();
        let buf = &mut [0_u8; I];
        let framed_read = FramedRead::new(read, codec, buf);

        let items = collect_items!(framed_read);

        assert_eq!(items, result_bytes);

        // Test with `LinesCodec`

        let result_strings = result_bytes
            .clone()
            .into_iter()
            .map(|b| heapless::String::from_utf8(b).unwrap())
            .collect::<Vec<_>>();

        let (read, mut write) = tokio::io::duplex(1024);

        tokio::spawn(async move {
            for chunk in chunks {
                write.write_all(&chunk).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let read = Compat::new(read);
        let codec = LinesCodec::<O>::new();
        let buf = &mut [0_u8; I];
        let framed_read = FramedRead::new(read, codec, buf);

        let items = collect_items!(framed_read);

        assert_eq!(items, result_strings);
    }

    #[tokio::test]
    async fn one_item_one_stroke() {
        init_tracing();

        one_from_slice::<5, 3>().await;
    }

    #[tokio::test]
    async fn four_items_one_stroke() {
        init_tracing();

        four_from_slice::<11, 5>().await;
    }

    #[tokio::test]
    async fn four_items_many_strokes() {
        init_tracing();

        // Input buffer will refill 4 times.
        four_from_slice::<3, 5>().await;
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

        // Test with `LineBytesCodec`

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
            let mut framed_write =
                FramedWrite::new(Compat::new(write), LineBytesCodec::<O>::new(), write_buf);

            for item in items_clone {
                framed_write.send(item).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            framed_write.close().await.unwrap();
        });

        let read_buf = &mut [0_u8; 1024];
        let framed_read = FramedRead::new(Compat::new(read), LineBytesCodec::<O>::new(), read_buf);

        let collected_items: Vec<_> = framed_read
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        handle.await.unwrap();

        assert_eq!(collected_items, items);

        // Test with `LinesCodec`

        let items = std::vec![
            heapless::String::<O>::from_str("jh asjd").unwrap(),
            heapless::String::<O>::from_str("k hb").unwrap(),
            heapless::String::<O>::from_str("jsjuwjal kadj").unwrap(),
            heapless::String::<O>::from_str("jsadhjiu").unwrap(),
            heapless::String::<O>::from_str("w").unwrap(),
            heapless::String::<O>::from_str("jal kadjjsadhjiuwqens ").unwrap(),
            heapless::String::<O>::from_str("nd yxxcjajsdi").unwrap(),
            heapless::String::<O>::from_str("askdn asjdasd").unwrap(),
            heapless::String::<O>::from_str("iouqw essd").unwrap(),
        ];

        let items_clone = items.clone();

        let (read, write) = tokio::io::duplex(1024);

        let handle = tokio::spawn(async move {
            let write_buf = &mut [0_u8; 1024];
            let mut framed_write =
                FramedWrite::new(Compat::new(write), LinesCodec::<O>::new(), write_buf);

            for item in items_clone {
                framed_write.send(item).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            framed_write.close().await.unwrap();
        });

        let read_buf = &mut [0_u8; 1024];
        let framed_read = FramedRead::new(Compat::new(read), LinesCodec::<O>::new(), read_buf);

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
