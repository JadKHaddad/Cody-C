use crc32fast::Hasher;
use zerocopy::{
    big_endian::U32, byteorder::big_endian::U16, FromBytes, Immutable, IntoBytes, KnownLayout,
};

use super::{payload::Payload, payload_type::PayloadType};

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Debug, Clone)]
#[repr(C)]
pub struct Header {
    packet_length: U16,
    raw_payload_type: U16,
    checksum: U32,
}

impl Header {
    pub const fn size() -> usize {
        core::mem::size_of::<Header>()
    }

    pub fn calculate_checksum(data: &[u8]) -> u32 {
        let mut hasher = Hasher::new();

        hasher.update(data);

        hasher.finalize()
    }

    pub const fn packet_length(&self) -> u16 {
        self.packet_length.get()
    }

    pub const fn packet_length_usize(&self) -> usize {
        self.packet_length() as usize
    }

    pub fn set_packet_length(&mut self, length: u16) {
        self.packet_length.set(length);
    }

    pub const fn raw_payload_type(&self) -> u16 {
        self.raw_payload_type.get()
    }

    pub const fn payload_type(&self) -> Option<PayloadType> {
        PayloadType::from_u16(self.raw_payload_type.get())
    }

    pub fn set_raw_payload_type(&mut self, raw_payload_type: u16) {
        self.raw_payload_type.set(raw_payload_type);
    }

    /// Theoretical payload length. Calculated from [`Self::packet_length`] and [`Self::size`].
    pub const fn payload_length(&self) -> usize {
        self.packet_length.get() as usize - Self::size()
    }

    pub const fn checksum(&self) -> u32 {
        self.checksum.get()
    }

    pub fn clear_checksum(&mut self) {
        self.checksum.set(0);
    }

    pub fn set_checksum(&mut self, checksum: u32) {
        self.checksum.set(checksum);
    }

    pub fn make_ready_for_checksum(&mut self, payload: &Payload<'_>, payload_length: usize) {
        self.set_packet_length(Self::size() as u16 + payload_length as u16);
        self.set_raw_payload_type(payload.payload_type() as u16);
        self.clear_checksum();
    }

    pub fn maybe_mut_header_from_prefix(src: &mut [u8]) -> Option<(&mut Self, &mut [u8])> {
        Header::mut_from_prefix(src).ok()
    }
}
