//! Light storage for chunk sections.
//!
//! Light values are stored as 4-bit values (0-15), packed as two values per byte.
//! For a 16x16x16 section, this requires 2048 bytes (4096 blocks / 2).

use std::fmt::Debug;

/// The number of bytes needed to store light data for a 16x16x16 section.
/// 16*16*16 blocks = 4096 blocks, at 4 bits per block = 2048 bytes
pub const LIGHT_ARRAY_SIZE: usize = 2048;

/// Storage for light data in a chunk section.
/// Light values range from 0-15 (4 bits per block).
#[derive(Debug, Clone)]
pub enum LightStorage {
    /// All blocks in the section have the same light level (0-15).
    Homogeneous(u8),
    /// Blocks have different light levels, stored as packed nibbles.
    /// Each byte contains two 4-bit light values.
    Heterogeneous(Box<[u8; LIGHT_ARRAY_SIZE]>),
}

impl LightStorage {
    /// Creates a new homogeneous light storage with all blocks at the given light level.
    #[must_use]
    pub fn new_filled(light_level: u8) -> Self {
        debug_assert!(light_level <= 15, "Light level must be 0-15");
        Self::Homogeneous(light_level)
    }

    /// Creates a new empty (dark) light storage.
    #[must_use]
    pub fn new_empty() -> Self {
        Self::Homogeneous(0)
    }

    /// Gets the light level at the given position.
    ///
    /// # Arguments
    /// * `x` - X coordinate (0-15)
    /// * `y` - Y coordinate (0-15)
    /// * `z` - Z coordinate (0-15)
    #[must_use]
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> u8 {
        debug_assert!(x < 16 && y < 16 && z < 16, "Coordinates must be 0-15");

        match self {
            Self::Homogeneous(level) => *level,
            Self::Heterogeneous(data) => {
                // Calculate index: y * 16 * 16 + z * 16 + x
                let block_index = y * 256 + z * 16 + x;
                let byte_index = block_index >> 1;
                let is_upper_nibble = (block_index & 1) == 1;

                if is_upper_nibble {
                    (data[byte_index] >> 4) & 0x0F
                } else {
                    data[byte_index] & 0x0F
                }
            }
        }
    }

    /// Sets the light level at the given position.
    ///
    /// If currently homogeneous and setting a different value, upgrades to heterogeneous.
    ///
    /// # Arguments
    /// * `x` - X coordinate (0-15)
    /// * `y` - Y coordinate (0-15)
    /// * `z` - Z coordinate (0-15)
    /// * `light_level` - Light level (0-15)
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, light_level: u8) {
        debug_assert!(x < 16 && y < 16 && z < 16, "Coordinates must be 0-15");
        debug_assert!(light_level <= 15, "Light level must be 0-15");

        match self {
            Self::Homogeneous(current_level) => {
                if light_level == *current_level {
                    // No change needed
                    return;
                }

                // Upgrade to heterogeneous
                let mut data = if *current_level == 0 {
                    Box::new([0u8; LIGHT_ARRAY_SIZE])
                } else {
                    // Fill with current level in both nibbles
                    let packed = (*current_level & 0x0F) | ((*current_level & 0x0F) << 4);
                    Box::new([packed; LIGHT_ARRAY_SIZE])
                };

                // Set the new value
                let block_index = y * 256 + z * 16 + x;
                let byte_index = block_index >> 1;
                let is_upper_nibble = (block_index & 1) == 1;

                if is_upper_nibble {
                    data[byte_index] = (data[byte_index] & 0x0F) | ((light_level & 0x0F) << 4);
                } else {
                    data[byte_index] = (data[byte_index] & 0xF0) | (light_level & 0x0F);
                }

                *self = Self::Heterogeneous(data);
            }
            Self::Heterogeneous(data) => {
                let block_index = y * 256 + z * 16 + x;
                let byte_index = block_index >> 1;
                let is_upper_nibble = (block_index & 1) == 1;

                if is_upper_nibble {
                    data[byte_index] = (data[byte_index] & 0x0F) | ((light_level & 0x0F) << 4);
                } else {
                    data[byte_index] = (data[byte_index] & 0xF0) | (light_level & 0x0F);
                }
            }
        }
    }

    /// Returns the raw data for sending to the client.
    ///
    /// For homogeneous storage, creates a filled array.
    /// For heterogeneous storage, returns a clone of the data.
    #[must_use]
    pub fn to_packet_data(&self) -> Vec<u8> {
        match self {
            Self::Homogeneous(level) => {
                // Pack the level into both nibbles of each byte
                let packed = (*level & 0x0F) | ((*level & 0x0F) << 4);
                vec![packed; LIGHT_ARRAY_SIZE]
            }
            Self::Heterogeneous(data) => data.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homogeneous_get() {
        let storage = LightStorage::new_filled(15);
        assert_eq!(storage.get(0, 0, 0), 15);
        assert_eq!(storage.get(15, 15, 15), 15);
    }

    #[test]
    fn test_set_upgrades_to_heterogeneous() {
        let mut storage = LightStorage::new_empty();
        storage.set(5, 5, 5, 14);

        assert_eq!(storage.get(5, 5, 5), 14);
        assert_eq!(storage.get(0, 0, 0), 0);

        assert!(matches!(storage, LightStorage::Heterogeneous(_)));
    }

    #[test]
    fn test_heterogeneous_get_set() {
        let mut storage = LightStorage::new_empty();

        // Set various light levels
        storage.set(0, 0, 0, 15);
        storage.set(1, 0, 0, 14);
        storage.set(0, 1, 0, 7);
        storage.set(15, 15, 15, 1);

        assert_eq!(storage.get(0, 0, 0), 15);
        assert_eq!(storage.get(1, 0, 0), 14);
        assert_eq!(storage.get(0, 1, 0), 7);
        assert_eq!(storage.get(15, 15, 15), 1);
        assert_eq!(storage.get(8, 8, 8), 0); // Unchanged
    }

    #[test]
    fn test_packed_nibbles() {
        let mut storage = LightStorage::new_empty();

        // Set two adjacent blocks (they share a byte)
        storage.set(0, 0, 0, 5);
        storage.set(1, 0, 0, 10);

        assert_eq!(storage.get(0, 0, 0), 5);
        assert_eq!(storage.get(1, 0, 0), 10);
    }
}
