use super::frame::Frame;

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
