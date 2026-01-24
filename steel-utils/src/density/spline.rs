//! Cubic spline implementation for terrain shaping.
//!
//! Vanilla Minecraft uses cubic splines extensively to shape terrain based on
//! noise values like continentalness, erosion, and peaks/valleys.

use super::{DensityFn, DensityFunction, FunctionContext};

/// A cubic spline that interpolates between control points.
///
/// Each control point has a location, value, and derivative. The spline
/// uses Hermite interpolation between adjacent points.
#[derive(Clone)]
pub struct CubicSpline {
    /// The density function whose output determines where on the spline to sample.
    coordinate: DensityFn,
    /// Control points sorted by location.
    points: Vec<SplinePoint>,
    /// Cached minimum value.
    min_value: f64,
    /// Cached maximum value.
    max_value: f64,
}

/// A control point in a cubic spline.
#[derive(Clone)]
pub struct SplinePoint {
    /// The input value (location on the spline).
    pub location: f32,
    /// The output value at this point (can be another spline or a constant).
    pub value: SplineValue,
    /// The derivative at this point (affects curve shape).
    pub derivative: f32,
}

/// The value at a spline point - either a constant or a nested spline.
#[derive(Clone)]
pub enum SplineValue {
    /// A constant value.
    Constant(f32),
    /// A nested spline (for multi-dimensional interpolation).
    Spline(Box<CubicSpline>),
}

impl SplineValue {
    /// Computes the value, evaluating nested splines if necessary.
    #[must_use]
    pub fn compute(&self, context: &FunctionContext) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Spline(spline) => spline.compute_inner(context),
        }
    }

    /// Returns the minimum possible value.
    #[must_use]
    pub fn min_value(&self) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Spline(spline) => spline.min_value as f32,
        }
    }

    /// Returns the maximum possible value.
    #[must_use]
    pub fn max_value(&self) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Spline(spline) => spline.max_value as f32,
        }
    }
}

impl CubicSpline {
    /// Creates a new cubic spline.
    ///
    /// # Arguments
    /// * `coordinate` - The density function that provides the input value
    /// * `points` - Control points (must be sorted by location)
    #[must_use]
    pub fn new(coordinate: DensityFn, points: Vec<SplinePoint>) -> Self {
        let (min_value, max_value) = Self::calculate_bounds(&points);
        Self {
            coordinate,
            points,
            min_value,
            max_value,
        }
    }

    /// Creates a constant spline that always returns the same value.
    #[must_use]
    pub fn constant(coordinate: DensityFn, value: f32) -> Self {
        Self::new(
            coordinate,
            vec![SplinePoint {
                location: 0.0,
                value: SplineValue::Constant(value),
                derivative: 0.0,
            }],
        )
    }

    fn calculate_bounds(points: &[SplinePoint]) -> (f64, f64) {
        if points.is_empty() {
            return (0.0, 0.0);
        }

        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for point in points {
            let v_min = f64::from(point.value.min_value());
            let v_max = f64::from(point.value.max_value());
            min = min.min(v_min);
            max = max.max(v_max);
        }

        // Account for interpolation overshoot
        // Hermite splines can overshoot, but with reasonable derivatives it's bounded
        (min, max)
    }

    /// Computes the spline value at the current position.
    fn compute_inner(&self, context: &FunctionContext) -> f32 {
        let t = self.coordinate.compute(context) as f32;
        self.sample(t, context)
    }

