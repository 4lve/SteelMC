//! Simplified evaluator for the Pumpkin-style noise router component stack.
//!
//! This evaluates the density function stack at a given position to produce terrain density.

use rustc_hash::FxHashMap;

use crate::noise::{BlendedNoise, DoublePerlinNoise};
use crate::random::{PositionalRandom, Random, RandomSource, xoroshiro::Xoroshiro};

use super::{
    BaseNoiseFunctionComponent, BaseNoiseRouter, BinaryOperation, DoublePerlinNoiseParameters,
    LinearOperation, SplineRepr, UnaryOperation,
};

/// Position for noise sampling.
#[derive(Clone, Copy)]
pub struct NoisePos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl NoisePos {
    #[must_use]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Runtime noise sampler holder.
pub struct NoiseSampler {
    sampler: DoublePerlinNoise,
    xz_scale: f64,
    y_scale: f64,
}

impl NoiseSampler {
    #[must_use]
    pub fn sample(&self, pos: &NoisePos) -> f64 {
        self.sampler.sample(
            f64::from(pos.x) * self.xz_scale,
            f64::from(pos.y) * self.y_scale,
            f64::from(pos.z) * self.xz_scale,
        )
    }
}

/// Interpolated noise sampler (`BlendedNoise` equivalent).
pub struct InterpolatedNoiseSamplerRuntime {
    blended: BlendedNoise,
}

impl InterpolatedNoiseSamplerRuntime {
    #[must_use]
    pub fn sample(&self, pos: &NoisePos) -> f64 {
        self.blended.compute(pos.x, pos.y, pos.z)
    }
}

/// Evaluated spline runtime.
pub enum SplineRuntime {
    Fixed(f32),
    Spline {
        input_index: usize,
        points: Vec<SplinePointRuntime>,
    },
}

pub struct SplinePointRuntime {
    pub location: f32,
    pub value: SplineRuntime,
    pub derivative: f32,
}

/// Runtime component evaluation context.
pub struct NoiseRouterRuntime {
    /// Pre-computed values for each component at current position.
    values: Vec<f64>,
    /// Noise samplers by `noise_id` (uses static str keys to avoid allocations).
    noise_samplers: FxHashMap<&'static str, NoiseSampler>,
    /// Shift noise samplers (uses static str keys to avoid allocations).
    shift_samplers: FxHashMap<&'static str, DoublePerlinNoise>,
    /// Interpolated noise sampler (`BlendedNoise`).
    interpolated_sampler: Option<InterpolatedNoiseSamplerRuntime>,
    /// Spline runtimes (built once, reused).
    splines: Vec<Option<Box<SplineRuntime>>>,
    /// The component stack.
    components: &'static [BaseNoiseFunctionComponent],
    /// Final density index.
    final_density_index: usize,
}

impl NoiseRouterRuntime {
    /// Create a new runtime for the given router.
    #[must_use]
    pub fn new(router: &'static BaseNoiseRouter, seed: u64) -> Self {
        let components = router.full_component_stack;
        let mut noise_samplers = FxHashMap::default();
        let mut shift_samplers = FxHashMap::default();
        let mut interpolated_sampler = None;
        let mut splines: Vec<Option<Box<SplineRuntime>>> =
            (0..components.len()).map(|_| None).collect();

        // Create random source for noise initialization
        let mut rng = Xoroshiro::from_seed(seed);
        let splitter = rng.next_positional();

        // First pass: initialize all noise samplers
        for (idx, component) in components.iter().enumerate() {
            match component {
                BaseNoiseFunctionComponent::Noise { data } => {
                    use std::collections::hash_map::Entry;
                    if let Entry::Vacant(entry) = noise_samplers.entry(data.noise_id)
                        && let Some(params) =
                            DoublePerlinNoiseParameters::id_to_parameters(data.noise_id)
                    {
                        let mut random: RandomSource = splitter.with_hash_of(params.id());
                        let sampler = DoublePerlinNoise::new(
                            &mut random,
                            params.first_octave,
                            params.amplitudes,
                            false,
                        );
                        entry.insert(NoiseSampler {
                            sampler,
                            xz_scale: data.xz_scale,
                            y_scale: data.y_scale,
                        });
                    }
                }
                BaseNoiseFunctionComponent::ShiftA { noise_id }
                | BaseNoiseFunctionComponent::ShiftB { noise_id } => {
                    use std::collections::hash_map::Entry;
                    if let Entry::Vacant(entry) = shift_samplers.entry(*noise_id)
                        && let Some(params) =
                            DoublePerlinNoiseParameters::id_to_parameters(noise_id)
                    {
                        let mut random: RandomSource = splitter.with_hash_of(params.id());
                        let sampler = DoublePerlinNoise::new(
                            &mut random,
                            params.first_octave,
                            params.amplitudes,
                            false,
                        );
                        entry.insert(sampler);
                    }
                }
                BaseNoiseFunctionComponent::ShiftedNoise { data, .. } => {
                    use std::collections::hash_map::Entry;
                    if let Entry::Vacant(entry) = noise_samplers.entry(data.noise_id)
                        && let Some(params) =
                            DoublePerlinNoiseParameters::id_to_parameters(data.noise_id)
                    {
                        let mut random: RandomSource = splitter.with_hash_of(params.id());
                        let sampler = DoublePerlinNoise::new(
                            &mut random,
                            params.first_octave,
                            params.amplitudes,
                            false,
                        );
                        entry.insert(NoiseSampler {
                            sampler,
                            xz_scale: data.xz_scale,
                            y_scale: data.y_scale,
                        });
                    }
                }
                BaseNoiseFunctionComponent::InterpolatedNoiseSampler { data } => {
                    // Create BlendedNoise
                    let mut random: RandomSource = splitter.with_hash_of("minecraft:terrain");
                    let blended = BlendedNoise::new(
                        &mut random,
                        data.scaled_xz_scale / 684.412,
                        data.scaled_y_scale / 684.412,
                        data.xz_factor,
                        data.y_factor,
                        data.smear_scale_multiplier,
                    );
                    interpolated_sampler = Some(InterpolatedNoiseSamplerRuntime { blended });
                }
                BaseNoiseFunctionComponent::Spline { spline } => {
                    splines[idx] = Some(Box::new(build_spline_runtime(spline)));
                }
                _ => {}
            }
        }

        Self {
            values: vec![0.0; components.len()],
            noise_samplers,
            shift_samplers,
            interpolated_sampler,
            splines,
            components,
            final_density_index: router.final_density,
        }
    }

