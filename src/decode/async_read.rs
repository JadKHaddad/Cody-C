//! Asynchronous reader trait definition.

use core::future::Future;

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
