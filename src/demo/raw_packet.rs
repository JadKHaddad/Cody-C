use zerocopy::{FromBytes, Immutable, KnownLayout};

use super::header::Header;

#[derive(FromBytes, KnownLayout, Immutable, Debug)]
#[repr(C)]
pub struct RawPacket {
    header: Header,
    /// Might contain less or more bytes than the actual payload.
    raw_payload: [u8],
}

impl RawPacket {
    pub const fn header(&self) -> &Header {
        &self.header
    }

    pub const fn raw_payload(&self) -> &[u8] {
        &self.raw_payload
    }

    pub fn payload_bytes(&self) -> &[u8] {
        &self.raw_payload[..self.header.payload_length()]
    }

    /// Theoretical payload length as per the header.
    pub const fn payload_length(&self) -> usize {
        self.header.packet_length() as usize - Header::size()
    }

    pub fn maybe_raw_packet_from_prefix(src: &[u8]) -> Option<&Self> {
        match Header::maybe_header_from_prefix(src) {
            Some((header, rest)) => {
                if rest.len() < header.payload_length() {
                    return None;
                }

                RawPacket::ref_from_bytes(src).ok()
            }
            None => None,
        }
    }
}
