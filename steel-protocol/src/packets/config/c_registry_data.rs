use simdnbt::owned::NbtTag;
use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::config::C_REGISTRY_DATA;
use steel_utils::ResourceLocation;

#[derive(Clone, Debug, WriteTo)]
pub struct RegistryEntry {
    pub id: ResourceLocation,
    #[write_as(as = "option")]
    pub data: Option<NbtTag>,
}

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Config = "C_REGISTRY_DATA")]
pub struct CRegistryData {
    pub registry: ResourceLocation,
    #[write_as(as = "vec")]
    pub entries: Vec<RegistryEntry>,
}

impl CRegistryData {
    pub fn new(registry: ResourceLocation, entries: Vec<RegistryEntry>) -> Self {
        Self { registry, entries }
    }
}

impl RegistryEntry {
    pub fn new(id: ResourceLocation, data: Option<NbtTag>) -> Self {
        Self { id, data }
    }
}
