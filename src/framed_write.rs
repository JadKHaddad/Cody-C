//! Framed write sink. Transforms an [`AsyncWrite`] into a sink of frames.

use futures::Sink;

#[cfg(any(feature = "log", feature = "defmt", feature = "tracing"))]
use crate::logging::formatter::Formatter;

use crate::{debug, encode::Encoder, io::AsyncWrite, warn};

/// An error that can occur while writing a frame.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FramedWriteError<I, E> {
    /// An IO error occurred while writing to the underlying sink.
    IO(I),
    /// An error occurred while encoding a frame.
    Encode(E),
}

impl<I, E> core::fmt::Display for FramedWriteError<I, E>
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
impl<I, E> std::error::Error for FramedWriteError<I, E>
where
    I: core::fmt::Display + std::fmt::Debug,
    E: core::fmt::Display + std::fmt::Debug,
{
}

/// Internal state for writing a frame.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WriteFrame<const N: usize> {
    /// The underlying buffer to write to.
    buffer: [u8; N],
}

impl<const N: usize> Default for WriteFrame<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> WriteFrame<N> {
    /// Creates a new [`WriteFrame`].
    #[inline]
    pub const fn new() -> Self {
        Self { buffer: [0_u8; N] }
    }

    /// Creates a new [`WriteFrame`] with the given `buffer`.
    #[inline]
    pub const fn new_with_buffer(buffer: [u8; N]) -> Self {
        Self { buffer }
    }

    /// Returns a reference to the underlying buffer.
    #[inline]
    pub const fn buffer(&self) -> &[u8; N] {
        &self.buffer
    }

    /// Returns a mutable reference to the underlying buffer.
    #[inline]
    pub fn buffer_mut(&mut self) -> &mut [u8; N] {
        &mut self.buffer
    }
}

/// A sink that writes endoded frames into an underlying writable sink using an [`Encoder`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FramedWrite<const N: usize, E, W> {
    state: WriteFrame<N>,
    encoder: E,
    writer: W,
}

impl<const N: usize, E, W> FramedWrite<N, E, W> {
    /// Creates a new [`FramedWrite`] with the given `encoder` and `writer`.
    #[inline]
    pub fn new(encoder: E, writer: W) -> Self {
        Self {
            state: WriteFrame::new(),
            encoder,
            writer,
        }
    }

    /// Creates a new [`FramedWrite`] with the given `encoder`, `writer`, and `buffer`.
    #[inline]
    pub fn new_with_buffer(encoder: E, writer: W, buffer: [u8; N]) -> Self {
        Self {
            state: WriteFrame::new_with_buffer(buffer),
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

    /// Returns reference to the internal state.
    #[inline]
    pub const fn state(&self) -> &WriteFrame<N> {
        &self.state
    }

    /// Returns mutable reference to the internal state.
    #[inline]
    pub fn state_mut(&mut self) -> &mut WriteFrame<N> {
        &mut self.state
    }

    /// Consumes the [`FramedWrite`] and returns the `encoder`, `writer`, and `internal state`.
    #[inline]
    pub fn into_parts(self) -> (WriteFrame<N>, E, W) {
        (self.state, self.encoder, self.writer)
    }

    /// Creates a new [`FramedWrite`] from the given `encoder`, `writer`, and `internal state`.
    #[inline]
    pub fn from_parts(state: WriteFrame<N>, encoder: E, writer: W) -> Self {
        Self {
            state,
            encoder,
            writer,
        }
    }

    /// Writes a frame to the underlying `writer`.
    pub async fn write_frame<I>(
        &mut self,
        item: I,
    ) -> Result<(), FramedWriteError<W::Error, E::Error>>
    where
        E: Encoder<I>,
        W: AsyncWrite,
    {
        match self.encoder.encode(item, &mut self.state.buffer) {
            Ok(size) => match self.writer.write_all(&self.state.buffer[..size]).await {
                Ok(_) => {
                    debug!("Wrote. buffer: {:?}", Formatter(&self.state.buffer[..size]));

                    Ok(())
                }
                Err(err) => {
                    warn!("Failed to write frame");

                    Err(FramedWriteError::IO(err))
                }
            },
            Err(err) => {
                warn!("Failed to encode frame");

                Err(FramedWriteError::Encode(err))
            }
        }
    }

    /// Converts the [`FramedWrite`] into a sink.
    pub fn sink<'this, I>(
        &'this mut self,
    ) -> impl Sink<I, Error = FramedWriteError<W::Error, E::Error>> + 'this
    where
        I: 'this,
        E: Encoder<I>,
        W: AsyncWrite,
    {
        futures::sink::unfold(self, |this, item: I| async move {
            this.write_frame(item).await?;

            Ok::<_, FramedWriteError<W::Error, E::Error>>(this)
        })
    }
}
