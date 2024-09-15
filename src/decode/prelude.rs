//! Prelude module for the decode module.

pub use crate::io::AsyncRead;
pub use super::{
    decoder::Decoder,
    frame::Frame,
    framed_read::FramedRead,
    maybe_decoded::{FrameSize, MaybeDecoded},
};
