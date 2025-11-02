use std::{collections::HashMap, sync::Arc};

use steel_protocol::{codec::VarInt, packet_traits::CBoundPacket, packets::{common::c_update_tags_packet::CUpdateTagsPacket,  configuration::c_registry_data_packet::{CRegistryDataPacket, RegistryEntry}}, utils::ConnectionProtocol};
use steel_registry::{
    Registry, BANNER_PATTERN_REGISTRY, BIOMES_REGISTRY, BLOCKS_REGISTRY, CAT_VARIANT_REGISTRY,
    CHAT_TYPE_REGISTRY, CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, DAMAGE_TYPE_REGISTRY,
    DIMENSION_TYPE_REGISTRY, FROG_VARIANT_REGISTRY, INSTRUMENT_REGISTRY, ITEMS_REGISTRY,
    JUKEBOX_SONG_REGISTRY, PAINTING_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY,
    TRIM_MATERIAL_REGISTRY, TRIM_PATTERN_REGISTRY, WOLF_SOUND_VARIANT_REGISTRY,
    WOLF_VARIANT_REGISTRY,
};
use steel_utils::ResourceLocation;
use steel_protocol::packet_traits::EncodedPacket;

use crate::STEEL_CONFIG;

pub struct RegistryCache {
    pub compressed_registry_packets: Arc<Vec<EncodedPacket>>,
    pub compressed_tags_packet: Arc<EncodedPacket>,
}

impl RegistryCache {
    pub async fn new(registry: &Registry) -> Self {
        let registry_packets = Self::build_registry_packets(registry);
        let tags_by_registry_packet = Self::build_tags_packet(registry);

        let (compressed_registry_packets, compressed_tags_packet) =
            build_compressed_packets(registry_packets, tags_by_registry_packet).await;

        Self {
            compressed_registry_packets: Arc::new(compressed_registry_packets),
            compressed_tags_packet: Arc::new(compressed_tags_packet),
        }
    }

    fn build_registry_packets(registry: &Registry) -> Vec<CRegistryDataPacket> {
        let mut packets = Vec::new();

        macro_rules! add_registry {
            ($reg_key:expr, $field:ident) => {
                packets.push(CRegistryDataPacket::new(
                    $reg_key,
                    registry
                        .$field
                        .iter()
                        .map(|(_, entry)| RegistryEntry::new(entry.key.clone(), None))
                        .collect(),
                ));
            };
        }

        //TODO: For non vanilla entries we need to encode the data into nbt

        add_registry!(BIOMES_REGISTRY, biomes);
        add_registry!(CHAT_TYPE_REGISTRY, chat_types);
        add_registry!(TRIM_PATTERN_REGISTRY, trim_patterns);
        add_registry!(TRIM_MATERIAL_REGISTRY, trim_materials);
        add_registry!(WOLF_VARIANT_REGISTRY, wolf_variants);
        add_registry!(WOLF_SOUND_VARIANT_REGISTRY, wolf_sound_variants);
        add_registry!(PIG_VARIANT_REGISTRY, pig_variants);
        add_registry!(FROG_VARIANT_REGISTRY, frog_variants);
        add_registry!(CAT_VARIANT_REGISTRY, cat_variants);
        add_registry!(COW_VARIANT_REGISTRY, cow_variants);
        add_registry!(CHICKEN_VARIANT_REGISTRY, chicken_variants);
        add_registry!(PAINTING_VARIANT_REGISTRY, painting_variants);
        add_registry!(DIMENSION_TYPE_REGISTRY, dimension_types);
        add_registry!(DAMAGE_TYPE_REGISTRY, damage_types);
        add_registry!(BANNER_PATTERN_REGISTRY, banner_patterns);

        // TODO: Add enchantments when implemented in the registry
        //add_registry!(ResourceLocation::vanilla_static("enchantments"), enchantments);
        
        add_registry!(JUKEBOX_SONG_REGISTRY, jukebox_songs);
        add_registry!(INSTRUMENT_REGISTRY, instruments);

        packets
    }

    fn build_tags_packet(registry: &Registry) -> CUpdateTagsPacket {
        let mut tags_by_registry: HashMap<ResourceLocation, HashMap<ResourceLocation, Vec<VarInt>>> =
            HashMap::new();

        // Build block tags
        let mut block_tags: HashMap<ResourceLocation, Vec<VarInt>> = HashMap::new();
        for tag_key in registry.blocks.tag_keys() {
            let mut block_ids = Vec::new();

            for block in registry.blocks.iter_tag(tag_key) {
                let block_id = *registry.blocks.get_id(block);
                block_ids.push(VarInt(block_id as i32));
            };

            block_tags.insert(tag_key.clone(), block_ids);
        };

        tags_by_registry.insert(BLOCKS_REGISTRY, block_tags);


        // Build item tags
        let mut item_tags: HashMap<ResourceLocation, Vec<VarInt>> = HashMap::new();
        for tag_key in registry.items.tag_keys() {
            let mut item_ids = Vec::new();

            for item in registry.items.iter_tag(tag_key) {
                let item_id = *registry.items.get_id(item);
                item_ids.push(VarInt(item_id as i32));
            };

            item_tags.insert(tag_key.clone(), item_ids);
        };

        tags_by_registry.insert(ITEMS_REGISTRY, item_tags);

        // Build and return a CUpdateTagsPacket based on the registry data
        CUpdateTagsPacket::new(tags_by_registry)
    }
}

pub async fn convert_into_compressed_packet<P: CBoundPacket>(
    packet: P,
) -> Result<EncodedPacket, ()> {
    let compression_info = STEEL_CONFIG.compression;

    let encoded_packet = EncodedPacket::from_packet(
        &packet,
        compression_info,
        ConnectionProtocol::CONFIGURATION,
    )
    .await
    .map_err(|_| {
        log::error!(
            "Failed to encode packet: {:?}",
            packet.get_id(ConnectionProtocol::CONFIGURATION)
        );
    })?;

    Ok(encoded_packet)
}

pub async fn build_compressed_packets(registry_packets: Vec<CRegistryDataPacket>, tags_packet: CUpdateTagsPacket) -> (Vec<EncodedPacket>, EncodedPacket) {
    let mut compressed_registry_packets = Vec::new();

    for packet in registry_packets {
        compressed_registry_packets.push(convert_into_compressed_packet(packet).await.unwrap());
    };

    let compressed_tags_packet = convert_into_compressed_packet(tags_packet).await.unwrap();

    (compressed_registry_packets, compressed_tags_packet)
}
