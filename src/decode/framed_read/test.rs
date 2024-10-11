extern crate std;

use std::vec::Vec;

use futures::{pin_mut, StreamExt};
use tokio::io::AsyncWriteExt;

use crate::{
    decode::{decoder::Decoder, frame::Frame},
    test::init_tracing,
    tokio::Compat,
    FrameSize, MaybeDecoded,
};

use super::*;

struct DecoderReturningMoreSizeThanAvailable;

impl Decoder for DecoderReturningMoreSizeThanAvailable {
    type Item = ();
    type Error = ();

    fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        Ok(MaybeDecoded::Frame(Frame::new(2, ())))
    }
}

#[cfg(feature = "decoder-checks")]
struct DecoderReturningZeroSize;

#[cfg(feature = "decoder-checks")]
impl Decoder for DecoderReturningZeroSize {
    type Item = ();
    type Error = ();

    fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        Ok(MaybeDecoded::Frame(Frame::new(0, ())))
    }
}

#[tokio::test]
#[should_panic]
#[cfg(not(feature = "decoder-checks"))]
async fn over_size_panic() {
    init_tracing();

    let read: &[u8] = b"111111111111111";
    let codec = DecoderReturningMoreSizeThanAvailable;
    let buf = &mut [0_u8; 4];

    let framed_read = FramedRead::new(read, codec, buf).into_stream();
    framed_read.collect::<Vec<_>>().await;
}

#[tokio::test]
#[cfg(feature = "decoder-checks")]
async fn over_size_bad_decoder() {
    init_tracing();

    let read: &[u8] = b"111111111111111";
    let codec = DecoderReturningMoreSizeThanAvailable;
    let buf = &mut [0_u8; 4];

    let framed_read = FramedRead::new(read, codec, buf);
    let items: Vec<_> = framed_read.into_stream().collect().await;

    let last_item = items.last().expect("No items");
    assert!(matches!(last_item, Err(Error::BadDecoder)));
}

#[tokio::test]
#[cfg(feature = "decoder-checks")]
/// Zero size without "decoder-checks" loop forever. Not tested.
async fn zero_size_bad_decoder() {
    init_tracing();

    let read: &[u8] = b"111111111111111";
    let codec = DecoderReturningZeroSize;
    let buf = &mut [0_u8; 4];

    let framed_read = FramedRead::new(read, codec, buf);
    let items: Vec<_> = framed_read.into_stream().collect().await;

    let last_item = items.last().expect("No items");
    assert!(matches!(last_item, Err(Error::BadDecoder)));
}

struct FrameSizeAwareDecoder;

impl Decoder for FrameSizeAwareDecoder {
    type Item = ();
    type Error = ();

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(MaybeDecoded::None(FrameSize::Unknown));
        }

        let size = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if src.len() < size {
            return Ok(MaybeDecoded::None(FrameSize::Known(size)));
        }

        Ok(MaybeDecoded::Frame(Frame::new(size, ())))
    }
}

struct DecoderAlwaysReturningKnownFrameSize;

impl Decoder for DecoderAlwaysReturningKnownFrameSize {
    type Item = ();
    type Error = ();

    fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        Ok(MaybeDecoded::None(FrameSize::Known(4)))
    }
}

fn generate_chunks() -> (Vec<Vec<u8>>, usize) {
    let chunks = std::vec![
        Vec::from(b"\x00\x00\x00\x0f"),
        Vec::from(b"hello world"),
        Vec::from(b"\x00\x00\x00\x0f"),
        Vec::from(b"hello world\x00\x00\x00\x0f"),
        Vec::from(b"hello world"),
        Vec::from(b"\x00\x00"),
        Vec::from(b"\x00\x0fhello"),
        Vec::from(b" world"),
        Vec::from(b"\x00\x00\x00\x0fhello world\x00\x00\x00"),
        Vec::from(b"\x0f"),
        Vec::from(b"hell"),
        Vec::from(b"o wo"),
        Vec::from(b"rld"),
        Vec::from(b"\x00\x00\x00\x0f"),
        Vec::from(b"h"),
        Vec::from(b"e"),
        Vec::from(b"l"),
        Vec::from(b"l"),
        Vec::from(b"o"),
        Vec::from(b" "),
        Vec::from(b"w"),
        Vec::from(b"o"),
        Vec::from(b"r"),
        Vec::from(b"l"),
        Vec::from(b"d\x00\x00\x00\x0f"),
        Vec::from(b"hello world"),
        Vec::from(b"\x00\x00\x00\x0f"),
        Vec::from(b"hello world"),
    ];

    (chunks, 9)
}

