//! `QueueEntry` bit-packing system for light propagation.
//!
//! The `QueueEntry` packs all propagation metadata into a single u64:
//! - Bits 0-3: Light level (0-15)
//! - Bits 4-9: Direction flags (6 directions)
//! - Bit 10: Empty shape flag
//! - Bit 11: Increase from emission flag
//!
//! Using u64 (instead of u16) for native word size performance on 64-bit CPUs.

use super::direction::Direction;

/// A queue entry that encodes light propagation information in a bit-packed u64.
///
/// Bit layout:
/// ```text
/// Bit Position:  63.....................12  11  10  9  8  7  6  5  4  3  2  1  0
///                |         Unused        | F | F | D D D D D D | L L L L |
///                                         | | |               |         |
///                                         | | |               |         +-> Light Level (4 bits)
///                                         | | |               +----------> Direction Flags (6 bits)
///                                         | | +----------------------------> Empty Shape Flag
///                                         | +-------------------------------> Increase From Emission Flag
///                                         +---------------------------------> (Unused)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueEntry(u64);

impl QueueEntry {
    /// Mask for light level (bits 0-3)
    const LEVEL_MASK: u64 = 0x0F;

    /// Mask for all direction flags (bits 4-9)
    const DIRECTIONS_MASK: u64 = 0x3F0;

    /// Flag for empty collision shape (bit 10)
    const EMPTY_SHAPE_FLAG: u64 = 0x400;

    /// Flag for increase from light emission (bit 11)
    const EMISSION_FLAG: u64 = 0x800;

    /// Gets the light level from this queue entry (0-15).
    #[must_use]
    #[inline]
    pub fn level(self) -> u8 {
        (self.0 & Self::LEVEL_MASK) as u8
    }

    /// Checks if light should propagate in the given direction.
    #[must_use]
    #[inline]
    pub fn should_propagate(self, dir: Direction) -> bool {
        let bit = 1u64 << (dir as u8 + 4);
        (self.0 & bit) != 0
    }

    /// Checks if this entry is from a block with an empty collision shape.
    #[must_use]
    #[inline]
    pub fn is_from_empty_shape(self) -> bool {
        (self.0 & Self::EMPTY_SHAPE_FLAG) != 0
    }

    /// Checks if this entry represents light increase from an emitting block.
    #[must_use]
    #[inline]
    pub fn is_from_emission(self) -> bool {
        (self.0 & Self::EMISSION_FLAG) != 0
    }

    /// Sets the light level in the entry, preserving other flags.
    #[must_use]
    #[inline]
    fn with_level(self, level: u8) -> Self {
        debug_assert!(level <= 15, "Light level must be 0-15");
        // Clear level bits and set new level
        Self((self.0 & !Self::LEVEL_MASK) | (u64::from(level) & Self::LEVEL_MASK))
    }

    /// Adds a direction flag to the entry.
    #[must_use]
    #[inline]
    fn with_direction(self, dir: Direction) -> Self {
        Self(self.0 | (1u64 << (dir as u8 + 4)))
    }

    /// Removes a direction flag from the entry.
    #[must_use]
    #[inline]
    fn without_direction(self, dir: Direction) -> Self {
        Self(self.0 & !(1u64 << (dir as u8 + 4)))
    }

    /// Creates a queue entry for decreasing light in all directions.
    #[must_use]
    pub fn decrease_all_directions(level: u8) -> Self {
        debug_assert!(level <= 15, "Light level must be 0-15");
        Self(Self::DIRECTIONS_MASK).with_level(level)
    }

    /// Creates a queue entry for decreasing light in all directions except one.
    #[must_use]
    pub fn decrease_skip_one_direction(level: u8, skip_dir: Direction) -> Self {
        debug_assert!(level <= 15, "Light level must be 0-15");
        Self(Self::DIRECTIONS_MASK)
            .without_direction(skip_dir)
            .with_level(level)
    }

    /// Creates a queue entry for increasing light from an emitting block.
    #[must_use]
    pub fn increase_from_emission(level: u8, from_empty_shape: bool) -> Self {
        debug_assert!(level <= 15, "Light level must be 0-15");
        let mut entry = Self::DIRECTIONS_MASK | Self::EMISSION_FLAG;
        if from_empty_shape {
            entry |= Self::EMPTY_SHAPE_FLAG;
        }
        Self(entry).with_level(level)
    }

