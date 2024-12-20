//! Framed read stream. Transforms an [`AsyncRead`] into a stream of frames.

use futures::Stream;

use crate::{
    debug,
    decode::{Decoder, DecoderOwned},
    error,
    io::AsyncRead,
    trace, warn,
};

#[cfg(any(feature = "log", feature = "defmt", feature = "tracing"))]
use crate::logging::formatter::Formatter;

/// An error that can occur while reading a frame from an [`AsyncRead`] source.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FramedReadError<I, D> {
    /// An IO error occurred while reading from the underlying source.
    IO(I),
    /// An error occurred while decoding a frame.
    Decode(D),
    /// The buffer is too small to read a frame.
    BufferTooSmall,
    /// There are bytes remaining on the stream after decoding.
    BytesRemainingOnStream,
    /// EOF was reached while decoding. The caller should stop reading.
    EOF,
}

impl<I, D> core::fmt::Display for FramedReadError<I, D>
where
    I: core::fmt::Display,
    D: core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::IO(err) => write!(f, "IO error: {}", err),
            Self::BytesRemainingOnStream => write!(f, "Bytes remaining on stream"),
            Self::Decode(err) => write!(f, "Decode error: {}", err),
            Self::EOF => write!(f, "EOF"),
        }
    }
}

#[cfg(feature = "std")]
impl<I, D> std::error::Error for FramedReadError<I, D>
where
    I: core::fmt::Display + std::fmt::Debug,
    D: core::fmt::Display + std::fmt::Debug,
{
}

/// Internal state for reading a frame.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ReadFrame<const N: usize> {
    /// The current index in the buffer.
    ///
    /// Represents the number of bytes read into the buffer.
    index: usize,
    /// EOF was reached while decoding.
    eof: bool,
    /// The buffer is currently framable.
    is_framable: bool,
    /// The buffer must be shifted before reading more bytes.
    ///
    /// Makes room for more bytes to be read into the buffer, keeping the already read bytes.
    shift: bool,
    /// Total number of bytes decoded in a framing round.
    total_consumed: usize,
    /// The underlying buffer to read into.
    buffer: [u8; N],
}

impl<const N: usize> Default for ReadFrame<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ReadFrame<N> {
    /// Creates a new [`ReadFrame`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            index: 0,
            eof: false,
            is_framable: false,
            shift: false,
            total_consumed: 0,
            buffer: [0_u8; N],
        }
    }

    /// Creates a new [`ReadFrame`] with the given `buffer`.
    #[inline]
    pub const fn new_with_buffer(buffer: [u8; N]) -> Self {
        Self {
            index: 0,
            eof: false,
            is_framable: false,
            shift: false,
            total_consumed: 0,
            buffer,
        }
    }

    /// Returns the current index in the buffer.
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns whether EOF was reached while decoding.
    #[inline]
    pub const fn eof(&self) -> bool {
        self.eof
    }

    /// Returns whether the buffer is currently framable.
    #[inline]
    pub const fn is_framable(&self) -> bool {
        self.is_framable
    }

    /// Returns the total number of bytes decoded in a framing round.
    #[inline]
    pub const fn total_consumed(&self) -> usize {
        self.total_consumed
    }

    /// Returns the number of bytes that can be framed.
    #[inline]
    pub const fn framable(&self) -> usize {
        self.index - self.total_consumed
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

/// A framer that reads frames from an [`AsyncRead`] source and decodes them using a [`Decoder`] or [`DecoderOwned`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FramedRead<const N: usize, D, R> {
    state: ReadFrame<N>,
    decoder: D,
    reader: R,
}

impl<const N: usize, D, R> FramedRead<N, D, R> {
    /// Creates a new [`FramedRead`] with the given `decoder` and `reader`.
    #[inline]
    pub fn new(decoder: D, reader: R) -> Self {
        Self {
            state: ReadFrame::new(),
            decoder,
            reader,
        }
    }

    /// Creates a new [`FramedRead`] with the given `decoder`, `reader`, and `buffer`.
    #[inline]
    pub fn new_with_buffer(decoder: D, reader: R, buffer: [u8; N]) -> Self {
        Self {
            state: ReadFrame::new_with_buffer(buffer),
            decoder,
            reader,
        }
    }

    /// Returns reference to the decoder.
    #[inline]
    pub const fn decoder(&self) -> &D {
        &self.decoder
    }

    /// Returns mutable reference to the decoder.
    #[inline]
    pub fn decoder_mut(&mut self) -> &mut D {
        &mut self.decoder
    }

