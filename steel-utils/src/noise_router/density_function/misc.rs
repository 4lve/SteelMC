//! Miscellaneous density function components.
//!
//! This module contains various density functions like `EndIsland`, `WeirdScaled`,
//! `ClampedYGradient`, and `RangeChoice`.

// Noise code uses mathematical single-letter variables (x, y, z, i, j, k)
#![allow(clippy::many_single_char_names)]

use std::sync::Arc;

use crate::noise::{DoublePerlinNoise, SimplexNoise, clamped_map};
use crate::noise_router::{
    ClampedYGradientData, RangeChoiceData, WeirdScaledData, WeirdScaledMapper,
};
use crate::random::{Random, legacy_random::LegacyRandom};

use super::{
    IndexToNoisePos, NoiseFunctionComponentRange, NoisePos,
    StaticIndependentChunkNoiseFunctionComponentImpl,
};
use crate::noise_router::chunk_density_function::ChunkNoiseFunctionSampleOptions;
use crate::noise_router::chunk_noise_router::{
    ChunkNoiseFunctionComponent, StaticChunkNoiseFunctionComponentImpl,
};

/// End island density function for the End dimension.
#[derive(Clone)]
pub struct EndIsland {
    sampler: Arc<SimplexNoise>,
}

impl EndIsland {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut rand = LegacyRandom::from_seed(seed);
        // Skip 17292 values to match vanilla
        rand.consume_count(17292);
        Self {
            sampler: Arc::new(SimplexNoise::new(&mut rand)),
        }
    }

    fn sample_2d(sampler: &SimplexNoise, x: i32, z: i32) -> f32 {
        let i = x / 2;
        let j = z / 2;
        let k = x % 2;
        let l = z % 2;

        let f = ((x * x + z * z) as f32).sqrt().mul_add(-8.0, 100.0);
        let mut f = f.clamp(-100.0, 80.0);

        for m in -12..=12 {
            for n in -12..=12 {
                let o = i64::from(i + m);
                let p = i64::from(j + n);

                if (o * o + p * p) > 4096 && sampler.get_value_2d(o as f64, p as f64) < -0.9 {
                    let g = (o as f32).abs().mul_add(3439.0, (p as f32).abs() * 147.0) % 13.0 + 9.0;
                    let h = (k - m * 2) as f32;
                    let q = (l - n * 2) as f32;
                    let r = h.hypot(q).mul_add(-g, 100.0);
                    let s = r.clamp(-100.0, 80.0);

                    f = f.max(s);
                }
            }
        }

        f
    }
}

// These values are hardcoded from java
impl NoiseFunctionComponentRange for EndIsland {
    #[inline]
    fn min(&self) -> f64 {
        -0.843_75
    }

    #[inline]
    fn max(&self) -> f64 {
        0.562_5
    }
}

impl StaticIndependentChunkNoiseFunctionComponentImpl for EndIsland {
    fn sample(&self, pos: &impl NoisePos) -> f64 {
        (f64::from(Self::sample_2d(&self.sampler, pos.x() / 8, pos.z() / 8)) - 8.0) / 128.0
    }
}

/// Weird scaled noise density function for cave generation.
pub struct WeirdScaled {
    pub input_index: usize,
    pub sampler: DoublePerlinNoise,
    pub mapper: WeirdScaledMapper,
}

impl WeirdScaled {
    #[must_use]
    pub fn new(input_index: usize, sampler: DoublePerlinNoise, data: &WeirdScaledData) -> Self {
        Self {
            input_index,
            sampler,
            mapper: data.mapper,
        }
    }
}

impl StaticChunkNoiseFunctionComponentImpl for WeirdScaled {
    fn sample(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> f64 {
        let input_density = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut component_stack[..=self.input_index],
            pos,
            sample_options,
        );
        let scaled_density = self.mapper.scale(input_density);
        scaled_density
            * self
                .sampler
                .sample(
                    f64::from(pos.x()) / scaled_density,
                    f64::from(pos.y()) / scaled_density,
                    f64::from(pos.z()) / scaled_density,
                )
                .abs()
    }

    fn fill(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        array: &mut [f64],
        mapper: &impl IndexToNoisePos,
        sample_options: &mut ChunkNoiseFunctionSampleOptions,
    ) {
        ChunkNoiseFunctionComponent::fill_from_stack(
            &mut component_stack[..=self.input_index],
            array,
            mapper,
            sample_options,
        );

        array.iter_mut().enumerate().for_each(|(index, value)| {
            let pos = mapper.at(index, Some(sample_options));
            let scaled_density = self.mapper.scale(*value);
            *value = scaled_density
                * self
                    .sampler
                    .sample(
                        f64::from(pos.x()) / scaled_density,
                        f64::from(pos.y()) / scaled_density,
                        f64::from(pos.z()) / scaled_density,
                    )
                    .abs();
        });
    }
}

