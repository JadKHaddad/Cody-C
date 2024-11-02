//! Bincode codec for encoding and decoding bytes with a payload length prefix and bincode data structures.

use core::marker::PhantomData;

use bincode::{
    error::{DecodeError, EncodeError},
    BorrowDecode, Decode, Encode,
};

use crate::{Decoder, DecoderOwned, Encoder, SIZE_OF_LENGTH};

use super::LengthCodec;

/// A codec that decodes a sequence of bytes with a payload length prefix into a bincode data structure and encodes a bincode data structure into a sequence of bytes with a payload length prefix.
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

/// An error that can occur when decoding a sequence of bytes with a payload length prefix into a bincode data structure.
#[derive(Debug)]
pub enum BincodeDecodeError {
    /// A Bincode error occurred.
    Decode(DecodeError),
}

#[cfg(feature = "defmt")]
impl defmt::Format for BincodeDecodeError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Decode(_) => defmt::write!(f, "Decode error"),
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

/// An error that can occur when encoding a bincode data structure into a sequence of bytes with a payload length prefix.
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
            Self::BufferTooSmall => defmt::write!(f, "Buffer too small"),
            Self::Encode(_) => defmt::write!(f, "Encode error"),
            Self::PayloadTooLarge => defmt::write!(f, "Payload too large"),
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

        let payload_len = bincode::encode_into_slice(
            item,
            &mut dst[SIZE_OF_LENGTH..],
            bincode::config::standard(),
        )
        .map_err(BincodeEncodeError::Encode)?;

        if payload_len > u32::MAX as usize {
            return Err(BincodeEncodeError::PayloadTooLarge);
        }

        dst[0..SIZE_OF_LENGTH].copy_from_slice(&(payload_len as u32).to_be_bytes());

        let packet_len = payload_len + SIZE_OF_LENGTH;

        Ok(packet_len)
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

#[cfg(all(feature = "std", feature = "tokio"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "tokio"))))]
mod tokio {
    //! Tokio codec implementation for [`BincodeCodec`].

    use bincode::{
        error::{DecodeError, EncodeError},
        Decode, Encode,
    };
    use tokio_util::{
        bytes::{Buf, BufMut, BytesMut},
        codec::{Decoder, Encoder},
    };

    use crate::SIZE_OF_LENGTH;

    use super::BincodeCodec;

    /// An error that can occur when decoding a sequence of bytes with a payload length prefix into a bincode data structure.
    #[derive(Debug)]
    pub enum BincodeDecodeError {
        /// An IO error occurred.
        IO(std::io::Error),
        /// A Bincode error occurred.
        Decode(DecodeError),
    }

    impl From<std::io::Error> for BincodeDecodeError {
        fn from(err: std::io::Error) -> Self {
            Self::IO(err)
        }
    }

    impl core::fmt::Display for BincodeDecodeError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::IO(err) => write!(f, "IO error: {}", err),
                Self::Decode(err) => write!(f, "Decode error: {}", err),
            }
        }
    }

    impl std::error::Error for BincodeDecodeError {}

    impl<D> Decoder for BincodeCodec<D>
    where
        D: Decode,
    {
        type Item = D;
        type Error = BincodeDecodeError;

        fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
            if src.len() < SIZE_OF_LENGTH {
                return Ok(None);
            }

            let payload_len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

            let packet_len = payload_len + SIZE_OF_LENGTH;

            if src.len() < packet_len {
                src.reserve(packet_len - src.len());

                return Ok(None);
            }

            let (item, _) = bincode::decode_from_slice(
                &src[SIZE_OF_LENGTH..packet_len],
                bincode::config::standard(),
            )
            .map_err(BincodeDecodeError::Decode)?;

            src.advance(packet_len);

            Ok(Some(item))
        }
    }

    /// An error that can occur when encoding a bincode data structure into a sequence of bytes with a payload length prefix.
    #[derive(Debug)]
    pub enum BincodeEncodeError {
        /// An IO error occurred.
        IO(std::io::Error),
        /// A Bincode error occurred.
        Encode(EncodeError),
        /// The payload size is greater than u32::MAX.
        PayloadTooLarge,
    }

    impl From<std::io::Error> for BincodeEncodeError {
        fn from(err: std::io::Error) -> Self {
            Self::IO(err)
        }
    }

    impl core::fmt::Display for BincodeEncodeError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::IO(err) => write!(f, "IO error: {}", err),
                Self::Encode(err) => write!(f, "Encode error: {}", err),
                Self::PayloadTooLarge => write!(f, "Payload too large"),
            }
        }
    }

    impl std::error::Error for BincodeEncodeError {}

    impl<D> Encoder<D> for BincodeCodec<D>
    where
        D: Encode,
    {
        type Error = BincodeEncodeError;

        fn encode(&mut self, item: D, dst: &mut BytesMut) -> Result<(), Self::Error> {
            let start_len = dst.len();

            dst.put_u32(0);

            let payload_len = bincode::encode_into_std_write(
                item,
                &mut dst.writer(),
                bincode::config::standard(),
            )
            .map_err(BincodeEncodeError::Encode)?;

            if payload_len > u32::MAX as usize {
                return Err(BincodeEncodeError::PayloadTooLarge);
            }

            dst[start_len..start_len + SIZE_OF_LENGTH].copy_from_slice(&payload_len.to_be_bytes());

            Ok(())
        }
    }
}
