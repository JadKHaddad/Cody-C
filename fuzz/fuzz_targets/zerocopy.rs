//! If we panic!, we lose.
//!
//! ```not_rust
//! cargo +nightly fuzz run zerocopy
//! ```

#![no_main]

use std::{
    error::Error,
    fmt::{Debug, Display},
};

use cody_c::{
    codec::{
        any::AnyDelimiterCodec,
        bytes::BytesCodec,
        lines::{LinesCodec, StrLinesCodec},
    },
    decode::Decoder,
    encode::Encoder,
    FramedRead, FramedWrite, ReadError,
};
use embedded_io_adapters::tokio_1::FromTokio;
use libfuzzer_sys::fuzz_target;
use tokio::runtime::Runtime;

fuzz_target!(|data: &[u8]| {
    Runtime::new().expect("Runtime must build").block_on(async {
        fuzz(
            data,
            AnyDelimiterCodec::new(b"#"),
            AnyDelimiterCodec::new(b"#"),
            |data| (!data.contains(&b'#')).then_some(data).ok_or(()),
        )
        .await
        .unwrap();

        fuzz(data, BytesCodec::new(), BytesCodec::new(), Ok)
            .await
            .unwrap();

        fuzz(data, LinesCodec::new(), LinesCodec::new(), |data| {
            (!data.contains(&b'\n')).then_some(data).ok_or(())
        })
        .await
        .unwrap();

        fuzz(data, StrLinesCodec::new(), StrLinesCodec::new(), |data| {
            (!data.contains(&b'\n')).then_some(data).ok_or(())?;

            str::from_utf8(data).map_err(|_| ())
        })
        .await
        .unwrap();
    });
});

async fn fuzz<'data, D, E, F, T>(
    data: &'data [u8],
    encoder: E,
    decoder: D,
    map: F,
) -> Result<(), Box<dyn Error>>
where
    E: Encoder<T> + 'static,
    <E as Encoder<T>>::Error: Error + Display + 'static,
    D: for<'buf> Decoder<'buf> + 'static,
    for<'buf> <D as Decoder<'buf>>::Item: 'buf + Debug + PartialEq<T>,
    for<'buf> <D as Decoder<'buf>>::Error: Error + Display + 'static,
    F: FnOnce(&'data [u8]) -> Result<T, ()>,
    T: 'data + Clone + Debug + PartialEq,
{
    // If we cant create and item from the data, we dont have to bother
    let item = match map(data) {
        Ok(item) => item,
        Err(_) => return Ok(()),
    };

    let (read, write) = tokio::io::duplex(32);

    let item_clone = item.clone();
    let read_buf = &mut [0u8; 64];
    let mut framed_read = FramedRead::new(decoder, FromTokio::new(read), read_buf);

    let reader = async move {
        loop {
            match framed_read.read_frame().await {
                Ok(None) => {}
                Ok(Some(read_item)) => {
                    assert_eq!(read_item, item_clone);

                    return Ok(());
                }
                Err(err) => match err {
                    ReadError::EOF => return Ok::<(), Box<dyn Error>>(()),
                    _ => return Err(err.into()),
                },
            }
        }
    };

    let write_buf = &mut [0u8; 64];
    let mut framed_write = FramedWrite::new(encoder, FromTokio::new(write), write_buf);

    let writer = async move {
        framed_write.send_frame(item).await?;

        Ok::<(), Box<dyn Error>>(())
    };

    let (reader_result, writer_result) = tokio::join!(reader, writer);

    reader_result?;
    writer_result?;

    Ok(())
}

/*
TODO:

This fails because of the bytes codec and the duplex stream being 32 bytes long.
left: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
right: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]

TODO:
How do we want to handle the read/write buffer size?
*/
