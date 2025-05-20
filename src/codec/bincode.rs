//! Bincode codec for encoding and decoding bytes with a payload length prefix and bincode data structures.

use core::marker::PhantomData;

use bincode::{
    BorrowDecode, Decode, Encode,
    error::{DecodeError, EncodeError},
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
    /// The input buffer is too small to fit the encoded item.
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

impl<D> DecoderOwned for BincodeCodec<D>
where
    D: Decode,
{
    type Item = D;
    type Error = BincodeDecodeError;

    fn decode_owned(&mut self, src: &mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        match self
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

#[cfg(all(feature = "std", feature = "tokio"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "tokio"))))]
pub mod tokio_codec {
    //! Tokio codec implementation for [`BincodeCodec`].

    use bincode::{
        Decode, Encode,
        error::{DecodeError, EncodeError},
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

            dst[start_len..start_len + SIZE_OF_LENGTH]
                .copy_from_slice(&(payload_len as u32).to_be_bytes());

            Ok(())
        }
    }
}

#[cfg(all(test, feature = "std", feature = "tokio"))]
mod test {
    extern crate std;

    use core::str::FromStr;
    use std::vec::Vec;

    use bincode::serde::Compat as BincodeSerdeCompat;
    use futures::{SinkExt, StreamExt, pin_mut};
    use tokio_util::codec::{FramedRead as TokioFramedRead, FramedWrite as TokioFramedWrite};

    use crate::{FramedRead, FramedWrite, sink_stream, test::init_tracing, tokio::Compat};

    use super::*;

    #[derive(bincode::Encode, bincode::Decode)]
    pub enum BincodeMessage {
        Numbers(u32, u32, u32),
        String(BincodeSerdeCompat<heapless::String<32>>),
        Vec(BincodeSerdeCompat<heapless::Vec<u8, 32>>),
    }

    impl core::fmt::Debug for BincodeMessage {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Numbers(a, b, c) => write!(f, "Numbers({}, {}, {})", a, b, c),
                Self::String(s) => write!(f, "String({})", s.0),
                Self::Vec(v) => write!(f, "Vec({:?})", v.0),
            }
        }
    }

    impl Clone for BincodeMessage {
        fn clone(&self) -> Self {
            match self {
                Self::Numbers(a, b, c) => Self::Numbers(*a, *b, *c),
                Self::String(s) => Self::String(BincodeSerdeCompat(s.0.clone())),
                Self::Vec(v) => Self::Vec(BincodeSerdeCompat(v.0.clone())),
            }
        }
    }

    impl PartialEq for BincodeMessage {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Numbers(a, b, c), Self::Numbers(x, y, z)) => a == x && b == y && c == z,
                (Self::String(s), Self::String(t)) => s.0 == t.0,
                (Self::Vec(v), Self::Vec(w)) => v.0 == w.0,
                _ => false,
            }
        }
    }

    fn test_messages() -> Vec<BincodeMessage> {
        std::vec![
            BincodeMessage::Numbers(1, 2, 3),
            BincodeMessage::String(BincodeSerdeCompat(
                heapless::String::from_str("Hello").unwrap()
            )),
            BincodeMessage::Vec(BincodeSerdeCompat(
                heapless::Vec::from_slice(b"Hello, world!").unwrap()
            )),
        ]
    }

    #[tokio::test]
    async fn sink_stream() {
        init_tracing();

        let items = test_messages();

        let decoder = BincodeCodec::<BincodeMessage>::new();
        let encoder = BincodeCodec::<BincodeMessage>::new();

        sink_stream!(encoder, decoder, items);
    }

    macro_rules! collect_and_assert {
        ($read_1:ident, $read_2:ident, $read_3:ident) => {{
            let mut collected = Vec::<BincodeMessage>::new();
            let mut framer = FramedRead::new_with_buffer(
                BincodeCodec::<BincodeMessage>::new(),
                Compat::new($read_1),
                [0_u8; 1024],
            );

            loop {
                match framer.read_frame().await {
                    Ok(Some(item)) => {
                        collected.push(item);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        break;
                    }
                }
            }

            assert_eq!(test_messages(), collected);
        }
        {
            let mut framer = FramedRead::new_with_buffer(
                BincodeCodec::<BincodeMessage>::new(),
                Compat::new($read_2),
                [0_u8; 1024],
            );

            let stream = framer.stream();

            let collected = stream
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

            assert_eq!(test_messages(), collected);
        }
        {
            let framer = TokioFramedRead::new($read_3, BincodeCodec::<BincodeMessage>::new());

            let collected = framer
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

            assert_eq!(test_messages(), collected);
        }};
    }

    /// Test sending from a `FramedWrite` to a `FramedRead`, `FramedRead::stream` and `tokio_util::codec::FramedRead`.
    #[tokio::test]
    async fn crate_framed_write() {
        init_tracing();

        let (crate_framed_read_read, crate_framed_read_write) = tokio::io::duplex(8);
        let (crate_stream_read, crate_stream_write) = tokio::io::duplex(8);
        let (tokio_stream_read, tokio_stream_write) = tokio::io::duplex(8);

        for write in [
            crate_framed_read_write,
            crate_stream_write,
            tokio_stream_write,
        ] {
            tokio::spawn(async move {
                let mut writer = FramedWrite::new_with_buffer(
                    BincodeCodec::<BincodeMessage>::new(),
                    Compat::new(write),
                    [0_u8; 1024],
                );

                for item in test_messages() {
                    writer.send_frame(item).await.expect("Must send");
                }
            });
        }

        collect_and_assert!(crate_framed_read_read, crate_stream_read, tokio_stream_read);
    }

    /// Test sending from a `FramedWrite::sink` to a `FramedRead`, `FramedRead::stream` and `tokio_util::codec::FramedRead`.
    #[tokio::test]
    async fn crate_sink() {
        init_tracing();

        let (crate_framed_read_read, crate_framed_read_write) = tokio::io::duplex(8);
        let (crate_stream_read, crate_stream_write) = tokio::io::duplex(8);
        let (tokio_stream_read, tokio_stream_write) = tokio::io::duplex(8);

        for write in [
            crate_framed_read_write,
            crate_stream_write,
            tokio_stream_write,
        ] {
            tokio::spawn(async move {
                let mut writer = FramedWrite::new_with_buffer(
                    BincodeCodec::<BincodeMessage>::new(),
                    Compat::new(write),
                    [0_u8; 1024],
                );

                let sink = writer.sink();

                pin_mut!(sink);

                for item in test_messages() {
                    sink.send(item).await.expect("Must send");
                }
            });
        }

        collect_and_assert!(crate_framed_read_read, crate_stream_read, tokio_stream_read);
    }

    /// Test sending from a `tokio_util::codec::FramedWrite` to a `FramedRead`, `FramedRead::stream` and `tokio_util::codec::FramedRead`.
    #[tokio::test]
    async fn tokio_sink() {
        init_tracing();

        let (crate_framed_read_read, crate_framed_read_write) = tokio::io::duplex(8);
        let (crate_stream_read, crate_stream_write) = tokio::io::duplex(8);
        let (tokio_stream_read, tokio_stream_write) = tokio::io::duplex(8);

        for write in [
            crate_framed_read_write,
            crate_stream_write,
            tokio_stream_write,
        ] {
            tokio::spawn(async move {
                let mut sink = TokioFramedWrite::new(write, BincodeCodec::<BincodeMessage>::new());

                for item in test_messages() {
                    sink.send(item).await.expect("Must send");
                }
            });
        }

        collect_and_assert!(crate_framed_read_read, crate_stream_read, tokio_stream_read);
    }
}
