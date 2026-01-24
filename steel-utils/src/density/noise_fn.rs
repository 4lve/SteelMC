//! Noise-based density functions.

use std::sync::Arc;

use super::{DensityFn, DensityFunction, FunctionContext};
use crate::noise::NormalNoise;

/// A density function that samples from a noise generator.
///
/// This wraps a `NormalNoise` and samples it at the current position
/// with configurable scaling.
pub struct Noise {
    sampler: Arc<NormalNoise>,
    /// Horizontal scale applied to X and Z coordinates.
    xz_scale: f64,
    /// Vertical scale applied to Y coordinate.
    y_scale: f64,
}

impl Noise {
    /// Creates a new Noise density function.
    ///
    /// # Arguments
    /// * `sampler` - The noise generator to sample from
    /// * `xz_scale` - Scale factor for X and Z coordinates (smaller = more stretched)
    /// * `y_scale` - Scale factor for Y coordinate
    #[must_use]
    pub fn new(sampler: Arc<NormalNoise>, xz_scale: f64, y_scale: f64) -> Self {
        Self {
            sampler,
            xz_scale,
            y_scale,
        }
    }
}

impl DensityFunction for Noise {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let x = f64::from(context.block_x) * self.xz_scale;
        let y = f64::from(context.block_y) * self.y_scale;
        let z = f64::from(context.block_z) * self.xz_scale;
        self.sampler.get_value(x, y, z)
    }

    fn min_value(&self) -> f64 {
        -self.sampler.max_value()
    }

    fn max_value(&self) -> f64 {
        self.sampler.max_value()
    }
}

/// A density function that samples noise with coordinate shifts.
///
/// This is equivalent to vanilla's `ShiftedNoise`.
pub struct ShiftedNoise {
    shift_x: DensityFn,
    shift_y: DensityFn,
    shift_z: DensityFn,
    noise: Arc<NormalNoise>,
    xz_scale: f64,
    y_scale: f64,
}

impl ShiftedNoise {
    /// Creates a new `ShiftedNoise` density function.
    #[must_use]
    pub fn new(
        shift_x: DensityFn,
        shift_y: DensityFn,
        shift_z: DensityFn,
        noise: Arc<NormalNoise>,
        xz_scale: f64,
        y_scale: f64,
    ) -> Self {
        Self {
            shift_x,
            shift_y,
            shift_z,
            noise,
            xz_scale,
            y_scale,
        }
    }
}

impl DensityFunction for ShiftedNoise {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let shift_x = self.shift_x.compute(context);
        let shift_y = self.shift_y.compute(context);
        let shift_z = self.shift_z.compute(context);

        let x = (f64::from(context.block_x) + shift_x) * self.xz_scale;
        let y = (f64::from(context.block_y) + shift_y) * self.y_scale;
        let z = (f64::from(context.block_z) + shift_z) * self.xz_scale;

        self.noise.get_value(x, y, z)
    }

    fn min_value(&self) -> f64 {
        -self.noise.max_value()
    }

    fn max_value(&self) -> f64 {
        self.noise.max_value()
    }
}

/// A shift function used as input to `ShiftedNoise`.
///
/// This samples a noise with a fixed scale of 0.25 (vanilla's SHIFT scale).
pub struct Shift {
    noise: Arc<NormalNoise>,
}

impl Shift {
    /// The scale used by shift functions in vanilla.
    pub const SCALE: f64 = 0.25;

    /// Creates a new Shift function.
    #[must_use]
    pub fn new(noise: Arc<NormalNoise>) -> Self {
        Self { noise }
    }
}

impl DensityFunction for Shift {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let x = f64::from(context.block_x) * Self::SCALE;
        let y = f64::from(context.block_y) * Self::SCALE;
        let z = f64::from(context.block_z) * Self::SCALE;
        // Shift functions multiply by 4.0 (vanilla)
        self.noise.get_value(x, y, z) * 4.0
    }

    fn min_value(&self) -> f64 {
        -self.noise.max_value() * 4.0
    }

    fn max_value(&self) -> f64 {
        self.noise.max_value() * 4.0
    }
}

/// `ShiftA` - shift for X coordinate (samples at y=0).
pub struct ShiftA {
    noise: Arc<NormalNoise>,
}

impl ShiftA {
    /// Creates a new `ShiftA` function.
    #[must_use]
    pub fn new(noise: Arc<NormalNoise>) -> Self {
        Self { noise }
    }
}

impl DensityFunction for ShiftA {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let x = f64::from(context.block_x) * Shift::SCALE;
        let z = f64::from(context.block_z) * Shift::SCALE;
        self.noise.get_value(x, 0.0, z) * 4.0
    }

    fn min_value(&self) -> f64 {
        -self.noise.max_value() * 4.0
    }

    fn max_value(&self) -> f64 {
        self.noise.max_value() * 4.0
    }
}

/// `ShiftB` - shift for Z coordinate (samples at different offset).
pub struct ShiftB {
    noise: Arc<NormalNoise>,
}

impl ShiftB {
    /// Creates a new `ShiftB` function.
    #[must_use]
    pub fn new(noise: Arc<NormalNoise>) -> Self {
        Self { noise }
    }
}

impl DensityFunction for ShiftB {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let x = f64::from(context.block_z) * Shift::SCALE;
        let z = f64::from(context.block_x) * Shift::SCALE;
        self.noise.get_value(x, 0.0, z) * 4.0
    }

    fn min_value(&self) -> f64 {
        -self.noise.max_value() * 4.0
    }

    fn max_value(&self) -> f64 {
        self.noise.max_value() * 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::NoiseParameters;
    use crate::random::{Random, xoroshiro::Xoroshiro};

    #[test]
    fn test_noise_function() {
        let mut rng = Xoroshiro::from_seed(12345);
        let mut positional_random_factory = rng.next_positional();
        let params = NoiseParameters {
            first_octave: -3,
            amplitudes: vec![1.0, 1.0, 1.0],
        };
        let noise = Arc::new(NormalNoise::create(
            &mut positional_random_factory,
            "minecraft:test_noise_function",
            params,
        ));
        let f = Noise::new(noise, 1.0, 1.0);

        // Just verify it returns something in range
        let ctx = FunctionContext::new(0, 0, 0);
        let value = f.compute(&ctx);
        assert!(value >= f.min_value() && value <= f.max_value());
    }
}
