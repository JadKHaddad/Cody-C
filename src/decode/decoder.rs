use super::frame::Frame;

#[derive(Debug)]
pub enum Error {
    /// EOF was reached while decoding.
    BytesRemainingOnStream,
}

pub trait Decoder {
    type Item;
    type Error: core::convert::From<Error>;

    fn decode(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error>;

    fn decode_eof(&mut self, buf: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        match self.decode(buf) {
            Ok(Some(frame)) => Ok(Some(frame)),
            Ok(None) => {
                if buf.is_empty() {
                    return Ok(None);
                }

                Err(Self::Error::from(Error::BytesRemainingOnStream))
            }
            Err(err) => Err(err),
        }
    }
}
