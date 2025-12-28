//! Open screen packet.

use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_OPEN_SCREEN;
use steel_utils::text::TextComponent;

/// Opens a container screen on the client.
#[derive(WriteTo, ClientPacket, Clone, Debug)]
#[packet_id(Play = C_OPEN_SCREEN)]
pub struct COpenScreen {
    /// The container ID for this screen.
    #[write(as = VarInt)]
    pub container_id: i32,
    /// The menu type (from the minecraft:menu registry).
    #[write(as = VarInt)]
    pub menu_type: i32,
    /// The title to display on the screen.
    pub title: TextComponent,
}

impl COpenScreen {
    /// Creates a new open screen packet.
    #[must_use]
    pub fn new(container_id: i32, menu_type: i32, title: TextComponent) -> Self {
        Self {
            container_id,
            menu_type,
            title,
        }
    }
}
