//! Normal noise implementation for vanilla-accurate world generation.
//!
//! This is an exact port of Minecraft's `NormalNoise` class.

// Noise code uses mathematical single-letter variables (x, y, z, i, j, k)
#![allow(clippy::many_single_char_names)]

use crate::random::{RandomSource, RandomSplitter};

use super::PerlinNoise;

/// Input factor for slight variation between the two noise samplers.
const INPUT_FACTOR: f64 = 1.018_126_888_217_522_7;

/// Value factor numerator — vanilla uses 0.16666666666666666 (1/6), NOT `TARGET_DEVIATION` (1/3).
const VALUE_FACTOR_NUMERATOR: f64 = 0.166_666_666_666_666_66;

/// Parameters for creating a `NormalNoise` instance.
#[derive(Clone, Debug)]
pub struct NoiseParameters {
    /// The first octave index.
    pub first_octave: i32,
    /// Amplitude for each octave.
    pub amplitudes: Vec<f64>,
}

impl NoiseParameters {
    /// Creates new noise parameters with the given first octave and amplitudes.
    #[must_use]
    pub fn new(first_octave: i32, amplitudes: Vec<f64>) -> Self {
        Self {
            first_octave,
            amplitudes,
        }
    }

    /// Creates noise parameters from a first octave and a single amplitude with additional ones.
    #[must_use]
    pub fn with_amplitudes(first_octave: i32, first_amplitude: f64, rest: &[f64]) -> Self {
        let mut amplitudes = vec![first_amplitude];
        amplitudes.extend_from_slice(rest);
        Self {
            first_octave,
            amplitudes,
        }
    }
}

/// Normal noise generator that combines two Perlin noise samplers.
///
/// Used for climate/biome parameter noise, combining two `PerlinNoise` instances
/// with slightly different input factors for natural-looking results.
pub struct NormalNoise {
    /// First Perlin noise sampler.
    first: PerlinNoise,
    /// Second Perlin noise sampler with slightly offset inputs.
    second: PerlinNoise,
    /// Factor applied to output values for normalization.
    value_factor: f64,
    /// Maximum possible output value.
    max_value: f64,
    /// The parameters used to create this noise.
    parameters: NoiseParameters,
}

impl NormalNoise {
    /// Creates a new `NormalNoise` using the modern factory method.
    pub fn create(
        random_splitter: &mut RandomSplitter,
        name: &str,
        parameters: NoiseParameters,
    ) -> Self {
        let first = PerlinNoise::create_from_random_source(
            random_splitter,
            &format!("{name}/first"),
            parameters.first_octave,
            &parameters.amplitudes,
        );
        let second = PerlinNoise::create_from_random_source(
            random_splitter,
            &format!("{name}/second"),
            parameters.first_octave,
            &parameters.amplitudes,
        );

        Self::new_internal(first, second, parameters)
    }

    /// Creates a new `NormalNoise` with explicit first octave and amplitudes.
    pub fn create_with_amplitudes(
        random_splitter: &mut RandomSplitter,
        name: &str,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> Self {
        Self::create(
            random_splitter,
            name,
            NoiseParameters::new(first_octave, amplitudes.to_vec()),
        )
    }

    /// Creates a `NormalNoise` using legacy random for legacy nether biome.
    ///
    /// Vanilla passes the `RandomSource` directly through to
    /// `PerlinNoise.createLegacyForLegacyNetherBiome` without forking.
    pub fn create_legacy_nether_biome(
        random: &mut RandomSource,
        parameters: NoiseParameters,
    ) -> Self {
        let first = PerlinNoise::create_legacy_for_legacy_nether_biome(
            random,
            parameters.first_octave,
            &parameters.amplitudes,
        );
        let second = PerlinNoise::create_legacy_for_legacy_nether_biome(
            random,
            parameters.first_octave,
            &parameters.amplitudes,
        );

        Self::new_internal(first, second, parameters)
    }

    fn new_internal(first: PerlinNoise, second: PerlinNoise, parameters: NoiseParameters) -> Self {
        // Find min and max octave indices with non-zero amplitudes
        let mut min_octave = i32::MAX;
        let mut max_octave = i32::MIN;

        for (i, &amplitude) in parameters.amplitudes.iter().enumerate() {
            if amplitude != 0.0 {
                min_octave = min_octave.min(i as i32);
                max_octave = max_octave.max(i as i32);
            }
        }

        let octave_spread = max_octave - min_octave;
        let value_factor = VALUE_FACTOR_NUMERATOR / expected_deviation(octave_spread);
        let max_value = (first.max_value() + second.max_value()) * value_factor;

        Self {
            first,
            second,
            value_factor,
            max_value,
            parameters,
        }
    }

    /// Sample noise at the given coordinates.
    #[must_use]
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        let d = x * INPUT_FACTOR;
        let e = y * INPUT_FACTOR;
        let f = z * INPUT_FACTOR;

        (self.first.get_value(x, y, z) + self.second.get_value(d, e, f)) * self.value_factor
    }

    /// Get the maximum value this noise can produce.
    #[must_use]
    pub fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Get the parameters used to create this noise.
    #[must_use]
    pub fn parameters(&self) -> &NoiseParameters {
        &self.parameters
    }
}

/// Calculate expected deviation for a given number of octaves.
fn expected_deviation(octaves: i32) -> f64 {
    0.1 * (1.0 + 1.0 / f64::from(octaves + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Random;
    use crate::random::xoroshiro::Xoroshiro;

    #[test]
    fn test_normal_noise_deterministic() {
        let mut rng1 = Xoroshiro::from_seed(12345);
        let mut positional_random_factory1 = rng1.next_positional();
        let mut rng2 = Xoroshiro::from_seed(12345);
        let mut positional_random_factory2 = rng2.next_positional();

        let params = NoiseParameters::new(-4, vec![1.0, 1.0, 1.0, 1.0]);
        let noise1 = NormalNoise::create(
            &mut positional_random_factory1,
            "minecraft:test_normal_noise_deterministic",
            params.clone(),
        );
        let noise2 = NormalNoise::create(
            &mut positional_random_factory2,
            "minecraft:test_normal_noise_deterministic",
            params,
        );

        assert_eq!(
            noise1.get_value(0.5, 0.5, 0.5).to_bits(),
            noise2.get_value(0.5, 0.5, 0.5).to_bits()
        );
    }

    #[test]
    fn test_expected_deviation() {
        // Test some known values
        assert!((expected_deviation(0) - 0.2).abs() < 1e-10);
        assert!((expected_deviation(1) - 0.15).abs() < 1e-10);
        assert!((expected_deviation(3) - 0.125).abs() < 1e-10);
    }

    #[test]
    fn test_noise_parameters() {
        let params = NoiseParameters::with_amplitudes(-7, 1.0, &[1.0, 1.0, 1.0]);
        assert_eq!(params.first_octave, -7);
        assert_eq!(params.amplitudes.len(), 4);
        assert_eq!(params.amplitudes[0].to_bits(), 1.0_f64.to_bits());
    }
}
