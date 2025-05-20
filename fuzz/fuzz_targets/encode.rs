//! If we panic!, we lose.
//!
//! ```not_rust
//! cargo +nightly fuzz run encode
//! ```

#![no_main]

use cody_c::{
    codec::{
        any::{AnyDelimiterCodec, AnyDelimiterCodecOwned},
        bytes::{BytesCodec, BytesCodecOwned},
        lines::{LinesCodec, LinesCodecOwned, StrLinesCodec, StringLinesCodec},
    },
    encode::Encoder,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let buf = &mut [0_u8; 64];

    let mut codec = AnyDelimiterCodec::new(b"#");
    let _ = codec.encode(data, buf);

    let mut codec = AnyDelimiterCodecOwned::<64>::new(b"#");
    if let Ok(vector) = heapless::Vec::<u8, 64>::from_slice(data) {
        let _ = codec.encode(vector, buf);
    }

    let mut codec = BytesCodec::new();
    let _ = codec.encode(data, buf);

    let mut codec = BytesCodecOwned::<64>::new();
    if let Ok(vector) = heapless::Vec::<u8, 64>::from_slice(data) {
        let _ = codec.encode(vector, buf);
    }

    let mut codec = LinesCodec::new();
    let _ = codec.encode(data, buf);

    let mut codec = LinesCodecOwned::<64>::new();
    if let Ok(vector) = heapless::Vec::<u8, 64>::from_slice(data) {
        let _ = codec.encode(vector, buf);
    }

    let mut codec = StrLinesCodec::new();
    if let Ok(str) = str::from_utf8(data) {
        let _ = codec.encode(str, buf);
    }

    let mut codec = StringLinesCodec::<64>::new();
    if let Ok(vector) = heapless::Vec::<u8, 64>::from_slice(data) {
        if let Ok(string) = heapless::String::<64>::from_utf8(vector) {
            let _ = codec.encode(string, buf);
        }
    }
});
