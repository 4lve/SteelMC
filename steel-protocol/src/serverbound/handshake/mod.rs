use std::io::Write;

use crate::{
    packet_traits::{PacketRead, PacketWrite},
};
use bytes::Buf;
use steel_macros::{PacketRead, PacketWrite};

#[derive(Clone, Copy, PartialEq, Eq, PacketRead, PacketWrite)]
#[write_as(as = "var_int")]
pub enum ClientIntent {
    STATUS = 1,
    LOGIN = 2,
    TRANSFER = 3,
}

#[derive(PacketRead, PacketWrite)]
pub struct ClientIntentionPacket {
    #[read_as(as = "var_int")]
    #[write_as(as = "var_int")]
    pub protocol_version: i32,
    #[read_as(as = "string", bound = 255)]
    pub hostname: String,
    pub port: u16,
    pub intention: ClientIntent,
}
