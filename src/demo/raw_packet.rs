//! Raw packet module.

use zerocopy::{FromBytes, Immutable, KnownLayout};

use super::{header::Header, payload::Payload};

/// A raw packet that contains a header and a payload.
#[derive(FromBytes, KnownLayout, Immutable, Debug)]
#[repr(C)]
pub struct RawPacket {
    /// The header of the packet.
    header: Header,
    /// Might contain less or more bytes than the actual payload.
    raw_payload: [u8],
}

impl RawPacket {
    /// Returns a reference to the header.
    pub const fn header(&self) -> &Header {
        &self.header
    }

    /// Returns a reference to the raw payload.
    pub const fn raw_payload(&self) -> &[u8] {
        &self.raw_payload
    }

    /// Returns a reference to the payload bytes.
    pub fn payload_bytes(&self) -> &[u8] {
        &self.raw_payload[..self.header.payload_length()]
    }

    /// Theoretical payload length as per the header.
    pub const fn payload_length(&self) -> usize {
        self.header.packet_length() as usize - Header::size()
    }

    /// Writes the given payload to the given destination buffer.
    pub fn write_to(payload: &Payload<'_>, dst: &mut [u8]) -> Result<usize, RawPacketWriteError> {
        let packet_length = match Header::mut_from_prefix(dst) {
            Err(_) => return Err(RawPacketWriteError::HeaderWrite),
            Ok((header, rest)) => match payload.write_to(rest) {
                Err(_) => return Err(RawPacketWriteError::PayloadWrite),
                Ok(payload_length) => {
                    header.make_ready_for_checksum(payload, payload_length);
                    header.packet_length_usize()
                }
            },
        };

        let checksum = Header::calculate_checksum(&dst[..packet_length]);

        let (header, _) = Header::mut_from_prefix(dst).expect("We just checked this");

        header.set_checksum(checksum);

        Ok(packet_length)
    }

    /// Returns a reference to a raw packet if the given slice starts with a valid raw packet.
    pub fn maybe_raw_packet_from_prefix(
        src: &mut [u8],
    ) -> Result<Option<&Self>, RawPacketFromSliceError> {
        match Header::maybe_mut_header_from_prefix(src) {
            None => Ok(None),
            Some((header, rest)) => {
                let packet_length = header.packet_length_usize();
                let payload_length = header.payload_length();

                if rest.len() < payload_length {
                    return Ok(None);
                }

                let received_checksum = header.checksum();

                header.clear_checksum();

                let calculated_checksum = Header::calculate_checksum(&src[..packet_length]);

                if received_checksum != calculated_checksum {
                    return Err(RawPacketFromSliceError::Checksum);
                }

                match RawPacket::ref_from_bytes(src) {
                    Err(_) => Ok(None),
                    Ok(raw_packet) => Ok(Some(raw_packet)),
                }
            }
        }
    }
}

/// Error returned by [`RawPacket::write_to`].
#[derive(Debug)]
pub enum RawPacketWriteError {
    /// Failed to write header.
    HeaderWrite,
    /// Failed to write payload.
    PayloadWrite,
}

/// Error returned by [`RawPacket::maybe_raw_packet_from_prefix`].
#[derive(Debug)]
pub enum RawPacketFromSliceError {
    /// Invalid checksum.
    Checksum,
}
