use core::future::Future;

pub trait AsyncRead {
    type Error;

    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>>;
}
