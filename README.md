# Cody the codec

![Build Status](https://github.com/JadKHaddad/Cody-C/actions/workflows/build-and-test.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/cody-c.svg)](https://crates.io/crates/cody-c)
[![Crates.io (MSRV)](https://img.shields.io/crates/msrv/cody-c)](https://crates.io/crates/cody-c)
[![docs.rs](https://docs.rs/cody-c/badge.svg)](https://docs.rs/cody-c)
[![Crates.io (Downloads)](https://img.shields.io/crates/d/cody-c)](https://crates.io/crates/cody-c)
[![Crates.io (License)](https://img.shields.io/crates/l/cody-c)](https://crates.io/crates/cody-c)

A simple and `zerocopy` codec for encoding and decoding data in `no_std` environments.

This crate is based on [`embedded_io_async`](https://docs.rs/embedded-io-async/latest/embedded_io_async/)'s
[`Read`](https://docs.rs/embedded-io-async/latest/embedded_io_async/trait.Read.html) and [`Write`](https://docs.rs/embedded-io-async/latest/embedded_io_async/trait.Write.html) traits.

It's recommended to use [`embedded_io_adapters`](https://docs.rs/embedded-io-adapters/0.6.1/embedded_io_adapters/) if you are using other async `Read` and `Write` traits like [`tokio`](https://docs.rs/tokio/latest/tokio/index.html)'s [`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html) and [`AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html).

See the examples for more information.

## Features

- `log`: Enables logging using [`log`](https://docs.rs/log/latest/log/).
- `tracing`: Enables logging using [`tracing`](https://docs.rs/tracing/latest/tracing/).
- `defmt`: Enables logging using [`defmt`](https://docs.rs/defmt/latest/defmt/index.html)
and implements [`defmt::Format`](https://docs.rs/defmt/latest/defmt/trait.Format.html) for structs and enums.
- `pretty-hex-fmt`: Logs byte slices like `[0x00, 0x00, 0x00, 0x6F]` instead of `[00, 00, 00, 6F]`.
- `char-fmt`: Logs byte slices as char slices.
- `buffer-early-shift`: Moves the bytes in the encode buffer to the start of the buffer after the first successful decoded frame.
