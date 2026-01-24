//! Density function system for vanilla-accurate terrain generation.
//!
//! This module implements Minecraft's density function system, which computes
//! terrain density at any 3D position. The system uses composable functions
//! that can be combined to create complex terrain shapes.
//!
//! Density values:
//! - Positive values = solid (stone, etc.)
//! - Negative values = air
//! - Zero = surface boundary

mod combinators;
mod functions;
mod mapped;
mod noise_fn;
/// Vanilla noise parameter definitions.
pub mod noises;
mod router;
mod spline;
/// Full vanilla terrain shaper splines.
pub mod terrain_shaper;

pub use combinators::*;
pub use functions::*;
pub use mapped::*;
pub use noise_fn::*;
pub use router::*;
pub use spline::*;

use std::sync::Arc;

/// Context providing position information for density function evaluation.
#[derive(Debug, Clone, Copy)]
pub struct FunctionContext {
    /// X coordinate in block space.
    pub block_x: i32,
    /// Y coordinate in block space.
    pub block_y: i32,
    /// Z coordinate in block space.
    pub block_z: i32,
}

impl FunctionContext {
    /// Creates a new function context for the given block position.
    #[must_use]
    pub const fn new(block_x: i32, block_y: i32, block_z: i32) -> Self {
        Self {
            block_x,
            block_y,
            block_z,
        }
    }
}

/// A density function that can compute terrain density at any position.
///
/// This trait mirrors Minecraft's `DensityFunction` interface, providing
/// a composable system for terrain generation.
pub trait DensityFunction: Send + Sync {
    /// Computes the density at the given position.
    ///
    /// Returns a density value where:
    /// - Positive = solid terrain
    /// - Negative = air
    /// - Zero = surface boundary
    fn compute(&self, context: &FunctionContext) -> f64;

    /// Returns the minimum possible value this function can return.
    ///
    /// Used for optimization and value clamping.
    fn min_value(&self) -> f64;

    /// Returns the maximum possible value this function can return.
    ///
    /// Used for optimization and value clamping.
    fn max_value(&self) -> f64;
}

/// A boxed density function for dynamic dispatch.
pub type DensityFn = Arc<dyn DensityFunction>;

/// Extension trait for creating density function combinators.
pub trait DensityFunctionExt: DensityFunction + Sized {
    /// Adds this function to another, returning a new function.
    fn add(self, other: impl DensityFunction + 'static) -> Add
    where
        Self: 'static,
    {
        Add::new(Arc::new(self), Arc::new(other))
    }

    /// Multiplies this function by another, returning a new function.
    fn mul(self, other: impl DensityFunction + 'static) -> Mul
    where
        Self: 'static,
    {
        Mul::new(Arc::new(self), Arc::new(other))
    }

    /// Returns the minimum of this function and another.
    fn min(self, other: impl DensityFunction + 'static) -> Min
    where
        Self: 'static,
    {
        Min::new(Arc::new(self), Arc::new(other))
    }

    /// Returns the maximum of this function and another.
    fn max(self, other: impl DensityFunction + 'static) -> Max
    where
        Self: 'static,
    {
        Max::new(Arc::new(self), Arc::new(other))
    }

    /// Clamps this function's output to the given range.
    fn clamp(self, min: f64, max: f64) -> Clamp
    where
        Self: 'static,
    {
        Clamp::new(Arc::new(self), min, max)
    }

    /// Returns the absolute value of this function.
    fn abs(self) -> Abs
    where
        Self: 'static,
    {
        Abs::new(Arc::new(self))
    }

    /// Returns the square of this function.
    fn square(self) -> Square
    where
        Self: 'static,
    {
        Square::new(Arc::new(self))
    }

    /// Returns the cube of this function.
    fn cube(self) -> Cube
    where
        Self: 'static,
    {
        Cube::new(Arc::new(self))
    }

    /// Returns half the value if negative, otherwise returns the value unchanged.
    fn half_negative(self) -> HalfNegative
    where
        Self: 'static,
    {
        HalfNegative::new(Arc::new(self))
    }

    /// Returns quarter the value if negative, otherwise returns the value unchanged.
    fn quarter_negative(self) -> QuarterNegative
    where
        Self: 'static,
    {
        QuarterNegative::new(Arc::new(self))
    }

    /// Applies the squeeze function: clamps to [-1, 1] then applies x - x^3/24.
    fn squeeze(self) -> Squeeze
    where
        Self: 'static,
    {
        Squeeze::new(Arc::new(self))
    }
}

// Implement the extension trait for all density functions
impl<T: DensityFunction> DensityFunctionExt for T {}