    /// Compute the final density at a position.
    #[inline]
    pub fn compute(&mut self, pos: &NoisePos) -> f64 {
        // Evaluate all components in order
        for i in 0..self.components.len() {
            self.values[i] = self.evaluate_component(i, pos);
        }
        self.values[self.final_density_index]
    }

    #[allow(clippy::too_many_lines)] // Component evaluation matches on many component types
    #[inline]
    fn evaluate_component(&self, index: usize, pos: &NoisePos) -> f64 {
        match &self.components[index] {
            BaseNoiseFunctionComponent::Constant { value } => *value,

            BaseNoiseFunctionComponent::ClampedYGradient { data } => {
                let y = f64::from(pos.y);
                if y <= data.from_y {
                    data.from_value
                } else if y >= data.to_y {
                    data.to_value
                } else {
                    let t = (y - data.from_y) / (data.to_y - data.from_y);
                    data.from_value + t * (data.to_value - data.from_value)
                }
            }

            BaseNoiseFunctionComponent::Noise { data } => {
                if let Some(sampler) = self.noise_samplers.get(data.noise_id) {
                    sampler.sample(pos)
                } else {
                    0.0
                }
            }

            BaseNoiseFunctionComponent::ShiftA { noise_id } => {
                if let Some(sampler) = self.shift_samplers.get(*noise_id) {
                    sampler.sample(f64::from(pos.x), 0.0, f64::from(pos.z)) * 4.0
                } else {
                    0.0
                }
            }

            BaseNoiseFunctionComponent::ShiftB { noise_id } => {
                if let Some(sampler) = self.shift_samplers.get(*noise_id) {
                    sampler.sample(f64::from(pos.z), f64::from(pos.x), 0.0) * 4.0
                } else {
                    0.0
                }
            }

            BaseNoiseFunctionComponent::ShiftedNoise {
                shift_x_index,
                shift_y_index,
                shift_z_index,
                data,
            } => {
                let shift_x = self.values[*shift_x_index];
                let shift_y = self.values[*shift_y_index];
                let shift_z = self.values[*shift_z_index];

                if let Some(sampler) = self.noise_samplers.get(data.noise_id) {
                    let x = (f64::from(pos.x) + shift_x) * data.xz_scale;
                    let y = (f64::from(pos.y) + shift_y) * data.y_scale;
                    let z = (f64::from(pos.z) + shift_z) * data.xz_scale;
                    sampler.sampler.sample(x, y, z)
                } else {
                    0.0
                }
            }

            BaseNoiseFunctionComponent::InterpolatedNoiseSampler { .. } => {
                if let Some(ref sampler) = self.interpolated_sampler {
                    sampler.sample(pos)
                } else {
                    0.0
                }
            }

            BaseNoiseFunctionComponent::Binary {
                argument1_index,
                argument2_index,
                data,
            } => {
                let a = self.values[*argument1_index];
                let b = self.values[*argument2_index];
                match data.operation {
                    BinaryOperation::Add => a + b,
                    BinaryOperation::Mul => a * b,
                    BinaryOperation::Min => a.min(b),
                    BinaryOperation::Max => a.max(b),
                }
            }

            BaseNoiseFunctionComponent::Linear { input_index, data } => {
                let input = self.values[*input_index];
                match data.operation {
                    LinearOperation::Add => input + data.argument,
                    LinearOperation::Mul => input * data.argument,
                }
            }

            BaseNoiseFunctionComponent::Unary { input_index, data } => {
                let input = self.values[*input_index];
                match data.operation {
                    UnaryOperation::Abs => input.abs(),
                    UnaryOperation::Square => input * input,
                    UnaryOperation::Cube => input * input * input,
                    UnaryOperation::HalfNegative => {
                        if input > 0.0 {
                            input
                        } else {
                            input * 0.5
                        }
                    }
                    UnaryOperation::QuarterNegative => {
                        if input > 0.0 {
                            input
                        } else {
                            input * 0.25
                        }
                    }
                    UnaryOperation::Squeeze => {
                        let c = input.clamp(-1.0, 1.0);
                        c / 2.0 - c * c * c / 24.0
                    }
                }
            }

            BaseNoiseFunctionComponent::Clamp { input_index, data } => {
                self.values[*input_index].clamp(data.min_value, data.max_value)
            }

            BaseNoiseFunctionComponent::RangeChoice {
                input_index,
                when_in_range_index,
                when_out_range_index,
                data,
            } => {
                let input = self.values[*input_index];
                if input >= data.min_inclusive && input < data.max_exclusive {
                    self.values[*when_in_range_index]
                } else {
                    self.values[*when_out_range_index]
                }
            }

            BaseNoiseFunctionComponent::Wrapper { input_index, .. } => {
                // For now, just pass through (caching optimizations later)
                self.values[*input_index]
            }

            BaseNoiseFunctionComponent::Spline { .. } => {
                if let Some(Some(spline)) = self.splines.get(index) {
                    evaluate_spline(spline, &self.values)
                } else {
                    0.0
                }
            }

            BaseNoiseFunctionComponent::BlendAlpha => 1.0,
            BaseNoiseFunctionComponent::BlendDensity { input_index } => self.values[*input_index],
            BaseNoiseFunctionComponent::BlendOffset
            | BaseNoiseFunctionComponent::Beardifier
            | BaseNoiseFunctionComponent::EndIslands => 0.0,

            BaseNoiseFunctionComponent::WeirdScaled { input_index, data } => {
                let input = self.values[*input_index];
                let scale = data.mapper.scale(input);
                // Would need the noise sampler here, simplified for now
                input * scale
            }
        }
    }
}

fn build_spline_runtime(repr: &SplineRepr) -> SplineRuntime {
    match repr {
        SplineRepr::Fixed { value } => SplineRuntime::Fixed(*value),
        SplineRepr::Standard {
            location_function_index,
            points,
        } => SplineRuntime::Spline {
            input_index: *location_function_index,
            points: points
                .iter()
                .map(|p| SplinePointRuntime {
                    location: p.location,
                    value: build_spline_runtime(p.value),
                    derivative: p.derivative,
                })
                .collect(),
        },
    }
}

fn evaluate_spline(spline: &SplineRuntime, values: &[f64]) -> f64 {
    match spline {
        SplineRuntime::Fixed(v) => f64::from(*v),
        SplineRuntime::Spline {
            input_index,
            points,
        } => {
            let input = values[*input_index] as f32;

            if points.is_empty() {
                return 0.0;
            }

            // Binary search for the right segment
            let mut lo = 0;
            let mut hi = points.len();

            while lo < hi {
                let mid = usize::midpoint(lo, hi);
                if points[mid].location < input {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }

            if lo == 0 {
                // Before first point - use first point's value
                evaluate_spline(&points[0].value, values)
            } else if lo >= points.len() {
                // After last point - use last point's value
                evaluate_spline(&points[points.len() - 1].value, values)
            } else {
                // Interpolate between points[lo-1] and points[lo]
                let p0 = &points[lo - 1];
                let p1 = &points[lo];
                let t = (input - p0.location) / (p1.location - p0.location);

                let v0 = evaluate_spline(&p0.value, values) as f32;
                let v1 = evaluate_spline(&p1.value, values) as f32;

                // Hermite interpolation
                f64::from(hermite_interpolate(
                    t,
                    p0.location,
                    p1.location,
                    v0,
                    v1,
                    p0.derivative,
                    p1.derivative,
                ))
            }
        }
    }
}

fn hermite_interpolate(t: f32, x0: f32, x1: f32, y0: f32, y1: f32, d0: f32, d1: f32) -> f32 {
    let dx = x1 - x0;
    let t2 = t * t;
    let t3 = t2 * t;

    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    h00 * y0 + h10 * dx * d0 + h01 * y1 + h11 * dx * d1
}
