//! Double Perlin noise sampler - combines two octave Perlin noise samplers.
//!
//! This matches vanilla Minecraft's `DoublePerlinNoiseSampler`.

// Noise code uses mathematical single-letter variables (x, y, z, i, j, k)
#![allow(clippy::many_single_char_names)]

use crate::random::RandomSource;

use super::PerlinNoise;

/// A double Perlin noise sampler that combines two octave Perlin noise samplers.
pub struct DoublePerlinNoise {
    first_sampler: PerlinNoise,
    second_sampler: PerlinNoise,
    amplitude: f64,
    max_value: f64,
}

impl DoublePerlinNoise {
    /// Creates a new double Perlin noise sampler.
    pub fn new(
        random: &mut RandomSource,
        first_octave: i32,
        amplitudes: &[f64],
        _legacy: bool,
    ) -> Self {
        let first_sampler = PerlinNoise::create(random, first_octave, amplitudes);
        let second_sampler = PerlinNoise::create(random, first_octave, amplitudes);

        // Find the range of non-zero amplitudes
        let mut j = i32::MAX;
        let mut k = i32::MIN;

        for (index, amplitude) in amplitudes.iter().enumerate() {
            if *amplitude != 0.0 {
                j = j.min(index as i32);
                k = k.max(index as i32);
            }
        }

        let amplitude = 0.166_666_666_666_666_66 / Self::create_amplitude(k - j);
        let max_value = (first_sampler.max_value() + second_sampler.max_value()) * amplitude;

        Self {
            first_sampler,
            second_sampler,
            amplitude,
            max_value,
        }
    }

    fn create_amplitude(octaves: i32) -> f64 {
        0.1 * (1.0 + 1.0 / f64::from(octaves + 1))
    }

    /// Returns the maximum possible value.
    #[must_use]
    pub fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Samples the noise at the given position.
    #[inline]
    #[must_use]
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        const OFFSET: f64 = 1.018_126_888_217_522_7;
        let d = x * OFFSET;
        let e = y * OFFSET;
        let f = z * OFFSET;

        (self
            .first_sampler
            .get_value_with_y_params(x, y, z, 0.0, 0.0, false)
            + self
                .second_sampler
                .get_value_with_y_params(d, e, f, 0.0, 0.0, false))
            * self.amplitude
    }
}
