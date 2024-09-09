use core::{borrow::Borrow, future::Future};

use pin_project_lite::pin_project;

use crate::encode::{async_write::AsyncWrite, encoder::Encoder};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<I, E> {
    /// The buffer is too small to read a frame.
    BufferTooSmall,
    /// An IO error occurred while reading from the underlying source.
    IO(I),
    /// Zero bytes were written to the underlying source.
    WriteZero,
    /// The encoder wrote zero or more bytes than available in the buffer.
    #[cfg(feature = "encoder-checks")]
    BadEncoder,
    /// An error occurred while decoding a frame.
    Encode(E),
}

impl<I, E> core::fmt::Display for Error<I, E>
where
    I: core::fmt::Display,
    E: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::IO(err) => write!(f, "IO error: {}", err),
            Self::WriteZero => write!(f, "Write zero"),
            #[cfg(feature = "encoder-checks")]
            Self::BadEncoder => write!(f, "Bad encoder"),
            Self::Encode(err) => write!(f, "Encode error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl<I, E> std::error::Error for Error<I, E>
where
    I: std::error::Error,
    E: std::error::Error,
{
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WriteFrame<'a> {
    /// The current index in the buffer.
    index: usize,
    /// The maximum amount of used bytes, the buffer can hold before flushing is required to report readiness.
    ///
    /// Defaults to `3/4` of the buffer size.
    backpressure_boundary: usize,
    /// The underlying buffer to read into.
    buffer: &'a mut [u8],
}

impl<'a> WriteFrame<'a> {
    pub(crate) fn new(buffer: &'a mut [u8]) -> Self {
        let backpressure_boundary = buffer.len() / 4 * 3;

        Self {
            index: 0,
            backpressure_boundary,
            buffer,
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    fn set_backpressure_boundary(&mut self, boundary: usize) {
        self.backpressure_boundary = boundary;
    }

    pub const fn backpressure_boundary(&self) -> usize {
        self.backpressure_boundary
    }

    pub const fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }

    pub const fn available(&self) -> usize {
        self.buffer.len() - self.index
    }
}

pin_project! {
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct FramedWrite<'a, E, W> {
        state: WriteFrame<'a>,
        encoder: E,
        #[pin]
        inner: W,
    }
}

impl<'a, E, W> FramedWrite<'a, E, W> {
    pub fn new(inner: W, encoder: E, buffer: &'a mut [u8]) -> Self {
        Self {
            state: WriteFrame::new(buffer),
            encoder,
            inner,
        }
    }

    pub const fn state(&self) -> &WriteFrame<'a> {
        &self.state
    }

    pub fn set_backpressure_boundary(&mut self, boundary: usize) {
        self.state.set_backpressure_boundary(boundary);
    }

    pub const fn encoder(&self) -> &E {
        &self.encoder
    }

    pub const fn inner(&self) -> &W {
        &self.inner
    }

    pub fn into_encoder(self) -> E {
        self.encoder
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

const _: () = {
    use core::{
        borrow::BorrowMut,
        pin::{pin, Pin},
        task::{ready, Context, Poll},
    };

    use futures::Sink;

    #[cfg(all(feature = "logging", feature = "tracing"))]
    use crate::logging::formatter::Formatter;

    impl<'a, E, W, I> Sink<I> for FramedWrite<'a, E, W>
    where
        E: Encoder<I>,
        W: AsyncWrite + Unpin,
    {
        type Error = Error<W::Error, E::Error>;

        fn poll_ready(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            let state = self.state.borrow();

            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                tracing::trace!("Poll ready");
                tracing::debug!(index=%state.index, available=%state.available(), boundary=%state.backpressure_boundary);
            }

            if state.index >= state.backpressure_boundary {
                #[cfg(all(feature = "logging", feature = "tracing"))]
                tracing::debug!("Backpressure");

                return self.as_mut().poll_flush(cx);
            }

            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::trace!("Start send");

            let this = self.project();
            let state = this.state.borrow_mut();

            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                let buf = Formatter(&state.buffer[0..state.index]);
                tracing::debug!(index=%state.index, available=%state.available(), ?buf);
            }

            match this.encoder.encode(item, &mut state.buffer[state.index..]) {
                Ok(size) => {
                    #[cfg(feature = "encoder-checks")]
                    if size == 0 || size > state.available() {
                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        {
                            tracing::warn!(size=%size, index=%state.index, available=%state.available(), "Bad encoder");
                        }

                        return Err(Error::BadEncoder);
                    }

                    state.index += size;

                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    {
                        let buf = Formatter(&state.buffer[0..state.index]);
                        tracing::debug!(size=%size, index=%state.index, ?buf, "Frame encoded");
                    }

                    Ok(())
                }
                Err(err) => {
                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    tracing::warn!("Failed to encode frame");

                    Err(Error::Encode(err))
                }
            }
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::trace!("Poll flush");

            let mut this = self.project();
            let state = this.state.borrow_mut();

            let mut total_written = 0;

            while total_written < state.index {
                #[cfg(all(feature = "logging", feature = "tracing"))]
                {
                    let buf = Formatter(&state.buffer[total_written..state.index]);
                    tracing::debug!(%total_written, index=%state.index, ?buf, "Writing");
                }

                let fut = pin!(this.inner.write(&state.buffer[total_written..state.index]));

                match ready!(fut.poll(cx)) {
                    Ok(0) => return Poll::Ready(Err(Error::WriteZero)),
                    Ok(n) => {
                        total_written += n;

                        #[cfg(all(feature = "logging", feature = "tracing"))]
                        tracing::debug!(bytes=%n, %total_written, index=%state.index, "Wrote");
                    }
                    Err(err) => return Poll::Ready(Err(Error::IO(err))),
                }
            }

            state.index = 0;

            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::trace!("Flushing");

            let fut = pin!(this.inner.flush());
            match ready!(fut.poll(cx)) {
                Ok(()) => Poll::Ready(Ok(())),
                Err(err) => {
                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    tracing::warn!("Failed to flush");

                    Poll::Ready(Err(Error::IO(err)))
                }
            }
        }

        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::trace!("Poll close");

            ready!(self.as_mut().poll_flush(cx))?;
            let mut this = self.project();

            let fut = pin!(this.inner.shutdown());
            match ready!(fut.poll(cx)) {
                Ok(()) => Poll::Ready(Ok(())),
                Err(err) => {
                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    tracing::warn!("Failed to close");

                    Poll::Ready(Err(Error::IO(err)))
                }
            }
        }
    }
};

// TODO: test errors on features
