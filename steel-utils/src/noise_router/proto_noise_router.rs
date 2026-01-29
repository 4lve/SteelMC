//! Proto noise router for building the noise function component stack.
//!
//! This module contains the proto noise router which builds a stack of noise function
//! components from the base router data, initializing noise samplers with the world seed.
//!
//! # Overview
//!
//! The proto noise router is the **seed-dependent** stage of the noise router system.
//! It is built once per world and contains:
//! - Initialized noise samplers with the world seed
//! - The full component stack with computed min/max ranges
//! - Indices to key output functions
//!
//! # Three Router Types
//!
//! [`ProtoNoiseRouters`] contains three separate routers:
//!
//! | Router | Purpose |
//! |--------|---------|
//! | `noise` | Main terrain density functions |
//! | `surface_estimator` | Surface height estimation for aquifers |
//! | `multi_noise` | Biome parameter functions (temperature, humidity, etc.) |
//!
//! # Component Stack
//!
//! The stack is built from [`BaseNoiseFunctionComponent`]s by:
//! 1. Converting base components to proto components
//! 2. Initializing noise samplers with seed
//! 3. Computing min/max ranges for each component
//!
//! # Key Outputs
//!
//! The noise router provides indices to these density functions:
//! - `final_density` - Main terrain shape
//! - `barrier_noise`, `fluid_level_*` - Aquifer parameters
//! - `vein_toggle`, `vein_ridged`, `vein_gap` - Ore veins
//! - `erosion`, `depth` - Terrain features
//!
//! [`BaseNoiseFunctionComponent`]: crate::noise_router::BaseNoiseFunctionComponent

use enum_dispatch::enum_dispatch;

use crate::noise::DoublePerlinNoise;
use crate::noise_router::{
    BaseNoiseFunctionComponent, BaseNoiseRouters, BinaryOperation, DoublePerlinNoiseParameters,
    LinearOperation, SplineRepr, UnaryOperation,
};
use crate::random::{PositionalRandom, Random, RandomSource, RandomSplitter, xoroshiro::Xoroshiro};

use super::chunk_density_function::ChunkNoiseFunctionSampleOptions;
use super::chunk_noise_router::{
    ChunkNoiseFunctionComponent, StaticChunkNoiseFunctionComponentImpl,
};
use super::density_function::{
    IndexToNoisePos, NoiseFunctionComponentRange, NoisePos, PassThrough,
    StaticIndependentChunkNoiseFunctionComponentImpl, Wrapper,
    math::{Binary, Clamp, Constant, Linear, Unary},
    misc::{ClampedYGradient, EndIsland, RangeChoice, WeirdScaled},
    noise::{InterpolatedNoiseSampler, Noise, ShiftA, ShiftB, ShiftedNoise},
    spline::{Spline, SplineFunction, SplinePoint, SplineValue},
};

/// Independent proto noise function components.
///
/// These components don't depend on other components in the stack and can
/// be sampled directly without recursion. They are shared across chunks.
#[enum_dispatch(
    StaticIndependentChunkNoiseFunctionComponentImpl,
    NoiseFunctionComponentRange
)]
pub enum IndependentProtoNoiseFunctionComponent {
    /// Constant value.
    Constant(Constant),
    /// End dimension islands.
    EndIsland(EndIsland),
    /// Simple Perlin noise.
    Noise(Noise),
    /// X-offset shift noise.
    ShiftA(ShiftA),
    /// Z-offset shift noise.
    ShiftB(ShiftB),
    /// Blended interpolated noise.
    InterpolatedNoise(InterpolatedNoiseSampler),
    /// Y-based gradient.
    ClampedYGradient(ClampedYGradient),
}

/// Dependent proto noise function components.
///
/// These components depend on other components in the stack and require
/// recursive sampling. They are shared across chunks.
#[enum_dispatch(StaticChunkNoiseFunctionComponentImpl, NoiseFunctionComponentRange)]
pub enum DependentProtoNoiseFunctionComponent {
    /// Linear transformation (add/mul by constant).
    Linear(Linear),
    /// Unary operation (abs, square, etc.).
    Unary(Unary),
    /// Binary operation (add, mul, min, max).
    Binary(Binary),
    /// Domain-warped noise.
    ShiftedNoise(ShiftedNoise),
    /// Cave-scaled noise.
    WeirdScaled(WeirdScaled),
    /// Clamping operation.
    Clamp(Clamp),
    /// Conditional range choice.
    RangeChoice(RangeChoice),
    /// Cubic spline terrain shaping.
    Spline(SplineFunction),
}

