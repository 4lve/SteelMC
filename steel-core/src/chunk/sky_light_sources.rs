//! Sky light source tracking for chunk columns.

/// Tracks the lowest Y-coordinate where sky light enters each column of a chunk.
///
/// This structure maintains a 16x16 grid (one entry per XZ column) indicating
/// the lowest block position where sky light (level 15) enters from above.
/// This allows the lighting engine to skip unnecessary work when propagating
/// sky light.
///
/// # Storage Format
///
/// Heights are stored as Y-coordinates relative to world minimum Y (typically -64).
/// A value of 0 means the lowest possible Y, higher values mean higher positions.
///
/// Special value: `i32::MIN` indicates "no sky light source" (column is fully occluded).
#[derive(Debug, Clone)]
pub struct ChunkSkyLightSources {
    /// Minimum Y coordinate of the world.
    min_y: i32,

    /// Maximum Y coordinate of the world.
    max_y: i32,

    /// Sky light source heights for each column (16x16 = 256 entries).
    /// Stored in Z-major order: index = z * 16 + x
    heights: Box<[i32; 256]>,
}

impl ChunkSkyLightSources {
    /// Creates a new sky light sources tracker.
    ///
    /// # Arguments
    /// * `min_y` - World minimum Y coordinate (typically -64)
    /// * `max_y` - World maximum Y coordinate (typically 320)
    #[must_use]
    pub fn new(min_y: i32, max_y: i32) -> Self {
        Self {
            min_y,
            max_y,
            heights: Box::new([max_y; 256]),
        }
    }

    /// Creates an empty sources tracker (all columns fully occluded).
    #[must_use]
    pub fn empty(min_y: i32, max_y: i32) -> Self {
        Self {
            min_y,
            max_y,
            heights: Box::new([i32::MIN; 256]),
        }
    }

    /// Gets the sky light source Y-coordinate for a column.
    ///
    /// # Arguments
    /// * `x` - Column X coordinate (0-15)
    /// * `z` - Column Z coordinate (0-15)
    ///
    /// # Returns
    /// The Y-coordinate where sky light enters, or `i32::MIN` if fully occluded.
    #[inline]
    #[must_use]
    pub fn get(&self, x: usize, z: usize) -> i32 {
        debug_assert!(x < 16 && z < 16, "Column coordinates must be 0-15");
        self.heights[z * 16 + x]
    }

    /// Sets the sky light source Y-coordinate for a column.
    ///
    /// # Arguments
    /// * `x` - Column X coordinate (0-15)
    /// * `z` - Column Z coordinate (0-15)
    /// * `y` - Y-coordinate where sky light enters
    #[inline]
    pub fn set(&mut self, x: usize, z: usize, y: i32) {
        debug_assert!(x < 16 && z < 16, "Column coordinates must be 0-15");
        debug_assert!(
            y == i32::MIN || (y >= self.min_y && y <= self.max_y),
            "Y coordinate must be within world bounds or i32::MIN"
        );
        self.heights[z * 16 + x] = y;
    }

    /// Updates sky light sources based on chunk sections.
    ///
    /// Scans from top to bottom to find the highest solid block in each column,
    /// then sets the sky light source to one block above that position.
    pub fn update_from_chunk_sections(
        &mut self,
        chunk_sections: &[std::sync::Arc<
            steel_utils::locks::SyncRwLock<crate::chunk::section::ChunkSection>,
        >],
        chunk_min_y: i32,
    ) {
        use crate::chunk::paletted_container::PalettedContainer;
        use steel_utils::BlockStateId;

        let num_sections = chunk_sections.len();

        for z in 0..16 {
            for x in 0..16 {
                let mut sky_source_y = self.max_y;
                let mut found_solid = false;

                // Scan from top to bottom
                for section_idx in (0..num_sections).rev() {
                    let section = chunk_sections[section_idx].read();
                    let section_base_y = chunk_min_y + (section_idx as i32 * 16);

                    // Check if section is all air - skip if so
                    let is_all_air = match &section.states {
                        PalettedContainer::Homogeneous(id) => *id == BlockStateId(0),
                        PalettedContainer::Heterogeneous(_) => false,
                    };

                    if is_all_air {
                        continue;
                    }

                    // Scan this section from top to bottom
                    for y in (0..16).rev() {
                        let block_state = section.states.get(x, y, z);
                        let is_air = block_state == BlockStateId(0);

                        if !is_air {
                            // Found highest solid block
                            sky_source_y = section_base_y + y as i32 + 1;
                            found_solid = true;
                            break;
                        }
                    }

                    if found_solid {
                        break;
                    }
                }

                // If no solid block found, sky light comes from above max_y
                // If solid block found, sky light comes from one block above it
                self.set(x, z, sky_source_y);
            }
        }
    }

    /// Checks if a column has sky light access.
    #[inline]
    #[must_use]
    pub fn has_sky_light(&self, x: usize, z: usize) -> bool {
        self.get(x, z) != i32::MIN
    }

    /// Gets the minimum Y across all columns (lowest sky light entry point).
    #[must_use]
    pub fn min_source_y(&self) -> i32 {
        let mut min = self.max_y;
        for &height in self.heights.iter() {
            if height != i32::MIN && height < min {
                min = height;
            }
        }
        min
    }

    /// Gets the maximum Y across all columns (highest sky light entry point).
    #[must_use]
    pub fn max_source_y(&self) -> i32 {
        let mut max = self.min_y;
        for &height in self.heights.iter() {
            if height != i32::MIN && height > max {
                max = height;
            }
        }
        max
    }
}

impl Default for ChunkSkyLightSources {
    fn default() -> Self {
        // Default to standard Minecraft world height
        Self::new(-64, 320)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_set() {
        let mut sources = ChunkSkyLightSources::new(-64, 320);

        sources.set(0, 0, 100);
        assert_eq!(sources.get(0, 0), 100);

        sources.set(15, 15, -64);
        assert_eq!(sources.get(15, 15), -64);
    }

    #[test]
    fn test_empty() {
        let sources = ChunkSkyLightSources::empty(-64, 320);

        for z in 0..16 {
            for x in 0..16 {
                assert_eq!(sources.get(x, z), i32::MIN);
                assert!(!sources.has_sky_light(x, z));
            }
        }
    }

    #[test]
    fn test_min_max_source_y() {
        // Start with empty sources (all i32::MIN)
        let mut sources = ChunkSkyLightSources::empty(-64, 320);

        sources.set(0, 0, 100);
        sources.set(5, 5, 50);
        sources.set(15, 15, 200);

        assert_eq!(sources.min_source_y(), 50);
        assert_eq!(sources.max_source_y(), 200);
    }
}
