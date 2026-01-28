//! Density function combinators for combining multiple functions.

use super::{DensityFn, DensityFunction, FunctionContext};

/// Adds two density functions together.
pub struct Add {
    argument1: DensityFn,
    argument2: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl Add {
    /// Creates a new Add combinator.
    #[must_use]
    pub fn new(argument1: DensityFn, argument2: DensityFn) -> Self {
        let min_value = argument1.min_value() + argument2.min_value();
        let max_value = argument1.max_value() + argument2.max_value();
        Self {
            argument1,
            argument2,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for Add {
    fn compute(&self, context: &FunctionContext) -> f64 {
        self.argument1.compute(context) + self.argument2.compute(context)
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Multiplies two density functions together.
pub struct Mul {
    argument1: DensityFn,
    argument2: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl Mul {
    /// Creates a new Mul combinator.
    #[must_use]
    pub fn new(argument1: DensityFn, argument2: DensityFn) -> Self {
        // For multiplication, we need to consider all combinations of min/max
        let a_min = argument1.min_value();
        let a_max = argument1.max_value();
        let b_min = argument2.min_value();
        let b_max = argument2.max_value();

        let products = [a_min * b_min, a_min * b_max, a_max * b_min, a_max * b_max];

        let min_value = products
            .iter()
            .copied()
            .reduce(f64::min)
            .unwrap_or(f64::NEG_INFINITY);
        let max_value = products
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or(f64::INFINITY);

        Self {
            argument1,
            argument2,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for Mul {
    fn compute(&self, context: &FunctionContext) -> f64 {
        self.argument1.compute(context) * self.argument2.compute(context)
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Returns the minimum of two density functions.
pub struct Min {
    argument1: DensityFn,
    argument2: DensityFn,
    lower: f64,
    upper: f64,
}

impl Min {
    /// Creates a new Min combinator.
    #[must_use]
    pub fn new(argument1: DensityFn, argument2: DensityFn) -> Self {
        let lower = argument1.min_value().min(argument2.min_value());
        let upper = argument1.max_value().min(argument2.max_value());
        Self {
            argument1,
            argument2,
            lower,
            upper,
        }
    }
}

impl DensityFunction for Min {
    fn compute(&self, context: &FunctionContext) -> f64 {
        self.argument1
            .compute(context)
            .min(self.argument2.compute(context))
    }

    fn min_value(&self) -> f64 {
        self.lower
    }

    fn max_value(&self) -> f64 {
        self.upper
    }
}

/// Returns the maximum of two density functions.
pub struct Max {
    argument1: DensityFn,
    argument2: DensityFn,
    lower: f64,
    upper: f64,
}

impl Max {
    /// Creates a new Max combinator.
    #[must_use]
    pub fn new(argument1: DensityFn, argument2: DensityFn) -> Self {
        let lower = argument1.min_value().max(argument2.min_value());
        let upper = argument1.max_value().max(argument2.max_value());
        Self {
            argument1,
            argument2,
            lower,
            upper,
        }
    }
}

impl DensityFunction for Max {
    fn compute(&self, context: &FunctionContext) -> f64 {
        self.argument1
            .compute(context)
            .max(self.argument2.compute(context))
    }

    fn min_value(&self) -> f64 {
        self.lower
    }

    fn max_value(&self) -> f64 {
        self.upper
    }
}

/// Clamps a density function's output to a range.
pub struct Clamp {
    input: DensityFn,
    lower_bound: f64,
    upper_bound: f64,
}

impl Clamp {
    /// Creates a new Clamp function.
    #[must_use]
    pub fn new(input: DensityFn, lower_bound: f64, upper_bound: f64) -> Self {
        Self {
            input,
            lower_bound,
            upper_bound,
        }
    }
}

impl DensityFunction for Clamp {
    fn compute(&self, context: &FunctionContext) -> f64 {
        self.input
            .compute(context)
            .clamp(self.lower_bound, self.upper_bound)
    }

    fn min_value(&self) -> f64 {
        self.lower_bound.max(self.input.min_value())
    }

    fn max_value(&self) -> f64 {
        self.upper_bound.min(self.input.max_value())
    }
}

/// Selects between two functions based on whether input is in a range.
///
/// This is equivalent to vanilla's `RangeChoice`.
pub struct RangeChoice {
    input: DensityFn,
    min_inclusive: f64,
    max_exclusive: f64,
    when_in_range: DensityFn,
    when_out_of_range: DensityFn,
    min_value: f64,
    max_value: f64,
}

impl RangeChoice {
    /// Creates a new `RangeChoice` function.
    #[must_use]
    pub fn new(
        input: DensityFn,
        min_inclusive: f64,
        max_exclusive: f64,
        when_in_range: DensityFn,
        when_out_of_range: DensityFn,
    ) -> Self {
        let min_value = when_in_range.min_value().min(when_out_of_range.min_value());
        let max_value = when_in_range.max_value().max(when_out_of_range.max_value());
        Self {
            input,
            min_inclusive,
            max_exclusive,
            when_in_range,
            when_out_of_range,
            min_value,
            max_value,
        }
    }
}

impl DensityFunction for RangeChoice {
    fn compute(&self, context: &FunctionContext) -> f64 {
        let input_value = self.input.compute(context);
        if input_value >= self.min_inclusive && input_value < self.max_exclusive {
            self.when_in_range.compute(context)
        } else {
            self.when_out_of_range.compute(context)
        }
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::density::Constant;
    use std::sync::Arc;

    #[test]
    fn test_add() {
        let f = Add::new(Arc::new(Constant::new(3.0)), Arc::new(Constant::new(2.0)));
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul() {
        let f = Mul::new(Arc::new(Constant::new(3.0)), Arc::new(Constant::new(2.0)));
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_min() {
        let f = Min::new(Arc::new(Constant::new(3.0)), Arc::new(Constant::new(2.0)));
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_max() {
        let f = Max::new(Arc::new(Constant::new(3.0)), Arc::new(Constant::new(2.0)));
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_clamp() {
        let f = Clamp::new(Arc::new(Constant::new(5.0)), 0.0, 3.0);
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((f.compute(&ctx) - 3.0).abs() < 1e-10);

        let f = Clamp::new(Arc::new(Constant::new(-5.0)), 0.0, 3.0);
        assert!((f.compute(&ctx) - 0.0).abs() < 1e-10);
    }
}
