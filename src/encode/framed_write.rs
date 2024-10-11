//! Framed write sink. Transforms an [`AsyncWrite`](crate::io::AsyncWrite) into a sink of frames.

use futures::Sink;

#[cfg(any(feature = "log", feature = "defmt", feature = "tracing"))]
use crate::logging::formatter::Formatter;

use crate::{debug, io::AsyncWrite, warn};

use super::encoder::Encoder;

/// An error that can occur while writing a frame.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<I, E> {
    /// An IO error occurred while writing to the underlying sink.
    IO(I),
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
    /// The underlying buffer to read into.
    buffer: &'a mut [u8],
}

impl<'a> WriteFrame<'a> {
    /// Creates a new [`WriteFrame`] with the given `buffer`.
    #[inline]
    pub(crate) fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer }
    }

    /// Returns a reference to the underlying buffer.
    #[inline]
    pub const fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }
}

/// A sink that writes frames to an underlying writable sink.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FramedWrite<'a, E, W> {
    state: WriteFrame<'a>,
    encoder: E,
    inner: W,
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

impl<'a, E, W> FramedWrite<'a, E, W>
where
    W: AsyncWrite,
{
    /// Converts the [`FramedWrite`] into a [`Sink`].
    pub fn sink<I>(&'a mut self) -> impl Sink<I, Error = Error<W::Error, E::Error>> + '_
    where
        I: 'a,
        E: Encoder<I>,
    {
        futures::sink::unfold(self, |this, item: I| async move {
            this.write_frame(item).await?;

            Ok::<_, Error<W::Error, E::Error>>(this)
        })
    }

    /// Converts the [`FramedWrite`] into a [`Sink`] consuming the [`FramedWrite`].
    pub fn into_sink<I>(self) -> impl Sink<I, Error = Error<W::Error, E::Error>> + 'a
    where
        I: 'a,
        E: Encoder<I> + 'a,
        W: 'a,
    {
        futures::sink::unfold(self, |mut this, item: I| async move {
            this.write_frame(item).await?;

            Ok::<_, Error<W::Error, E::Error>>(this)
        })
    }

    /// Writes a frame to the underlying sink.
    pub async fn write_frame<I>(&mut self, item: I) -> Result<(), Error<W::Error, E::Error>>
    where
        E: Encoder<I>,
    {
        match self.encoder.encode(item, self.state.buffer) {
            Ok(size) => match self.inner.write_all(&self.state.buffer[..size]).await {
                Ok(_) => {
                    debug!("Wrote. buffer: {:?}", Formatter(&self.state.buffer[..size]));

                    Ok(())
                }
                Err(err) => {
                    warn!("Failed to write frame");

                    Err(Error::IO(err))
                }
            },
            Err(err) => {
                warn!("Failed to encode frame");

                Err(Error::Encode(err))
            }
        }
    }
}
