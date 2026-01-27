//! Noise-based density function components.
//!
//! This module contains density functions that sample various noise sources
//! like Perlin noise, shifted noise, and interpolated noise.

// Noise code uses coordinate variables (input_x_index, input_y_index, input_z_index)
#![allow(clippy::similar_names, clippy::many_single_char_names)]

use crate::noise::{DoublePerlinNoise, PerlinNoise, clamped_lerp};
use crate::noise_router::{InterpolatedNoiseSamplerData, NoiseData, ShiftedNoiseData};
use crate::random::RandomSource;

use super::{
    NoiseFunctionComponentRange, NoisePos, StaticIndependentChunkNoiseFunctionComponentImpl,
};
use crate::noise_router::chunk_density_function::ChunkNoiseFunctionSampleOptions;
use crate::noise_router::chunk_noise_router::{
    ChunkNoiseFunctionComponent, StaticChunkNoiseFunctionComponentImpl,
};

/// A simple noise density function that samples Double Perlin noise.
///
/// Samples the noise at scaled coordinates:
/// ```text
/// sample(pos) = sampler.sample(x * xz_scale, y * y_scale, z * xz_scale)
/// ```
///
/// Range: `[-max_value, +max_value]` where `max_value` is the sampler's max.
pub struct Noise {
    /// The Double Perlin noise sampler.
    sampler: DoublePerlinNoise,
    /// Scale factors for sampling.
    data: &'static NoiseData,
}

impl Noise {
    #[must_use]
    pub fn new(sampler: DoublePerlinNoise, data: &'static NoiseData) -> Self {
        Self { sampler, data }
    }

    /// Get the noise data.
    #[must_use]
    pub fn data(&self) -> &'static NoiseData {
        self.data
    }

    /// Get the underlying sampler.
    #[must_use]
    pub fn sampler(&self) -> &DoublePerlinNoise {
        &self.sampler
    }
}

impl NoiseFunctionComponentRange for Noise {
    #[inline]
    fn min(&self) -> f64 {
        -self.max()
    }

    #[inline]
    fn max(&self) -> f64 {
        self.sampler.max_value()
    }
}

impl StaticIndependentChunkNoiseFunctionComponentImpl for Noise {
    fn sample(&self, pos: &impl NoisePos) -> f64 {
        self.sampler.sample(
            f64::from(pos.x()) * self.data.xz_scale,
            f64::from(pos.y()) * self.data.y_scale,
            f64::from(pos.z()) * self.data.xz_scale,
        )
    }
}

#[inline]
fn shift_sample_3d(sampler: &DoublePerlinNoise, x: f64, y: f64, z: f64) -> f64 {
    sampler.sample(x * 0.25f64, y * 0.25f64, z * 0.25f64) * 4f64
}

/// Shift A density function for domain warping.
///
/// Samples noise to produce an offset for the X coordinate.
/// Used to add organic variation to terrain features.
///
/// ```text
/// sample(pos) = sampler.sample(x * 0.25, 0, z * 0.25) * 4
/// ```
pub struct ShiftA {
    /// The offset noise sampler.
    sampler: DoublePerlinNoise,
}

impl ShiftA {
    #[must_use]
    pub fn new(sampler: DoublePerlinNoise) -> Self {
        Self { sampler }
    }

    /// Get the underlying sampler.
    #[must_use]
    pub fn sampler(&self) -> &DoublePerlinNoise {
        &self.sampler
    }
}

impl NoiseFunctionComponentRange for ShiftA {
    #[inline]
    fn min(&self) -> f64 {
        -self.max()
    }

    #[inline]
    fn max(&self) -> f64 {
        self.sampler.max_value() * 4.0
    }
}

impl StaticIndependentChunkNoiseFunctionComponentImpl for ShiftA {
    fn sample(&self, pos: &impl NoisePos) -> f64 {
        shift_sample_3d(&self.sampler, f64::from(pos.x()), 0.0, f64::from(pos.z()))
    }
}

