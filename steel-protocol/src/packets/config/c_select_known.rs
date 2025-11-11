use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::config::C_SELECT_KNOWN_PACKS;

use crate::packets::shared_implementation::KnownPack;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Config = C_SELECT_KNOWN_PACKS)]
pub struct CSelectKnownPacks {
    #[write(as = "vec")]
    pub packs: Vec<KnownPack>,
}

impl CSelectKnownPacks {
    pub fn new(packs: Vec<KnownPack>) -> Self {
        Self { packs }
    }
}