    /// Samples the spline at a given input value.
    fn sample(&self, t: f32, context: &FunctionContext) -> f32 {
        if self.points.is_empty() {
            return 0.0;
        }

        // Handle out-of-range cases
        if t <= self.points[0].location {
            return self.points[0].value.compute(context);
        }
        let last = self.points.len() - 1;
        if t >= self.points[last].location {
            return self.points[last].value.compute(context);
        }

        // Binary search for the correct segment
        let mut lo = 0;
        let mut hi = last;
        while lo < hi {
            let mid = usize::midpoint(lo, hi);
            if self.points[mid].location < t {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        // lo is now the first point with location >= t
        // We want to interpolate between points[lo-1] and points[lo]
        let i = if lo > 0 { lo - 1 } else { 0 };
        let p0 = &self.points[i];
        let p1 = &self.points[i + 1];

        // Hermite interpolation
        let dt = p1.location - p0.location;
        if dt <= 0.0 {
            return p0.value.compute(context);
        }

        let s = (t - p0.location) / dt; // Normalized parameter [0, 1]

        let v0 = p0.value.compute(context);
        let v1 = p1.value.compute(context);
        let d0 = p0.derivative * dt;
        let d1 = p1.derivative * dt;

        // Cubic Hermite spline formula
        let s2 = s * s;
        let s3 = s2 * s;

        let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
        let h10 = s3 - 2.0 * s2 + s;
        let h01 = -2.0 * s3 + 3.0 * s2;
        let h11 = s3 - s2;

        h00 * v0 + h10 * d0 + h01 * v1 + h11 * d1
    }
}

impl DensityFunction for CubicSpline {
    fn compute(&self, context: &FunctionContext) -> f64 {
        f64::from(self.compute_inner(context))
    }

    fn min_value(&self) -> f64 {
        self.min_value
    }

    fn max_value(&self) -> f64 {
        self.max_value
    }
}

/// Builder for creating cubic splines with a fluent API.
pub struct SplineBuilder {
    coordinate: DensityFn,
    points: Vec<SplinePoint>,
}

impl SplineBuilder {
    /// Creates a new spline builder.
    #[must_use]
    pub fn new(coordinate: DensityFn) -> Self {
        Self {
            coordinate,
            points: Vec::new(),
        }
    }

    /// Adds a constant point to the spline.
    #[must_use]
    pub fn add_point(mut self, location: f32, value: f32, derivative: f32) -> Self {
        self.points.push(SplinePoint {
            location,
            value: SplineValue::Constant(value),
            derivative,
        });
        self
    }

    /// Adds a nested spline point.
    #[must_use]
    pub fn add_spline(mut self, location: f32, spline: CubicSpline, derivative: f32) -> Self {
        self.points.push(SplinePoint {
            location,
            value: SplineValue::Spline(Box::new(spline)),
            derivative,
        });
        self
    }

    /// Builds the spline, sorting points by location.
    ///
    /// # Panics
    ///
    /// Panics if any spline point location is NaN.
    #[must_use]
    pub fn build(mut self) -> CubicSpline {
        self.points.sort_by(|a, b| {
            a.location
                .partial_cmp(&b.location)
                .expect("spline point locations must be valid floats (not NaN)")
        });
        CubicSpline::new(self.coordinate, self.points)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::density::Constant;

    #[test]
    fn test_constant_spline() {
        let coord = Arc::new(Constant::new(0.0));
        let spline = CubicSpline::constant(coord, 5.0);
        let ctx = FunctionContext::new(0, 0, 0);
        assert!((spline.compute(&ctx) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_linear_spline() {
        // Create a simple linear spline from 0 to 1
        let coord = Arc::new(Constant::new(0.5));
        let spline = SplineBuilder::new(coord)
            .add_point(0.0, 0.0, 1.0)
            .add_point(1.0, 1.0, 1.0)
            .build();

        let ctx = FunctionContext::new(0, 0, 0);
        let value = spline.compute(&ctx);
        // Should be close to 0.5 for linear interpolation
        assert!((value - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_spline_clamping() {
        let coord = Arc::new(Constant::new(-100.0));
        let spline = SplineBuilder::new(coord)
            .add_point(0.0, 0.0, 0.0)
            .add_point(1.0, 1.0, 0.0)
            .build();

        let ctx = FunctionContext::new(0, 0, 0);
        let value = spline.compute(&ctx);
        // Should clamp to first point's value
        assert!((value - 0.0).abs() < 1e-5);
    }
}