    /// Creates a queue entry for increasing light in all directions except one.
    #[must_use]
    pub fn increase_skip_one_direction(level: u8, from_empty_shape: bool, skip_dir: Direction) -> Self {
        debug_assert!(level <= 15, "Light level must be 0-15");
        let mut entry = Self::DIRECTIONS_MASK;
        if from_empty_shape {
            entry |= Self::EMPTY_SHAPE_FLAG;
        }
        Self(entry)
            .without_direction(skip_dir)
            .with_level(level)
    }

    /// Creates a queue entry for increasing light in only one direction.
    #[must_use]
    pub fn increase_only_one_direction(level: u8, from_empty_shape: bool, dir: Direction) -> Self {
        debug_assert!(level <= 15, "Light level must be 0-15");
        let mut entry = 0u64;
        if from_empty_shape {
            entry |= Self::EMPTY_SHAPE_FLAG;
        }
        Self(entry)
            .with_direction(dir)
            .with_level(level)
    }

    /// Creates a queue entry for sky light propagation with selective directions.
    ///
    /// Sky light always propagates at level 15.
    #[must_use]
    #[allow(clippy::fn_params_excessive_bools)] // Matches vanilla signature
    pub fn increase_sky_source_in_directions(
        down: bool,
        north: bool,
        south: bool,
        west: bool,
        east: bool,
    ) -> Self {
        let mut entry = Self(0).with_level(15);
        if down {
            entry = entry.with_direction(Direction::Down);
        }
        if north {
            entry = entry.with_direction(Direction::North);
        }
        if south {
            entry = entry.with_direction(Direction::South);
        }
        if west {
            entry = entry.with_direction(Direction::West);
        }
        if east {
            entry = entry.with_direction(Direction::East);
        }
        entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_extraction() {
        let entry = QueueEntry::decrease_all_directions(12);
        assert_eq!(entry.level(), 12);
    }

    #[test]
    fn test_direction_flags() {
        let entry = QueueEntry::decrease_all_directions(5);
        assert!(entry.should_propagate(Direction::Down));
        assert!(entry.should_propagate(Direction::Up));
        assert!(entry.should_propagate(Direction::North));
        assert!(entry.should_propagate(Direction::South));
        assert!(entry.should_propagate(Direction::West));
        assert!(entry.should_propagate(Direction::East));
    }

    #[test]
    fn test_skip_one_direction() {
        let entry = QueueEntry::decrease_skip_one_direction(8, Direction::Up);
        assert!(entry.should_propagate(Direction::Down));
        assert!(!entry.should_propagate(Direction::Up));
        assert!(entry.should_propagate(Direction::North));
        assert_eq!(entry.level(), 8);
    }

    #[test]
    fn test_emission_flag() {
        let entry = QueueEntry::increase_from_emission(14, true);
        assert_eq!(entry.level(), 14);
        assert!(entry.is_from_emission());
        assert!(entry.is_from_empty_shape());
    }

    #[test]
    fn test_only_one_direction() {
        let entry = QueueEntry::increase_only_one_direction(7, false, Direction::East);
        assert_eq!(entry.level(), 7);
        assert!(!entry.should_propagate(Direction::Down));
        assert!(!entry.should_propagate(Direction::Up));
        assert!(!entry.should_propagate(Direction::North));
        assert!(!entry.should_propagate(Direction::South));
        assert!(!entry.should_propagate(Direction::West));
        assert!(entry.should_propagate(Direction::East));
    }

    #[test]
    fn test_sky_source_directions() {
        let entry = QueueEntry::increase_sky_source_in_directions(
            true,  // down
            true,  // north
            false, // south
            false, // west
            true,  // east
        );
        assert_eq!(entry.level(), 15);
        assert!(entry.should_propagate(Direction::Down));
        assert!(entry.should_propagate(Direction::North));
        assert!(!entry.should_propagate(Direction::South));
        assert!(!entry.should_propagate(Direction::West));
        assert!(entry.should_propagate(Direction::East));
        assert!(!entry.should_propagate(Direction::Up));
    }
}
