//! Multi-octave Perlin noise implementation for vanilla-accurate world generation.
//!
//! This is an exact port of Minecraft's `PerlinNoise` class.

// Noise code uses mathematical single-letter variables (x, y, z, i, j, k)
#![allow(clippy::many_single_char_names)]

use crate::random::{PositionalRandom, Random, RandomSource, RandomSplitter};

use super::{ImprovedNoise, lfloor};

/// Skip an octave by consuming 262 random values.
/// This matches Minecraft's skipOctave(RandomSource) method.
fn skip_octave(random: &mut RandomSource) {
    random.consume_count(262);
}

/// Constant for coordinate wrapping to prevent floating-point discontinuities.
/// Vanilla uses 3.3554432E7F (float) cast to double.
const ROUND_OFF: f64 = 3.355_443_2E7;

/// Wrap a coordinate to prevent discontinuities in large worlds.
/// Matches Minecraft's `PerlinNoise.wrap` method exactly:
/// `x - (double)Mth.lfloor(x / 3.3554432E7 + 0.5) * 3.3554432E7`
#[inline]
#[must_use]
pub fn wrap(value: f64) -> f64 {
    value - (lfloor(value / ROUND_OFF + 0.5) as f64) * ROUND_OFF
}

/// Multi-octave Perlin noise generator.
///
/// Combines multiple `ImprovedNoise` instances at different frequencies
/// and amplitudes using fractal Brownian motion (FBM).
pub struct PerlinNoise {
    /// Noise samplers for each octave (may be None if amplitude is 0).
    noise_levels: Vec<Option<ImprovedNoise>>,
    /// The first octave index (can be negative for higher frequencies).
    first_octave: i32,
    /// Amplitude for each octave.
    amplitudes: Vec<f64>,
    /// Factor applied to input coordinates.
    lowest_freq_input_factor: f64,
    /// Factor applied to output values.
    lowest_freq_value_factor: f64,
    /// Maximum possible output value.
    max_value: f64,
}

impl PerlinNoise {
    /// Creates a new `PerlinNoise` using the new factory method (positional random).
    pub fn create<R: Random>(random: &mut R, first_octave: i32, amplitudes: &[f64]) -> Self {
        let RandomSplitter::Xoroshiro(positional_factory) = random.next_positional() else {
            todo!("Implement other random splitter types for PerlinNoise::create");
        };

        let octave_count = amplitudes.len();
        let neg_first_octave = -first_octave;

        let mut noise_levels: Vec<Option<ImprovedNoise>> =
            (0..octave_count).map(|_| None).collect();

        for k in 0..octave_count {
            if amplitudes[k] != 0.0 {
                let l = first_octave + k as i32;
                let mut octave_random: RandomSource =
                    positional_factory.with_hash_of(&format!("octave_{l}"));
                noise_levels[k] = Some(ImprovedNoise::new(&mut octave_random));
            }
        }

        Self::new_internal(
            noise_levels,
            first_octave,
            amplitudes.to_vec(),
            neg_first_octave,
            octave_count,
        )
    }

