use zerocopy::{FromBytes, Immutable, KnownLayout};

use super::{header::Header, payload::Payload};

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

                let recieved_checksum = header.checksum();

                header.clear_checksum();

                let calculated_checksum = Header::calculate_checksum(&src[..packet_length]);

                if recieved_checksum != calculated_checksum {
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

#[derive(Debug)]
pub enum RawPacketWriteError {
    /// Failed to write header.
    HeaderWrite,
    /// Failed to write payload.
    PayloadWrite,
}

#[derive(Debug)]
pub enum RawPacketFromSliceError {
    /// Invalid checksum.
    Checksum,
}