fn generate_chunks_2() -> Vec<Vec<u8>> {
    let chunks = std::vec![
        Vec::from(b"a"),
        Vec::from(b"aa"),
        Vec::from(b"aaa"),
        Vec::from(b"aaaa"),
        Vec::from(b"aaaaa"),
        Vec::from(b"aaaaaa"),
        Vec::from(b"aaaaaaa"),
        Vec::from(b"aaaaaaaa"),
        Vec::from(b"aaaaaaaaa"),
        Vec::from(b"aaaaaaaaaa"),
        Vec::from(b"aaaaaaaaaaa"),
        Vec::from(b"aaaaaaaaaaaa"),
        Vec::from(b"a"),
        Vec::from(b"aa"),
        Vec::from(b"aaa"),
        Vec::from(b"aaaa"),
        Vec::from(b"aaaaa"),
        Vec::from(b"aaaaaa"),
        Vec::from(b"aaaaaaa"),
        Vec::from(b"aaaaaaaa"),
        Vec::from(b"aaaaaaaaa"),
        Vec::from(b"aaaaaaaaaa"),
        Vec::from(b"aaaaaaaaaaa"),
        Vec::from(b"aaaaaaaaaaaa"),
    ];

    chunks
}

async fn decode_with_pending<const I: usize, D: Decoder>(
    decoder: D,
    byte_chunks: Vec<Vec<u8>>,
) -> Vec<Result<<D as Decoder>::Item, Error<std::io::Error, <D as Decoder>::Error>>> {
    let (read, mut write) = tokio::io::duplex(1);

    tokio::spawn(async move {
        for chunk in byte_chunks {
            write.write_all(&chunk).await.unwrap();
        }
    });

    let read = Compat::new(read);

    let buf = &mut [0_u8; I];

    let framed_read = FramedRead::new(read, decoder, buf);

    framed_read.into_stream().collect().await
}

async fn decode_with_pending_with_frame_size_aware_decoder<const I: usize>(
    byte_chunks: Vec<Vec<u8>>,
) -> Vec<Result<(), Error<std::io::Error, ()>>> {
    let codec = FrameSizeAwareDecoder;

    decode_with_pending::<I, _>(codec, byte_chunks).await
}

async fn decode_with_pending_with_alawys_returns_known_size_decoder<const I: usize>(
    byte_chunks: Vec<Vec<u8>>,
) -> Vec<Result<(), Error<std::io::Error, ()>>> {
    let codec = DecoderAlwaysReturningKnownFrameSize;

    decode_with_pending::<I, _>(codec, byte_chunks).await
}

#[tokio::test]
async fn decode_with_frame_size_aware_decoder_buffer_64() {
    init_tracing();

    let (chunks, decoded_len) = generate_chunks();

    let items = decode_with_pending_with_frame_size_aware_decoder::<64>(chunks).await;

    assert!(items.len() == decoded_len);
    assert!(items.iter().all(Result::is_ok));
}

#[tokio::test]
async fn decode_with_frame_size_aware_decoder_buffer_32() {
    init_tracing();

    let (chunks, decoded_len) = generate_chunks();

    let items = decode_with_pending_with_frame_size_aware_decoder::<32>(chunks).await;

    assert!(items.len() == decoded_len);
    assert!(items.iter().all(Result::is_ok));
}

#[tokio::test]
async fn decode_with_frame_size_aware_decoder_buffer_16() {
    init_tracing();

    let (chunks, decoded_len) = generate_chunks();

    let items = decode_with_pending_with_frame_size_aware_decoder::<16>(chunks).await;

    assert!(items.len() == decoded_len);
    assert!(items.iter().all(Result::is_ok));
}

#[tokio::test]
async fn decode_with_frame_size_aware_decoder_buffer_8() {
    init_tracing();

    let (chunks, _) = generate_chunks();

    let items = decode_with_pending_with_frame_size_aware_decoder::<8>(chunks).await;

    assert!(items.len() == 1);
    assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
}

