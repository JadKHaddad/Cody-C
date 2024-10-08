//! Decoder trait definition.

use super::maybe_decoded::MaybeDecoded;

/// A decoder that decodes a frame from a buffer.
///
/// - After returning a [`FrameSize::Unknown`](super::maybe_decoded::FrameSize::Unknown) from [`Decoder::decode`],
///   it is garanteed that the next call to [`Decoder::decode`]
///   will have a buffer with bigger size than the previous buffer.
///   [`Decoder::decode_eof`] maybe called with the same previous buffer size when `decode-enmpty-buffer` feature is enabled.
/// - After returning a [`FrameSize::Known`](super::maybe_decoded::FrameSize::Known) from [`Decoder::decode`],
///   it is garanteed that the next call to [`Decoder::decode`] or [`Decoder::decode_eof`]
///   will have a buffer of at least the size of the [`FrameSize::Known`](super::maybe_decoded::FrameSize::Known) returned.
pub trait Decoder {
    /// The type of item that this decoder decodes.
    type Item;
    /// The type of error that this decoder returns.
    type Error;

    /// Decodes a frame from the provided buffer.
    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error>;

    /// Decodes a frame from the provided buffer at the end of the stream.
    fn decode_eof(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        self.decode(src)
    }
}