/// Shift B density function for domain warping.
///
/// Samples noise to produce an offset for the Z coordinate.
/// Uses swapped X/Z inputs compared to ShiftA for varied warping.
///
/// ```text
/// sample(pos) = sampler.sample(z * 0.25, x * 0.25, 0) * 4
/// ```
pub struct ShiftB {
    /// The offset noise sampler.
    sampler: DoublePerlinNoise,
}

impl ShiftB {
    #[must_use]
    pub fn new(sampler: DoublePerlinNoise) -> Self {
        Self { sampler }
    }

    /// Get the underlying sampler.
    #[must_use]
    pub fn sampler(&self) -> &DoublePerlinNoise {
        &self.sampler
    }
}

impl NoiseFunctionComponentRange for ShiftB {
    #[inline]
    fn min(&self) -> f64 {
        -self.max()
    }

    #[inline]
    fn max(&self) -> f64 {
        self.sampler.max_value() * 4.0
    }
}

impl StaticIndependentChunkNoiseFunctionComponentImpl for ShiftB {
    fn sample(&self, pos: &impl NoisePos) -> f64 {
        shift_sample_3d(&self.sampler, f64::from(pos.z()), f64::from(pos.x()), 0.0)
    }
}

/// Shifted noise density function with domain warping.
///
/// Applies coordinate offsets from other density functions before
/// sampling noise. This creates organically warped terrain features.
///
/// ```text
/// translated_x = x * xz_scale + input_x.sample(pos)
/// translated_y = y * y_scale + input_y.sample(pos)  // or just input_y if y_scale == 0
/// translated_z = z * xz_scale + input_z.sample(pos)
/// sample(pos) = sampler.sample(translated_x, translated_y, translated_z)
/// ```
pub struct ShiftedNoise {
    /// Index of X offset component.
    pub input_x_index: usize,
    /// Index of Y offset component.
    pub input_y_index: usize,
    /// Index of Z offset component.
    pub input_z_index: usize,
    /// The noise sampler.
    sampler: DoublePerlinNoise,
    /// Scale factors.
    data: &'static ShiftedNoiseData,
}

impl ShiftedNoise {
    #[must_use]
    pub fn new(
        input_x_index: usize,
        input_y_index: usize,
        input_z_index: usize,
        sampler: DoublePerlinNoise,
        data: &'static ShiftedNoiseData,
    ) -> Self {
        Self {
            input_x_index,
            input_y_index,
            input_z_index,
            sampler,
            data,
        }
    }

    /// Get the noise data.
    #[must_use]
    pub fn data(&self) -> &'static ShiftedNoiseData {
        self.data
    }

    /// Get the underlying sampler.
    #[must_use]
    pub fn sampler(&self) -> &DoublePerlinNoise {
        &self.sampler
    }
}

impl NoiseFunctionComponentRange for ShiftedNoise {
    #[inline]
    fn min(&self) -> f64 {
        -self.max()
    }

    #[inline]
    fn max(&self) -> f64 {
        self.sampler.max_value()
    }
}

