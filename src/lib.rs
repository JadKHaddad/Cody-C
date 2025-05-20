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

mod framed_read;
pub use framed_read::{FramedRead, ReadError};

mod framed_write;
pub use framed_write::{FramedWrite, WriteError};

pub(crate) mod logging;

#[cfg(test)]
mod tests;
