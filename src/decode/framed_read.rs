//! Framed read stream. Transforms an [`AsyncRead`](crate::io::AsyncRead) into a stream of frames.

use futures::Stream;

#[cfg(any(feature = "log", feature = "defmt", feature = "tracing"))]
use crate::logging::formatter::Formatter;

use crate::{debug, io::AsyncRead, trace, warn};

use super::{
    decoder::Decoder,
    frame::Frame,
    maybe_decoded::{FrameSize, MaybeDecoded},
};

/// An error that can occur while decoding a frame from an [`AsyncRead`](crate::io::AsyncRead) source.
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
/// Internal state for reading a frame.
pub struct ReadFrame<'a> {
    /// The current index in the buffer.
    ///
    /// Represents the number of bytes read into the buffer.
    index: usize,
    /// EOF was reached while decoding.
    eof: bool,
    /// The buffer is currently framable.
    is_framable: bool,
    /// An error occurred while decoding a frame or reading from the underlying source.
    has_errored: bool,
    /// Total number of bytes decoded in a framing round.
    total_consumed: usize,
    /// The size of the next frame to decode.
    frame_size: Option<usize>,
    /// The underlying buffer to read into.
    buffer: &'a mut [u8],
}

impl<'a> ReadFrame<'a> {
    /// Creates a new [`ReadFrame`] with the given `buffer`.
    #[inline]
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

    /// Returns whether an error occurred while decoding a frame or reading from the underlying source.
    #[inline]
    pub const fn has_errored(&self) -> bool {
        self.has_errored
    }

    /// Returns the total number of bytes decoded in a framing round.
    #[inline]
    pub const fn total_consumed(&self) -> usize {
        self.total_consumed
    }

    /// Returns the size of the next frame to decode.
    #[inline]
    pub const fn frame_size(&self) -> Option<usize> {
        self.frame_size
    }

    /// Returns a reference to the underlying buffer.
    #[inline]
    pub const fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }

    /// Returns the number of bytes that can be framed.
    #[inline]
    pub const fn framable(&self) -> usize {
        self.index - self.total_consumed
    }
}

/// A stream of frames decoded from an underlying readable source.
///
/// - [`Stream`](futures::Stream) of frames decoded from an [`AsyncRead`](crate::io::AsyncRead) source using [`Self::stream`] or [`Self::into_stream`].
/// - [`Iterator`](core::iter::Iterator) of frames decoded from a [`Read`](crate::decode::read::Read) source. (Not yet implemented)
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FramedRead<'a, D, R> {
    state: ReadFrame<'a>,
    decoder: D,
    inner: R,
}

impl<'a, D, R> FramedRead<'a, D, R> {
    /// Creates a new [`FramedRead`] with the given `decoder`, `buffer`, and underlying `inner` reader.
    #[inline]
    pub fn new(inner: R, decoder: D, buffer: &'a mut [u8]) -> Self {
        Self {
            state: ReadFrame::new(buffer),
            decoder,
            inner,
        }
    }

    /// Returns a reference to the internal state.
    #[inline]
    pub const fn state(&self) -> &ReadFrame<'a> {
        &self.state
    }

    /// Returns a reference to the decoder.
    #[inline]
    pub const fn decoder(&self) -> &D {
        &self.decoder
    }

    /// Returns a reference to the underlying `inner` reader.
    #[inline]
    pub const fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns the decoder consuming the [`FramedRead`].
    #[inline]
    pub fn into_decoder(self) -> D {
        self.decoder
    }

    /// Returns the underlying `inner` reader consuming the [`FramedRead`].
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

#[cfg(test)]
mod test;

