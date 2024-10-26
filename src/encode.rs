//! Encoder trait definition.

/// An encoder that encodes a frame into a buffer.
pub trait Encoder<Item> {
    /// The type of error that this encoder returns.
    type Error;

    /// Encodes an item into the provided buffer.
    fn encode(&mut self, item: Item, dst: &mut [u8]) -> Result<usize, Self::Error>;
}

impl<E, Item> Encoder<Item> for &mut E
where
    E: Encoder<Item>,
{
    type Error = E::Error;

    fn encode(&mut self, item: Item, dst: &mut [u8]) -> Result<usize, Self::Error> {
        (*self).encode(item, dst)
    }
}
