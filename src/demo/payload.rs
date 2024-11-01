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

    #[allow(clippy::result_unit_err)]
    pub fn write_to(&self, dst: &mut [u8]) -> Result<usize, ()> {
        serde_json_core::to_slice(&self.content, dst).map_err(|_| ())
    }

    fn maybe_payload_content_from_json_slice_mapped<T>(
        src: &'a [u8],
    ) -> Option<(PayloadContent<'a>, usize)>
    where
        T: Deserialize<'a>,
        PayloadContent<'a>: From<T>,
    {
        serde_json_core::from_slice::<T>(src)
            .ok()
            .map(|(de, size)| (PayloadContent::from(de), size))
    }

    pub fn maybe_payload_from_json_slice(
        payload_type: PayloadType,
        src: &'a [u8],
    ) -> Option<(Self, usize)> {
        let (content, size) = match payload_type {
            PayloadType::Init => {
                Self::maybe_payload_content_from_json_slice_mapped::<Init<'a>>(src)
            }

            PayloadType::InitAck => {
                Self::maybe_payload_content_from_json_slice_mapped::<InitAck<'a>>(src)
            }
            PayloadType::Heartbeat => {
                Self::maybe_payload_content_from_json_slice_mapped::<Heartbeat>(src)
            }
            PayloadType::HeartbeatAck => {
                Self::maybe_payload_content_from_json_slice_mapped::<HeartbeatAck>(src)
            }
            PayloadType::DeviceConfig => {
                Self::maybe_payload_content_from_json_slice_mapped::<DeviceConfig<'a>>(src)
            }
            PayloadType::DeviceConfigAck => {
                Self::maybe_payload_content_from_json_slice_mapped::<DeviceConfigAck>(src)
            }
        }?;

        Some((Self { content }, size))
    }
}
