//! Payload type module.

/// The payload type of the packet.
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum PayloadType {
    /// Init the connection.
    Init = 1,
    /// Acknowledge the Init.
    InitAck = 2,
    /// Heartbeat to keep the connection alive.
    Heartbeat = 3,
    /// Acknowledge the Heartbeat.
    HeartbeatAck = 4,
    /// Device configuration.
    DeviceConfig = 5,
    /// Acknowledge the Device configuration.
    DeviceConfigAck = 6,
}

impl PayloadType {
    /// Converts the given u16 to an optional payload type.
    pub const fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::Init),
            2 => Some(Self::InitAck),
            3 => Some(Self::Heartbeat),
            4 => Some(Self::HeartbeatAck),
            5 => Some(Self::DeviceConfig),
            6 => Some(Self::DeviceConfigAck),
            _ => None,
        }
    }
}
