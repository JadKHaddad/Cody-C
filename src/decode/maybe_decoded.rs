use super::frame::Frame;

/// A frame that may or may not be decoded.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MaybeDecoded<T> {
    /// The frame is decoded.
    Frame(Frame<T>),
    /// The frame is not decoded.
    None(FrameSize),
}

/// Known or unknown frame size
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FrameSize {
    /// The frame size is unknown.
    Unknown,
    /// The frame size is known.
    Known(usize),
}
