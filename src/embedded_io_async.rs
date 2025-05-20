//! Compatibility wrapper for [`Embedded-io-async's Read`](embedded_io_async::Read) and [`Embedded-io-async's Write`](embedded_io_async::Write).

use core::borrow::{Borrow, BorrowMut};

use crate::io::{AsyncRead as CrateRead, AsyncWrite as CrateWrite};

/// Compatibility wrapper for [`Embedded-io-async's Read`](embedded_io_async::Read) and [`Embedded-io-async's Write`](embedded_io_async::Write).
///
/// - Converts an [`Embedded-io-async's Read`](embedded_io_async::Read) into a [`Crate's AsyncRead`](crate::io::AsyncRead).
/// - Converts an [`Embedded-io-async's Write`](embedded_io_async::Write) into a [`Crate's AsyncWrite`](crate::io::AsyncWrite).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Compat<R>(R);

impl<R> Compat<R> {
    /// Creates a new [`Compat`] from an [`Embedded-io-async's Read`](embedded_io_async::Read) or [`Embedded-io-async's Write`](embedded_io_async::Write).
    #[inline]
    pub const fn new(inner: R) -> Self {
        Compat(inner)
    }

    /// Returns a reference to the inner [`Embedded-io-async's Read`](embedded_io_async::Read) or [`Embedded-io-async's Write`](embedded_io_async::Write).
    #[inline]
    pub const fn inner(&self) -> &R {
        &self.0
    }

    /// Returns a mutable reference to the inner [`Embedded-io-async's Read`](embedded_io_async::Read) or [`Embedded-io-async's Write`](embedded_io_async::Write).
    #[inline]
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.0
    }

    /// Returns the inner [`Embedded-io-async's Read`](embedded_io_async::Read) or [`Embedded-io-async's Write`](embedded_io_async::Write) consuming this [`Compat`].
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
    use embedded_io_async::ErrorType;

    impl<R> CrateRead for Compat<R>
    where
        R: embedded_io_async::Read + Unpin,
    {
        type Error = <R as ErrorType>::Error;
        fn read<'a>(
            &'a mut self,
            buf: &'a mut [u8],
        ) -> impl core::future::Future<Output = Result<usize, Self::Error>> {
            self.0.read(buf)
        }
    }

    impl<W> CrateWrite for Compat<W>
    where
        W: embedded_io_async::Write + Unpin,
    {
        type Error = <W as ErrorType>::Error;

        fn write_all<'a>(
            &'a mut self,
            buf: &'a [u8],
        ) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            self.0.write_all(buf)
        }

        fn flush(&mut self) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            self.0.flush()
        }
    }
};
