use derive_more::derive::From;

use super::{
    header::Header,
    payload::{Payload, PayloadFromSliceError},
    raw_packet::{RawPacket, RawPacketFromSliceError, RawPacketWriteError},
};

#[derive(Debug, From)]
pub enum PacketWriteError {
    /// Failed to write raw packet.
    RawPacket(RawPacketWriteError),
}

#[derive(Debug, From)]
pub enum PacketFromSliceError {
    /// Invalid raw packet.
    RawPacket(RawPacketFromSliceError),
    /// Unknown payload type.
    UnknownPayloadType,
    /// Invalid payload.
    Payload(PayloadFromSliceError),
}

#[derive(Debug, PartialEq)]
pub struct Packet<'a> {
    payload: Payload<'a>,
}

impl<'a> Packet<'a> {
    pub const fn new(payload: Payload<'a>) -> Self {
        Self { payload }
    }

    pub const fn payload(&self) -> &Payload<'a> {
        &self.payload
    }

    pub fn write_to(&self, dst: &mut [u8]) -> Result<usize, PacketWriteError> {
        Ok(RawPacket::write_to(&self.payload, dst)?)
    }

    pub fn maybe_packet_from_prefix(
        src: &'a mut [u8],
    ) -> Result<Option<(Packet<'a>, usize)>, PacketFromSliceError> {
        match RawPacket::maybe_raw_packet_from_prefix(src) {
            Err(err) => Err(PacketFromSliceError::RawPacket(err)),
            Ok(None) => Ok(None),
            Ok(Some(raw_packet)) => {
                let payload_type = raw_packet
                    .header()
                    .payload_type()
                    .ok_or(PacketFromSliceError::UnknownPayloadType)?;

                let (payload, payload_size) = Payload::<'a>::payload_from_json_slice(
                    payload_type,
                    raw_packet.payload_bytes(),
                )?;

                let packet_length = Header::size() + payload_size;

                Ok(Some((Packet { payload }, packet_length)))
            }
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use crate::demo::payload_content::{DeviceConfig, PayloadContent};

    use super::*;

    #[test]
    fn encode_decode() {
        let buf = &mut [0; 100];

        let packet = Packet::new(Payload::new(PayloadContent::DeviceConfig(DeviceConfig {
            sequence_number: 12,
            config: "config",
        })));

        let written = packet.write_to(buf).expect("Must be ok");

        let (reconstructed, read) = Packet::maybe_packet_from_prefix(buf)
            .expect("Must be ok")
            .expect("Must be some");

        assert_eq!(written, read);
        assert_eq!(reconstructed, packet);
    }
}