#[tokio::test]
async fn decode_with_frame_size_aware_decoder_buffer_16_with_bytes_remaining_on_stream() {
    init_tracing();

    let (mut chunks, decoded_len) = generate_chunks();

    chunks.push(Vec::from(b"\x00\x00\x00\x0fhell"));

    let items = decode_with_pending_with_frame_size_aware_decoder::<16>(chunks).await;

    std::println!("{:?}", items);
    assert!(items.len() == decoded_len + 1);
    assert!(matches!(
        items.last(),
        Some(Err(Error::BytesRemainingOnStream))
    ));
}

#[tokio::test]
async fn decode_with_frame_large_size() {
    init_tracing();

    let chunks = std::vec![Vec::from(b"\x00\x00\xff\x00"), std::vec![0; 16]];

    let items = decode_with_pending_with_frame_size_aware_decoder::<64>(chunks).await;

    assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
}

#[tokio::test]
async fn decode_with_frame_size_aware_decoder_buffer_16_last_frame_large_size() {
    init_tracing();

    let (mut chunks, chunks_len) = generate_chunks();

    let bad_chunks = std::vec![Vec::from(b"\x00\x00\xff\x00"), std::vec![0; 16]];
    chunks.extend_from_slice(&bad_chunks);

    let items = decode_with_pending_with_frame_size_aware_decoder::<16>(chunks).await;

    assert!(items.len() == chunks_len + 1);
    assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
}

#[tokio::test]
#[cfg(feature = "decoder-checks")]
async fn decode_with_alawys_returns_known_size_decoder_bad_decoder() {
    init_tracing();

    let (chunks, _) = generate_chunks();

    let items = decode_with_pending_with_alawys_returns_known_size_decoder::<64>(chunks).await;

    assert!(items.len() == 1);
    assert!(matches!(items.last(), Some(Err(Error::BadDecoder))));
}

#[tokio::test]
#[cfg(not(feature = "decoder-checks"))]
/// The framer will keep reading from the stream until it can decode a frame.
async fn decoder_always_returns_known_size_decoder_buffer_too_small() {
    init_tracing();

    let (chunks, _) = generate_chunks();

    let items = decode_with_pending_with_alawys_returns_known_size_decoder::<64>(chunks).await;

    assert!(items.len() == 1);
    assert!(matches!(items.last(), Some(Err(Error::BufferTooSmall))));
}

struct DecoderAlwaysReturnsUnknonwnFrameSize;

impl Decoder for DecoderAlwaysReturnsUnknonwnFrameSize {
    type Item = ();
    type Error = ();

    fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        Ok(MaybeDecoded::None(FrameSize::Unknown))
    }
}

#[tokio::test]
async fn bytes_remainning_on_stream() {
    init_tracing();

    let (chunks, _) = generate_chunks();
    let chunks = chunks.into_iter().take(8).collect::<Vec<_>>();

    let codec = DecoderAlwaysReturnsUnknonwnFrameSize;

    let items = decode_with_pending::<64, _>(codec, chunks).await;

    assert!(items.len() == 1);
    assert!(matches!(
        items.last(),
        Some(Err(Error::BytesRemainingOnStream))
    ));
}

#[tokio::test]
#[ignore = "Not anymore. Fuse it."]
async fn after_none_is_none() {
    init_tracing();

    let read: &[u8] = b"\x00\x00\x00\x0fhello world";

    let codec = FrameSizeAwareDecoder;
    let buf = &mut [0_u8; 46];

    let framed_read = FramedRead::new(read, codec, buf).into_stream();
    pin_mut!(framed_read);

    while framed_read.next().await.is_some() {}

    let item = framed_read.next().await;

    assert!(item.is_none());
}

#[tokio::test]
async fn bytes_remaining_on_stream_after_oef_reached_and_promissed_frame_size_is_set_and_after_error(
) {
    init_tracing();

    let read: &[u8] = b"\x00\x00\x00\x0fhello world\x00\x00\x00\x0f";

    let codec = FrameSizeAwareDecoder;
    let buf = &mut [0_u8; 64];

    let framed_read = FramedRead::new(read, codec, buf).into_stream();
    pin_mut!(framed_read);

    let mut items = Vec::new();

    while let Some(item) = framed_read.next().await {
        items.push(item);
    }

    assert!(matches!(
        items.last(),
        Some(Err(Error::BytesRemainingOnStream))
    ));
}

