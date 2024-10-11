//! Compatibility wrapper for [`Futures-io' AsyncRead`](futures::io::AsyncRead) and [`Futures-io' AsyncWrite`](futures::io::AsyncWrite).

use crate::io::{AsyncRead as CrateAsyncRead, AsyncWrite as CrateAsyncWrite};
use core::borrow::{Borrow, BorrowMut};

/// Compatibility wrapper for [`Futures-io' AsyncRead`](futures::io::AsyncRead) and [`Futures-io' AsyncWrite`](futures::io::AsyncWrite).
///
/// - Converts a [`Futures-io' AsyncRead`](futures::io::AsyncRead) into a [`Crate's AsyncRead`](crate::io::AsyncRead).
/// - Converts a [`Futures-io' AsyncWrite`](futures::io::AsyncWrite) into a [`Crate's AsyncWrite`](crate::io::AsyncWrite).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Compat<R>(R);

impl<R> Compat<R> {
    /// Creates a new [`Compat`] from a [`Futures-io' AsyncRead`](futures::io::AsyncRead) or [`Futures-io' AsyncWrite`](futures::io::AsyncWrite).
    #[inline]
    pub const fn new(inner: R) -> Self {
        Compat(inner)
    }

    /// Returns a reference to the inner [`Futures-io' AsyncRead`](futures::io::AsyncRead) or [`Futures-io' AsyncWrite`](futures::io::AsyncWrite).
    #[inline]
    pub const fn inner(&self) -> &R {
        &self.0
    }

    /// Returns a mutable reference to the inner [`Futures-io' AsyncRead`](futures::io::AsyncRead) or [`Futures-io' AsyncWrite`](futures::io::AsyncWrite).
    #[inline]
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.0
    }

    /// Returns the inner [`Futures-io' AsyncRead`](futures::io::AsyncRead) or [`Futures-io' AsyncWrite`](futures::io::AsyncWrite) consuming this [`Compat`].
    #[inline]
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
    use futures::io::{AsyncReadExt, AsyncWriteExt};

    impl<R> CrateAsyncRead for Compat<R>
    where
        R: futures::io::AsyncRead + Unpin,
    {
        type Error = futures::io::Error;

        fn read<'a>(
            &'a mut self,
            buf: &'a mut [u8],
        ) -> impl core::future::Future<Output = Result<usize, Self::Error>> {
            self.0.read(buf)
        }
    }

    impl<W> CrateAsyncWrite for Compat<W>
    where
        W: futures::io::AsyncWrite + Unpin,
    {
        type Error = futures::io::Error;

        fn write_all<'a>(
            &'a mut self,
            buf: &'a [u8],
        ) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            self.0.write_all(buf)
        }
    }
};
