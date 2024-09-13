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

                if state.has_errored {
                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    tracing::trace!("Error already");

                    return Poll::Ready(None);
                }

                #[cfg(all(feature = "logging", feature = "tracing"))]
                {
                    let buf = Formatter(&state.buffer[state.total_consumed..state.index]);
                    tracing::debug!(total_consumed=%state.total_consumed, index=%state.index, ?buf);
                }

                if state.is_framable {
                    if state.eof {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::trace!("Framing on EOF");

                        match this
                            .decoder
                            .decode_eof(&mut state.buffer[state.total_consumed..state.index])
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

                                    // Check if we need to shift the buffer. Does the frame fit between the total_consumed and buffer.len()?
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

                let fut = pin!(this.inner.read(&mut state.buffer[state.index..]));
                match fut.poll(cx) {
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

                        // If polled again after EOF reached
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
                                // Avoid framing an empty buffer
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
mod test;