/// Proto noise function component (seed-initialized).
///
/// This enum represents the **second stage (Stage 2)** of the three-stage noise router
/// architecture. Proto components have:
/// - Initialized noise samplers seeded with the world seed
/// - Computed min/max value ranges for optimization
/// - No chunk-specific caches or interpolation data
///
/// # Architecture Overview
///
/// The noise router uses a three-stage architecture to efficiently generate terrain:
///
/// | Stage | Type | Scope | Purpose |
/// |-------|------|-------|---------|
/// | 1 | [`BaseNoiseFunctionComponent`] | Global (constant) | Static compile-time data |
/// | 2 | **`ProtoNoiseFunctionComponent`** | **Per-world** | **Seed-initialized samplers** |
/// | 3 | [`ChunkNoiseFunctionComponent`] | Per-chunk | Cached/interpolated values |
///
/// Proto components are built once per world from base components using
/// [`ProtoNoiseRouters::generate_proto_stack`]. They serve as templates for
/// creating chunk-specific routers.
///
/// # Variants
///
/// - [`Independent`](ProtoNoiseFunctionComponent::Independent) - Components that can be sampled
///   directly at any position without depending on other components. Examples: constants, raw
///   noise, Y-gradients.
///
/// - [`Dependent`](ProtoNoiseFunctionComponent::Dependent) - Components that reference other
///   components by index and require recursive sampling. Examples: math operations, splines,
///   range choices.
///
/// - [`Wrapper`](ProtoNoiseFunctionComponent::Wrapper) - Special components that wrap an input
///   and mark it for chunk-level caching. At proto stage, these just pass through; at chunk
///   stage, they become 2D/3D interpolation caches.
///
/// - [`PassThrough`](ProtoNoiseFunctionComponent::PassThrough) - Placeholder components for
///   features not yet implemented (e.g., blend density). These forward sampling to their
///   input with custom min/max bounds.
///
/// # Sampling Flow
///
/// When a chunk router samples a proto component:
///
/// 1. Proto components provide the base sampling logic via trait implementations
/// 2. Chunk components may add caching/interpolation on top
/// 3. Dependent components recursively sample their inputs from the component stack
/// 4. Final values are used for terrain density decisions
///
/// # Example
///
/// ```ignore
/// // Proto components are typically accessed through ProtoNoiseRouter
/// let proto_routers = ProtoNoiseRouters::generate(&base_routers, world_seed);
///
/// // Access a specific component
/// let final_density = &proto_routers.noise.full_component_stack[proto_routers.noise.final_density];
///
/// // Check the component's range
/// let (min, max) = (final_density.min(), final_density.max());
/// ```
///
/// [`BaseNoiseFunctionComponent`]: crate::noise_router::BaseNoiseFunctionComponent
/// [`ChunkNoiseFunctionComponent`]: super::chunk_noise_router::ChunkNoiseFunctionComponent
/// [`ProtoNoiseRouters::generate_proto_stack`]: ProtoNoiseRouters::generate_proto_stack
#[enum_dispatch(NoiseFunctionComponentRange)]
pub enum ProtoNoiseFunctionComponent {
    /// Independent component that can be sampled directly without dependencies.
    ///
    /// These components (constants, noise samplers, gradients) don't reference other
    /// components and can compute their value from just the position.
    Independent(IndependentProtoNoiseFunctionComponent),

    /// Dependent component that requires recursive sampling of other components.
    ///
    /// These components (math operations, splines, conditionals) reference other
    /// components by index and build complex density functions through composition.
    Dependent(DependentProtoNoiseFunctionComponent),

    /// Cache wrapper that marks an input for chunk-level caching.
    ///
    /// At proto stage, this just passes through to its input. At chunk stage,
    /// it becomes a 2D (flat cache) or 3D (interpolated) cache depending on
    /// the wrapper type.
    Wrapper(Wrapper),

    /// Pass-through placeholder for unimplemented features.
    ///
    /// Used for blend density and similar components that are not yet implemented.
    /// Forwards sampling to the input component with custom min/max bounds.
    PassThrough(PassThrough),
}

/// Builder for creating Double Perlin noise samplers from noise IDs.
///
/// This builder is used during proto router construction to create noise
/// samplers for each noise function in the component stack. It derives
/// random states from the world seed for consistent generation.
pub struct DoublePerlinNoiseBuilder {
    /// Random deriver for creating noise-specific random states.
    pub base_random_deriver: RandomSplitter,
}

