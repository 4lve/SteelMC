//! Sky light engine with empty section propagation optimization.

use crate::chunk::{
    light_storage::LightStorage,
    paletted_container::{BlockPalette, PalettedContainer},
    section::Sections,
};
use steel_utils::{BlockStateId, ChunkPos};

/// Sky light engine with optimizations for vertical light propagation.
pub struct SkyLightEngine {
    // Base light engine could be added here if needed for more complex propagation
}

impl SkyLightEngine {
    /// Creates a new sky light engine.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }

    /// Checks if a section is completely empty (all air blocks).
    fn is_section_empty(section: &BlockPalette) -> bool {
        match section {
            PalettedContainer::Homogeneous(block_state) => *block_state == BlockStateId(0),
            PalettedContainer::Heterogeneous(_) => false,
        }
    }

    /// Propagates light from empty sections in a single operation.
    ///
    /// This is the core optimization: when encountering an empty section,
    /// instead of propagating block-by-block, we skip to the bottom of all
    /// consecutive empty sections below.
    pub fn propagate_from_empty_sections(
        &mut self,
        _chunk_pos: ChunkPos,
        sections: &Sections,
        _chunk_min_y: i32,
    ) {
        let num_sections = sections.sections.len();

        // Start from top, find first non-empty section
        let mut top_section = None;
        for idx in (0..num_sections).rev() {
            let section = sections.sections[idx].read();
            if !Self::is_section_empty(&section.states) {
                top_section = Some(idx);
                break;
            }
        }

        // If all sections are empty (all air), fill everything with light 15
        let Some(top_section) = top_section else {
            for idx in 1..=num_sections {
                if idx < sections.sky_light.len() {
                    *sections.sky_light[idx].write() = LightStorage::new_filled(15);
                }
            }
            return;
        };

        // Fill all sections above top_section with full sky light (15)
        for idx in (top_section + 1)..num_sections {
            let light_idx = idx + 1; // +1 for padding
            if light_idx < sections.sky_light.len() {
                *sections.sky_light[light_idx].write() = LightStorage::new_filled(15);
            }
        }

        // Track which columns are still active (still propagating light downward)
        // 16x16 = 256 columns, indexed by z * 16 + x
        let mut column_active = [true; 256];

        // Process from top_section downward, tracking column state
        for section_idx in (0..=top_section).rev() {
            let section = sections.sections[section_idx].read();

            // Check if this entire section is empty
            if Self::is_section_empty(&section.states) {
                // Fast path: fill entire section with light 15
                let light_idx = section_idx + 1; // +1 for padding
                if light_idx < sections.sky_light.len() {
                    *sections.sky_light[light_idx].write() = LightStorage::new_filled(15);
                }
                // All columns remain active
            } else {
                // Process this section column by column
                let light_idx = section_idx + 1; // +1 for padding

                // Acquire write lock once for the entire section
                let mut sky_light_section = sections.sky_light[light_idx].write();

                for z in 0..16 {
                    for x in 0..16 {
                        let col_idx = z * 16 + x;

                        // Skip columns that have already hit solid blocks
                        if !column_active[col_idx] {
                            continue;
                        }

                        // Propagate downward in this column through this section
                        for y in (0..16).rev() {
                            let block_state = section.states.get(x, y, z);
                            let is_air = block_state == BlockStateId(0);

                            if is_air {
                                sky_light_section.set(x, y, z, 15);
                            } else {
                                // Hit solid block, mark column as terminated
                                sky_light_section.set(x, y, z, 0);
                                column_active[col_idx] = false;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Propagates sky light and updates source tracking.
    ///
    /// This method propagates the sky light, then the caller should update sources separately.
    pub fn propagate_with_sources(
        &mut self,
        chunk_pos: ChunkPos,
        sections: &mut Sections,
        chunk_min_y: i32,
    ) {
        // Use the optimized empty section propagation
        self.propagate_from_empty_sections(chunk_pos, sections, chunk_min_y);

        // Note: Caller should update sources after this call by calling:
        // sections.sky_light_sources.update_from_chunk_sections(&sections.sections, chunk_min_y);
    }
}

impl Default for SkyLightEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::section::ChunkSection;

    #[test]
    fn test_is_section_empty() {
        let empty = PalettedContainer::Homogeneous(BlockStateId(0));
        assert!(SkyLightEngine::is_section_empty(&empty));

        let non_empty = PalettedContainer::Homogeneous(BlockStateId(1));
        assert!(!SkyLightEngine::is_section_empty(&non_empty));
    }

    #[test]
    fn test_all_air_chunk() {
        let mut engine = SkyLightEngine::new();

        // Create a chunk with all air sections
        let num_sections = 24; // -64 to 320 in 16-block sections
        let sections_vec: Vec<ChunkSection> = (0..num_sections)
            .map(|_| ChunkSection::new_empty())
            .collect();

        let sections = Sections::from_owned(sections_vec.into_boxed_slice());

        engine.propagate_from_empty_sections(
            ChunkPos(steel_utils::math::Vector2::new(0, 0)),
            &sections,
            -64,
        );

        // All sections should be filled with light 15
        for idx in 1..=num_sections {
            let light = sections.sky_light[idx].read().get(0, 0, 0);
            assert_eq!(light, 15, "Section {idx} should have light 15");
        }
    }
}