    /// Returns reference to the reader.
    #[inline]
    pub const fn reader(&self) -> &R {
        &self.reader
    }

    /// Returns mutable reference to the reader.
    #[inline]
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Returns reference to the internal state.
    #[inline]
    pub const fn state(&self) -> &ReadFrame<N> {
        &self.state
    }

    /// Returns mutable reference to the internal state.
    #[inline]
    pub fn state_mut(&mut self) -> &mut ReadFrame<N> {
        &mut self.state
    }

    /// Consumes the [`FramedRead`] and returns the `decoder`, `reader`, and `internal state`.
    #[inline]
    pub fn into_parts(self) -> (ReadFrame<N>, D, R) {
        (self.state, self.decoder, self.reader)
    }

    /// Creates a new [`FramedRead`] from the given `decoder`, `reader`, and `internal state`.
    #[inline]
    pub fn from_parts(state: ReadFrame<N>, decoder: D, reader: R) -> Self {
        Self {
            state,
            decoder,
            reader,
        }
    }

    /// Tries to read a frame from the underlying reader.
    ///
    /// Returns:
    /// - `Ok(None)` if the buffer is not framable. Call `read_frame` again to read more bytes.
    /// - `Ok(Some(frame))` if a frame was successfully decoded. Call `read_frame` again to read more bytes.
    /// - `Err(error)` if an error occurred. The caller should stop reading.
    pub async fn read_frame<'this>(
        &'this mut self,
    ) -> Result<Option<D::Item>, FramedReadError<R::Error, D::Error>>
    where
        D: Decoder<'this>,
        R: AsyncRead,
    {
        debug!(
            "total_consumed: {}, index: {}, buffer: {:?}",
            self.state.total_consumed,
            self.state.index,
            Formatter(&self.state.buffer[self.state.total_consumed..self.state.index])
        );

        if self.state.shift {
            self.state
                .buffer
                .copy_within(self.state.total_consumed..self.state.index, 0);

            self.state.index -= self.state.total_consumed;
            self.state.total_consumed = 0;

            debug!("Buffer shifted. copied: {}", self.state.framable());

            self.state.shift = false;

            return Ok(None);
        }

        if self.state.is_framable {
            if self.state.eof {
                crate::trace!("Framing on EOF");

                match self
                    .decoder
                    .decode_eof(&mut self.state.buffer[self.state.total_consumed..self.state.index])
                {
                    Ok(Some((item, size))) => {
                        self.state.total_consumed += size;

                        debug!(
                            "Frame decoded, consumed: {}, total_consumed: {}",
                            size, self.state.total_consumed,
                        );

                        return Ok(Some(item));
                    }
                    Ok(None) => {
                        debug!("No frame decoded");

                        self.state.is_framable = false;

                        if self.state.index != self.state.total_consumed {
                            error!("Bytes remaining on stream");

                            return Err(FramedReadError::BytesRemainingOnStream);
                        }

                        return Err(FramedReadError::EOF);
                    }
                    Err(err) => {
                        error!("Failed to decode frame");

                        return Err(FramedReadError::Decode(err));
                    }
                };
            }

            trace!("Framing");

            #[cfg(not(feature = "buffer-early-shift"))]
            let buf_len = self.state.buffer.len();

            match self
                .decoder
                .decode(&mut self.state.buffer[self.state.total_consumed..self.state.index])
            {
                Ok(Some((item, size))) => {
                    self.state.total_consumed += size;

                    debug!(
                        "Frame decoded, consumed: {}, total_consumed: {}",
                        size, self.state.total_consumed,
                    );

                    return Ok(Some(item));
                }
                Ok(None) => {
                    debug!("No frame decoded");

                    #[cfg(feature = "buffer-early-shift")]
                    {
                        self.state.shift = self.state.total_consumed > 0;
                    }

                    #[cfg(not(feature = "buffer-early-shift"))]
                    {
                        self.state.shift = self.state.index >= buf_len;
                    }

                    self.state.is_framable = false;

                    return Ok(None);
                }
                Err(err) => {
                    error!("Failed to decode frame");

                    return Err(FramedReadError::Decode(err));
                }
            }
        }

        if self.state.index >= self.state.buffer.len() {
            error!("Buffer too small");

            return Err(FramedReadError::BufferTooSmall);
        }

        trace!("Reading");

        match self
            .reader
            .read(&mut self.state.buffer[self.state.index..])
            .await
        {
            Err(err) => {
                error!("Failed to read");

                Err(FramedReadError::IO(err))
            }
            Ok(0) => {
                warn!("Got EOF");

                self.state.eof = true;

                self.state.is_framable = true;

                Ok(None)
            }
            Ok(n) => {
                debug!("Bytes read. bytes: {}", n);

                self.state.index += n;

                self.state.is_framable = true;

                Ok(None)
            }
        }
    }

    /// Tries to read a frame from the underlying reader.
    ///
    /// Returns:
    /// - `Ok(frame)` if a frame was successfully decoded. Call `read_frame_owned` again to read more bytes.
    /// - `Err(error)` if an error occurred. The caller should stop reading.
    pub async fn read_frame_owned(&mut self) -> Result<D::Item, FramedReadError<R::Error, D::Error>>
    where
        D: DecoderOwned,
        R: AsyncRead,
    {
        loop {
            debug!(
                "total_consumed: {}, index: {}, buffer: {:?}",
                self.state.total_consumed,
                self.state.index,
                Formatter(&self.state.buffer[self.state.total_consumed..self.state.index])
            );

            if self.state.shift {
                self.state
                    .buffer
                    .copy_within(self.state.total_consumed..self.state.index, 0);

                self.state.index -= self.state.total_consumed;
                self.state.total_consumed = 0;

                debug!("Buffer shifted. copied: {}", self.state.framable());

                self.state.shift = false;

                continue;
            }

            if self.state.is_framable {
                if self.state.eof {
                    trace!("Framing on EOF");

                    match self.decoder.decode_eof_owned(
                        &mut self.state.buffer[self.state.total_consumed..self.state.index],
                    ) {
                        Ok(Some((item, size))) => {
                            self.state.total_consumed += size;

                            debug!(
                                "Frame decoded, consumed: {}, total_consumed: {}",
                                size, self.state.total_consumed,
                            );

                            return Ok(item);
                        }
                        Ok(None) => {
                            debug!("No frame decoded");

                            self.state.is_framable = false;

                            if self.state.index != self.state.total_consumed {
                                error!("Bytes remaining on stream");

                                return Err(FramedReadError::BytesRemainingOnStream);
                            }

                            return Err(FramedReadError::EOF);
                        }
                        Err(err) => {
                            error!("Failed to decode frame");

                            return Err(FramedReadError::Decode(err));
                        }
                    };
                }

                trace!("Framing");

                #[cfg(not(feature = "buffer-early-shift"))]
                let buf_len = self.state.buffer.len();

                match self.decoder.decode_owned(
                    &mut self.state.buffer[self.state.total_consumed..self.state.index],
                ) {
                    Ok(Some((item, size))) => {
                        self.state.total_consumed += size;

                        debug!(
                            "Frame decoded, consumed: {}, total_consumed: {}",
                            size, self.state.total_consumed,
                        );

                        return Ok(item);
                    }
                    Ok(None) => {
                        debug!("No frame decoded");
                        #[cfg(feature = "buffer-early-shift")]
                        {
                            self.state.shift = self.state.total_consumed > 0;
                        }

                        #[cfg(not(feature = "buffer-early-shift"))]
                        {
                            self.state.shift = self.state.index >= buf_len;
                        }

                        self.state.is_framable = false;

                        continue;
                    }
                    Err(err) => {
                        error!("Failed to decode frame");

                        return Err(FramedReadError::Decode(err));
                    }
                }
            }
            if self.state.index >= self.state.buffer.len() {
                error!("Buffer too small");

                return Err(FramedReadError::BufferTooSmall);
            }

            trace!("Reading");

            match self
                .reader
                .read(&mut self.state.buffer[self.state.index..])
                .await
            {
                Err(err) => {
                    error!("Failed to read");

                    return Err(FramedReadError::IO(err));
                }
                Ok(0) => {
                    warn!("Got EOF");

                    self.state.eof = true;

                    self.state.is_framable = true;

                    continue;
                }
                Ok(n) => {
                    debug!("Bytes read. bytes: {}", n);

                    self.state.index += n;

                    self.state.is_framable = true;

                    continue;
                }
            }
        }
    }

    /// Converts the [`FramedRead`] into a stream of frames.
    pub fn stream(
        &mut self,
    ) -> impl Stream<Item = Result<D::Item, FramedReadError<R::Error, D::Error>>> + '_
    where
        D: DecoderOwned,
        R: AsyncRead,
    {
        futures::stream::unfold((self, false), |(this, errored)| async move {
            if errored {
                return None;
            }

            match this.read_frame_owned().await {
                Ok(item) => Some((Ok(item), (this, false))),
                Err(err) => Some((Err(err), (this, true))),
            }
        })
    }
}
