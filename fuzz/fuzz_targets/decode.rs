//! If we panic!, we lose.
//!
//! ```not_rust
//! cargo +nightly fuzz run decode
//! ```

#![no_main]

use cody_c::{
    codec::{
        any::{AnyDelimiterCodec, AnyDelimiterCodecOwned},
        bytes::{BytesCodec, BytesCodecOwned},
        lines::{LinesCodec, LinesCodecOwned, StrLinesCodec, StringLinesCodec},
    },
    decode::{Decoder, DecoderOwned},
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let buf = &mut [0_u8; 64];
    let data = &mut std::vec::Vec::from(data);

    let mut codec = AnyDelimiterCodec::new(b"#");
    let _ = codec.decode(data).expect("Must be Infallible");

    let mut codec = AnyDelimiterCodecOwned::<64>::new(b"#");
    let _ = codec.decode_owned(buf);

    let mut codec = BytesCodec::new();
    let _ = codec.decode(buf).expect("Must be Infallible");

    let mut codec = BytesCodecOwned::<64>::new();
    let _ = codec.decode_owned(buf);

    let mut codec = LinesCodec::new();
    let _ = codec.decode(buf).expect("Must be Infallible");

    let mut codec = LinesCodecOwned::<64>::new();
    let _ = codec.decode_owned(buf);

    let mut codec = StrLinesCodec::new();

    match core::str::from_utf8(data) {
        Ok(_) => {
            let _ = codec.decode(buf).expect("Must be Infallible");
        }
        Err(_) => {
            let _ = codec.decode(buf);
        }
    }

    let mut codec = StringLinesCodec::<64>::new();
    let _ = codec.decode_owned(buf);
});
