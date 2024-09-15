//! Prelude module for the decode module.

pub use super::{
    async_read::AsyncRead,
    decoder::Decoder,
    frame::Frame,
    framed_read::FramedRead,
    maybe_decoded::{FrameSize, MaybeDecoded},
};
