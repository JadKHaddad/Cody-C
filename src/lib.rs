//! # Cody the codec
//!
//! A simple and `zerocopy` codec for encoding and decoding data in `no_std` environments.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
// #![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod codec;
pub mod decode;
pub mod encode;
pub mod framed_read;
pub mod framed_write;
pub mod io;
pub(crate) mod logging;

pub use codec::*;
pub use decode::*;
pub use encode::*;
pub use framed_read::*;
pub use framed_write::*;
pub use io::*;

#[cfg(any(test, feature = "tokio"))]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

#[cfg(feature = "futures-io")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures-io")))]
pub mod futures_io;

#[cfg(feature = "embedded-io-async")]
#[cfg_attr(docsrs, doc(cfg(feature = "embedded-io-async")))]
pub mod embedded_io_async;

#[cfg(feature = "demo")]
#[cfg_attr(docsrs, doc(cfg(feature = "demo")))]
pub mod demo;

#[cfg(test)]
mod test;