    /// Creates `PerlinNoise` using a positional random factory (for `BlendedNoise` compatibility).
    ///
    /// This uses a factory to create the random sources for each octave.
    pub fn create_from_random_source(
        random_factory: &mut RandomSplitter,
        name: &str,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> Self {
        let octave_count = amplitudes.len();
        let mut noise_levels: Vec<Option<ImprovedNoise>> =
            (0..octave_count).map(|_| None).collect();

        for i in 0..octave_count {
            if amplitudes[i] != 0.0 {
                let absolute_octave = first_octave + i as i32;
                let mut random_source =
                    random_factory.with_hash_of(&format!("{name}/octave_{absolute_octave}"));
                noise_levels[i] = Some(ImprovedNoise::new(&mut random_source));
            }
        }

        Self::new_internal(
            noise_levels,
            first_octave,
            amplitudes.to_vec(),
            -first_octave, // neg_first_octave is derived here for new_internal
            octave_count,
        )
    }

    /// Creates `PerlinNoise` using legacy random (for `BlendedNoise` compatibility).
    ///
    /// This uses the LEGACY initialization path where noises are created directly
    /// from the random source (NOT using positional random factory).
    ///
    /// # Panics
    ///
    /// Panics if `octaves` is empty.
    pub fn create_legacy_for_blended_noise(random: &mut RandomSource, octaves: &[i32]) -> Self {
        // Sort octaves to find min and max
        let mut sorted_octaves = octaves.to_vec();
        sorted_octaves.sort_unstable();

        assert!(!sorted_octaves.is_empty(), "Need some octaves!");

        let first_int = *sorted_octaves
            .first()
            .expect("octaves not empty (checked by assert above)"); // Most negative (e.g., -15)
        let last_int = *sorted_octaves
            .last()
            .expect("octaves not empty (checked by assert above)"); // Least negative (e.g., 0)

        let low_freq_octaves = -first_int; // e.g., 15
        let high_freq_octaves = last_int; // e.g., 0
        let total_octaves = (low_freq_octaves + high_freq_octaves + 1) as usize;

        assert!(
            total_octaves >= 1,
            "Total number of octaves needs to be >= 1"
        );

        // Create amplitudes array with 1.0 for each octave in the set
        let mut amplitudes = vec![0.0; total_octaves];
        for &octave in &sorted_octaves {
            let index = (octave + low_freq_octaves) as usize;
            amplitudes[index] = 1.0;
        }

        let first_octave = -low_freq_octaves;
        let neg_first_octave = low_freq_octaves as usize;

        Self::create_legacy_internal(random, first_octave, amplitudes, neg_first_octave)
    }

    /// Creates `PerlinNoise` using legacy random for legacy nether biome.
    ///
    /// Vanilla passes the `RandomSource` directly to the legacy constructor
    /// without forking a positional random.
    pub fn create_legacy_for_legacy_nether_biome(
        random: &mut RandomSource,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> Self {
        let neg_first_octave = -first_octave;
        Self::create_legacy_internal(
            random,
            first_octave,
            amplitudes.to_vec(),
            neg_first_octave as usize,
        )
    }

    fn create_legacy_internal(
        random: &mut RandomSource,
        first_octave: i32,
        amplitudes: Vec<f64>,
        neg_first_octave: usize,
    ) -> Self {
        let octave_count = amplitudes.len();
        let mut noise_levels: Vec<Option<ImprovedNoise>> =
            (0..octave_count).map(|_| None).collect();

        // Create the first noise at neg_first_octave index (the "zero octave")
        let improved_noise = ImprovedNoise::new(random);

        if neg_first_octave < octave_count {
            let d = amplitudes[neg_first_octave];
            if d != 0.0 {
                noise_levels[neg_first_octave] = Some(improved_noise);
            }
        }

        // Create remaining noises in reverse order (from neg_first_octave-1 down to 0)
        for k in (0..neg_first_octave).rev() {
            if k < octave_count {
                let e = amplitudes[k];
                if e == 0.0 {
                    // Skip this octave by consuming 262 random values
                    skip_octave(random);
                } else {
                    noise_levels[k] = Some(ImprovedNoise::new(random));
                }
            } else {
                // Skip this octave by consuming 262 random values
                skip_octave(random);
            }
        }

        // Verify we have the right number of noise levels
        let non_null_count = noise_levels.iter().filter(|n| n.is_some()).count();
        let non_zero_amplitude_count = amplitudes.iter().filter(|&&a| a != 0.0).count();

        assert!(
            non_null_count == non_zero_amplitude_count,
            "Failed to create correct number of noise levels for given non-zero amplitudes"
        );

        assert!(
            neg_first_octave >= octave_count - 1,
            "Positive octaves are temporarily disabled"
        );

        Self::new_internal(
            noise_levels,
            first_octave,
            amplitudes,
            neg_first_octave as i32,
            octave_count,
        )
    }

    fn new_internal(
        noise_levels: Vec<Option<ImprovedNoise>>,
        first_octave: i32,
        amplitudes: Vec<f64>,
        neg_first_octave: i32,
        octave_count: usize,
    ) -> Self {
        let lowest_freq_input_factor = 2.0_f64.powi(-neg_first_octave);
        let lowest_freq_value_factor =
            2.0_f64.powi(octave_count as i32 - 1) / (2.0_f64.powi(octave_count as i32) - 1.0);

        let mut result = Self {
            noise_levels,
            first_octave,
            amplitudes,
            lowest_freq_input_factor,
            lowest_freq_value_factor,
            max_value: 0.0, // Computed below
        };

        result.max_value = result.edge_value(2.0);
        result
    }

    /// Sample noise at the given coordinates.
    #[inline]
    #[must_use]
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        self.get_value_with_y_params(x, y, z, 0.0, 0.0, false)
    }

    /// Sample noise with Y-axis scaling parameters.
    #[inline]
    #[must_use]
    pub fn get_value_with_y_params(
        &self,
        x: f64,
        y: f64,
        z: f64,
        y_scale: f64,
        y_max: f64,
        use_fixed_y: bool,
    ) -> f64 {
        let mut d = 0.0;
        let mut e = self.lowest_freq_input_factor;
        let mut f = self.lowest_freq_value_factor;

        for i in 0..self.noise_levels.len() {
            if let Some(ref noise) = self.noise_levels[i] {
                let g = noise.noise_with_y_params(
                    wrap(x * e),
                    if use_fixed_y { -noise.yo } else { wrap(y * e) },
                    wrap(z * e),
                    y_scale * e,
                    y_max * e,
                );
                d += self.amplitudes[i] * g * f;
            }

            e *= 2.0;
            f /= 2.0;
        }

        d
    }

    /// Sample noise for blended noise (`InterpolatedNoiseSampler`).
    ///
    /// This uses a fractions-based weighting approach like vanilla Minecraft's `BlendedNoise`,
    /// where each octave is sampled with coordinates scaled by a fraction, and the result
    /// is divided by that fraction (amplifying low-frequency octaves).
    #[inline]
    #[must_use]
    pub fn get_value_for_blended_noise(
        &self,
        x: f64,
        y: f64,
        z: f64,
        y_scale: f64,
        y_max: f64,
    ) -> f64 {
        let mut result = 0.0;
        let mut fraction = 1.0;

        // Iterate in reverse order (from highest index to lowest)
        // to match vanilla's behavior of pairing fractions [1.0, 0.5, 0.25, ...]
        // with octaves from high to low
        for i in (0..self.noise_levels.len()).rev() {
            if let Some(ref noise) = self.noise_levels[i] {
                // Scale coordinates by fraction
                let mapped_x = wrap(x * fraction);
                let mapped_y = wrap(y * fraction);
                let mapped_z = wrap(z * fraction);

                // Sample with y-params and divide by fraction (amplifying low-freq octaves)
                let sample = noise.noise_with_y_params(
                    mapped_x,
                    mapped_y,
                    mapped_z,
                    y_scale * fraction,
                    y_max * fraction,
                );
                result += sample / fraction;
            }

            fraction /= 2.0;
        }

        result
    }

    /// Get the noise sampler for a specific octave.
    #[must_use]
    pub fn get_octave_noise(&self, octave: i32) -> Option<&ImprovedNoise> {
        let index = self.noise_levels.len() as i32 - 1 - octave;
        if index >= 0 && (index as usize) < self.noise_levels.len() {
            self.noise_levels[index as usize].as_ref()
        } else {
            None
        }
    }

    /// Compute the maximum possible value given a multiplier.
    #[must_use]
    pub fn max_broken_value(&self, y_multiplier: f64) -> f64 {
        self.edge_value(y_multiplier + 2.0)
    }

    fn edge_value(&self, multiplier: f64) -> f64 {
        let mut d = 0.0;
        let mut e = self.lowest_freq_value_factor;

        for i in 0..self.noise_levels.len() {
            if self.noise_levels[i].is_some() {
                d += self.amplitudes[i] * multiplier * e;
            }
            e /= 2.0;
        }

        d
    }

    /// Get the maximum value this noise can produce.
    #[must_use]
    pub fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Get the first octave index.
    #[must_use]
    pub fn first_octave(&self) -> i32 {
        self.first_octave
    }

    /// Get the amplitudes.
    #[must_use]
    pub fn amplitudes(&self) -> &[f64] {
        &self.amplitudes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::xoroshiro::Xoroshiro;

    #[test]
    fn test_perlin_noise_deterministic() {
        let mut rng1 = Xoroshiro::from_seed(12345);
        let mut rng2 = Xoroshiro::from_seed(12345);

        let noise1 = PerlinNoise::create(&mut rng1, -4, &[1.0, 1.0, 1.0, 1.0]);
        let noise2 = PerlinNoise::create(&mut rng2, -4, &[1.0, 1.0, 1.0, 1.0]);

        assert_eq!(
            noise1.get_value(0.5, 0.5, 0.5).to_bits(),
            noise2.get_value(0.5, 0.5, 0.5).to_bits()
        );
    }

    #[test]
    fn test_wrap() {
        // Values close to 0 should remain unchanged
        assert!((wrap(0.0) - 0.0).abs() < 1e-10);
        assert!((wrap(100.0) - 100.0).abs() < 1e-10);

        // Very large values should be wrapped
        let large = ROUND_OFF * 2.0 + 1000.0;
        let wrapped = wrap(large);
        assert!(wrapped.abs() < ROUND_OFF);
    }

    #[test]
    fn test_perlin_noise_legacy_blended() {
        let rng = Xoroshiro::from_seed(42);
        let mut random_source = RandomSource::Xoroshiro(rng);
        let octaves: Vec<i32> = (-15..=0).collect();
        let noise = PerlinNoise::create_legacy_for_blended_noise(&mut random_source, &octaves);

        // Should have 16 octaves
        assert_eq!(noise.noise_levels.len(), 16);

        // All octaves should be non-null
        for level in &noise.noise_levels {
            assert!(level.is_some());
        }
    }

    /// Tests that `OctavePerlinNoiseSampler` (`PerlinNoise`) matches Pumpkin's expected values.
    /// Values from Pumpkin's `octave_perlin_noise_sampler_test::test_sample`.
    #[test]
    fn test_vanilla_parity_octave_perlin_sample() {
        let mut rng = Xoroshiro::from_seed(513_513_513);
        // Verify RNG state (this consumes one value from the RNG)
        assert_eq!(rng.next_i32(), 404_174_895);

        // Continue with the SAME RNG (don't reset - Pumpkin's test doesn't reset)
        let mut random_source = RandomSource::Xoroshiro(rng);

        // Create with octaves [1, 2, 3] which becomes first_octave=1, amplitudes=[1.0, 1.0, 1.0]
        let noise = PerlinNoise::create(&mut random_source, 1, &[1.0, 1.0, 1.0]);

        // Expected values from Pumpkin's test
        let test_cases: [((f64, f64, f64), f64); 5] = [
            (
                (
                    1.463_389_780_121_818_2E8,
                    3.360_929_121_402_108E8,
                    -1.737_618_451_504_316_3E8,
                ),
                -0.165_101_376_396_830_28,
            ),
            (
                (
                    -3.952_093_942_501_234E8,
                    -8.149_682_915_016_855E7,
                    2.076_170_953_539_757_4E8,
                ),
                -0.198_652_274_578_263_65,
            ),
            (
                (
                    1.060_351_881_286_149_3E8,
                    -1.602_805_003_963_030_3E8,
                    9.621_510_690_305_333E7,
                ),
                -0.161_575_484_929_447_98,
            ),
            (
                (
                    -2.278_928_160_986_075_4E8,
                    1.241_650_575_772_375_6E8,
                    -3.047_619_296_454_517E8,
                ),
                -0.057_625_751_185_408_47,
            ),
            (
                (
                    -1.636_132_260_469_006_6E8,
                    -1.862_652_364_900_794E8,
                    9.034_589_265_385_96E7,
                ),
                0.215_894_040_367_422_88,
            ),
        ];

        for ((x, y, z), expected) in test_cases {
            let result = noise.get_value(x, y, z);
            assert_eq!(
                result.to_bits(),
                expected.to_bits(),
                "Mismatch at ({x}, {y}, {z}): got {result}, expected {expected}"
            );
        }
    }

    /// Tests legacy path creation origins match Pumpkin's expected values.
    /// Values from Pumpkin's `perlin_noise_sampler_test::test_no_y_chunk`.
    #[test]
    fn test_vanilla_parity_legacy_octave_origins() {
        let mut rng = Xoroshiro::from_seed(0);
        let splitter = rng.next_positional();
        let mut rand = splitter.with_hash_of("minecraft:terrain");

        // Verify initial state
        assert_eq!(rand.next_i32(), 1_374_487_555);

        // Reset for actual test
        let mut rng = Xoroshiro::from_seed(0);
        let splitter = rng.next_positional();
        let rand = splitter.with_hash_of("minecraft:terrain");

        let octaves: Vec<i32> = (-15..=0).collect();
        let RandomSource::Xoroshiro(rng_inner) = rand else {
            panic!("Expected Xoroshiro");
        };
        let mut random_source = RandomSource::Xoroshiro(rng_inner);
        let noise = PerlinNoise::create_legacy_for_blended_noise(&mut random_source, &octaves);

        // The "zero" octave (first created, stored at noise_levels[15]) is accessed via get_octave_noise(0)
        // because get_octave_noise does index transformation: index = len - 1 - octave
        // For octave=0 with len=16: index = 16 - 1 - 0 = 15
        // From Pumpkin's test
        if let Some(sampler) = noise.get_octave_noise(0) {
            assert_eq!(
                sampler.xo.to_bits(),
                18.223_354_299_069_797_f64.to_bits(),
                "xo mismatch"
            );
            assert_eq!(
                sampler.yo.to_bits(),
                93.992_989_078_035_95_f64.to_bits(),
                "yo mismatch"
            );
            assert_eq!(
                sampler.zo.to_bits(),
                184.481_988_757_458_23_f64.to_bits(),
                "zo mismatch"
            );
        } else {
            panic!("Expected noise at octave 0");
        }
    }
}
