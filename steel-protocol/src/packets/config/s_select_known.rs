use steel_macros::{ReadFrom, ServerPacket};

use crate::packets::shared_implementation::KnownPack;

#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SSelectKnownPacks {
    #[read_as(as = "vec")]
    pub packs: Vec<KnownPack>,
}