struct ErrorCodec;

impl Decoder for ErrorCodec {
    type Item = ();
    type Error = ();

    fn decode(&mut self, _: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        Err(())
    }
}

#[tokio::test]
async fn codec_error_with_unknown_frame_size() {
    init_tracing();

    let read: &[u8] = b"hello world\r\nhello worl";

    let codec = ErrorCodec;
    let buf = &mut [0_u8; 46];

    let framed_read = FramedRead::new(read, codec, buf).into_stream();
    pin_mut!(framed_read);

    let mut items = Vec::new();

    while let Some(item) = framed_read.next().await {
        items.push(item);
    }

    assert!(matches!(items.last(), Some(Err(Error::Decode(_)))));
}

#[cfg(feature = "codec")]
/// `codec` feauture is needed to bring [`LinesCodec`](crate::codec::lines::LinesCodec) into scope.
mod codec {
    use super::*;
    use crate::codec::lines::LinesCodec;

    #[tokio::test]
    async fn bytes_remaining_on_stream_after_oef_reached_with_unknown_frame_size() {
        init_tracing();

        let read: &[u8] = b"hello world\r\nhello worl";

        let codec = LinesCodec::<16>::new();
        let buf = &mut [0_u8; 46];

        let framed_read = FramedRead::new(read, codec, buf).into_stream();
        pin_mut!(framed_read);

        let mut items = Vec::new();

        while let Some(item) = framed_read.next().await {
            items.push(item);
        }

        assert!(matches!(
            items.last(),
            Some(Err(Error::BytesRemainingOnStream))
        ));
    }
}

struct DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
    previous_buffer_size: Option<usize>,
}

impl Decoder for DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
    type Item = ();
    type Error = ();

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        if let Some(previous_buffer_size) = self.previous_buffer_size {
            if src.len() < previous_buffer_size {
                panic!("Buffer is not bigger than previous buffer");
            }
        }

        if src.len() >= 4 {
            self.previous_buffer_size = None;
            return Ok(MaybeDecoded::Frame(Frame::new(4, ())));
        }

        self.previous_buffer_size = Some(src.len());

        Ok(MaybeDecoded::None(FrameSize::Unknown))
    }

    fn decode_eof(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        self.previous_buffer_size = None;

        self.decode(src)
    }
}

#[tokio::test]
async fn decode_with_decoder_checks_buffer_length_buffer_16() {
    init_tracing();

    let mut chunks = generate_chunks_2();

    let decoder = DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
        previous_buffer_size: None,
    };

    // will decode_of with empty buffer
    let _ = decode_with_pending::<16, _>(decoder, chunks.clone()).await;

    chunks.push(Vec::from(b"a"));
    let decoder = DecoderChecksThatBufferIsBiggerThanPreviousBuffer {
        previous_buffer_size: None,
    };

    // will decode_of with buffer of size 1
    let _ = decode_with_pending::<16, _>(decoder, chunks).await;
}

struct DecoderChecksThatBufferIsBiggerOrEqualToGivenFrameSize {
    frame_size: Option<usize>,
}

impl Decoder for DecoderChecksThatBufferIsBiggerOrEqualToGivenFrameSize {
    type Item = ();
    type Error = ();

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        if let Some(frame_size) = self.frame_size {
            if src.len() < frame_size {
                panic!("Buffer is not bigger or equal to given frame size");
            }
        }

        if src.len() >= 4 {
            self.frame_size = None;
            return Ok(MaybeDecoded::Frame(Frame::new(4, ())));
        }

        self.frame_size = Some(4);

        Ok(MaybeDecoded::None(FrameSize::Known(4)))
    }
}

#[tokio::test]
async fn decode_with_decoder_checks_frame_size_buffer_16() {
    init_tracing();

    let chunks = generate_chunks_2();

    let decoder = DecoderChecksThatBufferIsBiggerOrEqualToGivenFrameSize { frame_size: None };

    let _ = decode_with_pending::<16, _>(decoder, chunks).await;
}
