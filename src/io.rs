//! I/O traits definition.

use core::future::Future;

/// An asynchronous reader.
pub trait AsyncRead {
    /// The type of error that can be returned by [`AsyncRead`] operations.
    type Error;

    /// Reads bytes from the underlying source into the provided buffer returning how many bytes were read.
    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>>;
}

impl<T: AsyncRead> AsyncRead for &mut T {
    type Error = T::Error;

    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>> {
        (*self).read(buf)
    }
}

/// An asynchronous writer.
///
/// The core `Sink` functionality of this crate is built around this trait.
pub trait AsyncWrite {
    /// The type of error that can be returned by [`AsyncWrite`] operations.
    type Error;

    /// Writes all bytes from the provided buffer into the underlying sink returning how many bytes were written.
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<(), Self::Error>>;

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

impl AsyncWrite for &mut [u8] {
    type Error = core::convert::Infallible;

    async fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> Result<(), Self::Error> {
        let amt = core::cmp::min(buf.len(), self.len());
        let (a, b) = core::mem::take(self).split_at_mut(amt);
        a.copy_from_slice(&buf[..amt]);
        *self = b;
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
