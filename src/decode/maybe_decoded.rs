use super::frame::Frame;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MaybeDecoded<T> {
    Frame(Frame<T>),
    None(FrameSize),
}

/// Known or unknown frame size
///
/// - After returning a [`FrameSize::Unknown`] from [`Decoder::decode`](super::decoder::Decoder::decode),
///   it is garanteed that the next call to [`Decoder::decode`](super::decoder::Decoder::decode)
///   will have a buffer with bigger size than the previous buffer.
///   [`Decoder::decode_eof`](super::decoder::Decoder::decode_eof) maybe called with the same previous buffer size.
/// - After returning a [`FrameSize::Known`] from [`Decoder::decode`](super::decoder::Decoder::decode),
///   it is garanteed that the next call to [`Decoder::decode`](super::decoder::Decoder::decode) or [`Decoder::decode_eof`](super::decoder::Decoder::decode_eof)
///   will have a buffer of at least the size of the [`FrameSize::Known`] returned.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FrameSize {
    Unknown,
    Known(usize),
}
