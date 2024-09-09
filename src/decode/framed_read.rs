use pin_project_lite::pin_project;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<I, D> {
    /// The buffer is too small to read a frame.
    BufferTooSmall,
    /// An IO error occurred while reading from the underlying source.
    IO(I),
    /// Decoder consumed more bytes than available in the buffer.
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
            #[cfg(feature = "decoder-checks")]
            Self::BadDecoder => write!(f, "Bad decoder"),
            Self::Decode(err) => write!(f, "Decode error: {}", err),
            Self::IO(err) => write!(f, "IO error: {}", err),
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
    /// The underlying buffer to read into.
    buffer: &'a mut [u8],
}

impl<'a> ReadFrame<'a> {
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
}

pin_project! {
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct FramedRead<'a, D, R> {
        state: ReadFrame<'a>,
        codec: D,
        #[pin]
        inner: R,
    }
}

impl<'a, D, R> FramedRead<'a, D, R> {
    pub fn new(inner: R, codec: D, buffer: &'a mut [u8]) -> Self {
        Self {
            state: ReadFrame {
                index: 0,
                eof: false,
                is_framable: false,
                has_errored: false,
                total_consumed: 0,
                buffer,
            },
            codec,
            inner,
        }
    }

    pub const fn state(&self) -> &ReadFrame<'a> {
        &self.state
    }

    pub const fn codec(&self) -> &D {
        &self.codec
    }

    pub const fn inner(&self) -> &R {
        &self.inner
    }

    pub fn into_codec(self) -> D {
        self.codec
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

const _: () = {
    use core::{
        borrow::BorrowMut,
        pin::{pin, Pin},
        task::{ready, Context, Poll},
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

                        #[cfg(not(feature = "decode-enmpty-buffer"))]
                        if state.total_consumed == state.index {
                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            {
                                tracing::debug!("Buffer empty");
                                tracing::trace!("Setting unframable");
                            }

                            state.is_framable = false;

                            return Poll::Ready(None);
                        }

                        // pausing
                        match this
                            .codec
                            .decode_eof(&mut state.buffer[state.total_consumed..state.index])
                        {
                            // implicit pausing -> pausing or pausing -> paused
                            Ok(Some(Frame { size, item })) => {
                                state.total_consumed += size;

                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                tracing::debug!(consumed=%size, total_consumed=%state.total_consumed, "Frame decoded");

                                #[cfg(feature = "decoder-checks")]
                                if state.total_consumed > state.index || size == 0 {
                                    #[cfg(all(feature = "logging", feature = "tracing"))]
                                    {
                                        tracing::warn!(consumed=%size, index=%state.index, "Bad decoder");
                                        tracing::trace!("Setting error");
                                    }

                                    state.has_errored = true;

                                    return Poll::Ready(Some(Err(Error::BadDecoder)));
                                }

                                return Poll::Ready(Some(Ok(item)));
                            }
                            Ok(None) => {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::debug!("No frame decoded");
                                    tracing::trace!("Setting unframable");
                                }

                                // prepare pausing -> paused
                                state.is_framable = false;

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
                        .codec
                        .decode(&mut state.buffer[state.total_consumed..state.index])
                    {
                        Ok(None) => {
                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            {
                                tracing::debug!("No frame decoded");
                                tracing::trace!("Setting unframable");
                            }

                            if state.total_consumed > 0 {
                                state
                                    .buffer
                                    .copy_within(state.total_consumed..state.index, 0);
                                state.index -= state.total_consumed;
                                state.total_consumed = 0;

                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                tracing::debug!("Buffer shifted")
                            }

                            // framing -> reading
                            state.is_framable = false;
                        }
                        Ok(Some(Frame { size, item })) => {
                            state.total_consumed += size;

                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            tracing::debug!(consumed=%size, total_consumed=%state.total_consumed, "Frame decoded");

                            #[cfg(feature = "decoder-checks")]
                            if state.total_consumed > state.index || size == 0 {
                                #[cfg(all(feature = "logging", feature = "tracing"))]
                                {
                                    tracing::warn!(consumed=%size, index=%state.index, "Bad decoder");
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

                            // implicit framing -> framing
                            return Poll::Ready(Some(Ok(item)));
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
                match ready!(fut.poll(cx)) {
                    // Pending -> implicit reading -> reading or implicit paused -> paused
                    Err(err) => {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        {
                            tracing::warn!("Failed to read");
                            tracing::trace!("Setting error");
                        }

                        state.has_errored = true;

                        return Poll::Ready(Some(Err(Error::IO(err))));
                    }
                    Ok(0) => {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::debug!("Got EOF");

                        if state.eof {
                            #[cfg(all(feature = "logging", feature = "tracing"))]
                            tracing::debug!("Already at EOF");

                            // We're already at an EOF, and since we've reached this path
                            // we're also not readable. This implies that we've already finished
                            // our `decode_eof` handling, so we can simply return `None`.
                            // implicit paused -> paused
                            return Poll::Ready(None);
                        }

                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::trace!("Setting EOF");

                        // prepare reading -> paused
                        state.eof = true;
                    }
                    Ok(n) => {
                        state.index += n;

                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::debug!(bytes=%n, "Bytes read");

                        // prepare paused -> framing or noop reading -> framing
                        state.eof = false;
                    }
                }

                #[cfg(all(feature = "logging", feature = "tracing"))]
                tracing::trace!("Setting framable");

                // paused -> framing or reading -> framing or reading -> pausing
                state.is_framable = true;
            }
        }
    }
};

#[cfg(test)]
mod test {
    extern crate std;

    use std::vec::Vec;

    use futures::StreamExt;

    use crate::{
        decode::{decoder::Decoder, frame::Frame},
        test::init_tracing,
    };

    use super::*;

    struct DecoderReturningMoreSizeThanAvailable;

    impl Decoder for DecoderReturningMoreSizeThanAvailable {
        type Item = ();
        type Error = ();

        fn decode(&mut self, _: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
            Ok(Some(Frame::new(2, ())))
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
}
