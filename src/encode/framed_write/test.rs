extern crate std;

use std::vec::Vec;

use crate::{
    encode::{
        encoder::Encoder, framed_write::Error as FramedWriteError, framed_write::FramedWrite,
    },
    test::init_tracing,
};

use futures::SinkExt;

struct EncodeOne;

impl Encoder<u8> for EncodeOne {
    type Error = ();

    fn encode(&mut self, item: u8, dst: &mut [u8]) -> Result<usize, Self::Error> {
        if dst.is_empty() {
            return Err(());
        }

        dst[0] = item;

        Ok(1)
    }
}

struct EncoderZero;

impl Encoder<u8> for EncoderZero {
    type Error = ();

    fn encode(&mut self, _item: u8, _dst: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

struct EncoderMoreThanAvailable;

impl Encoder<u8> for EncoderMoreThanAvailable {
    type Error = ();

    fn encode(&mut self, _item: u8, dst: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(dst.len() + 1)
    }
}

#[tokio::test]
#[cfg(feature = "encoder-checks")]
async fn zero_bad_encoder() {
    init_tracing();

    let mut write = [0_u8; 32];

    let codec = EncoderZero;
    let mut buf = [0_u8; 16];

    let mut framed_write = FramedWrite::new(&mut write[..], codec, &mut buf);

    let result = framed_write.send(10u8).await;

    assert!(matches!(result, Err(FramedWriteError::BadEncoder)));
}

#[tokio::test]
#[cfg(feature = "encoder-checks")]
async fn more_than_available_bad_encoder() {
    init_tracing();

    let mut write = [0_u8; 32];

    let codec = EncoderMoreThanAvailable;
    let mut buf = [0_u8; 16];

    let mut framed_write = FramedWrite::new(&mut write[..], codec, &mut buf);

    let result = framed_write.send(10u8).await;

    assert!(matches!(result, Err(FramedWriteError::BadEncoder)));
}

#[tokio::test]
#[cfg(not(feature = "encoder-checks"))]
async fn zero_nothing_written() {
    init_tracing();

    let items = [10u8; 32];

    let mut write = [0_u8; 32];

    let codec = EncoderZero;
    let mut buf = [0_u8; 16];

    let mut framed_write = FramedWrite::new(&mut write[..], codec, &mut buf);

    for item in items.iter() {
        framed_write.send(*item).await.unwrap();
    }

    assert!(write.iter().all(|&b| b == 0));
}

#[tokio::test]
#[should_panic]
#[cfg(not(feature = "encoder-checks"))]
async fn more_than_available_panic() {
    init_tracing();

    let items = [10u8; 32];

    let mut write = [0_u8; 32];

    let codec = EncoderMoreThanAvailable;
    let mut buf = [0_u8; 16];

    let mut framed_write = FramedWrite::new(&mut write[..], codec, &mut buf);

    for item in items.iter() {
        framed_write.send(*item).await.unwrap();
    }
}

#[tokio::test]
async fn one() {
    init_tracing();

    let items = [10u8; 32];

    let mut write = [0_u8; 32];

    let codec = EncodeOne;
    let mut buf = [0_u8; 16];

    let mut framed_write = FramedWrite::new(&mut write[..], codec, &mut buf);

    for item in items.iter() {
        framed_write.send(*item).await.unwrap();
    }

    assert!(write.iter().all(|&b| b == 10));
}

#[tokio::test]
async fn write_zero() {
    init_tracing();

    let items = [10u8; 5];

    let mut write = [0_u8; 4];

    let codec = EncodeOne;
    let mut buf = [0_u8; 16];

    let mut framed_write = FramedWrite::new(&mut write[..], codec, &mut buf);

    let mut results = Vec::new();

    for item in items.iter() {
        results.push(framed_write.send(*item).await);
    }

    assert!(matches!(
        results.last(),
        Some(Err(FramedWriteError::WriteZero))
    ));
}
