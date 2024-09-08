use core::borrow::{Borrow, BorrowMut};

use tokio_util::bytes::Buf;

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
pub struct CodecCompat<D>(D);

impl<D> CodecCompat<D> {
    pub const fn new(inner: D) -> Self {
        CodecCompat(inner)
    }

    pub const fn inner(&self) -> &D {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut D {
        &mut self.0
    }

    pub fn into_inner(self) -> D {
        self.0
    }
}

impl<D> Borrow<D> for CodecCompat<D> {
    fn borrow(&self) -> &D {
        self.inner()
    }
}

impl<D> BorrowMut<D> for CodecCompat<D> {
    fn borrow_mut(&mut self) -> &mut D {
        self.inner_mut()
    }
}

impl<D> AsRef<D> for CodecCompat<D> {
    fn as_ref(&self) -> &D {
        self.inner()
    }
}

impl<D> AsMut<D> for CodecCompat<D> {
    fn as_mut(&mut self) -> &mut D {
        self.inner_mut()
    }
}

impl<D> From<D> for CodecCompat<D> {
    fn from(inner: D) -> Self {
        Self::new(inner)
    }
}

const _: () = {
    use crate::decode::decoder::Decoder as CrateDecoder;

    impl<D> tokio_util::codec::Decoder for CodecCompat<D>
    where
        D: CrateDecoder,
        <D as CrateDecoder>::Error: core::convert::From<tokio::io::Error>,
    {
        type Item = <D as CrateDecoder>::Item;
        type Error = <D as CrateDecoder>::Error;

        fn decode(
            &mut self,
            src: &mut tokio_util::bytes::BytesMut,
        ) -> Result<Option<Self::Item>, Self::Error> {
            match self.as_mut().decode(src.as_mut()) {
                Ok(None) => Ok(None),
                Ok(Some(frame)) => {
                    src.advance(frame.size());

                    Ok(Some(frame.into_item()))
                }
                Err(err) => Err(err),
            }
        }
    }

    // TODO: implement Encoder
};
