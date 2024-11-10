//! Payload content module.

use derive_more::derive::From;
use serde::{Deserialize, Serialize};

use super::payload_type::PayloadType;

/// The content of a payload.
#[derive(Debug, Clone, PartialEq, Serialize, From)]
#[serde(untagged)]
pub enum PayloadContent<'a> {
    /// Init the connection
    Init(Init<'a>),
    /// Acknowledge the Init.
    InitAck(InitAck<'a>),
    /// Heartbeat to keep the connection alive.
    Heartbeat(Heartbeat),
    /// Acknowledge the Heartbeat.
    HeartbeatAck(HeartbeatAck),
    /// Device configuration.
    DeviceConfig(DeviceConfig<'a>),
    /// Acknowledge the Device configuration.
    DeviceConfigAck(DeviceConfigAck),
}

/// Init the connection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Init<'a> {
    /// The sequence number to identify the packet.
    pub sequence_number: u32,
    /// Version of the protocol.
    pub version: &'a str,
}

/// Acknowledge the Init.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitAck<'a> {
    /// The sequence number to identify the packet.
    pub sequence_number: u32,
    /// Version of the protocol.
    pub version: &'a str,
}

/// Heartbeat to keep the connection alive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heartbeat {
    /// The sequence number to identify the packet.
    pub sequence_number: u32,
}

/// Acknowledge the Heartbeat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatAck {
    /// The sequence number to identify the packet.
    pub sequence_number: u32,
}

/// Device configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig<'a> {
    /// The sequence number to identify the packet.
    pub sequence_number: u32,
    /// The configuration of the device.
    pub config: &'a str,
}

/// Acknowledge the Device configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfigAck {
    /// The sequence number to identify the packet.
    pub sequence_number: u32,
}

impl PayloadContent<'_> {
    /// Returns the payload type associated with the content.
    pub const fn payload_type(&self) -> PayloadType {
        match self {
            PayloadContent::Init(_) => PayloadType::Init,
            PayloadContent::InitAck(_) => PayloadType::InitAck,
            PayloadContent::Heartbeat(_) => PayloadType::Heartbeat,
            PayloadContent::HeartbeatAck(_) => PayloadType::HeartbeatAck,
            PayloadContent::DeviceConfig(_) => PayloadType::DeviceConfig,
            PayloadContent::DeviceConfigAck(_) => PayloadType::DeviceConfigAck,
        }
    }
}
