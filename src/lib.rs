//! # Cody the codec
//!
//! A simple and `zerocopy` codec for encoding and decoding data in `no_std` environments.

#![no_std]
#![deny(unsafe_code)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod codec;
pub mod decode;
pub mod encode;
pub mod io;

mod framed_read;
pub use framed_read::{FramedRead, ReadError};

mod framed_write;
pub use framed_write::{FramedWrite, WriteError};

#[cfg(any(test, feature = "tokio"))]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

#[cfg(feature = "futures-io")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures-io")))]
pub mod futures_io;

#[cfg(feature = "embedded-io-async")]
#[cfg_attr(docsrs, doc(cfg(feature = "embedded-io-async")))]
pub mod embedded_io_async;

pub(crate) mod logging;

#[cfg(test)]
mod tests;
