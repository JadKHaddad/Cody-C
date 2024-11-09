use derive_more::derive::From;
use serde::Deserialize;

use crate::demo::payload_content::{Init, InitAck};

use super::{
    payload_content::{DeviceConfig, DeviceConfigAck, Heartbeat, HeartbeatAck, PayloadContent},
    payload_type::PayloadType,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Payload<'a> {
    pub content: PayloadContent<'a>,
}

impl<'a> Payload<'a> {
    pub const fn new(content: PayloadContent<'a>) -> Self {
        Self { content }
    }

    pub const fn payload_type(&self) -> PayloadType {
        self.content.payload_type()
    }

    pub fn write_to(&self, dst: &mut [u8]) -> Result<usize, PayloadWriteError> {
        serde_json_core::to_slice(&self.content, dst).map_err(PayloadWriteError::Serialize)
    }

    fn payload_content_from_json_slice_mapped<T>(
        src: &'a [u8],
    ) -> Result<(PayloadContent<'a>, usize), PayloadFromSliceError>
    where
        T: Deserialize<'a>,
        PayloadContent<'a>: From<T>,
    {
        serde_json_core::from_slice::<T>(src)
            .map(|(de, size)| (PayloadContent::from(de), size))
            .map_err(PayloadFromSliceError::Deserialize)
    }

    pub fn payload_from_json_slice(
        payload_type: PayloadType,
        src: &'a [u8],
    ) -> Result<(Self, usize), PayloadFromSliceError> {
        let (content, size) = match payload_type {
            PayloadType::Init => Self::payload_content_from_json_slice_mapped::<Init<'a>>(src),
            PayloadType::InitAck => {
                Self::payload_content_from_json_slice_mapped::<InitAck<'a>>(src)
            }
            PayloadType::Heartbeat => {
                Self::payload_content_from_json_slice_mapped::<Heartbeat>(src)
            }
            PayloadType::HeartbeatAck => {
                Self::payload_content_from_json_slice_mapped::<HeartbeatAck>(src)
            }
            PayloadType::DeviceConfig => {
                Self::payload_content_from_json_slice_mapped::<DeviceConfig<'a>>(src)
            }
            PayloadType::DeviceConfigAck => {
                Self::payload_content_from_json_slice_mapped::<DeviceConfigAck>(src)
            }
        }?;

        Ok((Self { content }, size))
    }
}

#[derive(Debug, From)]
pub enum PayloadWriteError {
    Serialize(serde_json_core::ser::Error),
}

#[derive(Debug, From)]
pub enum PayloadFromSliceError {
    Deserialize(serde_json_core::de::Error),
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;

    #[test]
    fn encode_decode() {
        let buf = &mut [0; 100];

        let payload = Payload::new(PayloadContent::DeviceConfig(DeviceConfig {
            sequence_number: 12,
            config: "config",
        }));

        let written = payload.write_to(buf).expect("Must be ok");

        let (reconstructed, read) =
            Payload::payload_from_json_slice(PayloadType::DeviceConfig, &buf[..written])
                .expect("Must be ok");

        assert_eq!(written, read);
        assert_eq!(reconstructed, payload);
    }
}
