//! `NoiseRouter` - container for all terrain-related density functions.

use std::sync::Arc;

use super::{
    Add, Constant, DensityFn, DensityFunction, FunctionContext, HalfNegative, Mul, Noise,
    QuarterNegative, ShiftA, ShiftB, ShiftedNoise, Squeeze, YClampedGradient, noises,
    terrain_shaper,
};
use crate::noise::{BlendedNoise, NormalNoise};
use crate::random::{PositionalRandom, Random, RandomSource, RandomSplitter, xoroshiro::Xoroshiro};

/// The `NoiseRouter` contains all density functions needed for terrain generation.
///
/// This mirrors vanilla Minecraft's `NoiseRouter` record.
pub struct NoiseRouter {
    /// Barrier noise for aquifers.
    pub barrier_noise: DensityFn,
    /// Fluid level floodedness for aquifers.
    pub fluid_level_floodedness_noise: DensityFn,
    /// Fluid level spread for aquifers.
    pub fluid_level_spread_noise: DensityFn,
    /// Lava noise for aquifers.
    pub lava_noise: DensityFn,
    /// Temperature for biomes.
    pub temperature: DensityFn,
    /// Vegetation (humidity) for biomes.
    pub vegetation: DensityFn,
    /// Continentalness - controls continent/ocean distribution.
    pub continents: DensityFn,
    /// Erosion - controls terrain erosion.
    pub erosion: DensityFn,
    /// Depth from surface.
    pub depth: DensityFn,
    /// Ridges (weirdness) - controls peaks and valleys.
    pub ridges: DensityFn,
    /// Final terrain density - positive = solid, negative = air.
    pub final_density: DensityFn,
    /// Ore vein toggle.
    pub vein_toggle: DensityFn,
    /// Ore vein ridged.
    pub vein_ridged: DensityFn,
    /// Ore vein gap.
    pub vein_gap: DensityFn,
}

/// Global offset applied to terrain height.
pub const GLOBAL_OFFSET: f64 = -0.503_75;

/// Sea level Y coordinate.
pub const SEA_LEVEL: i32 = 63;

/// Minimum Y for terrain generation.
pub const MIN_Y: i32 = -64;

/// Maximum Y for terrain generation.
pub const MAX_Y: i32 = 320;

