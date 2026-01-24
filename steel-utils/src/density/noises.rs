//! Vanilla noise parameter definitions.
//!
//! This module contains the noise parameters used by vanilla Minecraft
//! for terrain generation. These parameters were extracted from vanilla's
//! data files.

use crate::noise::NoiseParameters;

/// Creates the noise parameters for continentalness.
/// Controls large-scale continent/ocean distribution.
#[must_use]
pub fn continentalness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -9,
        amplitudes: vec![1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0],
    }
}

/// Creates the noise parameters for erosion.
/// Controls terrain erosion patterns.
#[must_use]
pub fn erosion() -> NoiseParameters {
    NoiseParameters {
        first_octave: -9,
        amplitudes: vec![1.0, 1.0, 0.0, 1.0, 1.0],
    }
}

/// Creates the noise parameters for ridges (weirdness).
/// Controls ridge and valley formation.
#[must_use]
pub fn ridge() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0, 2.0, 1.0, 0.0, 0.0, 0.0],
    }
}

/// Creates the noise parameters for the shift function.
/// Used to offset noise sampling positions.
#[must_use]
pub fn shift() -> NoiseParameters {
    NoiseParameters {
        first_octave: -3,
        amplitudes: vec![1.0, 1.0, 1.0, 0.0],
    }
}

/// Creates the noise parameters for temperature.
/// Used for biome temperature distribution.
#[must_use]
pub fn temperature() -> NoiseParameters {
    NoiseParameters {
        first_octave: -10,
        amplitudes: vec![1.5, 0.0, 1.0, 0.0, 0.0, 0.0],
    }
}

/// Creates the noise parameters for vegetation (humidity).
/// Used for biome humidity distribution.
#[must_use]
pub fn vegetation() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    }
}

/// Creates the noise parameters for jagged terrain.
/// Controls jaggedness of mountain peaks.
#[must_use]
pub fn jagged() -> NoiseParameters {
    NoiseParameters {
        first_octave: -16,
        amplitudes: vec![
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ],
    }
}

/// Creates the noise parameters for cave entrances.
#[must_use]
pub fn cave_entrance() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![0.4, 0.5, 1.0],
    }
}

/// Creates the noise parameters for cave layers.
#[must_use]
pub fn cave_layer() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for cave cheese.
#[must_use]
pub fn cave_cheese() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![0.5, 1.0, 2.0, 1.0, 2.0, 1.0, 0.0, 2.0, 0.0],
    }
}

/// Creates the noise parameters for aquifer barriers.
#[must_use]
pub fn aquifer_barrier() -> NoiseParameters {
    NoiseParameters {
        first_octave: -3,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for aquifer fluid level floodedness.
#[must_use]
pub fn aquifer_fluid_level_floodedness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for aquifer fluid level spread.
#[must_use]
pub fn aquifer_fluid_level_spread() -> NoiseParameters {
    NoiseParameters {
        first_octave: -5,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for aquifer lava.
#[must_use]
pub fn aquifer_lava() -> NoiseParameters {
    NoiseParameters {
        first_octave: -1,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for ore veininess.
#[must_use]
pub fn ore_veininess() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for ore vein A.
#[must_use]
pub fn ore_vein_a() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for ore vein B.
#[must_use]
pub fn ore_vein_b() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for ore gaps.
#[must_use]
pub fn ore_gap() -> NoiseParameters {
    NoiseParameters {
        first_octave: -5,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for pillars.
#[must_use]
pub fn pillar() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0, 1.0],
    }
}

/// Creates the noise parameters for pillar rareness.
#[must_use]
pub fn pillar_rareness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for pillar thickness.
#[must_use]
pub fn pillar_thickness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 2D.
#[must_use]
pub fn spaghetti_2d() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 2D elevation.
#[must_use]
pub fn spaghetti_2d_elevation() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 2D modulator.
#[must_use]
pub fn spaghetti_2d_modulator() -> NoiseParameters {
    NoiseParameters {
        first_octave: -11,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 2D thickness.
#[must_use]
pub fn spaghetti_2d_thickness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -11,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 3D 1.
#[must_use]
pub fn spaghetti_3d_1() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 3D 2.
#[must_use]
pub fn spaghetti_3d_2() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 3D rarity.
#[must_use]
pub fn spaghetti_3d_rarity() -> NoiseParameters {
    NoiseParameters {
        first_octave: -11,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti 3D thickness.
#[must_use]
pub fn spaghetti_3d_thickness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -11,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti roughness.
#[must_use]
pub fn spaghetti_roughness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -5,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for spaghetti roughness modulator.
#[must_use]
pub fn spaghetti_roughness_modulator() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for noodle.
#[must_use]
pub fn noodle() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for noodle thickness.
#[must_use]
pub fn noodle_thickness() -> NoiseParameters {
    NoiseParameters {
        first_octave: -8,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for noodle ridge A.
#[must_use]
pub fn noodle_ridge_a() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}

/// Creates the noise parameters for noodle ridge B.
#[must_use]
pub fn noodle_ridge_b() -> NoiseParameters {
    NoiseParameters {
        first_octave: -7,
        amplitudes: vec![1.0],
    }
}
