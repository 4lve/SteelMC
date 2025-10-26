use serde::Serialize;
use steel_macros::{CBoundPacket, PacketWrite};
use steel_registry::packets::clientbound::status::CLIENTBOUND_STATUS_RESPONSE;

#[derive(Serialize, Clone, Debug)]
pub struct Sample {
    /// The player's name.
    pub name: String,
    /// The player's UUID.
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Players {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<Sample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Version {
    pub name: &'static str,
    pub protocol: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub description: String,
    pub players: Option<Players>,
    pub version: Option<Version>,
    pub favicon: Option<String>,
    pub enforce_secure_chat: bool,
}

#[derive(PacketWrite, CBoundPacket, Clone, Debug)]
#[packet_id(STATUS = "CLIENTBOUND_STATUS_RESPONSE")]
pub struct CStatusResponsePacket {
    #[write_as(as = "json")]
    status: Status,
}

impl CStatusResponsePacket {
    pub fn new(status: Status) -> Self {
        Self { status }
    }
}
