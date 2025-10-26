use steel_macros::{CBoundPacket, PacketWrite};
use steel_registry::packets::clientbound::config::CLIENTBOUND_SELECT_KNOWN_PACKS;

use crate::packets::shared_implementation::KnownPack;

#[derive(PacketWrite, CBoundPacket, Clone, Debug)]
#[packet_id(CONFIGURATION = "CLIENTBOUND_SELECT_KNOWN_PACKS")]
pub struct CSelectKnownPacks {
    #[write_as(as = "vec")]
    pub packs: Vec<KnownPack>,
}

impl CSelectKnownPacks {
    pub fn new(packs: Vec<KnownPack>) -> Self {
        Self { packs }
    }
}
