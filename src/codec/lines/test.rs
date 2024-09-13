extern crate std;

use core::str::FromStr;
use std::vec::Vec;

use futures::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;

use super::*;
use crate::{
    decode::framed_read::FramedRead, encode::framed_write::FramedWrite, test::init_tracing,
    tokio::Compat,
};

macro_rules! collect_items {
    ($framed_read:expr) => {{
        let items: Vec<_> = $framed_read
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        items
    }};
}

async fn one_from_slice<const I: usize, const O: usize>() {
    // Test with `LineBytesCodec`

    let read: &[u8] = b"1\r\n";

    let result = std::vec![heapless::Vec::<_, O>::from_slice(b"1").unwrap(),];

    let codec = LineBytesCodec::<O>::new();
    let buf = &mut [0_u8; I];
    let framed_read = FramedRead::new(read, codec, buf);

    let items = collect_items!(framed_read);

    assert_eq!(items, result);

    // Test with `LinesCodec`

    let read: &[u8] = b"1\r\n";
    let result = std::vec![heapless::String::<O>::from_str("1").unwrap(),];

    let codec = LinesCodec::<O>::new();
    let buf = &mut [0_u8; I];
    let framed_read = FramedRead::new(read, codec, buf);

    let items = collect_items!(framed_read);

    assert_eq!(items, result);
}

async fn four_from_slice<const I: usize, const O: usize>() {
    // Test with `LineBytesCodec`

    let read: &[u8] = b"1\r\n2\n3\n4\r\n";
    let result = std::vec![
        heapless::Vec::<_, O>::from_slice(b"1").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"2").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"3").unwrap(),
        heapless::Vec::<_, O>::from_slice(b"4").unwrap(),
    ];

    let codec = LineBytesCodec::<O>::new();
    let buf = &mut [0_u8; I];
    let framed_read = FramedRead::new(read, codec, buf);

    let items = collect_items!(framed_read);

    assert_eq!(items, result);

    // Test with `LinesCodec`

    let read: &[u8] = b"1\r\n2\n3\n4\r\n";
    let result = std::vec![
        heapless::String::<O>::from_str("1").unwrap(),
        heapless::String::<O>::from_str("2").unwrap(),
        heapless::String::<O>::from_str("3").unwrap(),
        heapless::String::<O>::from_str("4").unwrap(),
    ];

    let codec = LinesCodec::<O>::new();
    let buf = &mut [0_u8; I];
    let framed_read = FramedRead::new(read, codec, buf);

    let items = collect_items!(framed_read);

    assert_eq!(items, result);
}

async fn from_slow_reader<const I: usize, const O: usize>() {
    let chunks = std::vec![
        Vec::from(b"jh asjd\r\n"),
        Vec::from(b"k hb\njsjuwjal kadj\njsadhjiu\r\nw"),
        Vec::from(b"\r\njal kadjjsadhjiuwqens \n"),
        Vec::from(b"nd "),
        Vec::from(b"yxxcjajsdi\naskdn as"),
        Vec::from(b"jdasd\r\niouqw es"),
        Vec::from(b"sd\n"),
    ];

    // Test with `LineBytesCodec`

    let chunks_clone = chunks.clone();

    let result_bytes = std::vec![
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
        for chunk in chunks_clone {
            write.write_all(&chunk).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    let read = Compat::new(read);
    let codec = LineBytesCodec::<O>::new();
    let buf = &mut [0_u8; I];
    let framed_read = FramedRead::new(read, codec, buf);

    let items = collect_items!(framed_read);

    assert_eq!(items, result_bytes);

    // Test with `LinesCodec`

    let result_strings = result_bytes
        .clone()
        .into_iter()
        .map(|b| heapless::String::from_utf8(b).unwrap())
        .collect::<Vec<_>>();

    let (read, mut write) = tokio::io::duplex(1024);

    tokio::spawn(async move {
        for chunk in chunks {
            write.write_all(&chunk).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    let read = Compat::new(read);
    let codec = LinesCodec::<O>::new();
    let buf = &mut [0_u8; I];
    let framed_read = FramedRead::new(read, codec, buf);

    let items = collect_items!(framed_read);

    assert_eq!(items, result_strings);
}

#[tokio::test]
async fn one_item_one_stroke() {
    init_tracing();

    one_from_slice::<5, 3>().await;
}

#[tokio::test]
async fn four_items_one_stroke() {
    init_tracing();

    four_from_slice::<11, 5>().await;
}

#[tokio::test]
async fn four_items_many_strokes() {
    init_tracing();

    // Input buffer will refill 4 times.
    four_from_slice::<3, 5>().await;
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

    // Test with `LineBytesCodec`

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
        let mut framed_write =
            FramedWrite::new(Compat::new(write), LineBytesCodec::<O>::new(), write_buf);

        for item in items_clone {
            framed_write.send(item).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        framed_write.close().await.unwrap();
    });

    let read_buf = &mut [0_u8; 1024];
    let framed_read = FramedRead::new(Compat::new(read), LineBytesCodec::<O>::new(), read_buf);

    let collected_items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    handle.await.unwrap();

    assert_eq!(collected_items, items);

    // Test with `LinesCodec`

    let items = std::vec![
        heapless::String::<O>::from_str("jh asjd").unwrap(),
        heapless::String::<O>::from_str("k hb").unwrap(),
        heapless::String::<O>::from_str("jsjuwjal kadj").unwrap(),
        heapless::String::<O>::from_str("jsadhjiu").unwrap(),
        heapless::String::<O>::from_str("w").unwrap(),
        heapless::String::<O>::from_str("jal kadjjsadhjiuwqens ").unwrap(),
        heapless::String::<O>::from_str("nd yxxcjajsdi").unwrap(),
        heapless::String::<O>::from_str("askdn asjdasd").unwrap(),
        heapless::String::<O>::from_str("iouqw essd").unwrap(),
    ];

    let items_clone = items.clone();

    let (read, write) = tokio::io::duplex(1024);

    let handle = tokio::spawn(async move {
        let write_buf = &mut [0_u8; 1024];
        let mut framed_write =
            FramedWrite::new(Compat::new(write), LinesCodec::<O>::new(), write_buf);

        for item in items_clone {
            framed_write.send(item).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        framed_write.close().await.unwrap();
    });

    let read_buf = &mut [0_u8; 1024];
    let framed_read = FramedRead::new(Compat::new(read), LinesCodec::<O>::new(), read_buf);

    let collected_items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    handle.await.unwrap();

    assert_eq!(collected_items, items);
}
