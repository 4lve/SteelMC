//! Container close packet (clientbound).

use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_CONTAINER_CLOSE;

/// Tells the client to close a container screen.
#[derive(WriteTo, ClientPacket, Clone, Debug)]
#[packet_id(Play = C_CONTAINER_CLOSE)]
pub struct CContainerClose {
    /// The container ID to close.
    #[write(as = VarInt)]
    pub container_id: i32,
}

impl CContainerClose {
    /// Creates a new container close packet.
    #[must_use]
    pub const fn new(container_id: i32) -> Self {
        Self { container_id }
    }
}

