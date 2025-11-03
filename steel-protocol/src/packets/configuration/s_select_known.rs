use steel_macros::PacketRead;

use crate::packets::shared_implementation::KnownPack;

#[derive(PacketRead, Clone, Debug)]
pub struct SSelectKnownPacks {
    #[read_as(as = "vec")]
    pub packs: Vec<KnownPack>,
}
