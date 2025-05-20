//! Framed write sink. Transforms an [`Write`] into a sink of frames.

use embedded_io_async::Write;
use futures::Sink;

#[cfg(any(feature = "log", feature = "defmt", feature = "tracing"))]
use crate::logging::Formatter;

use crate::{
    encode::Encoder,
    logging::{debug, error, trace},
};

/// An error that can occur while writing a frame.
#[non_exhaustive]
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WriteError<I, E> {
    /// An IO error occurred while writing to the underlying sink.
    IO(I),
    /// An error occurred while encoding a frame.
    Encode(E),
}

impl<I, E> core::fmt::Display for WriteError<I, E>
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

impl<I, E> core::error::Error for WriteError<I, E>
where
    I: core::fmt::Display + core::fmt::Debug,
    E: core::fmt::Display + core::fmt::Debug,
{
}

/// Internal state for writing a frame.
#[derive(Debug)]
struct WriteFrame<'buf> {
    /// The underlying buffer to write to.
    buffer: &'buf mut [u8],
}

impl<'buf> WriteFrame<'buf> {
    /// Creates a new [`WriteFrame`].
    #[inline]
    const fn new(buffer: &'buf mut [u8]) -> Self {
        Self { buffer }
    }
}

/// A sink that writes encoded frames into an underlying writable sink using an [`Encoder`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FramedWrite<'buf, E, W> {
    state: WriteFrame<'buf>,
    encoder: E,
    writer: W,
}

impl<'buf, E, W> FramedWrite<'buf, E, W> {
    /// Creates a new [`FramedWrite`] with the given `encoder` and `writer`.
    #[inline]
    pub fn new(buffer: &'buf mut [u8], encoder: E, writer: W) -> Self {
        Self {
            state: WriteFrame::new(buffer),
            encoder,
            writer,
        }
    }

    /// Returns reference to the encoder.
    #[inline]
    pub const fn encoder(&self) -> &E {
        &self.encoder
    }

    /// Returns mutable reference to the encoder.
    #[inline]
    pub fn encoder_mut(&mut self) -> &mut E {
        &mut self.encoder
    }

    /// Returns reference to the writer.
    #[inline]
    pub const fn writer(&self) -> &W {
        &self.writer
    }

    /// Returns mutable reference to the writer.
    #[inline]
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Consumes the [`FramedWrite`] and returns the `encoder` and `writer`.
    #[inline]
    pub fn into_parts(self) -> (E, W) {
        (self.encoder, self.writer)
    }

    /// Writes a frame to the underlying `writer` and flushes it.
    pub async fn send_frame<I>(&mut self, item: I) -> Result<(), WriteError<W::Error, E::Error>>
    where
        E: Encoder<I>,
        W: Write,
    {
        match self.encoder.encode(item, self.state.buffer) {
            Ok(size) => match self.writer.write_all(&self.state.buffer[..size]).await {
                Ok(_) => {
                    debug!("Wrote. buffer: {:?}", Formatter(&self.state.buffer[..size]));

                    match self.writer.flush().await {
                        Ok(_) => {
                            trace!("Flushed");

                            Ok(())
                        }
                        Err(err) => {
                            error!("Failed to flush");

                            Err(WriteError::IO(err))
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to write frame");

                    Err(WriteError::IO(err))
                }
            },
            Err(err) => {
                error!("Failed to encode frame");

                Err(WriteError::Encode(err))
            }
        }
    }

    /// Converts the [`FramedWrite`] into a sink.
    pub fn sink<'this, I>(
        &'this mut self,
    ) -> impl Sink<I, Error = WriteError<W::Error, E::Error>> + 'this
    where
        I: 'this,
        E: Encoder<I>,
        W: Write,
    {
        futures::sink::unfold(self, |this, item: I| async move {
            this.send_frame(item).await?;

            Ok::<_, WriteError<W::Error, E::Error>>(this)
        })
    }
}
