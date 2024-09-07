use core::borrow::{Borrow, BorrowMut};

use tokio_util::bytes::Buf;

/// Compatibility wrapper for [`Tokio's AsyncRead`](tokio::io::AsyncRead).
///
/// Converts a [`Tokio's AsyncRead`](tokio::io::AsyncRead) into a [`Crate's AsyncRead`](crate::decode::async_read::AsyncRead).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AsyncReadCompat<R>(R);

impl<R> AsyncReadCompat<R> {
    pub const fn new(inner: R) -> Self {
        AsyncReadCompat(inner)
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

impl<R> Borrow<R> for AsyncReadCompat<R> {
    fn borrow(&self) -> &R {
        self.inner()
    }
}

impl<R> BorrowMut<R> for AsyncReadCompat<R> {
    fn borrow_mut(&mut self) -> &mut R {
        self.inner_mut()
    }
}

impl<R> AsRef<R> for AsyncReadCompat<R> {
    fn as_ref(&self) -> &R {
        &self.0
    }
}

impl<R> AsMut<R> for AsyncReadCompat<R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.0
    }
}

impl<R> From<R> for AsyncReadCompat<R> {
    fn from(inner: R) -> Self {
        Self::new(inner)
    }
}

pub trait TokioAsyncReadExt {
    fn compat(self) -> AsyncReadCompat<Self>
    where
        Self: Sized;
}

impl<R: tokio::io::AsyncRead> TokioAsyncReadExt for R {
    fn compat(self) -> AsyncReadCompat<Self> {
        AsyncReadCompat(self)
    }
}

#[cfg(feature = "futures")]
const _: () = {
    use crate::decode::async_read::AsyncRead as CrateAsyncRead;
    use tokio::io::AsyncReadExt;

    impl<R> CrateAsyncRead for AsyncReadCompat<R>
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
};

/// Compatibility wrapper for [`Tokio's Decoder`](tokio_util::codec::Decoder).
///
/// Converts a [`Crate's Decoder`](crate::decode::decoder::Decoder) into a [`Tokio's Decoder`](tokio_util::codec::Decoder).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TokioDecoderCompat<D>(D);

impl<D> TokioDecoderCompat<D> {
    pub const fn new(inner: D) -> Self {
        TokioDecoderCompat(inner)
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

impl<D> Borrow<D> for TokioDecoderCompat<D> {
    fn borrow(&self) -> &D {
        self.inner()
    }
}

impl<D> BorrowMut<D> for TokioDecoderCompat<D> {
    fn borrow_mut(&mut self) -> &mut D {
        self.inner_mut()
    }
}

impl<D> AsRef<D> for TokioDecoderCompat<D> {
    fn as_ref(&self) -> &D {
        self.inner()
    }
}

impl<D> AsMut<D> for TokioDecoderCompat<D> {
    fn as_mut(&mut self) -> &mut D {
        self.inner_mut()
    }
}

impl<D> From<D> for TokioDecoderCompat<D> {
    fn from(inner: D) -> Self {
        Self::new(inner)
    }
}

pub trait DecoderExt {
    fn compat(self) -> TokioDecoderCompat<Self>
    where
        Self: Sized;
}

impl<D: crate::decode::decoder::Decoder> DecoderExt for D {
    fn compat(self) -> TokioDecoderCompat<Self> {
        TokioDecoderCompat(self)
    }
}

const _: () = {
    use crate::decode::decoder::Decoder as CrateDecoder;

    impl<D> tokio_util::codec::Decoder for TokioDecoderCompat<D>
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
};