impl DoublePerlinNoiseBuilder {
    /// Creates a new noise builder from the world seed.
    ///
    /// The seed is used to derive deterministic random states for each
    /// noise sampler, ensuring consistent terrain generation.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut rng = Xoroshiro::from_seed(seed);
        Self {
            base_random_deriver: rng.next_positional(),
        }
    }

    /// Returns a noise sampler for the given noise ID.
    ///
    /// # Panics
    ///
    /// Panics if `id` is not a recognized noise parameter ID.
    #[must_use]
    pub fn get_noise_sampler_for_id(&self, id: &str) -> DoublePerlinNoise {
        let parameters = DoublePerlinNoiseParameters::id_to_parameters(id)
            .unwrap_or_else(|| panic!("Unknown noise id: {id}"));

        // Note that the parameters' id is different than `id`
        let mut random: RandomSource = self.base_random_deriver.with_hash_of(parameters.id());
        DoublePerlinNoise::new(
            &mut random,
            parameters.first_octave,
            parameters.amplitudes,
            false,
        )
    }
}

/// Seed-initialized noise router for terrain density evaluation.
///
/// This is the first stage of the noise router architecture. It is built
/// once per world from the seed and contains:
/// - The full component stack with initialized noise samplers
/// - Indices to key output density functions
///
/// The proto router is then used to create [`ChunkNoiseRouter`]s for each chunk.
///
/// [`ChunkNoiseRouter`]: super::chunk_noise_router::ChunkNoiseRouter
pub struct ProtoNoiseRouter {
    /// The full stack of density function components.
    pub full_component_stack: Box<[ProtoNoiseFunctionComponent]>,
    /// Index for barrier noise (aquifer barriers).
    pub barrier_noise: usize,
    /// Index for fluid level floodedness noise.
    pub fluid_level_floodedness_noise: usize,
    /// Index for fluid level spread noise.
    pub fluid_level_spread_noise: usize,
    /// Index for lava placement noise.
    pub lava_noise: usize,
    /// Index for erosion value.
    pub erosion: usize,
    /// Index for depth below surface.
    pub depth: usize,
    /// Index for final density (solid vs air decision).
    pub final_density: usize,
    /// Index for ore vein toggle (copper vs iron).
    pub vein_toggle: usize,
    /// Index for ore vein ridged noise.
    pub vein_ridged: usize,
    /// Index for ore vein gap noise.
    pub vein_gap: usize,
}

/// Surface height estimator for aquifer calculations.
///
/// This router samples downward to find the first solid block,
/// providing surface height estimates for the aquifer system.
pub struct ProtoSurfaceEstimator {
    /// Component stack for surface estimation.
    pub full_component_stack: Box<[ProtoNoiseFunctionComponent]>,
}

/// Multi-noise router for biome parameter calculation.
///
/// This router provides the climate parameters used for biome selection:
/// temperature, humidity/vegetation, continentalness, erosion, depth, and ridges.
pub struct ProtoMultiNoiseRouter {
    /// Component stack for biome parameters.
    pub full_component_stack: Box<[ProtoNoiseFunctionComponent]>,
    /// Index for temperature parameter.
    pub temperature: usize,
    /// Index for vegetation/humidity parameter.
    pub vegetation: usize,
    /// Index for continentalness parameter.
    pub continents: usize,
    /// Index for erosion parameter.
    pub erosion: usize,
    /// Index for depth parameter.
    pub depth: usize,
    /// Index for ridges/weirdness parameter.
    pub ridges: usize,
}

/// Collection of all proto noise routers for a world.
///
/// Contains three routers for different purposes:
/// - `noise`: Main terrain density functions
/// - `surface_estimator`: Surface height for aquifers
/// - `multi_noise`: Biome climate parameters
pub struct ProtoNoiseRouters {
    /// Main terrain generation router.
    pub noise: ProtoNoiseRouter,
    /// Surface height estimation router.
    pub surface_estimator: ProtoSurfaceEstimator,
    /// Multi-noise router for biome parameters.
    pub multi_noise: ProtoMultiNoiseRouter,
}