impl NoiseRouter {
    /// Creates a new `NoiseRouter` for the overworld with the given seed.
    #[allow(clippy::too_many_lines)] // Router initialization inherently requires many noise sources
    #[must_use]
    pub fn overworld(seed: u64) -> Self {
        let mut rng = Xoroshiro::from_seed(seed);
        let mut positional_random_factory = rng.next_positional();

        // Create noise samplers
        let shift_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:shift",
            noises::shift(),
        ));
        let continentalness_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:continentalness",
            noises::continentalness(),
        ));
        let erosion_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:erosion",
            noises::erosion(),
        ));
        let ridge_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:ridge",
            noises::ridge(),
        ));
        let jagged_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:jagged",
            noises::jagged(),
        ));
        let temperature_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:temperature",
            noises::temperature(),
        ));
        let vegetation_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:vegetation",
            noises::vegetation(),
        ));

        // Aquifer noises
        let barrier_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:aquifer_barrier",
            noises::aquifer_barrier(),
        ));
        let floodedness_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:aquifer_floodedness",
            noises::aquifer_fluid_level_floodedness(),
        ));
        let spread_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:aquifer_spread",
            noises::aquifer_fluid_level_spread(),
        ));
        let lava_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:aquifer_lava",
            noises::aquifer_lava(),
        ));

        // Ore noises
        let veininess_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:ore_veininess",
            noises::ore_veininess(),
        ));
        let vein_a_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:ore_vein_a",
            noises::ore_vein_a(),
        ));
        let _vein_b_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:ore_vein_b",
            noises::ore_vein_b(),
        ));
        let vein_gap_noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:ore_gap",
            noises::ore_gap(),
        ));

        // Create shift functions
        let shift_x: DensityFn = Arc::new(ShiftA::new(shift_noise.clone()));
        let shift_z: DensityFn = Arc::new(ShiftB::new(shift_noise));

        // Create 2D shifted noises for continents, erosion, ridges
        let continents: DensityFn = Arc::new(ShiftedNoise::new(
            shift_x.clone(),
            Arc::new(Constant::new(0.0)),
            shift_z.clone(),
            continentalness_noise,
            0.25,
            0.0, // 2D noise, no Y scale
        ));

        let erosion: DensityFn = Arc::new(ShiftedNoise::new(
            shift_x.clone(),
            Arc::new(Constant::new(0.0)),
            shift_z.clone(),
            erosion_noise,
            0.25,
            0.0,
        ));

        let ridges: DensityFn = Arc::new(ShiftedNoise::new(
            shift_x.clone(),
            Arc::new(Constant::new(0.0)),
            shift_z.clone(),
            ridge_noise,
            0.25,
            0.0,
        ));

        // Temperature and vegetation (for biomes)
        let temperature: DensityFn = Arc::new(ShiftedNoise::new(
            shift_x.clone(),
            Arc::new(Constant::new(0.0)),
            shift_z.clone(),
            temperature_noise,
            0.25,
            0.0,
        ));

        let vegetation: DensityFn = Arc::new(ShiftedNoise::new(
            shift_x,
            Arc::new(Constant::new(0.0)),
            shift_z,
            vegetation_noise,
            0.25,
            0.0,
        ));

        // Create peaks_and_valleys from ridges
        // peaksAndValleys(x) = -(||x| - 2/3| - 1/3) * 3
        let ridges_folded: DensityFn = Arc::new(PeaksAndValleys::new(ridges.clone()));

        // Create full vanilla offset spline
        // offset = -0.503_75 + spline(overworldOffset(...))
        let offset_spline = terrain_shaper::overworld_offset(
            continents.clone(),
            erosion.clone(),
            ridges_folded.clone(),
            false, // not amplified
        );
        let offset: DensityFn = Arc::new(Add::new(
            Arc::new(Constant::new(GLOBAL_OFFSET)),
            Arc::new(offset_spline),
        ));

        // Create full vanilla factor spline
        let factor: DensityFn = Arc::new(terrain_shaper::overworld_factor(
            continents.clone(),
            erosion.clone(),
            ridges.clone(),
            ridges_folded.clone(),
            false, // not amplified
        ));

        // Create full vanilla jaggedness spline
        let jaggedness: DensityFn = Arc::new(terrain_shaper::overworld_jaggedness(
            continents.clone(),
            erosion.clone(),
            ridges.clone(),
            ridges_folded.clone(),
            false, // not amplified
        ));

        // depth = yClampedGradient(-64, 320, 1.5, -1.5) + offset
        let y_gradient: DensityFn = Arc::new(YClampedGradient::new(MIN_Y, MAX_Y, 1.5, -1.5));
        let depth: DensityFn = Arc::new(Add::new(y_gradient, offset));

        // jagged = jaggedness * jaggedNoise.halfNegative()
        let jagged_sampler: DensityFn = Arc::new(Noise::new(jagged_noise, 1500.0, 0.0));
        let jagged_half_negative: DensityFn = Arc::new(HalfNegative::new(jagged_sampler));
        let jagged: DensityFn = Arc::new(Mul::new(jaggedness, jagged_half_negative));

        // noiseGradientDensity(factor, depth + jagged)
        // = 4.0 * ((depth + jagged) * factor).quarterNegative()
        let depth_plus_jagged: DensityFn = Arc::new(Add::new(depth.clone(), jagged));
        let gradient_density = noise_gradient_density(factor, depth_plus_jagged);

        // Create base 3D noise (BlendedNoise)
        let base_3d_noise =
            create_base_3d_noise(&mut positional_random_factory, "minecraft:terrain");

        // final_density = gradient_density + base_3d_noise
        // Then apply slides for smooth transitions at top/bottom
        let sloped_cheese: DensityFn = Arc::new(Add::new(gradient_density, base_3d_noise));

        // Apply overworld slide
        let final_density = apply_overworld_slide(sloped_cheese);

        // Aquifer density functions (simplified)
        let barrier: DensityFn = Arc::new(Noise::new(barrier_noise, 1.0, 1.0));
        let floodedness: DensityFn = Arc::new(Noise::new(floodedness_noise, 1.0, 1.0));
        let spread: DensityFn = Arc::new(Noise::new(spread_noise, 1.0, 1.0));
        let lava: DensityFn = Arc::new(Noise::new(lava_noise, 1.0, 1.0));

        // Ore vein density functions (simplified)
        let vein_toggle: DensityFn = Arc::new(Noise::new(veininess_noise, 1.0, 1.0));
        let vein_ridged: DensityFn = Arc::new(Noise::new(vein_a_noise, 1.0, 1.0));
        let vein_gap: DensityFn = Arc::new(Noise::new(vein_gap_noise, 1.0, 1.0));

        Self {
            barrier_noise: barrier,
            fluid_level_floodedness_noise: floodedness,
            fluid_level_spread_noise: spread,
            lava_noise: lava,
            temperature,
            vegetation,
            continents,
            erosion,
            depth,
            ridges,
            final_density,
            vein_toggle,
            vein_ridged,
            vein_gap,
        }
    }

    /// Creates a `NoiseRouter` where all functions return zero.
    #[must_use]
    pub fn none() -> Self {
        let zero: DensityFn = Arc::new(Constant::new(0.0));
        Self {
            barrier_noise: zero.clone(),
            fluid_level_floodedness_noise: zero.clone(),
            fluid_level_spread_noise: zero.clone(),
            lava_noise: zero.clone(),
            temperature: zero.clone(),
            vegetation: zero.clone(),
            continents: zero.clone(),
            erosion: zero.clone(),
            depth: zero.clone(),
            ridges: zero.clone(),
            final_density: zero.clone(),
            vein_toggle: zero.clone(),
            vein_ridged: zero.clone(),
            vein_gap: zero,
        }
    }
}

