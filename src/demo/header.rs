//! Header module.

use crc32fast::Hasher;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, big_endian::U32, byteorder::big_endian::U16,
};

use super::{payload::Payload, payload_type::PayloadType};

/// The header of a packet.
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Debug, Clone)]
#[repr(C)]
pub struct Header {
    /// The length of the packet.
    packet_length: U16,
    /// The type of the payload as a raw u16.
    raw_payload_type: U16,
    /// The crc32 checksum of the packet.
    checksum: U32,
}

impl Header {
    /// Returns the size of the header.
    pub const fn size() -> usize {
        core::mem::size_of::<Header>()
    }

    /// Calculates the checksum of the given data.
    pub fn calculate_checksum(data: &[u8]) -> u32 {
        let mut hasher = Hasher::new();

        hasher.update(data);

        hasher.finalize()
    }

    /// Returns the packet length.
    pub const fn packet_length(&self) -> u16 {
        self.packet_length.get()
    }

    /// Returns the packet length as usize.
    pub const fn packet_length_usize(&self) -> usize {
        self.packet_length() as usize
    }

    /// Sets the packet length.
    pub fn set_packet_length(&mut self, length: u16) {
        self.packet_length.set(length);
    }

    /// Returns the raw payload type.
    pub const fn raw_payload_type(&self) -> u16 {
        self.raw_payload_type.get()
    }

    /// Returns the payload type.
    pub const fn payload_type(&self) -> Option<PayloadType> {
        PayloadType::from_u16(self.raw_payload_type.get())
    }

    /// Sets the raw payload type.
    pub fn set_raw_payload_type(&mut self, raw_payload_type: u16) {
        self.raw_payload_type.set(raw_payload_type);
    }

    /// Theoretical payload length. Calculated from [`Self::packet_length`] and [`Self::size`].
    pub const fn payload_length(&self) -> usize {
        self.packet_length.get() as usize - Self::size()
    }

    /// Returns the checksum.
    pub const fn checksum(&self) -> u32 {
        self.checksum.get()
    }

    /// Clears the checksum.
    pub fn clear_checksum(&mut self) {
        self.checksum.set(0);
    }

    /// Sets the checksum.
    pub fn set_checksum(&mut self, checksum: u32) {
        self.checksum.set(checksum);
    }

    /// Makes the header ready for checksum calculation.
    ///
    /// - Sets the packet length.
    /// - Sets the raw payload type.
    /// - Clears the checksum.
    pub fn make_ready_for_checksum(&mut self, payload: &Payload<'_>, payload_length: usize) {
        self.set_packet_length(Self::size() as u16 + payload_length as u16);
        self.set_raw_payload_type(payload.payload_type() as u16);
        self.clear_checksum();
    }

    /// Returns a reference to the header if the given slice starts with a valid header.
    pub fn maybe_mut_header_from_prefix(src: &mut [u8]) -> Option<(&mut Self, &mut [u8])> {
        Header::mut_from_prefix(src).ok()
    }
}
