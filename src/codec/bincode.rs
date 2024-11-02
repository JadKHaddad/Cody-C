//! Bincode codec for encoding and decoding bincode bytes with a length prefix.

use core::marker::PhantomData;

use bincode::{
    error::{DecodeError, EncodeError},
    BorrowDecode, Decode, Encode,
};

use crate::{Decoder, DecoderOwned, Encoder, SIZE_OF_LENGTH};

use super::LengthCodec;

/// A codec that decodes a sequence of bincode bytes with a length prefix into a sequence of bytes and encodes a sequence of bytes into a sequence of bincode bytes with a length prefix.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BincodeCodec<D> {
    length_codec: LengthCodec,
    _de: PhantomData<D>,
}

impl<D> BincodeCodec<D> {
    /// Creates a new [`BincodeCodec`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            length_codec: LengthCodec::new(),
            _de: PhantomData,
        }
    }
}

/// An error that can occur when decoding a sequence of bincode bytes with a length prefix into a sequence of bytes.
#[derive(Debug)]
pub enum BincodeDecodeError {
    /// A Bincode error occurred.
    Decode(DecodeError),
}

#[cfg(feature = "defmt")]
impl defmt::Format for BincodeDecodeError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Decode(err) => f.error().field("Decode", err),
        }
    }
}

impl core::fmt::Display for BincodeDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(err) => write!(f, "Decode error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BincodeDecodeError {}

impl<'buf, D> Decoder<'buf> for BincodeCodec<D>
where
    D: BorrowDecode<'buf>,
{
    type Item = D;
    type Error = BincodeDecodeError;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        match self
            .length_codec
            .decode(src)
            .expect("<LengthCodec as Decoder>::Error must be infallible")
        {
            None => Ok(None),
            Some((bytes, size)) => {
                let (de, _) = bincode::borrow_decode_from_slice(bytes, bincode::config::standard())
                    .map_err(BincodeDecodeError::Decode)?;

                let item = (de, size);

                Ok(Some(item))
            }
        }
    }
}

/// An error that can occur when encoding a sequence of bytes into a sequence of bincode bytes with a length prefix.
#[derive(Debug)]
pub enum BincodeEncodeError {
    /// The input buffer is too small to fit the encoded line.
    BufferTooSmall,
    /// A Bincode error occurred.
    Encode(EncodeError),
    /// The payload size is greater than u32::MAX.
    PayloadTooLarge,
}

#[cfg(feature = "defmt")]
impl defmt::Format for BincodeEncodeError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::BufferTooSmall => f.error().field("Buffer too small", &true),
            Self::Decode(err) => f.error().field("Encode", err),
            Self::PayloadTooLarge => f.error().field("Payload too large", &true),
        }
    }
}

impl core::fmt::Display for BincodeEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::Encode(err) => write!(f, "Encode error: {}", err),
            Self::PayloadTooLarge => write!(f, "Payload too large"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BincodeEncodeError {}

impl<D> Encoder<D> for BincodeCodec<D>
where
    D: Encode,
{
    type Error = BincodeEncodeError;

    fn encode(&mut self, item: D, dst: &mut [u8]) -> Result<usize, Self::Error> {
        if dst.len() < SIZE_OF_LENGTH {
            return Err(BincodeEncodeError::BufferTooSmall);
        }

        let payload_size = bincode::encode_into_slice(
            item,
            &mut dst[SIZE_OF_LENGTH..],
            bincode::config::standard(),
        )
        .map_err(BincodeEncodeError::Encode)?;

        if payload_size > u32::MAX as usize {
            return Err(BincodeEncodeError::PayloadTooLarge);
        }

        dst[0..SIZE_OF_LENGTH].copy_from_slice(&(payload_size as u32).to_be_bytes());

        let size = payload_size + SIZE_OF_LENGTH;

        Ok(size)
    }
}

/// An owned [`BincodeCodec`].
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BincodeCodecOwned<D> {
    inner: BincodeCodec<D>,
}

impl<D> BincodeCodecOwned<D> {
    /// Creates a new [`BincodeCodecOwned`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: BincodeCodec::new(),
        }
    }
}

impl<D> From<BincodeCodec<D>> for BincodeCodecOwned<D> {
    fn from(inner: BincodeCodec<D>) -> Self {
        Self { inner }
    }
}

impl<D> DecoderOwned for BincodeCodecOwned<D>
where
    D: Decode,
{
    type Item = D;
    type Error = BincodeDecodeError;

    fn decode_owned(&mut self, src: &mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        match self
            .inner
            .length_codec
            .decode(src)
            .expect("<LengthCodec as Decoder>::Error must be infallible")
        {
            None => Ok(None),
            Some((bytes, size)) => {
                let (de, _) = bincode::decode_from_slice(bytes, bincode::config::standard())
                    .map_err(BincodeDecodeError::Decode)?;

                let item = (de, size);

                Ok(Some(item))
            }
        }
    }
}

impl<D> Encoder<D> for BincodeCodecOwned<D>
where
    D: Encode,
{
    type Error = BincodeEncodeError;

    fn encode(&mut self, item: D, dst: &mut [u8]) -> Result<usize, Self::Error> {
        Encoder::encode(&mut self.inner, item, dst)
    }
}
