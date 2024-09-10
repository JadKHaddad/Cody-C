use super::maybe_decoded::MaybeDecoded;

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

pub trait Decoder {
    type Item;
    type Error;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error>;

    fn decode_eof(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        self.decode(src)
    }
}
