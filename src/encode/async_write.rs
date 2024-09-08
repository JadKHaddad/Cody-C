use core::future::Future;

pub trait AsyncWrite {
    type Error;

    fn write<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>>;

    fn shutdown(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}
