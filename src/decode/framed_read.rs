use pin_project_lite::pin_project;

use crate::decode::maybe_decoded::{FrameSize, MaybeDecoded};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<I, D> {
    /// The buffer is too small to read a frame.
    BufferTooSmall,
    /// An IO error occurred while reading from the underlying source.
    IO(I),
    /// Bytes remaining on the stream after EOF.
    BytesRemainingOnStream,
    /// Decoder consumed zero or more bytes than available in the buffer or promissed a frame size and failed to decode it.
    #[cfg(feature = "decoder-checks")]
    BadDecoder,
    /// An error occurred while decoding a frame.
    Decode(D),
}

impl<I, D> core::fmt::Display for Error<I, D>
where
    I: core::fmt::Display,
    D: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::IO(err) => write!(f, "IO error: {}", err),
            Self::BytesRemainingOnStream => write!(f, "Bytes remaining on stream"),
            #[cfg(feature = "decoder-checks")]
            Self::BadDecoder => write!(f, "Bad decoder"),
            Self::Decode(err) => write!(f, "Decode error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl<I, D> std::error::Error for Error<I, D>
where
    I: std::error::Error,
    D: std::error::Error,
{
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ReadFrame<'a> {
    /// The current index in the buffer.
    ///
    /// Represents the number of bytes read into the buffer.
    index: usize,
    /// EOF was reached while decoding.
    eof: bool,
    /// The buffer is currently framable.
    is_framable: bool,
    /// An error occurred while decoding a frame.
    has_errored: bool,
    /// Total number of bytes decoded in a framing round.
    total_consumed: usize,
    frame_size: Option<usize>,
    /// The underlying buffer to read into.
    buffer: &'a mut [u8],
}

impl<'a> ReadFrame<'a> {
    pub(crate) fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            index: 0,
            eof: false,
            is_framable: false,
            has_errored: false,
            total_consumed: 0,
            frame_size: None,
            buffer,
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn eof(&self) -> bool {
        self.eof
    }

    pub const fn is_framable(&self) -> bool {
        self.is_framable
    }

    pub const fn has_errored(&self) -> bool {
        self.has_errored
    }

    pub const fn total_consumed(&self) -> usize {
        self.total_consumed
    }

    pub const fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }

    pub const fn framable(&self) -> usize {
        self.index - self.total_consumed
    }
}

pin_project! {
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct FramedRead<'a, D, R> {
        state: ReadFrame<'a>,
        decoder: D,
        #[pin]
        inner: R,
    }
}

impl<'a, D, R> FramedRead<'a, D, R> {
    pub fn new(inner: R, decoder: D, buffer: &'a mut [u8]) -> Self {
        Self {
            state: ReadFrame::new(buffer),
            decoder,
            inner,
        }
    }

