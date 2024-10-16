//! I/O traits.

use futures::Future;

/// An asynchronous writer.
///
/// The core `Sink` functionality of this crate is built around this trait.
pub trait AsyncWrite {
    /// The type of error that can be returned by [`AsyncWrite`] operations.
    type Error;

    /// Writes all bytes from the provided buffer into the underlying sink returning how many bytes were written.
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<(), Self::Error>>;
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
}

/// An asynchronous reader.
///
/// The core `Stream` functionality of this crate is built around this trait.
pub trait AsyncRead {
    /// The type of error that can be returned by [`AsyncRead`] operations.
    type Error;

    /// Reads bytes from the underlying source into the provided buffer returning how many bytes were read.
    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>>;
}

impl AsyncRead for &[u8] {
    type Error = core::convert::Infallible;

    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize, Self::Error> {
        let amt = core::cmp::min(buf.len(), self.len());
        let (a, b) = self.split_at(amt);

        if amt == 1 {
            buf[0] = a[0];
        } else {
            buf[..amt].copy_from_slice(a);
        }

        *self = b;
        Ok(amt)
    }
}