impl<'a, D, R> FramedRead<'a, D, R>
where
    D: Decoder,
    R: AsyncRead,
{
    /// Converts the [`FramedRead`] into a [`Stream`](futures::Stream) of frames.
    pub fn stream(
        &'a mut self,
    ) -> impl Stream<Item = Result<D::Item, Error<R::Error, D::Error>>> + 'a {
        futures::stream::unfold(self, |this| async {
            if this.state.has_errored {
                trace!("Error already");

                return None;
            }

            match this.read_frame().await {
                Ok(Some(item)) => Some((Ok(item), this)),
                Ok(None) => None,
                Err(err) => {
                    this.state.has_errored = true;

                    Some((Err(err), this))
                }
            }
        })
    }

    /// Converts the [`FramedRead`] into a [`Stream`](futures::Stream) of frames consuming the [`FramedRead`].
    pub fn into_stream(self) -> impl Stream<Item = Result<D::Item, Error<R::Error, D::Error>>> + 'a
    where
        D: 'a,
        R: 'a,
    {
        futures::stream::unfold(self, |mut this| async {
            if this.state.has_errored {
                trace!("Error already");

                return None;
            }

            match this.read_frame().await {
                Ok(None) => None,
                Ok(Some(item)) => Some((Ok(item), this)),
                Err(err) => {
                    this.state.has_errored = true;

                    Some((Err(err), this))
                }
            }
        })
    }

    /// Reads the next frame from the underlying source.
    pub async fn read_frame(&mut self) -> Result<Option<D::Item>, Error<R::Error, D::Error>> {
        loop {
            trace!("Entering loop");
            debug!(
                "total_consumed: {}, index: {}, buffer: {:?}",
                self.state.total_consumed,
                self.state.index,
                Formatter(&self.state.buffer[self.state.total_consumed..self.state.index])
            );

            if self.state.is_framable {
                if self.state.eof {
                    crate::trace!("Framing on EOF");

                    match self.decoder.decode_eof(
                        &mut self.state.buffer[self.state.total_consumed..self.state.index],
                    ) {
                        Ok(MaybeDecoded::Frame(Frame { size, item })) => {
                            self.state.total_consumed += size;

                            debug!(
                                "Frame decoded, consumed: {}, total_consumed: {}",
                                size, self.state.total_consumed,
                            );

                            #[cfg(feature = "decoder-checks")]
                            if self.state.total_consumed > self.state.index || size == 0 {
                                #[cfg(any(
                                    feature = "log",
                                    feature = "defmt",
                                    feature = "tracing"
                                ))]
                                {
                                    if size == 0 {
                                        warn!("Bad decoder. Decoder consumed 0 bytes");
                                    }

                                    if self.state.total_consumed > self.state.index {
                                        let availalbe =
                                            self.state.index - self.state.total_consumed;

                                        warn!("Bad decoder. Decoder consumed more bytes than available. consumed: {}, index: {}, availalbe: {}", size, self.state.index, availalbe);
                                    }

                                    trace!("Setting error");
                                }

                                return Err(Error::BadDecoder);
                            }

                            return Ok(Some(item));
                        }
                        Ok(MaybeDecoded::None(_)) => {
                            debug!("No frame decoded");
                            trace!("Setting unframable");

                            self.state.is_framable = false;

                            if self.state.index != self.state.total_consumed {
                                warn!("Bytes remaining on stream");

                                return Err(Error::BytesRemainingOnStream);
                            }

                            return Ok(None);
                        }
                        Err(err) => {
                            warn!("Failed to decode frame");

                            return Err(Error::Decode(err));
                        }
                    }
                }

                trace!("Framing");

                match self
                    .decoder
                    .decode(&mut self.state.buffer[self.state.total_consumed..self.state.index])
                {
                    Ok(MaybeDecoded::Frame(Frame { size, item })) => {
                        self.state.total_consumed += size;

                        debug!(
                            "Frame decoded, consumed: {}, total_consumed: {}",
                            size, self.state.total_consumed,
                        );

                        #[cfg(feature = "decoder-checks")]
                        if self.state.total_consumed > self.state.index || size == 0 {
                            #[cfg(any(feature = "log", feature = "defmt", feature = "tracing"))]
                            {
                                if size == 0 {
                                    warn!("Bad decoder. Decoder consumed 0 bytes");
                                }

                                if self.state.total_consumed > self.state.index {
                                    let availalbe = self.state.index - self.state.total_consumed;

                                    warn!("Bad decoder. Decoder consumed more bytes than available. consumed: {}, index: {}, availalbe: {}", size, self.state.index, availalbe);
                                }
                            }

                            return Err(Error::BadDecoder);
                        }

                        // Avoid framing an empty buffer
                        #[cfg(not(feature = "decode-enmpty-buffer"))]
                        if self.state.total_consumed == self.state.index {
                            debug!("Resetting empty buffer");
                            trace!("Setting unframable");

                            self.state.total_consumed = 0;
                            self.state.index = 0;

                            self.state.is_framable = false;
                        }

                        #[cfg(feature = "decoder-checks")]
                        {
                            trace!("Unsetting frame size");

                            self.state.frame_size = None;
                        }

                        return Ok(Some(item));
                    }
                    Ok(MaybeDecoded::None(frame_size)) => {
                        debug!("No frame decoded");

                        #[cfg(feature = "decoder-checks")]
                        if let Some(_frame_size) = self.state.frame_size {
                            warn!("Bad decoder. Decoder promissed to decode a slice of a known frame size in a previous iteration and failed to decode in this iteration. frame_size: {}", _frame_size);

                            return Err(Error::BadDecoder);
                        }

                        match frame_size {
                            FrameSize::Unknown => {
                                trace!("Unknown frame size");

                                #[cfg(feature = "buffer-early-shift")]
                                let shift = self.state.total_consumed > 0;

                                #[cfg(not(feature = "buffer-early-shift"))]
                                let shift = self.state.index >= self.state.buffer.len();

                                if shift {
                                    self.state.buffer.copy_within(
                                        self.state.total_consumed..self.state.index,
                                        0,
                                    );
                                    self.state.index -= self.state.total_consumed;
                                    self.state.total_consumed = 0;

                                    debug!("Buffer shifted. copied: {}", self.state.framable());
                                }
                            }
                            FrameSize::Known(frame_size) => {
                                trace!("Known frame size. frame_size = {}", frame_size);

                                #[cfg(feature = "decoder-checks")]
                                if frame_size == 0 {
                                    warn!("Bad decoder. Decoder promissed a frame size of 0. frame_size: {}", frame_size);

                                    return Err(Error::BadDecoder);
                                }

                                if frame_size > self.state.buffer.len() {
                                    warn!(
                                        "Frame size too large. frame_size: {}, buffer: {}",
                                        frame_size,
                                        self.state.buffer.len()
                                    );

                                    return Err(Error::BufferTooSmall);
                                }

                                // Check if we need to shift the buffer. Does the frame fit between the total_consumed and buffer.len()?
                                if self.state.buffer.len() - self.state.total_consumed < frame_size
                                {
                                    self.state.buffer.copy_within(
                                        self.state.total_consumed..self.state.index,
                                        0,
                                    );
                                    self.state.index -= self.state.total_consumed;
                                    self.state.total_consumed = 0;

                                    debug!("Buffer shifted. copied: {}", self.state.framable());
                                }

                                trace!("Setting frame size");

                                self.state.frame_size = Some(frame_size);
                            }
                        }

                        trace!("Setting unframable");

                        self.state.is_framable = false;
                    }
                    Err(err) => {
                        warn!("Failed to decode frame");

                        return Err(Error::Decode(err));
                    }
                }
            }

            if self.state.index >= self.state.buffer.len() {
                warn!("Buffer too small");

                return Err(Error::BufferTooSmall);
            }

            trace!("Reading");

            match self
                .inner
                .read(&mut self.state.buffer[self.state.index..])
                .await
            {
                Err(err) => {
                    warn!("Failed to read");

                    return Err(Error::IO(err));
                }
                Ok(0) => {
                    warn!("Got EOF");

                    // If polled again after EOF reached
                    if self.state.eof {
                        warn!("Already EOF");

                        return Ok(None);
                    }

                    trace!("Setting EOF");

                    self.state.eof = true;

                    match self.state.frame_size {
                        Some(_) => {
                            warn!("Bytes remaining on stream");

                            return Err(Error::BytesRemainingOnStream);
                        }
                        None => {
                            // Avoid framing an empty buffer
                            #[cfg(not(feature = "decode-enmpty-buffer"))]
                            if self.state.total_consumed == self.state.index {
                                debug!("Buffer empty");

                                return Ok(None);
                            }

                            trace!("Setting framable");

                            self.state.is_framable = true;
                        }
                    }
                }
                Ok(n) => {
                    self.state.index += n;

                    debug!("Bytes read. bytes: {}", n);

                    match self.state.frame_size {
                        Some(frame_size) => {
                            let frame_size_reached =
                                self.state.index - self.state.total_consumed >= frame_size;

                            if !frame_size_reached {
                                trace!(
                                    "Frame size not reached. frame_size: {}, index: {}",
                                    frame_size,
                                    self.state.index
                                );

                                continue;
                            }

                            trace!("Frame size reached. frame_size: {}", frame_size);
                            trace!("Setting framable");

                            self.state.is_framable = true;

                            #[cfg(not(feature = "decoder-checks"))]
                            {
                                trace!("Unsetting frame size");

                                self.state.frame_size = None;
                            }
                        }
                        None => {
                            trace!("Setting framable");

                            self.state.is_framable = true;
                        }
                    }
                }
            }
        }
    }
}