impl StaticChunkNoiseFunctionComponentImpl for ShiftedNoise {
    fn sample(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> f64 {
        let translated_x = f64::from(pos.x()) * self.data.xz_scale
            + ChunkNoiseFunctionComponent::sample_from_stack(
                &mut component_stack[..=self.input_x_index],
                pos,
                sample_options,
            );

        // Optimization: For 2D terrain noises, y_scale is 0.0, so we skip the Y scaling.
        // The shift_y is typically Constant(0.0) for 2D noises, so this also avoids
        // unnecessary multiplication.
        let translated_y = if self.data.y_scale == 0.0 {
            ChunkNoiseFunctionComponent::sample_from_stack(
                &mut component_stack[..=self.input_y_index],
                pos,
                sample_options,
            )
        } else {
            f64::from(pos.y()) * self.data.y_scale
                + ChunkNoiseFunctionComponent::sample_from_stack(
                    &mut component_stack[..=self.input_y_index],
                    pos,
                    sample_options,
                )
        };

        let translated_z = f64::from(pos.z()) * self.data.xz_scale
            + ChunkNoiseFunctionComponent::sample_from_stack(
                &mut component_stack[..=self.input_z_index],
                pos,
                sample_options,
            );

        self.sampler
            .sample(translated_x, translated_y, translated_z)
    }
}

/// Interpolated noise sampler for blended terrain generation.
///
/// This combines three Perlin noise samplers to create smooth,
/// natural-looking terrain. It interpolates between "lower" and
/// "upper" noise using a "selector" noise.
///
/// # Algorithm
///
/// 1. Sample selector noise (8 octaves) to get blend factor `q`
/// 2. Sample lower noise (16 octaves) if `q < 1`
/// 3. Sample upper noise (16 octaves) if `q > 0`
/// 4. Lerp between lower and upper using clamped `q`
///
/// This creates terrain that smoothly transitions between different
/// noise patterns, avoiding harsh boundaries.
pub struct InterpolatedNoiseSampler {
    /// Lower bound noise (16 octaves).
    lower_noise: PerlinNoise,
    /// Upper bound noise (16 octaves).
    upper_noise: PerlinNoise,
    /// Selector/blend noise (8 octaves).
    noise: PerlinNoise,
    /// Configuration data.
    data: &'static InterpolatedNoiseSamplerData,
    /// Maximum possible output value.
    max_value: f64,
}

impl InterpolatedNoiseSampler {
    pub fn new(data: &'static InterpolatedNoiseSamplerData, random: &mut RandomSource) -> Self {
        // Use legacy initialization (sequential random consumption, no splitter)
        // This matches vanilla Minecraft's BlendedNoise initialization
        let big_octaves: Vec<i32> = (-15..=0).collect();
        let little_octaves: Vec<i32> = (-7..=0).collect();

        let lower_noise = PerlinNoise::create_legacy_for_blended_noise(random, &big_octaves);
        let upper_noise = PerlinNoise::create_legacy_for_blended_noise(random, &big_octaves);
        let noise = PerlinNoise::create_legacy_for_blended_noise(random, &little_octaves);

        let max_value = lower_noise.max_broken_value(data.scaled_y_scale);

        Self {
            lower_noise,
            upper_noise,
            noise,
            data,
            max_value,
        }
    }

    /// Get the noise data.
    #[must_use]
    pub fn data(&self) -> &'static InterpolatedNoiseSamplerData {
        self.data
    }
}

impl NoiseFunctionComponentRange for InterpolatedNoiseSampler {
    #[inline]
    fn min(&self) -> f64 {
        -self.max()
    }

    #[inline]
    fn max(&self) -> f64 {
        self.max_value
    }
}

impl StaticIndependentChunkNoiseFunctionComponentImpl for InterpolatedNoiseSampler {
    fn sample(&self, pos: &impl NoisePos) -> f64 {
        let d = f64::from(pos.x()) * self.data.scaled_xz_scale;
        let e = f64::from(pos.y()) * self.data.scaled_y_scale;
        let f = f64::from(pos.z()) * self.data.scaled_xz_scale;

        let g = d / self.data.xz_factor;
        let h = e / self.data.y_factor;
        let i = f / self.data.xz_factor;

        let j = self.data.scaled_y_scale * self.data.smear_scale_multiplier;
        let k = j / self.data.y_factor;

        // Sample the selector noise using blended noise method
        let n: f64 = self.noise.get_value_for_blended_noise(g, h, i, k, h);

        let q = f64::midpoint(n / 10f64, 1f64);
        let bl2 = q >= 1f64;
        let bl3 = q <= 0f64;

        let l = if bl2 {
            0.0
        } else {
            self.lower_noise.get_value_for_blended_noise(d, e, f, j, e)
        };

        let m = if bl3 {
            0.0
        } else {
            self.upper_noise.get_value_for_blended_noise(d, e, f, j, e)
        };

        clamped_lerp(l / 512f64, m / 512f64, q) / 128f64
    }
}
