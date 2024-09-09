use super::frame::Frame;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DecodeError {
    /// EOF was reached while decoding.
    BytesRemainingOnStream,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BytesRemainingOnStream => write!(f, "Bytes remaining on stream"),
        }
    }
}

impl From<DecodeError> for () {
    fn from(_: DecodeError) -> Self {}
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

pub trait Decoder {
    type Item;
    type Error: core::convert::From<DecodeError>;

    fn decode(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error>;

    fn decode_eof(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        match self.decode(src) {
            Ok(Some(frame)) => Ok(Some(frame)),
            Ok(None) => {
                if src.is_empty() {
                    return Ok(None);
                }

                Err(Self::Error::from(DecodeError::BytesRemainingOnStream))
            }
            Err(err) => Err(err),
        }
    }
}

// TODO: we need a way to make the decoder give the Framer a hint about the incoming frame size
// most protocols have a fixed size header that contains the size of the frame
// the decoder will tipically read the header and determine the size of the frame. Currently the decoder gets only a buffer for what has been read so far and has no idea about the size of the frame
// the decoder should be able to tell the framer how many bytes it needs to read to get the full frame, to avoid multiple reads

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MaybeDecoded<T> {
    /// A frame was decoded.
    Frame(Frame<T>),
    /// No frame was decoded, but more bytes are needed to decode a frame.
    ///
    /// - If `Some`, the value is the number of bytes still needed to decode a frame.
    /// - If `None`, unknown number of bytes still needed to decode a frame.
    ///
    /// It is recommended to return the number of bytes still needed to decode a frame, if known.
    /// This will allow the framer to decide early whether to read more bytes, shift the buffer, or return an error.
    /// Setting the value to `None` will force the framer to fill up the buffer before calling the decoder again.
    MoreBytesNeeded(Option<usize>),
}
