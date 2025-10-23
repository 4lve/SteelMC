use serde::Serialize;
use steel_macros::PacketWrite;

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
    pub name: String,
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

#[derive(PacketWrite, Clone, Debug)]
pub struct CStatusResponsePacket {
    #[write_as(as = "json")]
    status: Status,
}

impl CStatusResponsePacket {
    pub fn new(status: Status) -> Self {
        Self { status }
    }
}
