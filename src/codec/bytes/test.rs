extern crate std;

use std::vec::Vec;

use futures::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;

use super::*;
use crate::{
    decode::framed_read::FramedRead, encode::framed_write::FramedWrite, test::init_tracing,
    tokio::Compat,
};

async fn from_slice<const I: usize, const O: usize>() {
    let read: &[u8] = b"jh asjdk hbjsjuwjal kadjjsadhjiuwqens nd yxxcjajsdiaskdn asjdasdiouqw essd";
    let codec = BytesCodec::<O>;
    let buf = &mut [0_u8; I];

    let framed_read = FramedRead::new(read, codec, buf);
    let byte_chunks: Vec<_> = framed_read.into_stream().collect().await;

    let bytes = byte_chunks
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(bytes, read);
}

async fn from_slow_reader<const I: usize, const O: usize>() {
    let chunks = std::vec![
        Vec::from(b"jh asjd"),
        Vec::from(b"k hbjsjuwjal kadjjsadhjiuw"),
        Vec::from(b"jal kadjjsadhjiuwqens "),
        Vec::from(b"nd "),
        Vec::from(b"yxxcjajsdiaskdn as"),
        Vec::from(b"jdasdiouqw es"),
        Vec::from(b"sd"),
    ];

    let chunks_copy = chunks.clone();

    let (read, mut write) = tokio::io::duplex(1);

    tokio::spawn(async move {
        for chunk in chunks {
            write.write_all(&chunk).await.unwrap();
        }
    });

    let read = Compat::new(read);

    let codec = BytesCodec::<O>;
    let buf = &mut [0_u8; I];

    let framed_read = FramedRead::new(read, codec, buf);
    let byte_chunks: Vec<_> = framed_read.into_stream().collect().await;

    let bytes = byte_chunks
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(bytes, chunks_copy.concat());
}

#[tokio::test]
async fn from_slice_tiny_buffers() {
    init_tracing();

    from_slice::<1, 1>().await;
}

#[tokio::test]
async fn from_slice_same_size() {
    init_tracing();

    from_slice::<5, 5>().await;
}

#[tokio::test]
async fn from_slice_input_larger() {
    init_tracing();

    from_slice::<5, 3>().await;
}

#[tokio::test]
async fn from_slice_output_larger() {
    init_tracing();

    from_slice::<3, 5>().await;
}

#[tokio::test]
async fn from_slow_reader_tiny_buffers() {
    init_tracing();

    from_slow_reader::<1, 1>().await;
}

#[tokio::test]
async fn from_slow_reader_same_size() {
    init_tracing();

    from_slow_reader::<5, 5>().await;
}

#[tokio::test]
async fn from_slow_reader_input_larger() {
    init_tracing();

    from_slow_reader::<5, 3>().await;
}

#[tokio::test]
async fn from_slow_reader_output_larger() {
    init_tracing();

    from_slow_reader::<3, 5>().await;
}

#[tokio::test]
async fn sink_stream() {
    const O: usize = 24;

    init_tracing();

    let chunks = std::vec![
        heapless::Vec::<_, O>::from_slice(b"jh asjd").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"k hb").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"jsjuwjal kadj").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"jsadhjiu").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"w").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"jal kadjjsadhjiuwqens ").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"nd yxxcjajsdi").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"askdn asjdasd").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"iouqw essd").unwrap(),
    ];

    let chunks_clone = chunks.clone();

    let (read, write) = tokio::io::duplex(24);

    let handle = tokio::spawn(async move {
        let write_buf = &mut [0_u8; 1024];
        let mut framed_write = FramedWrite::new(Compat::new(write), BytesCodec::<O>, write_buf);

        for item in chunks_clone {
            framed_write.send(item).await.unwrap();
        }

        framed_write.close().await.unwrap();
    });

    let read_buf = &mut [0_u8; 1024];
    let framed_read = FramedRead::new(Compat::new(read), BytesCodec::<O>, read_buf);

    let collected_bytes: Vec<_> = framed_read
        .into_stream()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>();

    let bytes: Vec<_> = chunks.into_iter().flatten().collect();

    handle.await.unwrap();

    assert_eq!(collected_bytes, bytes);
}