/// Computes peaks and valleys from ridges.
/// Formula: -(||x| - 2/3| - 1/3) * 3
pub struct PeaksAndValleys {
    input: DensityFn,
}

impl PeaksAndValleys {
    /// Creates a new peaks and valleys function.
    #[must_use]
    pub fn new(input: DensityFn) -> Self {
        Self { input }
    }

    /// Computes the peaks and valleys value from weirdness.
    /// Formula: -(||weirdness| - 2/3| - 1/3) * 3
    /// Returns values in range [-1, 1]
    /// - At weirdness = 0: returns -1 (valleys)
    /// - At weirdness = ±2/3: returns 1 (peaks)
    /// - At weirdness = ±1: returns 0
    #[must_use]
    pub fn compute_value(weirdness: f64) -> f64 {
        let a = weirdness.abs();
        let b = (a - 2.0 / 3.0).abs();
        -(b - 1.0 / 3.0) * 3.0
    }
}

impl DensityFunction for PeaksAndValleys {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let v = self.input.compute(context);
        Self::compute_value(v)
    }

    fn min_value(&self) -> f64 {
        -1.0
    }

    fn max_value(&self) -> f64 {
        1.0
    }
}

/// Computes noise gradient density.
/// Formula: `4.0 * (depth * factor).quarter_negative()`
fn noise_gradient_density(factor: DensityFn, depth: DensityFn) -> DensityFn {
    let mul: DensityFn = Arc::new(Mul::new(depth, factor));
    let quarter_neg: DensityFn = Arc::new(QuarterNegative::new(mul));
    Arc::new(Mul::new(Arc::new(Constant::new(4.0)), quarter_neg))
}

/// Creates the base 3D noise (`BlendedNoise`).
fn create_base_3d_noise(random_splitter: &mut RandomSplitter, name: &str) -> DensityFn {
    Arc::new(BlendedNoiseDensity::new(random_splitter, name))
}

/// Wrapper for `BlendedNoise` as a density function.
struct BlendedNoiseDensity {
    blended: BlendedNoise,
}

impl BlendedNoiseDensity {
    fn new(random_splitter: &mut RandomSplitter, name: &str) -> Self {
        // Get a RandomSource from the splitter for the blended noise
        let mut random_source: RandomSource =
            random_splitter.with_hash_of(&format!("{name}/blended_noise"));
        let blended = BlendedNoise::new(
            &mut random_source,
            0.25,  // xz_scale
            0.125, // y_scale
            80.0,  // xz_factor
            160.0, // y_factor
            8.0,   // smear_scale_multiplier
        );
        Self { blended }
    }
}

impl DensityFunction for BlendedNoiseDensity {
    fn compute(&self, context: &FunctionContext) -> f64 {
        self.blended
            .compute(context.block_x, context.block_y, context.block_z)
    }

    fn min_value(&self) -> f64 {
        -1.0
    }

    fn max_value(&self) -> f64 {
        1.0
    }
}

/// Applies the overworld slide to smooth terrain at top and bottom,
/// then applies postProcess (multiply by 0.64 and squeeze).
fn apply_overworld_slide(input: DensityFn) -> DensityFn {
    // Apply slides at Y boundaries to create smooth transitions
    let slide: DensityFn = Arc::new(OverworldSlide::new(input));

    // Apply postProcess: multiply by 0.64, then squeeze
    // Note: vanilla also applies blendDensity and interpolated, which we skip for now
    let scaled: DensityFn = Arc::new(Mul::new(slide, Arc::new(Constant::new(0.64))));
    Arc::new(Squeeze::new(scaled))
}

