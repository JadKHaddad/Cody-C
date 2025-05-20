use core::error::Error;

use cody_c::{FramedRead, FramedWrite, ReadError, codec::lines::StrLinesCodec};
use embedded_io_adapters::tokio_1::FromTokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("reader=info,writer=info")
        .init();

    let (read, write) = tokio::io::duplex(1024);

    let read_buf = &mut [0u8; 1024];
    let mut framed_read = FramedRead::new(StrLinesCodec::new(), FromTokio::new(read), read_buf);

    let reader = async move {
        loop {
            match framed_read.read_frame().await {
                Ok(None) => {}
                Ok(Some(item)) => {
                    tracing::info!(target: "reader", item, "received frame");
                }
                Err(err) => match err {
                    ReadError::EOF => return Ok::<(), Box<dyn Error>>(()),
                    _ => return Err(err.into()),
                },
            }
        }
    };

    let write_buf = &mut [0u8; 1024];
    let mut framed_write = FramedWrite::new(StrLinesCodec::new(), FromTokio::new(write), write_buf);

    let writer = async move {
        let items = ["Hello, world!", "How are you?", "Goodbye!"];

        for item in items {
            tracing::info!(target: "writer", item, "sending frame");

            framed_write.send_frame(item).await?;
        }

        Ok::<(), Box<dyn Error>>(())
    };

    let (reader_result, writer_result) = tokio::join!(reader, writer);

    reader_result?;
    writer_result?;

    Ok(())
}