    pub const fn state(&self) -> &ReadFrame<'a> {
        &self.state
    }

    pub const fn decoder(&self) -> &D {
        &self.decoder
    }

    pub const fn inner(&self) -> &R {
        &self.inner
    }

    pub fn into_decoder(self) -> D {
        self.decoder
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

const _: () = {
    use core::{
        borrow::BorrowMut,
        pin::{pin, Pin},
        task::{Context, Poll},
    };

    use futures::{Future, Stream};

    #[cfg(all(feature = "logging", feature = "tracing"))]
    use crate::logging::formatter::Formatter;

    use super::{async_read::AsyncRead, decoder::Decoder, frame::Frame};

    impl<'a, D, R> Stream for FramedRead<'a, D, R>
    where
        D: Decoder,
        R: AsyncRead + Unpin,
    {
        type Item = Result<D::Item, Error<R::Error, D::Error>>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();
            let state = this.state.borrow_mut();

            loop {
                #[cfg(all(feature = "logging", feature = "tracing"))]
                tracing::trace!("Entering loop");

                // Return `None` if we have encountered an error from the underlying decoder
                if state.has_errored {
                    // preparing has_errored -> paused

                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    tracing::trace!("Error already");

                    return Poll::Ready(None);
                }

                #[cfg(all(feature = "logging", feature = "tracing"))]
                {
                    let buf = Formatter(&state.buffer[state.total_consumed..state.index]);
                    tracing::debug!(total_consumed=%state.total_consumed, index=%state.index, ?buf);
                }

                // Repeatedly call `decode` or `decode_eof` while the buffer is "readable",
                // i.e. it _might_ contain data consumable as a frame or closing frame.
                // Both signal that there is no such data by returning `None`.
                //
                // If `decode` couldn't read a frame and the upstream source has returned eof,
                // `decode_eof` will attempt to decode the remaining bytes as closing frames.
                //
                // If the underlying AsyncRead is resumable, we may continue after an EOF,
                // but must finish emitting all of it's associated `decode_eof` frames.
                // Furthermore, we don't want to emit any `decode_eof` frames on retried
                // reads after an EOF unless we've actually read more data.
                if state.is_framable {
                    // pausing or framing
                    if state.eof {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::trace!("Framing on EOF");

                        // pausing
                        match this
                            .decoder
                            .decode_eof(&mut state.buffer[state.total_consumed..state.index])
                        {
                            // implicit pausing -> pausing or pausing -> paused
                            Ok(MaybeDecoded::Frame(Frame { size, item })) => {
                                state.total_consumed += size;

                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                tracing::debug!(consumed=%size, total_consumed=%state.total_consumed, "Frame decoded");

                                #[cfg(feature = "decoder-checks")]
                                if state.total_consumed > state.index || size == 0 {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    {
                                        if size == 0 {
                                            tracing::warn!(consumed=%size, "Bad decoder. Decoder consumed 0 bytes");
                                        }

                                        if state.total_consumed > state.index {
                                            let availalbe = state.index - state.total_consumed;
                                            tracing::warn!(consumed=%size, index=%state.index, %availalbe, "Bad decoder. Decoder consumed more bytes than available");
                                        }

                                        tracing::trace!("Setting error");
                                    }

                                    state.has_errored = true;

                                    return Poll::Ready(Some(Err(Error::BadDecoder)));
                                }

                                return Poll::Ready(Some(Ok(item)));
                            }
                            Ok(MaybeDecoded::None(_)) => {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::debug!("No frame decoded");
                                    tracing::trace!("Setting unframable");
                                }

                                // prepare pausing -> paused
                                state.is_framable = false;

                                if state.index != state.total_consumed {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    {
                                        tracing::warn!("Bytes remaining on stream");
                                        tracing::trace!("Setting error");
                                    }

                                    state.has_errored = true;

                                    return Poll::Ready(Some(Err(Error::BytesRemainingOnStream)));
                                }

                                return Poll::Ready(None);
                            }
                            Err(err) => {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::warn!("Failed to decode frame");
                                    tracing::trace!("Setting error");
                                }

                                state.has_errored = true;

                                return Poll::Ready(Some(Err(Error::Decode(err))));
                            }
                        }
                    }

                    // framing
                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    tracing::trace!("Framing");

                    match this
                        .decoder
                        .decode(&mut state.buffer[state.total_consumed..state.index])
                    {
                        Ok(MaybeDecoded::Frame(Frame { size, item })) => {
                            state.total_consumed += size;

                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            tracing::debug!(consumed=%size, total_consumed=%state.total_consumed, "Frame decoded");

                            #[cfg(feature = "decoder-checks")]
                            if state.total_consumed > state.index || size == 0 {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    if size == 0 {
                                        tracing::warn!(consumed=%size, "Bad decoder. Decoder consumed 0 bytes");
                                    }

                                    if state.total_consumed > state.index {
                                        let availalbe = state.framable();
                                        tracing::warn!(consumed=%size, index=%state.index, %availalbe, "Bad decoder. Decoder consumed more bytes than available");
                                    }

                                    tracing::trace!("Setting error");
                                }

                                state.has_errored = true;

                                return Poll::Ready(Some(Err(Error::BadDecoder)));
                            }

                            // Avoid framing an empty buffer
                            #[cfg(not(feature = "decode-enmpty-buffer"))]
                            if state.total_consumed == state.index {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::debug!("Resetting empty buffer");
                                    tracing::trace!("Setting unframable");
                                }

                                state.total_consumed = 0;
                                state.index = 0;

                                state.is_framable = false;
                            }

                            #[cfg(feature = "decoder-checks")]
                            {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                tracing::trace!("Unsetting frame size");

                                state.frame_size = None;
                            }

                            // implicit framing -> framing
                            return Poll::Ready(Some(Ok(item)));
                        }
                        Ok(MaybeDecoded::None(frame_size)) => {
                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            tracing::debug!("No frame decoded");

                            #[cfg(feature = "decoder-checks")]
                            if let Some(_frame_size) = state.frame_size {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::warn!(frame_size=%_frame_size, "Bad decoder. Decoder promissed to decode a slice of a known frame size in a previous iteration and failed to decode in this iteration");
                                    tracing::trace!("Setting error");
                                }

                                state.has_errored = true;

                                return Poll::Ready(Some(Err(Error::BadDecoder)));
                            }

                            match frame_size {
                                FrameSize::Unknown => {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    tracing::trace!("Unknown frame size");

                                    #[cfg(feature = "buffer-early-shift")]
                                    let shift = state.total_consumed > 0;

                                    #[cfg(not(feature = "buffer-early-shift"))]
                                    let shift = state.index >= state.buffer.len();

                                    if shift {
                                        state
                                            .buffer
                                            .copy_within(state.total_consumed..state.index, 0);
                                        state.index -= state.total_consumed;
                                        state.total_consumed = 0;

                                        #[cfg(all(feature = "logging", feature = "tracing"))]
                                        {
                                            let copied = state.framable();
                                            tracing::debug!(%copied, "Buffer shifted");
                                        }
                                    }
                                }
                                FrameSize::Known(frame_size) => {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    tracing::trace!(frame_size, "Known frame size");

                                    #[cfg(feature = "decoder-checks")]
                                    if frame_size == 0 {
                                        #[cfg(all(feature = "logging", feature = "tracing"))]
                                        {
                                            tracing::warn!(%frame_size, "Bad decoder. Decoder promissed a frame size of 0");
                                            tracing::trace!("Setting error");
                                        }

                                        state.has_errored = true;

                                        return Poll::Ready(Some(Err(Error::BadDecoder)));
                                    }

                                    if frame_size > state.buffer.len() {
                                        #[cfg(all(feature = "logging", feature = "tracing"))]
                                        {
                                            tracing::warn!(frame_size, buffer=%state.buffer.len(), "Frame size too large");
                                            tracing::trace!("Setting error");
                                        }

                                        state.has_errored = true;

                                        return Poll::Ready(Some(Err(Error::BufferTooSmall)));
                                    }

                                    // check if we need to shift the buffer. does the frame fit between the total_consumed and buffer.len()?
                                    if state.buffer.len() - state.total_consumed < frame_size {
                                        state
                                            .buffer
                                            .copy_within(state.total_consumed..state.index, 0);
                                        state.index -= state.total_consumed;
                                        state.total_consumed = 0;

                                        #[cfg(all(feature = "logging", feature = "tracing"))]
                                        {
                                            let copied = state.framable();
                                            tracing::debug!(%copied, "Buffer shifted");
                                        }
                                    }

                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    tracing::trace!("Setting frame size");

                                    state.frame_size = Some(frame_size);
                                }
                            }

                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            tracing::trace!("Setting unframable");

                            // framing -> reading
                            state.is_framable = false;
                        }
                        Err(err) => {
                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            {
                                tracing::warn!("Failed to decode frame");
                                tracing::trace!("Setting error");
                            }

                            state.has_errored = true;

                            return Poll::Ready(Some(Err(Error::Decode(err))));
                        }
                    }
                }

                if state.index >= state.buffer.len() {
                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    {
                        tracing::warn!("Buffer too small");
                        tracing::trace!("Setting error");
                    }

                    state.has_errored = true;

                    return Poll::Ready(Some(Err(Error::BufferTooSmall)));
                }

                #[cfg(all(feature = "logging", feature = "tracing"))]
                tracing::trace!("Reading");

                // reading or paused
                // If we can't build a frame yet, try to read more data and try again.

                let fut = pin!(this.inner.read(&mut state.buffer[state.index..]));
                match fut.poll(cx) {
                    // Pending -> implicit reading -> reading or implicit paused -> paused
                    Poll::Ready(Err(err)) => {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        {
                            tracing::warn!("Failed to read");
                            tracing::trace!("Setting error");
                        }

                        state.has_errored = true;

                        return Poll::Ready(Some(Err(Error::IO(err))));
                    }
                    Poll::Ready(Ok(0)) => {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::warn!("Got EOF");

                        // if polled again after EOF reached
                        if state.eof {
                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            tracing::warn!("Already EOF");

                            return Poll::Ready(None);
                        }

                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::trace!("Setting EOF");

                        state.eof = true;

                        match state.frame_size {
                            Some(_) => {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::warn!("Bytes remaining on stream");
                                    tracing::trace!("Setting error");
                                }

                                state.has_errored = true;

                                return Poll::Ready(Some(Err(Error::BytesRemainingOnStream)));
                            }
                            None => {
                                #[cfg(not(feature = "decode-enmpty-buffer"))]
                                if state.total_consumed == state.index {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    {
                                        tracing::debug!("Buffer empty");
                                    }

                                    return Poll::Ready(None);
                                }

                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                tracing::trace!("Setting framable");

                                // paused -> framing or reading -> framing or reading -> pausing
                                state.is_framable = true;
                            }
                        }
                    }
                    Poll::Ready(Ok(n)) => {
                        state.index += n;

                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        {
                            tracing::debug!(bytes=%n, "Bytes read");
                            tracing::trace!("Unsetting EOF");
                        }

                        state.eof = false;

                        match state.frame_size {
                            Some(frame_size) => {
                                let frame_size_reached =
                                    state.index - state.total_consumed >= frame_size;

                                if !frame_size_reached {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    tracing::trace!(frame_size, index=%state.index, "Frame size not reached");

                                    continue;
                                }

                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::trace!(frame_size, "Frame size reached");
                                    tracing::trace!("Setting framable");
                                }

                                state.is_framable = true;

                                #[cfg(not(feature = "decoder-checks"))]
                                {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    tracing::trace!("Unsetting frame size");

                                    state.frame_size = None;
                                }
                            }
                            None => {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                tracing::trace!("Setting framable");

                                state.is_framable = true;
                            }
                        }
                    }
                    Poll::Pending => {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::trace!("Pending");

                        return Poll::Pending;
                    }
                }
            }
        }
    }
};

