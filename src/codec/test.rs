#[cfg(all(
    feature = "logging",
    any(feature = "log", feature = "defmt", feature = "tracing")
))]
use crate::logging::formatter::Formatter;
use crate::{
    decode::{
        decoder::Decoder,
        frame::Frame,
        framed_read::FramedRead,
        maybe_decoded::{FrameSize, MaybeDecoded},
    },
    encode::{encoder::Encoder, framed_write::FramedWrite},
    test::init_tracing,
    tokio::Compat,
};

extern crate std;

use std::vec::Vec;

use futures::{SinkExt, StreamExt};

pub struct FrameSizeAwareCodec;

impl Decoder for FrameSizeAwareCodec {
    type Item = heapless::Vec<u8, 256>;
    type Error = tokio::io::Error;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let src = Formatter(src);
            tracing::debug!(?src, "Decoding");
        }

        if src.len() < 4 {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::debug!("Not enough bytes to read frame size");

            return Ok(MaybeDecoded::None(FrameSize::Unknown));
        }

        let frame_size = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        #[cfg(all(feature = "logging", feature = "tracing"))]
        tracing::debug!(frame_size, "Frame size");

        if src.len() < frame_size {
            #[cfg(all(feature = "logging", feature = "tracing"))]
            tracing::debug!("Not enough bytes to read frame");

            return Ok(MaybeDecoded::None(FrameSize::Known(frame_size)));
        }

        let item = heapless::Vec::from_slice(&src[4..frame_size]).map_err(|_| {
            tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                "Frame too large. Max 256 bytes",
            )
        })?;

        Ok(MaybeDecoded::Frame(Frame::new(frame_size, item)))
    }
}

impl Encoder<heapless::Vec<u8, 256>> for FrameSizeAwareCodec {
    type Error = tokio::io::Error;

    fn encode(
        &mut self,
        item: heapless::Vec<u8, 256>,
        dst: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let item_size = item.len();
        let frame_size = item_size + 4;

        #[cfg(all(feature = "logging", feature = "tracing"))]
        {
            let item = Formatter(&item);
            tracing::debug!(frame=?item, %item_size, %frame_size, available=%dst.len(), "Encoding Frame");
        }

        if dst.len() < frame_size {
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                "Destination buffer too small",
            ));
        }

        let frame_size_bytes = (frame_size as u32).to_be_bytes();
        dst[..4].copy_from_slice(&frame_size_bytes);

        dst[4..frame_size].copy_from_slice(&item);

        Ok(frame_size)
    }
}

#[tokio::test]
async fn crate_sink_stream_frame_size_aware() {
    init_tracing();

    let items = std::vec![
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

    let (read, write) = tokio::io::duplex(1);

    let handle = tokio::spawn(async move {
        let write_buf = &mut [0_u8; 1024];
        let mut framed_write = FramedWrite::new(Compat::new(write), FrameSizeAwareCodec, write_buf);

        for item in items_clone {
            framed_write.send(item).await.unwrap();
        }

        framed_write.close().await.unwrap();
    });

    let read_buf = &mut [0_u8; 1024];
    let framed_read = FramedRead::new(Compat::new(read), FrameSizeAwareCodec, read_buf);

    let collected_items: Vec<_> = framed_read
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    handle.await.unwrap();

    assert_eq!(collected_items, items);
}
