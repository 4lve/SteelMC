use steel_macros::{ReadFrom, ServerPacket};

#[derive(Clone, Copy, PartialEq, Eq, ReadFrom, Debug)]
#[read(as = VarInt)]
pub enum ClientIntent {
    STATUS = 1,
    LOGIN = 2,
    TRANSFER = 3,
}

#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SClientIntention {
    #[read(as = VarInt)]
    pub protocol_version: i32,
    #[read(as = Prefixed(VarInt), bound = 255)]
    pub hostname: String,
    pub port: u16,
    pub intention: ClientIntent,
}

impl SClientIntention {
    #[must_use]
    pub fn new(
        protocol_version: i32,
        hostname: String,
        port: u16,
        intention: ClientIntent,
    ) -> Self {
        Self {
            protocol_version,
            hostname,
            port,
            intention,
        }
    }
}
