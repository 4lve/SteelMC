//! Mapped density functions that transform a single input.

use super::{DensityFn, DensityFunction, FunctionContext};

/// Returns the absolute value of the input function.
pub struct Abs {
    input: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl Abs {
    /// Creates a new Abs function.
    #[must_use]
    pub fn new(input: DensityFn) -> Self {
        let in_min = input.min_value();
        let in_max = input.max_value();

        // If range crosses zero, min becomes 0
        let min_value = if in_min <= 0.0 && in_max >= 0.0 {
            0.0
        } else {
            in_min.abs().min(in_max.abs())
        };
        let max_value = in_min.abs().max(in_max.abs());

        Self {
            input,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for Abs {
    fn compute(&self, context: &FunctionContext) -> f64 {
        self.input.compute(context).abs()
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Returns the square of the input function.
pub struct Square {
    input: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl Square {
    /// Creates a new Square function.
    #[must_use]
    pub fn new(input: DensityFn) -> Self {
        let in_min = input.min_value();
        let in_max = input.max_value();

        // If range crosses zero, min becomes 0
        let min_value = if in_min <= 0.0 && in_max >= 0.0 {
            0.0
        } else {
            (in_min * in_min).min(in_max * in_max)
        };
        let max_value = (in_min * in_min).max(in_max * in_max);

        Self {
            input,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for Square {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let v = self.input.compute(context);
        v * v
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Returns the cube of the input function.
pub struct Cube {
    input: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl Cube {
    /// Creates a new Cube function.
    #[must_use]
    pub fn new(input: DensityFn) -> Self {
        let in_min = input.min_value();
        let in_max = input.max_value();

        // Cube preserves sign, so just cube the bounds
        let min_value = (in_min * in_min * in_min).min(in_max * in_max * in_max);
        let max_value = (in_min * in_min * in_min).max(in_max * in_max * in_max);

        Self {
            input,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for Cube {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let v = self.input.compute(context);
        v * v * v
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Returns half the value if negative, otherwise returns unchanged.
///
/// This is used in vanilla for smoother terrain transitions.
pub struct HalfNegative {
    input: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl HalfNegative {
    /// Creates a new `HalfNegative` function.
    #[must_use]
    pub fn new(input: DensityFn) -> Self {
        let in_min = input.min_value();
        let in_max = input.max_value();

        // Negative values are halved
        let min_value = if in_min < 0.0 { in_min * 0.5 } else { in_min };
        let max_value = in_max; // Positive values unchanged

        Self {
            input,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for HalfNegative {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let v = self.input.compute(context);
        if v < 0.0 { v * 0.5 } else { v }
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Returns quarter the value if negative, otherwise returns unchanged.
///
/// This is used in vanilla for even smoother terrain transitions.
pub struct QuarterNegative {
    input: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl QuarterNegative {
    /// Creates a new `QuarterNegative` function.
    #[must_use]
    pub fn new(input: DensityFn) -> Self {
        let in_min = input.min_value();
        let in_max = input.max_value();

        // Negative values are quartered
        let min_value = if in_min < 0.0 { in_min * 0.25 } else { in_min };
        let max_value = in_max; // Positive values unchanged

        Self {
            input,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for QuarterNegative {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let v = self.input.compute(context);
        if v < 0.0 { v * 0.25 } else { v }
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Applies the squeeze function: clamps to [-1, 1] then applies c/2 - c³/24.
///
/// This creates a smooth S-curve that maps [-1, 1] to approximately [-0.458, 0.458].
/// Used in vanilla's postProcess to constrain terrain density values.
pub struct Squeeze {
    input: DensityFn,
}

impl Squeeze {
    /// Creates a new Squeeze function.
    #[must_use]
    pub fn new(input: DensityFn) -> Self {
        Self { input }
    }

    /// Applies the squeeze transformation: c/2 - c³/24.
    /// Matches vanilla Minecraft's DensityFunctions.Mapped.Type.SQUEEZE.
    fn squeeze(v: f64) -> f64 {
        let c = v.clamp(-1.0, 1.0);
        c / 2.0 - c * c * c / 24.0
    }
}

impl DensityFunction for Squeeze {
    fn compute(&self, context: &FunctionContext) -> f64 {
        Self::squeeze(self.input.compute(context))
    }

    fn min_value(&self) -> f64 {
        // Squeeze of -1 = -0.5 + 1/24 ≈ -0.458
        Self::squeeze(-1.0)
    }

    fn max_value(&self) -> f64 {
        // Squeeze of 1 = 0.5 - 1/24 ≈ 0.458
        Self::squeeze(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::density::Constant;
    use std::sync::Arc;

    #[test]
    fn test_abs() {
        let f = Abs::new(Arc::new(Constant::new(-5.0)));
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_square() {
        let f = Square::new(Arc::new(Constant::new(3.0)));
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 9.0).abs() < 1e-10);

        let f = Square::new(Arc::new(Constant::new(-3.0)));
        assert!((f.compute(&ctx) - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_cube() {
        let f = Cube::new(Arc::new(Constant::new(2.0)));
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 8.0).abs() < 1e-10);

        let f = Cube::new(Arc::new(Constant::new(-2.0)));
        assert!((f.compute(&ctx) - (-8.0)).abs() < 1e-10);
    }

    #[test]
    fn test_half_negative() {
        let ctx = FunctionContext::new(0, 0, 0);

        let f = HalfNegative::new(Arc::new(Constant::new(-4.0)));
        assert!((f.compute(&ctx) - (-2.0)).abs() < 1e-10);

        let f = HalfNegative::new(Arc::new(Constant::new(4.0)));
        assert!((f.compute(&ctx) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_quarter_negative() {
        let ctx = FunctionContext::new(0, 0, 0);

        let f = QuarterNegative::new(Arc::new(Constant::new(-4.0)));
        assert!((f.compute(&ctx) - (-1.0)).abs() < 1e-10);

        let f = QuarterNegative::new(Arc::new(Constant::new(4.0)));
        assert!((f.compute(&ctx) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_squeeze() {
        let ctx = FunctionContext::new(0, 0, 0);

        // At 0, squeeze returns 0
        let f = Squeeze::new(Arc::new(Constant::new(0.0)));
        assert!(f.compute(&ctx).abs() < 1e-10);

        // At 1, squeeze returns 1/2 - 1/24 ≈ 0.458
        let f = Squeeze::new(Arc::new(Constant::new(1.0)));
        let expected = 0.5 - 1.0 / 24.0;
        assert!((f.compute(&ctx) - expected).abs() < 1e-10);

        // At -1, squeeze returns -1/2 + 1/24 ≈ -0.458
        let f = Squeeze::new(Arc::new(Constant::new(-1.0)));
        let expected = -0.5 + 1.0 / 24.0;
        assert!((f.compute(&ctx) - expected).abs() < 1e-10);
    }
}
