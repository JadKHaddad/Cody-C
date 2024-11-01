use zerocopy::{
    byteorder::little_endian::{U16, U32},
    FromBytes, Immutable, IntoBytes, KnownLayout,
};

use super::payload_type::PayloadType;

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Debug, Clone)]
#[repr(C)]
pub struct Header {
    packet_length: U32,
    raw_payload_type: U16,
}

impl Header {
    pub const fn size() -> usize {
        core::mem::size_of::<Header>()
    }

    pub const fn packet_length(&self) -> u32 {
        self.packet_length.get()
    }

    pub const fn packet_length_usize(&self) -> usize {
        self.packet_length() as usize
    }

    pub fn set_packet_length(&mut self, length: u32) {
        self.packet_length.set(length);
    }

    pub const fn raw_payload_type(&self) -> u16 {
        self.raw_payload_type.get()
    }

    pub const fn payload_type(&self) -> Option<PayloadType> {
        PayloadType::from_u16(self.raw_payload_type.get())
    }

    /// Theoretical payload length. Calculated from [`Self::packet_length`] and [`Self::size`].
    pub const fn payload_length(&self) -> usize {
        self.packet_length.get() as usize - Self::size()
    }

    pub fn maybe_header_from_prefix(src: &[u8]) -> Option<(&Self, &[u8])> {
        Header::ref_from_prefix(src).ok()
    }
}
