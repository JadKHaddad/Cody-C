use crate::decode::{
    decoder::{Decoder, Error as DecoderError},
    frame::Frame,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LineBytesCodec<const N: usize> {
    /// The number of bytes of the slice that have been seen so far.
    seen: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineBytesCodecError {
    /// The decoded sequesnce of bytes is too large to fit into the return buffer.
    OutputBufferTooSmall,
    DecoderError(DecoderError),
}

impl From<DecoderError> for LineBytesCodecError {
    fn from(err: DecoderError) -> Self {
        Self::DecoderError(err)
    }
}

impl core::fmt::Display for LineBytesCodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutputBufferTooSmall => write!(f, "Output buffer too small"),
            Self::DecoderError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LineBytesCodecError {}

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
}

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
pub enum LinesCodecError {
    Utf8Error(core::str::Utf8Error),
    LineBytesCodecError(LineBytesCodecError),
    DecoderError(DecoderError),
}

#[cfg(feature = "defmt")]
impl defmt::Format for LinesCodecError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Utf8Error(_) => defmt::write!(f, "UTF-8 error"),
            Self::LineBytesCodecError(err) => defmt::write!(f, "Line bytes codec error: {}", err),
            Self::DecoderError(err) => defmt::write!(f, "Decoder error: {}", err),
        }
    }
}

impl From<core::str::Utf8Error> for LinesCodecError {
    fn from(err: core::str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

impl From<LineBytesCodecError> for LinesCodecError {
    fn from(err: LineBytesCodecError) -> Self {
        Self::LineBytesCodecError(err)
    }
}

impl From<DecoderError> for LinesCodecError {
    fn from(err: DecoderError) -> Self {
        Self::DecoderError(err)
    }
}

impl core::fmt::Display for LinesCodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Utf8Error(err) => write!(f, "UTF-8 error: {}", err),
            Self::LineBytesCodecError(err) => write!(f, "Line bytes codec error: {}", err),
            Self::DecoderError(err) => write!(f, "Decoder error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LinesCodecError {}

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
}

impl<const N: usize> Default for LinesCodec<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Decoder for LinesCodec<N> {
    type Item = heapless::String<N>;
    type Error = LinesCodecError;

    fn decode(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        match self.inner.decode(buf)? {
            Some(frame) => {
                let size = frame.size();
                let item = heapless::String::from_utf8(frame.into_item())
                    .map_err(LinesCodecError::Utf8Error)?;

                Ok(Some(Frame::new(size, item)))
            }
            None => Ok(None),
        }
    }
}

const _: () = {
    #[cfg(all(
        feature = "logging",
        any(feature = "log", feature = "defmt", feature = "tracing")
    ))]
    use crate::logging::formatter::Formatter;

    impl<const N: usize> Decoder for LineBytesCodec<N> {
        type Item = heapless::Vec<u8, N>;
        type Error = LineBytesCodecError;

        fn decode(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                let buf = Formatter(buf);
                tracing::debug!(seen=%self.seen, buf=?buf, "Decoding");
            }

            while self.seen < buf.len() {
                if buf[self.seen] == b'\n' {
                    let line_bytes_with_n = &buf[..self.seen + 1];

                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    {
                        let buf = Formatter(line_bytes_with_n);
                        tracing::debug!(line=?buf, "Found");
                    }

                    let line_bytes_without_n = &buf[..self.seen];

                    let line_bytes = match line_bytes_without_n.last() {
                        Some(b'\r') => &line_bytes_without_n[..self.seen - 1],
                        _ => line_bytes_without_n,
                    };

                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    {
                        let buf = Formatter(line_bytes);
                        let consuming = self.seen + 1;
                        tracing::debug!(frame=?buf, %consuming, "Framing");
                    }

                    let item = heapless::Vec::from_slice(line_bytes)
                        .map_err(|_| LineBytesCodecError::OutputBufferTooSmall)?;

                    let frame = Frame::new(self.seen + 1, item);

                    self.seen = 0;

                    return Ok(Some(frame));
                }

                self.seen += 1;
            }

            Ok(None)
        }
    }
};

#[cfg(all(test, feature = "futures"))]
mod test {
    extern crate std;

    use core::str::FromStr;
    use std::vec::Vec;

    use futures::StreamExt;

    use super::*;
    use crate::{decode::framed_read::FramedRead, test::init_tracing};

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
}
