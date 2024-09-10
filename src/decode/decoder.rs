use super::frame::Frame;

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

pub trait Decoder {
    type Item;
    type Error;

    fn decode(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error>;

    fn decode_eof(&mut self, src: &mut [u8]) -> Result<Option<Frame<Self::Item>>, Self::Error> {
        self.decode(src)
    }
}

// TODO: we need a way to make the decoder give the Framer a hint about the incoming frame size
// most protocols have a fixed size header that contains the size of the frame
// the decoder will tipically read the header and determine the size of the frame. Currently the decoder gets only a buffer for what has been read so far and has no idea about the size of the frame
// the decoder should be able to tell the framer how many bytes it needs to read to get the full frame, to avoid multiple reads

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MaybeDecoded<T> {
    Frame(Frame<T>),
    None(FrameSize),
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FrameSize {
    Unknown,
    Known(usize),
}
