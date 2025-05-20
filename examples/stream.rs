use core::error::Error;
use std::{pin::pin, str::FromStr};

use cody_c::{FramedRead, FramedWrite, codec::lines::StringLinesCodec};
use embedded_io_adapters::tokio_1::FromTokio;
use futures::{SinkExt, TryStreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("reader=info,writer=info")
        .init();

    let (read, write) = tokio::io::duplex(1024);

    let read_buf = &mut [0u8; 1024];
    let mut framed_read = FramedRead::new(
        StringLinesCodec::<32>::new(),
        FromTokio::new(read),
        read_buf,
    );

    let reader = async move {
        let _ = framed_read
            .stream()
            .inspect_ok(|item| {
                tracing::info!(target: "reader", %item, "received frame");
            })
            .try_collect::<Vec<heapless::String<32>>>()
            .await;

        Ok::<(), Box<dyn Error>>(())
    };

    let write_buf = &mut [0u8; 1024];
    let mut framed_write = FramedWrite::new(
        StringLinesCodec::<32>::new(),
        FromTokio::new(write),
        write_buf,
    );

    let writer = async move {
        let items = ["Hello, world!", "How are you?", "Goodbye!"]
            .into_iter()
            .map(heapless::String::<32>::from_str)
            .collect::<Result<Vec<_>, ()>>()
            .expect("Failed to create heapless strings");

        let mut sink = framed_write.sink();
        let mut sink = pin!(sink);

        for item in items {
            tracing::info!(target: "writer", %item, "sending frame");

            sink.send(item).await?;
        }

        Ok::<(), Box<dyn Error>>(())
    };

    let (reader_result, writer_result) = tokio::join!(reader, writer);

    reader_result?;
    writer_result?;

    Ok(())
}
