//! Payload module.

use derive_more::derive::From;
use serde::Deserialize;

use crate::demo::payload_content::{Init, InitAck};

use super::{
    payload_content::{DeviceConfig, DeviceConfigAck, Heartbeat, HeartbeatAck, PayloadContent},
    payload_type::PayloadType,
};

/// A payload that contains some content.
#[derive(Debug, Clone, PartialEq)]
pub struct Payload<'a> {
    /// The content of the payload.
    pub content: PayloadContent<'a>,
}

impl<'a> Payload<'a> {
    /// Creates a new payload with the given content.
    pub const fn new_raw(content: PayloadContent<'a>) -> Self {
        Self { content }
    }

    /// Creates a new payload with the given content.
    pub fn new(content: impl Into<PayloadContent<'a>>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Returns the payload type.
    pub const fn payload_type(&self) -> PayloadType {
        self.content.payload_type()
    }

    /// Writes the payload to the given destination buffer.
    pub fn write_to(&self, dst: &mut [u8]) -> Result<usize, PayloadWriteError> {
        serde_json_core::to_slice(&self.content, dst).map_err(PayloadWriteError::Serialize)
    }

    /// Returns a payload (mapped from payload content) from the given JSON slice.
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

    /// Returns a payload from the given JSON slice.
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

/// Error returned by [`Payload::write_to`].
#[derive(Debug, From)]
pub enum PayloadWriteError {
    /// Serialization error.
    Serialize(serde_json_core::ser::Error),
}

/// Error returned by [`Payload::payload_from_json_slice`].
#[derive(Debug, From)]
pub enum PayloadFromSliceError {
    /// Deserialization error.
    Deserialize(serde_json_core::de::Error),
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;

    #[test]
    fn encode_decode() {
        let buf = &mut [0; 100];

        let payload = Payload::new_raw(PayloadContent::DeviceConfig(DeviceConfig {
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
