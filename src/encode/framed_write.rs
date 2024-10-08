//! Framed write sink. Transforms an [`AsyncWrite`](crate::io::AsyncWrite) into a sink of frames.

use core::{
    borrow::{Borrow, BorrowMut},
    future::Future,
    pin::{pin, Pin},
    task::{ready, Context, Poll},
};

use futures::Sink;

#[cfg(all(feature = "logging", feature = "tracing"))]
use crate::logging::formatter::Formatter;

use crate::io::AsyncWrite;

use super::encoder::Encoder;

use pin_project_lite::pin_project;

/// An error that can occur while writing a frame.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<I, E> {
    /// An IO error occurred while writing to the underlying sink.
    IO(I),
    /// Zero bytes were written to the underlying sink.
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

/// Internal state for writing a frame.
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
    /// The total number of bytes written to the underlying sink after the last flush.
    total_written: usize,
}

impl<'a> WriteFrame<'a> {
    /// Creates a new [`WriteFrame`] with the given `buffer`.
    #[inline]
    pub(crate) fn new(buffer: &'a mut [u8]) -> Self {
        let backpressure_boundary = buffer.len() / 4 * 3;

        Self {
            index: 0,
            backpressure_boundary,
            buffer,
            total_written: 0,
        }
    }

    /// Returns the current index in the buffer.
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Sets the backpressure boundary.
    #[inline]
    fn set_backpressure_boundary(&mut self, boundary: usize) {
        self.backpressure_boundary = boundary;
    }

    /// Returns the backpressure boundary.
    #[inline]
    pub const fn backpressure_boundary(&self) -> usize {
        self.backpressure_boundary
    }

    /// Returns a reference to the underlying buffer.
    #[inline]
    pub const fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }

    /// Returns the total number of bytes written to the underlying sink after the last flush.
    #[inline]
    pub const fn total_written(&self) -> usize {
        self.total_written
    }

    /// Returns the number of bytes available in the buffer.
    #[inline]
    pub const fn available(&self) -> usize {
        self.buffer.len() - self.index
    }
}

pin_project! {
    /// A sink that writes frames to an underlying writable sink.
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
    /// Creates a new [`FramedWrite`] with the given `encoder`, and `buffer`, and the underlying `inner` writer.
    #[inline]
    pub fn new(inner: W, encoder: E, buffer: &'a mut [u8]) -> Self {
        Self {
            state: WriteFrame::new(buffer),
            encoder,
            inner,
        }
    }

    /// Returns a reference to the internal state.
    #[inline]
    pub const fn state(&self) -> &WriteFrame<'a> {
        &self.state
    }

    /// Sets the backpressure boundary.
    #[inline]
    pub fn set_backpressure_boundary(&mut self, boundary: usize) {
        self.state.set_backpressure_boundary(boundary);
    }

    /// Returns a reference to the encoder.
    #[inline]
    pub const fn encoder(&self) -> &E {
        &self.encoder
    }

    /// Returns a reference to the underlying `inner` writer.
    #[inline]
    pub const fn inner(&self) -> &W {
        &self.inner
    }

    /// Returns the encoder consuming the [`FramedWrite`].
    #[inline]
    pub fn into_encoder(self) -> E {
        self.encoder
    }

    /// Returns the underlying `inner` writer consuming the [`FramedWrite`].
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<'a, E, W> FramedWrite<'a, E, W> {
    /// Asserts that the [`FramedWrite`] is a [`Sink`].
    ///
    /// Use this function to to make sure that the [`FramedWrite`] is a [`Sink`].
    pub fn assert_sink<I>(self)
    where
        Self: Sink<I>,
    {
    }
}

impl<'a, E, W, I> Sink<I> for FramedWrite<'a, E, W>
where
    E: Encoder<I>,
    W: AsyncWrite + Unpin,
{
    type Error = Error<W::Error, E::Error>;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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

        while state.total_written < state.index {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            {
                let buf = Formatter(&state.buffer[state.total_written..state.index]);
                tracing::debug!(total_written=%state.total_written, index=%state.index, ?buf, "Writing");
            }

            let fut = pin!(this
                .inner
                .write(&state.buffer[state.total_written..state.index]));

            match ready!(fut.poll(cx)) {
                Ok(0) => return Poll::Ready(Err(Error::WriteZero)),
                Ok(n) => {
                    state.total_written += n;

                    #[cfg(all(feature = "logging", feature = "tracing"))]
                    tracing::debug!(bytes=%n, total_written=%state.total_written, index=%state.index, "Wrote");
                }
                Err(err) => return Poll::Ready(Err(Error::IO(err))),
            }
        }

        state.total_written = 0;
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

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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

#[cfg(test)]
mod test;
