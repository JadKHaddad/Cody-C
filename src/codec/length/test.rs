use crate::{
    codec::length::LengthDelimitedCodec, decode::framed_read::FramedRead,
    encode::framed_write::FramedWrite, test::init_tracing, tokio::Compat,
};

extern crate std;

use std::vec::Vec;

use futures::{SinkExt, StreamExt};

#[tokio::test]
async fn sink_stream() {
    init_tracing();

    let items = std::vec![
        heapless::Vec::<_, 256>::from_slice(b"").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"jh asjd").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"k hb").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"jsjuwjal kadj").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"jsadhjiu").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"w").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"jal kadjjsadhjiuwqens ").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"nd yxxcjajsdi").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"askdn asjdasd").unwrap(),
        heapless::Vec::<_, 256>::from_slice(b"iouqw essd").unwrap(),
    ];

    let items_clone = items.clone();

    let (read, write) = tokio::io::duplex(1024);

    let handle = tokio::spawn(async move {
        let write_buf = &mut [0_u8; 1024];
        let mut framed_write =
            FramedWrite::new(Compat::new(write), LengthDelimitedCodec::<256>, write_buf);

        for item in items_clone {
            framed_write.send(item).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        framed_write.close().await.unwrap();
    });

    let read_buf = &mut [0_u8; 1024];
    let framed_read = FramedRead::new(Compat::new(read), LengthDelimitedCodec::<256>, read_buf);

    let collected_items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    handle.await.unwrap();

    assert_eq!(collected_items, items);
}

#[tokio::test]
async fn sink_stream_small_duplex() {
    init_tracing();

    let items: Vec<_> = (0..45).map(|i| std::vec![i as u8; i]).collect();
    let items: Vec<_> = items
        .iter()
        .map(|v| heapless::Vec::<_, 1024>::from_slice(v).unwrap())
        .collect();

    let items_clone = items.clone();

    let (read, write) = tokio::io::duplex(1);

    let handle = tokio::spawn(async move {
        let write_buf = &mut [0_u8; 1024];
        let mut framed_write =
            FramedWrite::new(Compat::new(write), LengthDelimitedCodec::<1024>, write_buf);

        for item in items_clone {
            framed_write.send(item).await.unwrap();
        }

        framed_write.close().await.unwrap();
    });

    let read_buf = &mut [0_u8; 1024];
    let framed_read = FramedRead::new(Compat::new(read), LengthDelimitedCodec::<256>, read_buf);

    let collected_items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    handle.await.unwrap();

    assert_eq!(collected_items, items);
}
