//! Density function implementations for the Pumpkin-style noise router.
//!
//! This module contains the core density function components that are evaluated
//! during terrain generation to produce density values at each position.
//!
//! # Overview
//!
//! Density functions are composable building blocks that output a density value
//! at any 3D position. The terrain generator uses these to determine whether
//! a position should be solid (`density > 0`) or air (`density < 0`).
//!
//! # Component Categories
//!
//! ## Independent Components
//! No dependencies on other components:
//! - [`Constant`]: Fixed value everywhere
//! - [`Noise`]: Perlin noise sampling with scaling
//! - [`ShiftA`], [`ShiftB`]: Coordinate shift functions
//! - [`InterpolatedNoiseSampler`]: 3D interpolated noise
//! - [`ClampedYGradient`]: Linear gradient based on Y
//! - [`EndIsland`]: End dimension island shape
//!
//! ## Dependent Components
//! Reference earlier components in the stack:
//! - [`Linear`]: `ax + b` transformation
//! - [`Binary`]: Add, Mul, Min, Max operations
//! - [`Unary`]: Abs, Square, Cube, etc.
//! - [`Clamp`]: Value clamping
//! - [`RangeChoice`]: Conditional selection
//! - [`SplineFunction`]: Cubic spline interpolation
//!
//! # Range Tracking
//!
//! All components implement [`NoiseFunctionComponentRange`] to track
//! min/max possible outputs, enabling early-exit optimizations.

use crate::noise_router::WrapperType;
use enum_dispatch::enum_dispatch;

use super::chunk_density_function::ChunkNoiseFunctionSampleOptions;

pub mod math;
pub mod misc;
pub mod noise;
pub mod spline;

// Re-export main types
pub use math::{Binary, Clamp, Constant, Linear, Unary};
pub use misc::{ClampedYGradient, EndIsland, RangeChoice, WeirdScaled};
pub use noise::{InterpolatedNoiseSampler, Noise, ShiftA, ShiftB, ShiftedNoise};
pub use spline::{Spline, SplineFunction, SplinePoint, SplineValue};

/// A trait for positions used in noise sampling.
pub trait NoisePos {
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn z(&self) -> i32;
}

/// A trait for mapping indices to noise positions.
pub trait IndexToNoisePos {
    fn at(
        &self,
        index: usize,
        sample_options: Option<&mut ChunkNoiseFunctionSampleOptions>,
    ) -> impl NoisePos + 'static;
}

/// A trait for independent noise function components that can be sampled statically.
#[enum_dispatch]
pub trait StaticIndependentChunkNoiseFunctionComponentImpl {
    fn sample(&self, pos: &impl NoisePos) -> f64;
    fn fill(&self, array: &mut [f64], mapper: &impl IndexToNoisePos) {
        array.iter_mut().enumerate().for_each(|(index, value)| {
            let pos = mapper.at(index, None);
            *value = self.sample(&pos);
        });
    }
}

/// An unblended noise position.
pub struct UnblendedNoisePos {
    x: i32,
    y: i32,
    z: i32,
}

impl UnblendedNoisePos {
    #[must_use]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl NoisePos for UnblendedNoisePos {
    #[inline]
    fn x(&self) -> i32 {
        self.x
    }

    #[inline]
    fn y(&self) -> i32 {
        self.y
    }

    #[inline]
    fn z(&self) -> i32 {
        self.z
    }
}

/// A trait for getting the min/max range of a noise function component.
#[enum_dispatch]
pub trait NoiseFunctionComponentRange {
    fn min(&self) -> f64;
    fn max(&self) -> f64;
}

/// A wrapper density function component.
#[derive(Clone)]
pub struct Wrapper {
    pub input_index: usize,
    pub wrapper_type: WrapperType,
    min_value: f64,
    max_value: f64,
}

impl Wrapper {
    #[must_use]
    pub const fn new(
        input_index: usize,
        wrapper_type: WrapperType,
        min_value: f64,
        max_value: f64,
    ) -> Self {
        Self {
            input_index,
            wrapper_type,
            min_value,
            max_value,
        }
    }
}

impl NoiseFunctionComponentRange for Wrapper {
    #[inline]
    fn min(&self) -> f64 {
        self.min_value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.max_value
    }
}

/// A pass-through density function component that just references another component.
#[derive(Clone, Copy)]
pub struct PassThrough {
    input_index: usize,
    min_value: f64,
    max_value: f64,
}

impl PassThrough {
    #[must_use]
    pub fn new(input_index: usize, min_value: f64, max_value: f64) -> Self {
        Self {
            input_index,
            min_value,
            max_value,
        }
    }

    #[must_use]
    pub fn input_index(&self) -> usize {
        self.input_index
    }
}

impl NoiseFunctionComponentRange for PassThrough {
    #[inline]
    fn min(&self) -> f64 {
        self.min_value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.max_value
    }
}
