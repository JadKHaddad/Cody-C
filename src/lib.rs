//! # Cody the codec
//!
//! A simple and `zerocopy` codec for encoding and decoding data in `no_std` environments.
//!
//! This crate is based on [`embedded_io_async`](https://docs.rs/embedded-io-async/latest/embedded_io_async/)'s
//! [`Read`](https://docs.rs/embedded-io-async/latest/embedded_io_async/trait.Read.html) and [`Write`](https://docs.rs/embedded-io-async/latest/embedded_io_async/trait.Write.html) traits.
//!
//! It's recommended to use [`embedded_io_adapters`](https://docs.rs/embedded-io-adapters/0.6.1/embedded_io_adapters/) if you are using other async `Read` and `Write` traits like [`tokio`](https://docs.rs/tokio/latest/tokio/index.html)'s [`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html) and [`AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html).
//!
//! See the examples for more information.

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
