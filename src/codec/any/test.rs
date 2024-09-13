extern crate std;

use std::vec::Vec;

use futures::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;

use super::*;
use crate::{
    decode::framed_read::FramedRead, encode::framed_write::FramedWrite, test::init_tracing,
    tokio::Compat,
};

async fn one_from_slice<const I: usize, const O: usize>() {
    let read: &[u8] = b"1##";
    let result = std::vec![heapless::Vec::<_, O>::from_slice(b"1").unwrap(),];

    let codec = AnyDelimiterCodec::<O>::new(b"##");
    let buf = &mut [0_u8; I];

    let framed_read = FramedRead::new(read, codec, buf);
    let items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(items, result);
}

async fn three_from_slice<const I: usize, const O: usize>() {
    let read: &[u8] = b"1##2##3##";
    let result = std::vec![
        heapless::Vec::<_, O>::from_slice(b"1").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"2").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"3").unwrap(),
    ];

    let codec = AnyDelimiterCodec::<O>::new(b"##");
    let buf = &mut [0_u8; I];

    let framed_read = FramedRead::new(read, codec, buf);
    let items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(items, result);
}

async fn from_slow_reader<const I: usize, const O: usize>() {
    let chunks = std::vec![
        Vec::from(b"jh asjd##"),
        Vec::from(b"k hb##jsjuwjal kadj##jsadhjiu##w"),
        Vec::from(b"##jal kadjjsadhjiuwqens ##"),
        Vec::from(b"nd "),
        Vec::from(b"yxxcjajsdi##askdn as"),
        Vec::from(b"jdasd##iouqw es"),
        Vec::from(b"sd##"),
    ];

    let result = std::vec![
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

    let (read, mut write) = tokio::io::duplex(1024);

    tokio::spawn(async move {
        for chunk in chunks {
            write.write_all(&chunk).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    let read = Compat::new(read);

    let codec = AnyDelimiterCodec::<O>::new(b"##");
    let buf = &mut [0_u8; I];

    let framed_read = FramedRead::new(read, codec, buf);
    let items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(items, result);
}

#[tokio::test]
async fn one_item_one_stroke() {
    init_tracing();

    one_from_slice::<5, 3>().await;
}

#[tokio::test]
async fn three_items_one_stroke() {
    init_tracing();

    three_from_slice::<9, 5>().await;
}

#[tokio::test]
async fn three_items_many_strokes() {
    init_tracing();

    // Input buffer will refill 3 times.
    three_from_slice::<3, 5>().await;
}

#[tokio::test]
async fn from_slow_reader_small_buffer() {
    init_tracing();

    from_slow_reader::<32, 24>().await;
}

#[tokio::test]
async fn from_slow_reader_large_buffer() {
    init_tracing();

    from_slow_reader::<1024, 24>().await;
}

#[tokio::test]
async fn sink_stream() {
    const O: usize = 24;

    init_tracing();

    let items = std::vec![
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

    let items_clone = items.clone();

    let (read, write) = tokio::io::duplex(1024);

    let handle = tokio::spawn(async move {
        let write_buf = &mut [0_u8; 1024];
        let mut framed_write = FramedWrite::new(
            Compat::new(write),
            AnyDelimiterCodec::<O>::new(b"##"),
            write_buf,
        );

        for item in items_clone {
            framed_write.send(item).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        framed_write.close().await.unwrap();
    });

    let read_buf = &mut [0_u8; 1024];
    let framed_read = FramedRead::new(
        Compat::new(read),
        AnyDelimiterCodec::<O>::new(b"##"),
        read_buf,
    );

    let collected_items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    handle.await.unwrap();

    assert_eq!(collected_items, items);
}
