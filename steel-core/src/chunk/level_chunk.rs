//! This module contains the `LevelChunk` struct, which is a chunk that is ready to be sent to the client.
use std::{
    io::Cursor,
    sync::{Arc, atomic::AtomicBool},
};

use steel_protocol::packets::game::{
    ChunkPacketData, HeightmapType, Heightmaps, LightUpdatePacketData,
};
use steel_utils::{ChunkPos, codec::BitSet};

use crate::chunk::{proto_chunk::ProtoChunk, section::Sections};

/// A chunk that is ready to be sent to the client.
#[derive(Debug, Clone)]
pub struct LevelChunk {
    /// The sections of the chunk.
    pub sections: Sections,
    /// The position of the chunk.
    pub pos: ChunkPos,
    /// Whether the chunk has been modified since last save.
    pub dirty: Arc<AtomicBool>,
}

impl LevelChunk {
    /// Creates a new `LevelChunk` from a `ProtoChunk`.
    #[must_use]
    pub fn from_proto(proto_chunk: ProtoChunk) -> Self {
        Self {
            sections: proto_chunk.sections,
            pos: proto_chunk.pos,
            dirty: proto_chunk.dirty.clone(),
        }
    }

    /// Creates a new `LevelChunk` that was loaded from disk (not dirty).
    #[must_use]
    pub fn from_disk(sections: Sections, pos: ChunkPos) -> Self {
        Self {
            sections,
            pos,
            dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Extracts the chunk data for sending to the client.
    #[must_use]
    pub fn extract_chunk_data(&self) -> ChunkPacketData {
        let data = Vec::new();

        let mut cursor = Cursor::new(data);
        for section in &self.sections.sections {
            section.read().write(&mut cursor);
        }

        ChunkPacketData {
            heightmaps: Heightmaps {
                heightmaps: vec![
                    (HeightmapType::WorldSurface, vec![0; 37]),
                    (HeightmapType::MotionBlocking, vec![0; 37]),
                    (HeightmapType::MotionBlockingNoLeaves, vec![0; 37]),
                ],
            },
            data: cursor.into_inner(),
            block_entities: Vec::new(),
        }
    }

    /// Extracts only the changed light sections for sending to the client.
    ///
    /// # Arguments
    /// * `sky_changed` - Bit flags indicating which sky light sections changed
    /// * `block_changed` - Bit flags indicating which block light sections changed
    #[must_use]
    pub fn extract_changed_light_data(
        &self,
        sky_changed: u32,
        block_changed: u32,
    ) -> LightUpdatePacketData {
        use crate::chunk::light_storage::LightStorage;

        let section_count = self.sections.sections.len();
        // Note: light storage has section_count + 2 entries (padding above and below)
        let light_section_count = section_count + 1; // We send section_count + 1 sections (skip top padding)

        let mut sky_y_mask = BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());
        let mut block_y_mask = BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());
        let mut empty_sky_y_mask =
            BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());
        let mut empty_block_y_mask =
            BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());

        let mut sky_updates = Vec::new();
        let mut block_updates = Vec::new();

        // Only extract sections that changed
        // Iterate through light storage indices (0 to section_count inclusive)
        for i in 0..light_section_count {
            let has_sky_change = (sky_changed & (1 << i)) != 0;
            let has_block_change = (block_changed & (1 << i)) != 0;

            if has_sky_change && i < self.sections.sky_light.len() {
                // Check if section is empty (all 0s) or has data
                if let LightStorage::Homogeneous(0) = self.sections.sky_light[i] {
                    // Section is empty - set in empty mask, don't send data
                    empty_sky_y_mask.set(i, true);
                } else {
                    // Section has data - set in data mask and send array
                    sky_y_mask.set(i, true);
                    sky_updates.push(self.sections.sky_light[i].to_packet_data());
                }
            }

            if has_block_change && i < self.sections.block_light.len() {
                // Check if section is empty (all 0s) or has data
                if let LightStorage::Homogeneous(0) = self.sections.block_light[i] {
                    // Section is empty - set in empty mask, don't send data
                    empty_block_y_mask.set(i, true);
                } else {
                    // Section has data - set in data mask and send array
                    block_y_mask.set(i, true);
                    block_updates.push(self.sections.block_light[i].to_packet_data());
                }
            }
        }

        LightUpdatePacketData {
            sky_y_mask,
            block_y_mask,
            empty_sky_y_mask,
            empty_block_y_mask,
            sky_updates,
            block_updates,
        }
    }

    /// Extracts the light data for sending to the client.
    #[must_use]
    pub fn extract_light_data(&self) -> LightUpdatePacketData {
        let section_count = self.sections.sections.len();
        // We send section_count + 1 sections (indices 0 through section_count inclusive)
        let light_section_count = section_count + 1;
        let mut sky_y_mask = BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());
        let mut block_y_mask = BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());
        let empty_sky_y_mask = BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());
        let empty_block_y_mask =
            BitSet(vec![0; light_section_count.div_ceil(64)].into_boxed_slice());

        let mut sky_updates = Vec::new();
        let mut block_updates = Vec::new();

        // Extract light data from stored sections
        // Note: sky_light and block_light have section_count + 2 entries (padding above/below)
        // Indices: 0 = bottom padding, 1..=section_count = actual sections, section_count+1 = top padding
        // We skip the TOP padding (always Homogeneous(15)) but include bottom padding (can have light if bedrock broken)
        // So we send indices 0 through section_count (inclusive), which is section_count+1 total sections
        for i in 0..=section_count {
            // Set masks to indicate we have light data for this section
            sky_y_mask.set(i, true);
            block_y_mask.set(i, true);

            // Get the packet data for this section (index i maps directly to storage)
            sky_updates.push(self.sections.sky_light[i].to_packet_data());
            block_updates.push(self.sections.block_light[i].to_packet_data());
        }

        LightUpdatePacketData {
            sky_y_mask,
            block_y_mask,
            empty_sky_y_mask,
            empty_block_y_mask,
            sky_updates,
            block_updates,
        }
    }
}
