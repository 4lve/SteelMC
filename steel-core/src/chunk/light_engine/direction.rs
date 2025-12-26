//! Cardinal direction enum for light propagation.

use steel_utils::BlockPos;

/// Six cardinal directions for light propagation.
///
/// The ordinal values (0-5) match Minecraft's Java implementation and are critical
/// for `QueueEntry` bit manipulation (direction flags use bits 4-9).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Downward (-Y direction) - ordinal 0, uses bit 4 in `QueueEntry`
    Down = 0,
    /// Upward (+Y direction) - ordinal 1, uses bit 5 in `QueueEntry`
    Up = 1,
    /// North (-Z direction) - ordinal 2, uses bit 6 in `QueueEntry`
    North = 2,
    /// South (+Z direction) - ordinal 3, uses bit 7 in `QueueEntry`
    South = 3,
    /// West (-X direction) - ordinal 4, uses bit 8 in `QueueEntry`
    West = 4,
    /// East (+X direction) - ordinal 5, uses bit 9 in `QueueEntry`
    East = 5,
}

impl Direction {
    /// All six directions in array form for iteration.
    pub const ALL: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    /// Returns the opposite direction.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    /// Gets the offset in the given direction.
    ///
    /// Returns (dx, dy, dz) for this direction.
    #[must_use]
    pub const fn offset(self) -> (i32, i32, i32) {
        match self {
            Self::Down => (0, -1, 0),
            Self::Up => (0, 1, 0),
            Self::North => (0, 0, -1),
            Self::South => (0, 0, 1),
            Self::West => (-1, 0, 0),
            Self::East => (1, 0, 0),
        }
    }

    /// Returns a new `BlockPos` relative to the given position in this direction.
    #[must_use]
    pub fn relative(self, pos: BlockPos) -> BlockPos {
        use steel_utils::math::Vector3;
        let (dx, dy, dz) = self.offset();
        BlockPos(Vector3::new(pos.0.x + dx, pos.0.y + dy, pos.0.z + dz))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordinals() {
        assert_eq!(Direction::Down as u8, 0);
        assert_eq!(Direction::Up as u8, 1);
        assert_eq!(Direction::North as u8, 2);
        assert_eq!(Direction::South as u8, 3);
        assert_eq!(Direction::West as u8, 4);
        assert_eq!(Direction::East as u8, 5);
    }

    #[test]
    fn test_opposite() {
        assert_eq!(Direction::Down.opposite(), Direction::Up);
        assert_eq!(Direction::North.opposite(), Direction::South);
        assert_eq!(Direction::West.opposite(), Direction::East);
    }

    #[test]
    fn test_offset() {
        assert_eq!(Direction::Down.offset(), (0, -1, 0));
        assert_eq!(Direction::Up.offset(), (0, 1, 0));
        assert_eq!(Direction::North.offset(), (0, 0, -1));
        assert_eq!(Direction::South.offset(), (0, 0, 1));
        assert_eq!(Direction::West.offset(), (-1, 0, 0));
        assert_eq!(Direction::East.offset(), (1, 0, 0));
    }
}
