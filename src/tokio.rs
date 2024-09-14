use core::borrow::{Borrow, BorrowMut};

/// Compatibility wrapper for [`Tokio's AsyncRead`](tokio::io::AsyncRead) ans [`Tokio's AsyncWrite`](tokio::io::AsyncWrite)
///
/// - Converts a [`Tokio's AsyncRead`](tokio::io::AsyncRead) into a [`Crate's AsyncRead`](crate::decode::async_read::AsyncRead).
/// - Converts a [`Tokio's AsyncWrite`](tokio::io::AsyncWrite) into a [`Crate's AsyncWrite`](crate::encode::async_write::AsyncWrite).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Compat<R>(R);

impl<R> Compat<R> {
    /// Creates a new [`Compat`] from a [`Tokio's AsyncRead`](tokio::io::AsyncRead) or [`Tokio's AsyncWrite`](tokio::io::AsyncWrite).
    #[inline]
    pub const fn new(inner: R) -> Self {
        Compat(inner)
    }

    /// Returns a reference to the inner [`Tokio's AsyncRead`](tokio::io::AsyncRead) or [`Tokio's AsyncWrite`](tokio::io::AsyncWrite).
    #[inline]
    pub const fn inner(&self) -> &R {
        &self.0
    }

    /// Returns a mutable reference to the inner [`Tokio's AsyncRead`](tokio::io::AsyncRead) or [`Tokio's AsyncWrite`](tokio::io::AsyncWrite).
    #[inline]
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.0
    }

    /// Returns the inner [`Tokio's AsyncRead`](tokio::io::AsyncRead) or [`Tokio's AsyncWrite`](tokio::io::AsyncWrite) consuming this [`Compat`].
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
