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
