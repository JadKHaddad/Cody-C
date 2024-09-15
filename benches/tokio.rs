use core::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};
use tokio_util::bytes::Bytes;

const MAX_ITEM_LENGTH: usize = 1024;
const MAX_FRAME_LENGTH: usize = 4 + MAX_ITEM_LENGTH;

mod cody_c {
    use cody_c::codec::length::LengthDelimitedCodec;
    use cody_c::codec::lines::LinesCodec;
    use cody_c::decode::framed_read::FramedRead;
    use cody_c::tokio::Compat;
    use cody_c::FramedWrite;
    use futures::{SinkExt, StreamExt};

    use crate::{MAX_FRAME_LENGTH, MAX_ITEM_LENGTH};

    pub fn bench_lines(input: &[u8]) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let codec = LinesCodec::<10>::new();
                let buf = &mut [0_u8; 1024];
                let framed_read = FramedRead::new(input, codec, buf);
                framed_read.collect::<Vec<_>>().await;
            })
    }

    pub fn bench_length(items: Vec<heapless::Vec<u8, MAX_ITEM_LENGTH>>) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let items_clone = items.clone();

                let (read, write) = tokio::io::duplex(1024);

                let handle = tokio::spawn(async move {
                    let write_buf = &mut [0_u8; MAX_FRAME_LENGTH * 2];
                    let mut framed_write = FramedWrite::new(
                        Compat::new(write),
                        LengthDelimitedCodec::<MAX_ITEM_LENGTH>,
                        write_buf,
                    );

                    for item in items_clone {
                        framed_write.send(item).await.unwrap();
                    }

                    framed_write.close().await.unwrap();
                });

                let read_buf = &mut [0_u8; MAX_FRAME_LENGTH * 2];
                let framed_read = FramedRead::new(
                    Compat::new(read),
                    LengthDelimitedCodec::<MAX_ITEM_LENGTH>,
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
            })
    }
}

mod tokio_codec {
    use futures::{SinkExt, StreamExt};
    use tokio_util::{
        bytes::Bytes,
        codec::{FramedRead, FramedWrite, LengthDelimitedCodec, LinesCodec},
    };

    pub fn bench_lines(input: &[u8]) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let codec = LinesCodec::new();
                let framed_read = FramedRead::new(input, codec);
                framed_read.collect::<Vec<_>>().await;
            });
    }

    pub fn bench_length(items: Vec<Bytes>) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let items_clone = items.clone();

                let (read, write) = tokio::io::duplex(1024);

                let handle = tokio::spawn(async move {
                    let codec = LengthDelimitedCodec::new();
                    let mut framed_write = FramedWrite::new(write, codec);

                    for item in items_clone {
                        framed_write.send(item).await.unwrap();
                    }

                    framed_write.close().await.unwrap();
                });

                let codec = LengthDelimitedCodec::new();
                let framed_read = FramedRead::new(read, codec);

                let collected_items: Vec<_> = framed_read
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                handle.await.unwrap();

                assert_eq!(collected_items, items);
            })
    }
}

fn generate_lines_input(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| format!("{}\r\n", i).into_bytes())
        .collect::<Vec<_>>()
        .concat()
}

fn generate_length_input() -> Vec<Vec<u8>> {
    (0..MAX_ITEM_LENGTH).map(|i| vec![0; i]).collect()
}

fn criterion_benchmark(c: &mut Criterion) {
    let n = 1000000;
    let lines_input = generate_lines_input(n);

    c.bench_function("cody_c_lines", |b| {
        b.iter(|| cody_c::bench_lines(black_box(&lines_input)))
    });
    c.bench_function("tokio_codec_lines", |b| {
        b.iter(|| tokio_codec::bench_lines(black_box(&lines_input)))
    });

    let length_input = generate_length_input();

    let cody_c_length_input: Vec<_> = length_input
        .iter()
        .map(|v| heapless::Vec::<_, MAX_ITEM_LENGTH>::from_slice(v).unwrap())
        .collect();

    let tokio_length_input: Vec<_> = length_input
        .iter()
        .map(|v| Bytes::from(v.clone()))
        .collect();

    // Cody's codec returns a `heapleass::Vec` which is O(n) to create from a slice.
    c.bench_function("cody_c_length", |b| {
        b.iter(|| cody_c::bench_length(black_box(cody_c_length_input.clone())))
    });

    // Tokio's codec returns a `Bytes` which is O(1) to create from a `BytesMut`.
    c.bench_function("tokio_codec_length", |b| {
        b.iter(|| tokio_codec::bench_length(black_box(tokio_length_input.clone())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
