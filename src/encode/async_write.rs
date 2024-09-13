use core::future::Future;

/// An asynchronous writer.
///
/// The core `Sink` functionality of this crate is built around this trait.
pub trait AsyncWrite {
    type Error;

    /// Writes bytes from the provided buffer into the underlying sink returning how many bytes were written.
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Flushes the underlying sink, ensuring that all intermediately buffered contents reach their destination.
    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>>;

    /// Shuts down the underlying sink, ensuring that no more data can be written.
    fn shutdown(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}