impl NoiseFunctionComponentRange for WeirdScaled {
    #[inline]
    fn min(&self) -> f64 {
        -self.max()
    }

    #[inline]
    fn max(&self) -> f64 {
        self.sampler.max_value() * self.mapper.max_multiplier()
    }
}

/// Clamped Y gradient density function.
#[derive(Clone)]
pub struct ClampedYGradient {
    data: &'static ClampedYGradientData,
}

impl ClampedYGradient {
    #[must_use]
    pub fn new(data: &'static ClampedYGradientData) -> Self {
        Self { data }
    }

    /// Get the gradient data.
    #[must_use]
    pub fn data(&self) -> &'static ClampedYGradientData {
        self.data
    }
}

impl NoiseFunctionComponentRange for ClampedYGradient {
    #[inline]
    fn min(&self) -> f64 {
        self.data.from_value.min(self.data.to_value)
    }

    #[inline]
    fn max(&self) -> f64 {
        self.data.from_value.max(self.data.to_value)
    }
}

impl StaticIndependentChunkNoiseFunctionComponentImpl for ClampedYGradient {
    fn sample(&self, pos: &impl NoisePos) -> f64 {
        clamped_map(
            f64::from(pos.y()),
            self.data.from_y,
            self.data.to_y,
            self.data.from_value,
            self.data.to_value,
        )
    }
}

/// Range choice density function that selects between two inputs based on a condition.
#[derive(Clone)]
pub struct RangeChoice {
    pub input_index: usize,
    pub when_in_index: usize,
    pub when_out_index: usize,
    data: &'static RangeChoiceData,
    min_value: f64,
    max_value: f64,
}

impl RangeChoice {
    #[must_use]
    pub fn new(
        input_index: usize,
        when_in_index: usize,
        when_out_index: usize,
        min_value: f64,
        max_value: f64,
        data: &'static RangeChoiceData,
    ) -> Self {
        Self {
            input_index,
            when_in_index,
            when_out_index,
            data,
            min_value,
            max_value,
        }
    }

    /// Check if the input value is in range.
    #[inline]
    #[must_use]
    pub fn is_in_range(&self, input: f64) -> bool {
        self.data.min_inclusive <= input && input < self.data.max_exclusive
    }

    /// Get the range choice data.
    #[must_use]
    pub fn data(&self) -> &'static RangeChoiceData {
        self.data
    }
}

impl NoiseFunctionComponentRange for RangeChoice {
    #[inline]
    fn min(&self) -> f64 {
        self.min_value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.max_value
    }
}

impl StaticChunkNoiseFunctionComponentImpl for RangeChoice {
    fn sample(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> f64 {
        let input_sample = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut component_stack[..=self.input_index],
            pos,
            sample_options,
        );

        if self.data.min_inclusive <= input_sample && input_sample < self.data.max_exclusive {
            ChunkNoiseFunctionComponent::sample_from_stack(
                &mut component_stack[..=self.when_in_index],
                pos,
                sample_options,
            )
        } else {
            ChunkNoiseFunctionComponent::sample_from_stack(
                &mut component_stack[..=self.when_out_index],
                pos,
                sample_options,
            )
        }
    }

    fn fill(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        array: &mut [f64],
        mapper: &impl IndexToNoisePos,
        sample_options: &mut ChunkNoiseFunctionSampleOptions,
    ) {
        ChunkNoiseFunctionComponent::fill_from_stack(
            &mut component_stack[..=self.input_index],
            array,
            mapper,
            sample_options,
        );

        array.iter_mut().enumerate().for_each(|(index, value)| {
            let pos = mapper.at(index, Some(sample_options));
            *value = if self.data.min_inclusive <= *value && *value < self.data.max_exclusive {
                ChunkNoiseFunctionComponent::sample_from_stack(
                    &mut component_stack[..=self.when_in_index],
                    &pos,
                    sample_options,
                )
            } else {
                ChunkNoiseFunctionComponent::sample_from_stack(
                    &mut component_stack[..=self.when_out_index],
                    &pos,
                    sample_options,
                )
            };
        });
    }
}