/// Applies slide transformations for smooth terrain boundaries.
///
/// Matches vanilla Minecraft's slide function:
/// - Top slide: Y 240-256, lerp towards `-0.078_125` (air at world top)
/// - Bottom slide: Y -64 to -40, lerp towards `0.117_187_5` (solid at world bottom)
struct OverworldSlide {
    input: DensityFn,
}

impl OverworldSlide {
    fn new(input: DensityFn) -> Self {
        Self { input }
    }
}

impl DensityFunction for OverworldSlide {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let y = context.block_y;

        // Early exit optimization: at extreme Y levels, the slide function
        // completely overrides the input density, so we can skip computing it.
        // This is the Lithium-style Y-level skip optimization.

        // At Y >= 256: topFactor = 0.0, bottomFactor = 1.0
        // result = lerp(1.0, 0.117_187_5, lerp(0.0, -0.078_125, density))
        //        = lerp(1.0, 0.117_187_5, -0.078_125) = -0.078_125
        if y >= 256 {
            return -0.078_125;
        }

        // At Y <= -64: topFactor = 1.0, bottomFactor = 0.0
        // result = lerp(0.0, 0.117_187_5, after_top) = 0.117_187_5
        if y <= -64 {
            return 0.117_187_5;
        }

        let density = self.input.compute(context);

        // Top slide: Y 240-256
        // topFactor = yClampedGradient(240, 256, 1.0, 0.0)
        // result = lerp(topFactor, topTarget, density)
        // lerp(t, a, b) = a*(1-t) + b*t
        // At Y≤240: factor=1.0, result = density
        // At Y≥256: factor=0.0, result = topTarget (-0.078_125)
        let top_factor = y_clamped_gradient(y, 240, 256, 1.0, 0.0);
        let after_top = lerp(top_factor, -0.078_125, density);

        // Bottom slide: Y -64 to -40
        // bottomFactor = yClampedGradient(-64, -40, 0.0, 1.0)
        // result = lerp(bottomFactor, bottomTarget, afterTop)
        // At Y≤-64: factor=0.0, result = bottomTarget (0.117_187_5)
        // At Y≥-40: factor=1.0, result = afterTop
        let bottom_factor = y_clamped_gradient(y, -64, -40, 0.0, 1.0);
        lerp(bottom_factor, 0.117_187_5, after_top)
    }

    fn min_value(&self) -> f64 {
        self.input.min_value().min(-0.078_125).min(0.117_187_5)
    }

    fn max_value(&self) -> f64 {
        self.input.max_value().max(-0.078_125).max(0.117_187_5)
    }
}

/// Computes a Y-clamped gradient value.
/// Returns `from_value` when y <= `from_y`, `to_value` when y >= `to_y`,
/// and linearly interpolates between them.
#[inline]
fn y_clamped_gradient(y: i32, from_y: i32, to_y: i32, from_value: f64, to_value: f64) -> f64 {
    if y <= from_y {
        from_value
    } else if y >= to_y {
        to_value
    } else {
        let t = f64::from(y - from_y) / f64::from(to_y - from_y);
        from_value + t * (to_value - from_value)
    }
}

/// Standard linear interpolation: lerp(t, a, b) = a*(1-t) + b*t
#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a * (1.0 - t) + b * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peaks_and_valleys() {
        // At weirdness = 0, peaks_and_valleys should return -1 (valleys)
        assert!((PeaksAndValleys::compute_value(0.0) - (-1.0)).abs() < 0.01);

        // At weirdness = ±2/3, peaks_and_valleys should return 1 (peaks)
        assert!((PeaksAndValleys::compute_value(2.0 / 3.0) - 1.0).abs() < 0.01);
        assert!((PeaksAndValleys::compute_value(-2.0 / 3.0) - 1.0).abs() < 0.01);

        // At weirdness = ±1, peaks_and_valleys should return 0
        assert!(PeaksAndValleys::compute_value(1.0).abs() < 0.01);
        assert!(PeaksAndValleys::compute_value(-1.0).abs() < 0.01);
    }

    #[test]
    fn test_noise_router_creation() {
        let router = NoiseRouter::overworld(12345);
        let ctx = FunctionContext::new(0, 64, 0);

        // Just verify we can compute without panicking
        let _density = router.final_density.compute(&ctx);
    }
}
