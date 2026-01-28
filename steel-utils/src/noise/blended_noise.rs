//! Blended noise implementation for vanilla terrain density.
//!
//! This is an exact port of Minecraft's `BlendedNoise` class.

// Noise code uses mathematical single-letter variables (x, y, z, i, j, k)
#![allow(clippy::many_single_char_names)]

use crate::random::RandomSource;

use super::{PerlinNoise, clamped_lerp, perlin_noise::wrap};

/// Coordinate multiplier for terrain scale.
const XZ_MULTIPLIER_BASE: f64 = 684.412;

/// Blended noise generator for terrain density.
///
/// Combines three Perlin noise samplers (min limit, max limit, and main)
/// to create smooth terrain density values used for basic terrain shape.
pub struct BlendedNoise {
    /// Noise for minimum density limit.
    min_limit_noise: PerlinNoise,
    /// Noise for maximum density limit.
    max_limit_noise: PerlinNoise,
    /// Noise for blending between min and max limits.
    main_noise: PerlinNoise,
    /// Horizontal coordinate multiplier.
    xz_multiplier: f64,
    /// Vertical coordinate multiplier.
    y_multiplier: f64,
    /// Horizontal scale factor.
    xz_factor: f64,
    /// Vertical scale factor.
    y_factor: f64,
    /// Vertical smearing multiplier.
    smear_scale_multiplier: f64,
    /// Maximum output value.
    max_value: f64,
    /// Horizontal scale (stored for serialization).
    xz_scale: f64,
    /// Vertical scale (stored for serialization).
    y_scale: f64,
}

impl BlendedNoise {
    /// Creates a new `BlendedNoise` with the given parameters using legacy initialization.
    ///
    /// This uses the LEGACY initialization path where noises are created directly
    /// from the random source, matching Minecraft's `BlendedNoise` constructor.
    pub fn new(
        random: &mut RandomSource,
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear_scale_multiplier: f64,
    ) -> Self {
        // Create the three noise samplers using the LEGACY creation method
        // This matches Java's: PerlinNoise.createLegacyForBlendedNoise(random, IntStream.rangeClosed(-15, 0))
        let limit_octaves: Vec<i32> = (-15..=0).collect();
        let blend_octaves: Vec<i32> = (-7..=0).collect();

        let min_limit_noise = PerlinNoise::create_legacy_for_blended_noise(random, &limit_octaves);
        let max_limit_noise = PerlinNoise::create_legacy_for_blended_noise(random, &limit_octaves);
        let main_noise = PerlinNoise::create_legacy_for_blended_noise(random, &blend_octaves);

        Self::new_with_noises(
            min_limit_noise,
            max_limit_noise,
            main_noise,
            xz_scale,
            y_scale,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
        )
    }

    /// Creates a `BlendedNoise` with pre-existing noise samplers.
    #[allow(clippy::too_many_arguments)] // Noise configuration requires multiple scale/factor parameters
    fn new_with_noises(
        min_limit_noise: PerlinNoise,
        max_limit_noise: PerlinNoise,
        main_noise: PerlinNoise,
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear_scale_multiplier: f64,
    ) -> Self {
        let xz_multiplier = XZ_MULTIPLIER_BASE * xz_scale;
        let y_multiplier = XZ_MULTIPLIER_BASE * y_scale;
        let max_value = min_limit_noise.max_broken_value(y_multiplier);

        Self {
            min_limit_noise,
            max_limit_noise,
            main_noise,
            xz_multiplier,
            y_multiplier,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
            max_value,
            xz_scale,
            y_scale,
        }
    }

    /// Creates a new `BlendedNoise` with a different random source but same parameters.
    ///
    /// This uses the LEGACY initialization path, matching Java's withNewRandom method.
    #[must_use]
    pub fn with_new_random(&self, random: &mut RandomSource) -> Self {
        Self::new(
            random,
            self.xz_scale,
            self.y_scale,
            self.xz_factor,
            self.y_factor,
            self.smear_scale_multiplier,
        )
    }

