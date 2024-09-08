pub trait Encoder<Item> {
    type Error;

    fn encode(&mut self, item: Item, dst: &mut [u8]) -> Result<usize, Self::Error>;
}
