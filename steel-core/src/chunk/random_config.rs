//! World random configuration for terrain generation.
//!
//! This module provides random number generator derivers for various
//! terrain generation systems.

use steel_utils::random::RandomSource;
use steel_utils::random::{PositionalRandom, Random, RandomSplitter, xoroshiro::Xoroshiro};

/// Random configuration for world generation.
pub struct WorldRandomConfig {
    /// The world seed.
    pub seed: u64,
    /// Base random deriver.
    pub base_deriver: RandomSplitter,
    /// Aquifer random deriver.
    pub aquifer_deriver: RandomSplitter,
    /// Ore vein random deriver.
    pub ore_deriver: RandomSplitter,
}

impl WorldRandomConfig {
    /// Creates a new world random configuration.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut rng = Xoroshiro::from_seed(seed);
        let base_deriver = rng.next_positional();

        // Create specialized derivers
        let aquifer_deriver = base_deriver.with_hash_of("minecraft:aquifer");
        let aquifer_deriver = match aquifer_deriver {
            RandomSource::Xoroshiro(mut x) => x.next_positional(),
            RandomSource::Legacy(mut l) => l.next_positional(),
        };

        let ore_deriver = base_deriver.with_hash_of("minecraft:ore");
        let ore_deriver = match ore_deriver {
            RandomSource::Xoroshiro(mut x) => x.next_positional(),
            RandomSource::Legacy(mut l) => l.next_positional(),
        };

        Self {
            seed,
            base_deriver,
            aquifer_deriver,
            ore_deriver,
        }
    }
}
