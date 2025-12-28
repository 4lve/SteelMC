//! Container close packet (serverbound).

use std::io::{Read, Result};

use steel_macros::ServerPacket;
use steel_utils::{codec::VarInt, serial::ReadFrom};

/// Client closes a container.
#[derive(ServerPacket, Debug, Clone)]
pub struct SContainerClose {
    /// The container ID being closed.
    pub container_id: i32,
}

impl ReadFrom for SContainerClose {
    fn read(data: &mut impl Read) -> Result<Self> {
        let container_id = VarInt::read(data)?.0;
        Ok(Self { container_id })
    }
}
