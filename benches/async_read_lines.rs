use core::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};

mod cody_c {
    use cody_c::codec::lines::LinesCodec;
    use cody_c::decode::framed_read::FramedRead;
    use futures::StreamExt;

    pub fn bench(input: &[u8]) {
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
}

mod tokio_codec {
    use futures::StreamExt;
    use tokio_util::codec::{FramedRead, LinesCodec};

    pub fn bench(input: &[u8]) {
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
}

fn generate_input(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| format!("{}\r\n", i).into_bytes())
        .collect::<Vec<_>>()
        .concat()
}

fn criterion_benchmark(c: &mut Criterion) {
    let n = 1000000;
    let input = generate_input(n);

    c.bench_function("cody_c", |b| b.iter(|| cody_c::bench(black_box(&input))));
    c.bench_function("tokio_codec", |b| {
        b.iter(|| tokio_codec::bench(black_box(&input)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