#[cfg(test)]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    #[cfg(all(
        feature = "logging",
        any(feature = "log", feature = "defmt", feature = "tracing")
    ))]
    use crate::logging::formatter::Formatter;

    use crate::{
        decode::{decoder::Decoder, frame::Frame},
        test::init_tracing,
        tokio::Compat,
    };

    use super::*;

    struct DecoderReturningMoreSizeThanAvailable;

    impl Decoder for DecoderReturningMoreSizeThanAvailable {
        type Item = ();
        type Error = ();

        fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            Ok(MaybeDecoded::Frame(Frame::new(2, ())))
        }
    }

    #[cfg(feature = "decoder-checks")]
    struct DecoderReturningZeroSize;

    #[cfg(feature = "decoder-checks")]
    impl Decoder for DecoderReturningZeroSize {
        type Item = ();
        type Error = ();

        fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            Ok(MaybeDecoded::Frame(Frame::new(0, ())))
        }
    }

    #[tokio::test]
    #[should_panic]
    #[cfg(not(feature = "decoder-checks"))]
    async fn over_size_panic() {
        init_tracing();

        let read: &[u8] = b"111111111111111";
        let codec = DecoderReturningMoreSizeThanAvailable;
        let buf = &mut [0_u8; 4];

        let framed_read = FramedRead::new(read, codec, buf);
        framed_read.collect::<Vec<_>>().await;
    }

    #[tokio::test]
    #[cfg(feature = "decoder-checks")]
    async fn over_size_bad_decoder() {
        init_tracing();

        let read: &[u8] = b"111111111111111";
        let codec = DecoderReturningMoreSizeThanAvailable;
        let buf = &mut [0_u8; 4];

        let framed_read = FramedRead::new(read, codec, buf);
        let items: Vec<_> = framed_read.collect().await;

        let last_item = items.last().expect("No items");
        assert!(matches!(last_item, Err(Error::BadDecoder)));
    }

    #[tokio::test]
    #[cfg(feature = "decoder-checks")]
    /// Zero size without "decoder-checks" loop forever. Not tested.
    async fn zero_size_bad_decoder() {
        init_tracing();

        let read: &[u8] = b"111111111111111";
        let codec = DecoderReturningZeroSize;
        let buf = &mut [0_u8; 4];

        let framed_read = FramedRead::new(read, codec, buf);
        let items: Vec<_> = framed_read.collect().await;

        let last_item = items.last().expect("No items");
        assert!(matches!(last_item, Err(Error::BadDecoder)));
    }

    struct FrameSizeAwareDecoder;

    impl Decoder for FrameSizeAwareDecoder {
        type Item = ();
        type Error = ();

        fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                let src = Formatter(src);
                tracing::debug!(?src, "Decoding");
            }

            if src.len() < 4 {
                #[cfg(all(feature = "logging", feature = "tracing"))]
                tracing::debug!("Not enough bytes to read frame size");

                return Ok(MaybeDecoded::None(FrameSize::Unknown));
            }

            let size = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::debug!(size, "Frame size");

            if src.len() < size {
                #[cfg(all(feature = "logging", feature = "tracing"))]
                tracing::debug!("Not enough bytes to read frame");

                return Ok(MaybeDecoded::None(FrameSize::Known(size)));
            }

            Ok(MaybeDecoded::Frame(Frame::new(size, ())))
        }
    }

    struct DecoderAlwaysReturningKnownFrameSize;

    impl Decoder for DecoderAlwaysReturningKnownFrameSize {
        type Item = ();
        type Error = ();

        fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            Ok(MaybeDecoded::None(FrameSize::Known(4)))
        }
    }

    fn generate_chunks() -> (Vec<Vec<u8>>, usize) {
        let chunks = std::vec![
            Vec::from(b"\x00\x00\x00\x0f"),
            Vec::from(b"hello world"),
            Vec::from(b"\x00\x00\x00\x0f"),
            Vec::from(b"hello world\x00\x00\x00\x0f"),
            Vec::from(b"hello world"),
            Vec::from(b"\x00\x00"),
            Vec::from(b"\x00\x0fhello"),
            Vec::from(b" world"),
            Vec::from(b"\x00\x00\x00\x0fhello world\x00\x00\x00"),
            Vec::from(b"\x0f"),
            Vec::from(b"hell"),
            Vec::from(b"o wo"),
            Vec::from(b"rld"),
            Vec::from(b"\x00\x00\x00\x0f"),
            Vec::from(b"h"),
            Vec::from(b"e"),
            Vec::from(b"l"),
            Vec::from(b"l"),
            Vec::from(b"o"),
            Vec::from(b" "),
            Vec::from(b"w"),
            Vec::from(b"o"),
            Vec::from(b"r"),
            Vec::from(b"l"),
            Vec::from(b"d\x00\x00\x00\x0f"),
            Vec::from(b"hello world"),
            Vec::from(b"\x00\x00\x00\x0f"),
            Vec::from(b"hello world"),
        ];

        (chunks, 9)
    }

    fn generate_chunks_2() -> Vec<Vec<u8>> {
        let chunks = std::vec![
            Vec::from(b"a"),
            Vec::from(b"aa"),
            Vec::from(b"aaa"),
            Vec::from(b"aaaa"),
            Vec::from(b"aaaaa"),
            Vec::from(b"aaaaaa"),
            Vec::from(b"aaaaaaa"),
            Vec::from(b"aaaaaaaa"),
            Vec::from(b"aaaaaaaaa"),
            Vec::from(b"aaaaaaaaaa"),
            Vec::from(b"aaaaaaaaaaa"),
            Vec::from(b"aaaaaaaaaaaa"),
            Vec::from(b"a"),
            Vec::from(b"aa"),
            Vec::from(b"aaa"),
            Vec::from(b"aaaa"),
            Vec::from(b"aaaaa"),
            Vec::from(b"aaaaaa"),
            Vec::from(b"aaaaaaa"),
            Vec::from(b"aaaaaaaa"),
            Vec::from(b"aaaaaaaaa"),
            Vec::from(b"aaaaaaaaaa"),
            Vec::from(b"aaaaaaaaaaa"),
            Vec::from(b"aaaaaaaaaaaa"),
        ];

        chunks
    }

    async fn decode_with_latency<const I: usize, D: Decoder>(
        decoder: D,
        byte_chunks: Vec<Vec<u8>>,
    ) -> Vec<Result<<D as Decoder>::Item, Error<std::io::Error, <D as Decoder>::Error>>> {
        let (read, mut write) = tokio::io::duplex(1024);

        tokio::spawn(async move {
            for chunk in byte_chunks {
                write.write_all(&chunk).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        });

        let read = Compat::new(read);

        let buf = &mut [0_u8; I];

        let framed_read = FramedRead::new(read, decoder, buf);

        framed_read.collect().await
    }

    async fn decode_with_latency_with_frame_size_aware_decoder<const I: usize>(
        byte_chunks: Vec<Vec<u8>>,
    ) -> Vec<Result<(), Error<std::io::Error, ()>>> {
        let codec = FrameSizeAwareDecoder;

        decode_with_latency::<I, _>(codec, byte_chunks).await
    }

    async fn decode_with_latency_with_alawys_returns_known_size_decoder<const I: usize>(
        byte_chunks: Vec<Vec<u8>>,
    ) -> Vec<Result<(), Error<std::io::Error, ()>>> {
        let codec = DecoderAlwaysReturningKnownFrameSize;

        decode_with_latency::<I, _>(codec, byte_chunks).await
    }

    #[tokio::test]
    async fn decode_with_frame_size_aware_decoder_buffer_64() {
        init_tracing();

        let (chunks, decoded_len) = generate_chunks();

        let items = decode_with_latency_with_frame_size_aware_decoder::<64>(chunks).await;

        assert!(items.len() == decoded_len);
        assert!(items.iter().all(Result::is_ok));
    }

    #[tokio::test]
    async fn decode_with_frame_size_aware_decoder_buffer_32() {
        init_tracing();

        let (chunks, decoded_len) = generate_chunks();

        let items = decode_with_latency_with_frame_size_aware_decoder::<32>(chunks).await;

        assert!(items.len() == decoded_len);
        assert!(items.iter().all(Result::is_ok));
    }

    #[tokio::test]
    async fn decode_with_frame_size_aware_decoder_buffer_16() {
        init_tracing();

        let (chunks, decoded_len) = generate_chunks();

        let items = decode_with_latency_with_frame_size_aware_decoder::<16>(chunks).await;

        assert!(items.len() == decoded_len);
        assert!(items.iter().all(Result::is_ok));
    }

    #[tokio::test]
    async fn decode_with_frame_size_aware_decoder_buffer_8() {
        init_tracing();

        let (chunks, _) = generate_chunks();

        let items = decode_with_latency_with_frame_size_aware_decoder::<8>(chunks).await;

        assert!(items.len() == 1);
        assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
    }

    #[tokio::test]
    async fn decode_with_frame_size_aware_decoder_buffer_16_with_bytes_remaining_on_stream() {
        init_tracing();

        let (mut chunks, decoded_len) = generate_chunks();

        chunks.push(Vec::from(b"\x00\x00\x00\x0fhell"));

        let items = decode_with_latency_with_frame_size_aware_decoder::<16>(chunks).await;

        std::println!("{:?}", items);
        assert!(items.len() == decoded_len + 1);
        assert!(matches!(
            items.last(),
            Some(Err(Error::BytesRemainingOnStream))
        ));
    }

    #[tokio::test]
    async fn decode_with_frame_large_size() {
        init_tracing();

        let chunks = std::vec![Vec::from(b"\x00\x00\xff\x00"), std::vec![0; 16]];

        let items = decode_with_latency_with_frame_size_aware_decoder::<64>(chunks).await;

        assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
    }

    #[tokio::test]
    async fn decode_with_frame_size_aware_decoder_buffer_16_last_frame_large_size() {
        init_tracing();

        let (mut chunks, chunks_len) = generate_chunks();

        let bad_chunks = std::vec![Vec::from(b"\x00\x00\xff\x00"), std::vec![0; 16]];
        chunks.extend_from_slice(&bad_chunks);

        let items = decode_with_latency_with_frame_size_aware_decoder::<16>(chunks).await;

        assert!(items.len() == chunks_len + 1);
        assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
    }

    #[tokio::test]
    #[cfg(feature = "decoder-checks")]
    async fn decode_with_alawys_returns_known_size_decoder_bad_decoder() {
        init_tracing();

        let (chunks, _) = generate_chunks();

        let items = decode_with_latency_with_alawys_returns_known_size_decoder::<64>(chunks).await;

        assert!(items.len() == 1);
        assert!(matches!(items.last(), Some(Err(Error::BadDecoder))));
    }

    #[tokio::test]
    #[cfg(not(feature = "decoder-checks"))]
    /// The framer will keep reading from the stream until it can decode a frame.
    async fn decoder_always_returns_known_size_decoder_buffer_too_small() {
        init_tracing();

        let (chunks, _) = generate_chunks();

        let items = decode_with_latency_with_alawys_returns_known_size_decoder::<64>(chunks).await;

        assert!(items.len() == 1);
        assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
    }

    struct DecoderAlwaysReturnsUnknonwnFrameSize;

    impl Decoder for DecoderAlwaysReturnsUnknonwnFrameSize {
        type Item = ();
        type Error = ();

        fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            Ok(MaybeDecoded::None(FrameSize::Unknown))
        }
    }

    #[tokio::test]
    async fn bytes_remainning_on_stream() {
        init_tracing();

        let (chunks, _) = generate_chunks();
        let chunks = chunks.into_iter().take(8).collect::<Vec<_>>();

        let codec = DecoderAlwaysReturnsUnknonwnFrameSize;

        let items = decode_with_latency::<64, _>(codec, chunks).await;

        assert!(items.len() == 1);
        assert!(matches!(
            items.last(),
            Some(Err(Error::BytesRemainingOnStream))
        ));
    }

    #[tokio::test]
    async fn after_none_is_none() {
        init_tracing();

        let read: &[u8] = b"\x00\x00\x00\x0fhello world";

        let codec = FrameSizeAwareDecoder;
        let buf = &mut [0_u8; 46];

        let mut framed_read = FramedRead::new(read, codec, buf);

        while framed_read.next().await.is_some() {}

        let item = framed_read.next().await;

        assert!(item.is_none());
    }

    #[tokio::test]
    async fn bytes_remaining_on_stream_after_oef_reached_and_promissed_frame_size_is_set_and_after_error_is_none(
    ) {
        init_tracing();

        let read: &[u8] = b"\x00\x00\x00\x0fhello world\x00\x00\x00\x0f";

        let codec = FrameSizeAwareDecoder;
        let buf = &mut [0_u8; 64];

        let mut framed_read = FramedRead::new(read, codec, buf);

        let mut items = Vec::new();

        while let Some(item) = framed_read.next().await {
            items.push(item);
        }

        assert!(matches!(
            items.last(),
            Some(Err(Error::BytesRemainingOnStream))
        ));

        let item = framed_read.next().await;

        assert!(item.is_none());
    }

    struct ErrorCodec;

    impl Decoder for ErrorCodec {
        type Item = ();
        type Error = ();

        fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            Err(())
        }
    }

    #[tokio::test]
    async fn codec_error_and_after_error_is_none_with_unknown_frame_size() {
        init_tracing();

        let read: &[u8] = b"hello world\r\nhello worl";

        let codec = ErrorCodec;
        let buf = &mut [0_u8; 46];

        let mut framed_read = FramedRead::new(read, codec, buf);

        let mut items = Vec::new();

        while let Some(item) = framed_read.next().await {
            items.push(item);
        }

        assert!(matches!(items.last(), Some(Err(Error::Decode(_)))));

        let item = framed_read.next().await;

        assert!(item.is_none());
    }

    #[cfg(feature = "codec")]
    /// `codec` feauture is needed to bring [`LinesCodec`](crate::codec::lines::LinesCodec) into scope.
    mod codec {
        use super::*;
        use crate::codec::lines::LinesCodec;

        #[tokio::test]
        async fn bytes_remaining_on_stream_after_oef_reached_and_after_error_is_none_with_unknown_frame_size(
        ) {
            init_tracing();

            let read: &[u8] = b"hello world\r\nhello worl";

            let codec = LinesCodec::<16>::new();
            let buf = &mut [0_u8; 46];

            let mut framed_read = FramedRead::new(read, codec, buf);

            let mut items = Vec::new();

            while let Some(item) = framed_read.next().await {
                items.push(item);
            }

            assert!(matches!(
                items.last(),
                Some(Err(Error::BytesRemainingOnStream))
            ));

            let item = framed_read.next().await;

            assert!(item.is_none());
        }
    }

    struct DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
        previous_buffer_size: Option<usize>,
    }

    impl Decoder for DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
        type Item = ();
        type Error = ();

        fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            if let Some(previous_buffer_size) = self.previous_buffer_size {
                if src.len() < previous_buffer_size {
                    panic!("Buffer is not bigger than previous buffer");
                }
            }

            if src.len() >= 4 {
                self.previous_buffer_size = None;
                return Ok(MaybeDecoded::Frame(Frame::new(4, ())));
            }

            self.previous_buffer_size = Some(src.len());

            Ok(MaybeDecoded::None(FrameSize::Unknown))
        }

        fn decode_eof(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            self.previous_buffer_size = None;

            self.decode(src)
        }
    }

    #[tokio::test]
    async fn decode_with_decoder_checks_buffer_length_buffer_16() {
        init_tracing();

        let mut chunks = generate_chunks_2();

        let decoder = DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
            previous_buffer_size: None,
        };

        // will decode_of with empty buffer
        let _ = decode_with_latency::<16, _>(decoder, chunks.clone()).await;

        chunks.push(Vec::from(b"a"));
        let decoder = DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
            previous_buffer_size: None,
        };

        // will decode_of with buffer of size 1
        let _ = decode_with_latency::<16, _>(decoder, chunks).await;
    }

    struct DecoderChecksThatBufferIsBiggerOrEqualToGivenFrameSize {
        frame_size: Option<usize>,
    }

    impl Decoder for DecoderChecksThatBufferIsBiggerOrEqualToGivenFrameSize {
        type Item = ();
        type Error = ();

        fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
            if let Some(frame_size) = self.frame_size {
                if src.len() < frame_size {
                    panic!("Buffer is not bigger or equal to given frame size");
                }
            }

            if src.len() >= 4 {
                self.frame_size = None;
                return Ok(MaybeDecoded::Frame(Frame::new(4, ())));
            }

            self.frame_size = Some(4);

            Ok(MaybeDecoded::None(FrameSize::Known(4)))
        }
    }

    #[tokio::test]
    async fn decode_with_decoder_checks_frame_size_buffer_16() {
        init_tracing();

        let chunks = generate_chunks_2();

        let decoder = DecoderChecksThatBufferIsBiggerOrEqualToGivenFrameSize { frame_size: None };

        let _ = decode_with_latency::<16, _>(decoder, chunks).await;
    }
}