/// Recursively builds a spline from static representation.
///
/// Converts `SplineRepr` (static compile-time data) to `SplineValue`
/// (runtime-usable spline structure).
fn build_spline_recursive(spline_repr: &SplineRepr) -> SplineValue {
    match spline_repr {
        SplineRepr::Standard {
            location_function_index,
            points,
        } => {
            let points = points
                .iter()
                .map(|point| {
                    let value = build_spline_recursive(point.value);
                    SplinePoint::new(point.location, value, point.derivative)
                })
                .collect();
            SplineValue::Spline(Spline::new(*location_function_index, points))
        }
        // Top level splines always take a density function as input
        SplineRepr::Fixed { value } => SplineValue::Fixed(*value),
    }
}

impl ProtoNoiseRouters {
    /// Generates a proto component stack from a base component stack.
    ///
    /// This is the core transformation from Stage 1 (static data) to Stage 2
    /// (seed-initialized). For each `BaseNoiseFunctionComponent`:
    ///
    /// 1. Initializes noise samplers with the world seed
    /// 2. Computes min/max bounds from dependencies
    /// 3. Creates the corresponding `ProtoNoiseFunctionComponent`
    ///
    /// # Arguments
    ///
    /// * `base_stack` - Static component stack from `data.rs`
    /// * `seed` - World seed for noise initialization
    ///
    /// # Returns
    ///
    /// Boxed slice of proto components with initialized samplers.
    #[allow(clippy::too_many_lines)] // Construction iterates over all component types
    #[must_use]
    pub fn generate_proto_stack(
        base_stack: &[BaseNoiseFunctionComponent],
        seed: u64,
    ) -> Box<[ProtoNoiseFunctionComponent]> {
        let perlin_noise_builder = DoublePerlinNoiseBuilder::new(seed);

        // Contiguous memory for our function components
        let mut stack = Vec::<ProtoNoiseFunctionComponent>::with_capacity(base_stack.len());

        for component in base_stack {
            let converted = match component {
                BaseNoiseFunctionComponent::Spline { spline } => {
                    let spline = match build_spline_recursive(spline) {
                        SplineValue::Spline(spline) => spline,
                        // Top level splines always take in a density function
                        SplineValue::Fixed(_) => unreachable!(),
                    };

                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::Spline(SplineFunction::new(
                            spline, &stack,
                        )),
                    )
                }
                BaseNoiseFunctionComponent::EndIslands => ProtoNoiseFunctionComponent::Independent(
                    IndependentProtoNoiseFunctionComponent::EndIsland(EndIsland::new(seed)),
                ),
                BaseNoiseFunctionComponent::Noise { data } => {
                    let sampler = perlin_noise_builder.get_noise_sampler_for_id(data.noise_id);
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::Noise(Noise::new(sampler, data)),
                    )
                }
                BaseNoiseFunctionComponent::ShiftA { noise_id } => {
                    let sampler = perlin_noise_builder.get_noise_sampler_for_id(noise_id);
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::ShiftA(ShiftA::new(sampler)),
                    )
                }
                BaseNoiseFunctionComponent::ShiftB { noise_id } => {
                    let sampler = perlin_noise_builder.get_noise_sampler_for_id(noise_id);
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::ShiftB(ShiftB::new(sampler)),
                    )
                }
                BaseNoiseFunctionComponent::BlendDensity { input_index } => {
                    // TODO: Replace this when the blender is implemented
                    // Vanilla BlendedNoise returns -Infinity/Infinity for bounds
                    let min_value = f64::NEG_INFINITY;
                    let max_value = f64::INFINITY;

                    ProtoNoiseFunctionComponent::PassThrough(PassThrough::new(
                        *input_index,
                        min_value,
                        max_value,
                    ))
                }
                BaseNoiseFunctionComponent::BlendAlpha => {
                    // TODO: Replace this with the cache when the blender is implemented
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::Constant(Constant::new(1.0)),
                    )
                }
                BaseNoiseFunctionComponent::BlendOffset => {
                    // TODO: Replace this with the cache when the blender is implemented
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::Constant(Constant::new(0.0)),
                    )
                }
                BaseNoiseFunctionComponent::Beardifier => {
                    // TODO: Replace this when world structures are implemented
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::Constant(Constant::new(0.0)),
                    )
                }
                BaseNoiseFunctionComponent::ShiftedNoise {
                    shift_x_index,
                    shift_y_index,
                    shift_z_index,
                    data,
                } => {
                    let sampler = perlin_noise_builder.get_noise_sampler_for_id(data.noise_id);
                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::ShiftedNoise(ShiftedNoise::new(
                            *shift_x_index,
                            *shift_y_index,
                            *shift_z_index,
                            sampler,
                            data,
                        )),
                    )
                }
                BaseNoiseFunctionComponent::RangeChoice {
                    input_index,
                    when_in_range_index,
                    when_out_range_index,
                    data,
                } => {
                    let min_value = stack[*when_in_range_index]
                        .min()
                        .min(stack[*when_out_range_index].min());
                    let max_value = stack[*when_in_range_index]
                        .max()
                        .max(stack[*when_out_range_index].max());

                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::RangeChoice(RangeChoice::new(
                            *input_index,
                            *when_in_range_index,
                            *when_out_range_index,
                            min_value,
                            max_value,
                            data,
                        )),
                    )
                }
                BaseNoiseFunctionComponent::Binary {
                    argument1_index,
                    argument2_index,
                    data,
                } => {
                    let arg1_min = stack[*argument1_index].min();
                    let arg1_max = stack[*argument1_index].max();

                    let arg2_min = stack[*argument2_index].min();
                    let arg2_max = stack[*argument2_index].max();

                    let (min, max) = match data.operation {
                        BinaryOperation::Add => (arg1_min + arg2_min, arg1_max + arg2_max),
                        BinaryOperation::Mul => {
                            let min = if arg1_min > 0.0 && arg2_min > 0.0 {
                                arg1_min * arg2_min
                            } else if arg1_max < 0.0 && arg2_max < 0.0 {
                                arg1_max * arg2_max
                            } else {
                                (arg1_min * arg2_max).min(arg1_max * arg2_min)
                            };

                            let max = if arg1_min > 0.0 && arg2_min > 0.0 {
                                arg1_max * arg2_max
                            } else if arg1_max < 0.0 && arg2_max < 0.0 {
                                arg1_min * arg2_min
                            } else {
                                (arg1_min * arg2_min).max(arg1_max * arg2_max)
                            };

                            (min, max)
                        }
                        BinaryOperation::Min => (arg1_min.min(arg2_min), arg1_max.min(arg2_max)),
                        BinaryOperation::Max => (arg1_min.max(arg2_min), arg1_max.max(arg2_max)),
                    };

                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::Binary(Binary::new(
                            *argument1_index,
                            *argument2_index,
                            min,
                            max,
                            data,
                        )),
                    )
                }
                BaseNoiseFunctionComponent::ClampedYGradient { data } => {
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::ClampedYGradient(
                            ClampedYGradient::new(data),
                        ),
                    )
                }
                BaseNoiseFunctionComponent::Constant { value } => {
                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::Constant(Constant::new(*value)),
                    )
                }
                BaseNoiseFunctionComponent::Wrapper {
                    input_index,
                    wrapper,
                } => {
                    let min_value = stack[*input_index].min();
                    let max_value = stack[*input_index].max();

                    ProtoNoiseFunctionComponent::Wrapper(Wrapper::new(
                        *input_index,
                        *wrapper,
                        min_value,
                        max_value,
                    ))
                }
                BaseNoiseFunctionComponent::Linear { input_index, data } => {
                    let arg1_min = stack[*input_index].min();
                    let arg1_max = stack[*input_index].max();

                    let (min, max) = match data.operation {
                        LinearOperation::Add => {
                            (arg1_min + data.argument, arg1_max + data.argument)
                        }
                        LinearOperation::Mul => {
                            let min = if arg1_min > 0.0 && data.argument > 0.0 {
                                arg1_min * data.argument
                            } else if arg1_max < 0.0 && data.argument < 0.0 {
                                arg1_max * data.argument
                            } else {
                                (arg1_min * data.argument).min(arg1_max * data.argument)
                            };

                            let max = if arg1_min > 0.0 && data.argument > 0.0 {
                                arg1_max * data.argument
                            } else if arg1_max < 0.0 && data.argument < 0.0 {
                                arg1_min * data.argument
                            } else {
                                (arg1_min * data.argument).max(arg1_max * data.argument)
                            };

                            (min, max)
                        }
                    };

                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::Linear(Linear::new(
                            *input_index,
                            min,
                            max,
                            data,
                        )),
                    )
                }
                BaseNoiseFunctionComponent::Clamp { input_index, data } => {
                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::Clamp(Clamp::new(*input_index, data)),
                    )
                }
                BaseNoiseFunctionComponent::Unary { input_index, data } => {
                    let arg1_min = stack[*input_index].min();
                    let arg1_max = stack[*input_index].max();

                    let applied_min_value = data.apply_density(arg1_min);
                    let applied_max_value = data.apply_density(arg1_max);

                    let (min_value, max_value) = match data.operation {
                        UnaryOperation::Abs => {
                            if arg1_min >= 0.0 {
                                // All positive: abs is identity
                                (arg1_min, arg1_max)
                            } else if arg1_max <= 0.0 {
                                // All negative: abs flips the range
                                (-arg1_max, -arg1_min)
                            } else {
                                // Spans zero: min is 0, max is larger absolute value
                                (0.0, arg1_max.max(-arg1_min))
                            }
                        }
                        UnaryOperation::Square => {
                            if arg1_min >= 0.0 {
                                // All positive: square preserves order
                                (arg1_min * arg1_min, arg1_max * arg1_max)
                            } else if arg1_max <= 0.0 {
                                // All negative: square reverses order
                                (arg1_max * arg1_max, arg1_min * arg1_min)
                            } else {
                                // Spans zero: min is 0, max is larger squared value
                                (0.0, (arg1_min * arg1_min).max(arg1_max * arg1_max))
                            }
                        }
                        UnaryOperation::Squeeze
                        | UnaryOperation::Cube
                        | UnaryOperation::QuarterNegative
                        | UnaryOperation::HalfNegative => (applied_min_value, applied_max_value),
                    };

                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::Unary(Unary::new(
                            *input_index,
                            min_value,
                            max_value,
                            data,
                        )),
                    )
                }
                BaseNoiseFunctionComponent::WeirdScaled { input_index, data } => {
                    let sampler = perlin_noise_builder.get_noise_sampler_for_id(data.noise_id);
                    ProtoNoiseFunctionComponent::Dependent(
                        DependentProtoNoiseFunctionComponent::WeirdScaled(WeirdScaled::new(
                            *input_index,
                            sampler,
                            data,
                        )),
                    )
                }
                BaseNoiseFunctionComponent::InterpolatedNoiseSampler { data } => {
                    let mut random: RandomSource = perlin_noise_builder
                        .base_random_deriver
                        .with_hash_of("minecraft:terrain");

                    ProtoNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::InterpolatedNoise(
                            InterpolatedNoiseSampler::new(data, &mut random),
                        ),
                    )
                }
            };

            stack.push(converted);
        }

        stack.into()
    }

    /// Generates all proto noise routers from base routers.
    ///
    /// Creates the complete proto router collection for a world by
    /// generating proto stacks for:
    /// - Main terrain density (`noise`)
    /// - Surface height estimation (`surface_estimator`)
    /// - Biome climate parameters (`multi_noise`)
    ///
    /// # Arguments
    ///
    /// * `base` - Static base routers from `OVERWORLD_BASE_NOISE_ROUTER`
    /// * `seed` - World seed
    ///
    /// # Returns
    ///
    /// Complete `ProtoNoiseRouters` ready for chunk router creation.
    #[must_use]
    pub fn generate(base: &BaseNoiseRouters, seed: u64) -> Self {
        let noise_stack = Self::generate_proto_stack(base.noise.full_component_stack, seed);
        let surface_stack =
            Self::generate_proto_stack(base.surface_estimator.full_component_stack, seed);
        let multi_noise_stack =
            Self::generate_proto_stack(base.multi_noise.full_component_stack, seed);

        Self {
            noise: ProtoNoiseRouter {
                full_component_stack: noise_stack,
                barrier_noise: base.noise.barrier_noise,
                fluid_level_floodedness_noise: base.noise.fluid_level_floodedness_noise,
                fluid_level_spread_noise: base.noise.fluid_level_spread_noise,
                lava_noise: base.noise.lava_noise,
                erosion: base.noise.erosion,
                depth: base.noise.depth,
                final_density: base.noise.final_density,
                vein_toggle: base.noise.vein_toggle,
                vein_ridged: base.noise.vein_ridged,
                vein_gap: base.noise.vein_gap,
            },
            surface_estimator: ProtoSurfaceEstimator {
                full_component_stack: surface_stack,
            },
            multi_noise: ProtoMultiNoiseRouter {
                full_component_stack: multi_noise_stack,
                temperature: base.multi_noise.temperature,
                vegetation: base.multi_noise.vegetation,
                continents: base.multi_noise.continents,
                erosion: base.multi_noise.erosion,
                depth: base.multi_noise.depth,
                ridges: base.multi_noise.ridges,
            },
        }
    }
}
