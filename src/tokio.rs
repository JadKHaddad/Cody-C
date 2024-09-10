use core::borrow::{Borrow, BorrowMut};

use tokio_util::bytes::Buf;

use crate::decode::maybe_decoded::{FrameSize, MaybeDecoded};

/// Compatibility wrapper for [`Tokio's AsyncRead`](tokio::io::AsyncRead) ans [`Tokio's AsyncWrite`](tokio::io::AsyncWrite)
///
/// - Converts a [`Tokio's AsyncRead`](tokio::io::AsyncRead) into a [`Crate's AsyncRead`](crate::decode::async_read::AsyncRead).
/// - Converts a [`Tokio's AsyncWrite`](tokio::io::AsyncWrite) into a [`Crate's AsyncWrite`](crate::encode::async_write::AsyncWrite).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Compat<R>(R);

impl<R> Compat<R> {
    pub const fn new(inner: R) -> Self {
        Compat(inner)
    }

    pub const fn inner(&self) -> &R {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.0
    }

    pub fn into_inner(self) -> R {
        self.0
    }
}

impl<R> Borrow<R> for Compat<R> {
    fn borrow(&self) -> &R {
        self.inner()
    }
}

impl<R> BorrowMut<R> for Compat<R> {
    fn borrow_mut(&mut self) -> &mut R {
        self.inner_mut()
    }
}

impl<R> AsRef<R> for Compat<R> {
    fn as_ref(&self) -> &R {
        &self.0
    }
}

impl<R> AsMut<R> for Compat<R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.0
    }
}

impl<R> From<R> for Compat<R> {
    fn from(inner: R) -> Self {
        Self::new(inner)
    }
}

const _: () = {
    use crate::decode::async_read::AsyncRead as CrateAsyncRead;
    use crate::encode::async_write::AsyncWrite as CrateAsyncWrite;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    impl<R> CrateAsyncRead for Compat<R>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        type Error = tokio::io::Error;

        fn read<'a>(
            &'a mut self,
            buf: &'a mut [u8],
        ) -> impl core::future::Future<Output = Result<usize, Self::Error>> {
            self.0.read(buf)
        }
    }

    impl<W> CrateAsyncWrite for Compat<W>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        type Error = tokio::io::Error;

        fn write<'a>(
            &'a mut self,
            buf: &'a [u8],
        ) -> impl core::future::Future<Output = Result<usize, Self::Error>> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            self.0.flush()
        }

        fn shutdown(&mut self) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            self.0.shutdown()
        }
    }
};

/// Compatibility wrapper for [`Tokio's Decoder`](tokio_util::codec::Decoder) and [`Tokio's Encoder`](tokio_util::codec::Encoder).
///
/// - Converts a [`Crate's Decoder`](crate::decode::decoder::Decoder) into a [`Tokio's Decoder`](tokio_util::codec::Decoder).
/// - Converts a [`Crate's Encoder`](crate::encode::encoder::Encoder) into a [`Tokio's Encoder`](tokio_util::codec::Encoder).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CodecCompat<C>(C);

impl<C> CodecCompat<C> {
    pub const fn new(inner: C) -> Self {
        CodecCompat(inner)
    }

    pub const fn inner(&self) -> &C {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.0
    }

    pub fn into_inner(self) -> C {
        self.0
    }
}

impl<C> Borrow<C> for CodecCompat<C> {
    fn borrow(&self) -> &C {
        self.inner()
    }
}

impl<C> BorrowMut<C> for CodecCompat<C> {
    fn borrow_mut(&mut self) -> &mut C {
        self.inner_mut()
    }
}

impl<C> AsRef<C> for CodecCompat<C> {
    fn as_ref(&self) -> &C {
        self.inner()
    }
}

impl<C> AsMut<C> for CodecCompat<C> {
    fn as_mut(&mut self) -> &mut C {
        self.inner_mut()
    }
}

impl<C> From<C> for CodecCompat<C> {
    fn from(inner: C) -> Self {
        Self::new(inner)
    }
}

const _: () = {
    use crate::{
        decode::decoder::Decoder as CrateDecoder, encode::encoder::Encoder as CrateEncoder,
    };

    impl<C> tokio_util::codec::Decoder for CodecCompat<C>
    where
        C: CrateDecoder,
        <C as CrateDecoder>::Error: core::convert::From<tokio::io::Error>,
    {
        type Item = <C as CrateDecoder>::Item;
        type Error = <C as CrateDecoder>::Error;

        fn decode(
            &mut self,
            src: &mut tokio_util::bytes::BytesMut,
        ) -> Result<Option<Self::Item>, Self::Error> {
            match self.as_mut().decode(src.as_mut()) {
                Ok(MaybeDecoded::None(FrameSize::Unknown)) => Ok(None),
                Ok(MaybeDecoded::None(FrameSize::Known(size))) => {
                    src.reserve(size);

                    Ok(None)
                }
                Ok(MaybeDecoded::Frame(frame)) => {
                    src.advance(frame.size());

                    Ok(Some(frame.into_item()))
                }
                Err(err) => Err(err),
            }
        }
    }

    impl<C, Item> tokio_util::codec::Encoder<Item> for CodecCompat<C>
    where
        C: CrateEncoder<Item>,
        <C as CrateEncoder<Item>>::Error: core::convert::From<tokio::io::Error>,
    {
        type Error = <C as CrateEncoder<Item>>::Error;

        fn encode(
            &mut self,
            item: Item,
            dst: &mut tokio_util::bytes::BytesMut,
        ) -> Result<(), Self::Error> {
            match self.as_mut().encode(item, dst.as_mut()) {
                Ok(size) => {
                    dst.advance(size);

                    Ok(())
                }
                Err(err) => Err(err),
            }
        }
    }
};
