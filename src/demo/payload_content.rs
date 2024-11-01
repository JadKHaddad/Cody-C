use derive_more::derive::From;
use serde::{Deserialize, Serialize};

use super::payload_type::PayloadType;

#[derive(Debug, Clone, PartialEq, Serialize, From)]
#[serde(untagged)]
pub enum PayloadContent<'a> {
    Init(Init<'a>),
    InitAck(InitAck<'a>),
    Heartbeat(Heartbeat),
    HeartbeatAck(HeartbeatAck),
    DeviceConfig(DeviceConfig<'a>),
    DeviceConfigAck(DeviceConfigAck),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Init<'a> {
    pub sequence_number: u32,
    pub version: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitAck<'a> {
    pub sequence_number: u32,
    pub version: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heartbeat {
    pub sequence_number: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatAck {
    pub sequence_number: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig<'a> {
    pub sequence_number: u32,
    pub config: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfigAck {
    pub sequence_number: u32,
}

impl<'a> PayloadContent<'a> {
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
