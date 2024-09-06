#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "pretty-hex-fmt", feature = "char-fmt"))]
compile_error!(
    "Only one of the features `pretty-hex-fmt` and `char-fmt` can be enabled at a time."
);

pub mod decode;

#[cfg(feature = "codec")]
#[cfg_attr(docsrs, doc(cfg(feature = "codec")))]
pub mod codec;

#[cfg(feature = "futures-io")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures-io")))]
pub mod futures_io;

#[cfg(feature = "embedded-io-async")]
#[cfg_attr(docsrs, doc(cfg(feature = "embedded-io-async")))]
pub mod embedded_io_async;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

#[cfg(all(
    feature = "logging",
    any(feature = "log", feature = "defmt", feature = "tracing")
))]
#[cfg_attr(
    docsrs,
    doc(cfg(all(
        feature = "logging",
        any(feature = "log", feature = "defmt", feature = "tracing")
    )))
)]
pub mod logging;
