use crate::chunk::chunk_access::ChunkStatus;
use crate::chunk::chunk_pyramid::GENERATION_PYRAMID;
use crate::chunk::chunk_tracker::MAX_LEVEL;

/// Utilities for converting between chunk levels and statuses.
pub struct ChunkLevel;

impl ChunkLevel {
    /// Ticket levels at or below this threshold require a full chunk.
    pub const FULL_STATUS_LEVEL: u8 = 33;

    /// Returns the generation status for the given level.
    #[must_use]
    pub fn generation_status(level: u8) -> Option<ChunkStatus> {
        if level >= MAX_LEVEL {
            None
        } else if level <= Self::FULL_STATUS_LEVEL {
            Some(ChunkStatus::Full)
        } else {
            let distance = (level - Self::FULL_STATUS_LEVEL) as usize;

            let deps = &GENERATION_PYRAMID
                .get_step_to(ChunkStatus::Full)
                .accumulated_dependencies;

            let max_distance = deps.get_radius();
            let clamped_distance = distance.min(max_distance);

            deps.get(clamped_distance)
        }
    }

    /// Returns the full status for the given level.
    #[must_use]
    pub fn full_status(level: u8) -> Option<ChunkStatus> {
        Self::generation_status(level)
    }
}
