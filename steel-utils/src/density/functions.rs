//! Basic density functions.

use super::{DensityFunction, FunctionContext};

/// A constant density function that always returns the same value.
#[derive(Debug, Clone, Copy)]
pub struct Constant {
    value: f64,
}

impl Constant {
    /// Creates a new constant density function.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self { value }
    }

    /// Returns the constant value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }
}

impl DensityFunction for Constant {
    fn compute(&self, _context: &FunctionContext) -> f64 {
        self.value
    }

    fn min_value(&self) -> f64 {
        self.value
    }

    fn max_value(&self) -> f64 {
        self.value
    }
}

/// A density function that returns a Y-based clamped gradient.
///
/// This is equivalent to vanilla's `DensityFunctions.yClampedGradient`.
/// Returns `from_value` when y <= `from_y`, `to_value` when y >= `to_y`,
/// and linearly interpolates between them.
#[derive(Debug, Clone, Copy)]
pub struct YClampedGradient {
    from_y: i32,
    to_y: i32,
    from_value: f64,
    to_value: f64,
}

impl YClampedGradient {
    /// Creates a new Y-clamped gradient density function.
    ///
    /// # Arguments
    /// * `from_y` - The Y coordinate where the gradient starts
    /// * `to_y` - The Y coordinate where the gradient ends
    /// * `from_value` - The value returned at or below `from_y`
    /// * `to_value` - The value returned at or above `to_y`
    #[must_use]
    pub const fn new(from_y: i32, to_y: i32, from_value: f64, to_value: f64) -> Self {
        Self {
            from_y,
            to_y,
            from_value,
            to_value,
        }
    }
}

impl DensityFunction for YClampedGradient {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let y = context.block_y;
        if y <= self.from_y {
            self.from_value
        } else if y >= self.to_y {
            self.to_value
        } else {
            // Linear interpolation
            let t = f64::from(y - self.from_y) / f64::from(self.to_y - self.from_y);
            self.from_value + t * (self.to_value - self.from_value)
        }
    }

    fn min_value(&self) -> f64 {
        self.from_value.min(self.to_value)
    }

    fn max_value(&self) -> f64 {
        self.from_value.max(self.to_value)
    }
}

/// A density function that returns the block's Y coordinate directly.
///
/// Useful as input to other functions that need the Y position.
#[derive(Debug, Clone, Copy, Default)]
pub struct BlockY;

impl DensityFunction for BlockY {
    fn compute(&self, context: &FunctionContext) -> f64 {
        f64::from(context.block_y)
    }

    fn min_value(&self) -> f64 {
        f64::from(i32::MIN)
    }

    fn max_value(&self) -> f64 {
        f64::from(i32::MAX)
    }
}

/// Zero constant for convenience.
pub const ZERO: Constant = Constant::new(0.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant() {
        let f = Constant::new(42.0);
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 42.0).abs() < 1e-10);
        assert!((f.min_value() - 42.0).abs() < 1e-10);
        assert!((f.max_value() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_y_clamped_gradient() {
        // Vanilla overworld gradient: -64 to 320, values 1.5 to -1.5
        let f = YClampedGradient::new(-64, 320, 1.5, -1.5);

        // At minimum Y should return from_value
        let ctx = FunctionContext::new(0, -64, 0);
        assert!((f.compute(&ctx) - 1.5).abs() < 1e-10);

        // At maximum Y should return to_value
        let ctx = FunctionContext::new(0, 320, 0);
        assert!((f.compute(&ctx) - (-1.5)).abs() < 1e-10);

        // At midpoint (Y=128) should return 0
        let ctx = FunctionContext::new(0, 128, 0);
        assert!(f.compute(&ctx).abs() < 1e-10);

        // Below minimum should clamp to from_value
        let ctx = FunctionContext::new(0, -100, 0);
        assert!((f.compute(&ctx) - 1.5).abs() < 1e-10);

        // Above maximum should clamp to to_value
        let ctx = FunctionContext::new(0, 400, 0);
        assert!((f.compute(&ctx) - (-1.5)).abs() < 1e-10);

        // Check min/max values
        assert!((f.min_value() - (-1.5)).abs() < 1e-10);
        assert!((f.max_value() - 1.5).abs() < 1e-10);
    }
}
