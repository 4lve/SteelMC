use steel_macros::PacketRead;

#[derive(Clone, Copy, PartialEq, Eq, PacketRead, Debug)]
#[read_as(as = "var_int")]
pub enum ClientIntent {
    STATUS = 1,
    LOGIN = 2,
    TRANSFER = 3,
}

#[derive(PacketRead, Clone, Debug)]
pub struct ClientIntentionPacket {
    #[read_as(as = "var_int")]
    pub protocol_version: i32,
    #[read_as(as = "string", bound = 255)]
    pub hostname: String,
    pub port: u16,
    pub intention: ClientIntent,
}

impl ClientIntentionPacket {
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
