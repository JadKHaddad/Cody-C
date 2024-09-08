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

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

pub trait Decoder {
    type Item;
    type Error: core::convert::From<DecodeError>;

    fn decode(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error>;

    fn decode_eof(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        match self.decode(buf) {
            Ok(Some(frame)) => Ok(Some(frame)),
            Ok(None) => {
                if buf.is_empty() {
                    return Ok(None);
                }

                Err(Self::Error::from(DecodeError::BytesRemainingOnStream))
            }
            Err(err) => Err(err),
        }
    }
}