    /// Compute terrain density at the given block coordinates.
    /// Matches Pumpkin's `InterpolatedNoiseSampler` exactly.
    #[must_use]
    pub fn compute(&self, block_x: i32, block_y: i32, block_z: i32) -> f64 {
        let d = f64::from(block_x) * self.xz_multiplier;
        let e = f64::from(block_y) * self.y_multiplier;
        let f = f64::from(block_z) * self.xz_multiplier;

        let g = d / self.xz_factor;
        let h = e / self.y_factor;
        let i = f / self.xz_factor;

        let j = self.y_multiplier * self.smear_scale_multiplier;
        let k = j / self.y_factor;

        // Sample main noise (8 octaves) - compute all octaves without early exit
        let mut n = 0.0;
        let mut o = 1.0;

        for p in 0..8 {
            if let Some(improved_noise) = self.main_noise.get_octave_noise(p) {
                n += improved_noise.noise_with_y_params(
                    wrap(g * o),
                    wrap(h * o),
                    wrap(i * o),
                    k * o,
                    h * o,
                ) / o;
            }
            o /= 2.0;
        }

        // Compute blend factor q = (n/10 + 1) / 2
        let q = f64::midpoint(n / 10.0, 1.0);
        let is_max = q >= 1.0;
        let is_min = q <= 0.0;

        // Sample limit noises (16 octaves each)
        // Skip min_limit_noise if q >= 1.0 (is_max)
        // Skip max_limit_noise if q <= 0.0 (is_min)
        let mut l = 0.0;
        let mut m = 0.0;
        o = 1.0;

        for r in 0..16 {
            let s = wrap(d * o);
            let t = wrap(e * o);
            let u = wrap(f * o);
            let v = j * o;

            if !is_max && let Some(min_noise) = self.min_limit_noise.get_octave_noise(r) {
                l += min_noise.noise_with_y_params(s, t, u, v, e * o) / o;
            }

            if !is_min && let Some(max_noise) = self.max_limit_noise.get_octave_noise(r) {
                m += max_noise.noise_with_y_params(s, t, u, v, e * o) / o;
            }

            o /= 2.0;
        }

        // Blend between min and max limits, then scale
        clamped_lerp(l / 512.0, m / 512.0, q) / 128.0
    }

    /// Get the minimum value this noise can produce.
    #[must_use]
    pub fn min_value(&self) -> f64 {
        -self.max_value()
    }

    /// Get the maximum value this noise can produce.
    #[must_use]
    pub fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Get the horizontal scale.
    #[must_use]
    pub fn xz_scale(&self) -> f64 {
        self.xz_scale
    }

    /// Get the vertical scale.
    #[must_use]
    pub fn y_scale(&self) -> f64 {
        self.y_scale
    }

    /// Get the horizontal factor.
    #[must_use]
    pub fn xz_factor(&self) -> f64 {
        self.xz_factor
    }

    /// Get the vertical factor.
    #[must_use]
    pub fn y_factor(&self) -> f64 {
        self.y_factor
    }

    /// Get the smear scale multiplier.
    #[must_use]
    pub fn smear_scale_multiplier(&self) -> f64 {
        self.smear_scale_multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::xoroshiro::Xoroshiro;

    #[test]
    fn test_blended_noise_deterministic() {
        let rng1 = Xoroshiro::from_seed(12345);
        let mut random_source1 = RandomSource::Xoroshiro(rng1);
        let rng2 = Xoroshiro::from_seed(12345);
        let mut random_source2 = RandomSource::Xoroshiro(rng2);

        // Default overworld parameters
        let noise1 = BlendedNoise::new(&mut random_source1, 1.0, 1.0, 80.0, 160.0, 8.0);
        let noise2 = BlendedNoise::new(&mut random_source2, 1.0, 1.0, 80.0, 160.0, 8.0);

        assert_eq!(
            noise1.compute(0, 64, 0).to_bits(),
            noise2.compute(0, 64, 0).to_bits()
        );
    }

    #[test]
    fn test_blended_noise_varies() {
        let rng = Xoroshiro::from_seed(42);
        let mut random_source = RandomSource::Xoroshiro(rng);
        let noise = BlendedNoise::new(&mut random_source, 1.0, 1.0, 80.0, 160.0, 8.0);

        // Different positions should give different values
        let v1 = noise.compute(0, 64, 0);
        let v2 = noise.compute(100, 64, 100);

        assert_ne!(v1.to_bits(), v2.to_bits());
    }

    #[test]
    fn test_blended_noise_y_variation() {
        let rng = Xoroshiro::from_seed(42);
        let mut random_source = RandomSource::Xoroshiro(rng);
        let noise = BlendedNoise::new(&mut random_source, 1.0, 1.0, 80.0, 160.0, 8.0);

        // Different Y levels should give different densities
        let low = noise.compute(0, 0, 0);
        let mid = noise.compute(0, 64, 0);
        let high = noise.compute(0, 128, 0);

        // Generally, lower positions should be denser (more positive)
        // and higher positions should be less dense (more negative)
        // But this depends on the specific noise configuration
        assert!(
            low.to_bits() != mid.to_bits() || mid.to_bits() != high.to_bits(),
            "Y should affect density"
        );
    }
}
