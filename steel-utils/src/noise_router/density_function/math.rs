//! Math-based density function components.
//!
//! This module contains density functions for mathematical operations like
//! constants, linear transformations, binary operations, unary operations, and clamping.

use crate::noise_router::{BinaryData, BinaryOperation, ClampData, LinearData, UnaryData};

use super::{
    IndexToNoisePos, NoiseFunctionComponentRange, NoisePos,
    StaticIndependentChunkNoiseFunctionComponentImpl,
};
use crate::noise_router::chunk_density_function::ChunkNoiseFunctionSampleOptions;
use crate::noise_router::chunk_noise_router::{
    ChunkNoiseFunctionComponent, StaticChunkNoiseFunctionComponentImpl,
};

/// A constant density function that always returns the same value.
///
/// This is the simplest density function - it returns the same value
/// regardless of position. Used for baseline terrain values.
///
/// # Example
///
/// ```text
/// sample(x, y, z) = value
/// ```
#[derive(Clone)]
pub struct Constant {
    /// The constant value returned at all positions.
    value: f64,
}

impl Constant {
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    /// Get the constant value directly.
    #[inline]
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl NoiseFunctionComponentRange for Constant {
    #[inline]
    fn min(&self) -> f64 {
        self.value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.value
    }
}

impl StaticIndependentChunkNoiseFunctionComponentImpl for Constant {
    fn sample(&self, _pos: &impl NoisePos) -> f64 {
        self.value
    }

    fn fill(&self, array: &mut [f64], _mapper: &impl IndexToNoisePos) {
        array.fill(self.value);
    }
}

/// A linear transformation density function (add or multiply by a constant).
///
/// Applies a simple linear transformation to an input density:
/// - `Add`: `output = input + argument`
/// - `Mul`: `output = input * argument`
///
/// This is more efficient than `Binary` when one operand is constant.
#[derive(Clone)]
pub struct Linear {
    /// Index of the input component in the stack.
    pub input_index: usize,
    /// Minimum possible output value.
    min_value: f64,
    /// Maximum possible output value.
    max_value: f64,
    /// Operation data (Add or Mul with argument value).
    data: &'static LinearData,
}

impl NoiseFunctionComponentRange for Linear {
    #[inline]
    fn min(&self) -> f64 {
        self.min_value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.max_value
    }
}

impl StaticChunkNoiseFunctionComponentImpl for Linear {
    fn sample(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> f64 {
        let input_density = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut component_stack[..=self.input_index],
            pos,
            sample_options,
        );
        self.data.apply_density(input_density)
    }

    fn fill(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        array: &mut [f64],
        mapper: &impl IndexToNoisePos,
        sample_options: &mut ChunkNoiseFunctionSampleOptions,
    ) {
        ChunkNoiseFunctionComponent::fill_from_stack(
            &mut component_stack[..=self.input_index],
            array,
            mapper,
            sample_options,
        );
        for value in array {
            *value = self.data.apply_density(*value);
        }
    }
}

impl Linear {
    #[must_use]
    pub fn new(
        input_index: usize,
        min_value: f64,
        max_value: f64,
        data: &'static LinearData,
    ) -> Self {
        Self {
            input_index,
            min_value,
            max_value,
            data,
        }
    }

    /// Apply the linear transformation to an input density.
    #[inline]
    #[must_use]
    pub fn apply(&self, input_density: f64) -> f64 {
        self.data.apply_density(input_density)
    }

    /// Get the data for this linear operation.
    #[inline]
    #[must_use]
    pub fn data(&self) -> &'static LinearData {
        self.data
    }
}

/// A binary operation density function combining two inputs.
///
/// Applies one of four operations:
/// - `Add`: `input1 + input2`
/// - `Mul`: `input1 * input2` (short-circuits if input1 == 0)
/// - `Min`: `min(input1, input2)` (short-circuits if input1 < `input2.min()`)
/// - `Max`: `max(input1, input2)` (short-circuits if input1 > `input2.max()`)
///
/// # Optimizations
///
/// The `sample` method uses early-exit optimizations based on min/max bounds
/// to avoid sampling the second input when the result is already determined.
#[derive(Clone)]
pub struct Binary {
    /// Index of the first input component.
    pub input1_index: usize,
    /// Index of the second input component.
    pub input2_index: usize,
    /// Minimum possible output value.
    min_value: f64,
    /// Maximum possible output value.
    max_value: f64,
    /// Operation data.
    data: &'static BinaryData,
}

impl NoiseFunctionComponentRange for Binary {
    #[inline]
    fn min(&self) -> f64 {
        self.min_value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.max_value
    }
}

impl StaticChunkNoiseFunctionComponentImpl for Binary {
    fn sample(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> f64 {
        let input1_density = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut component_stack[..=self.input1_index],
            pos,
            sample_options,
        );

        match self.data.operation {
            BinaryOperation::Add => {
                let input2_density = ChunkNoiseFunctionComponent::sample_from_stack(
                    &mut component_stack[..=self.input2_index],
                    pos,
                    sample_options,
                );
                input1_density + input2_density
            }
            BinaryOperation::Mul => {
                if input1_density == 0.0 {
                    0.0
                } else {
                    let input2_density = ChunkNoiseFunctionComponent::sample_from_stack(
                        &mut component_stack[..=self.input2_index],
                        pos,
                        sample_options,
                    );
                    input1_density * input2_density
                }
            }
            BinaryOperation::Min => {
                let input2_min = component_stack[self.input2_index].min();

                if input1_density < input2_min {
                    input1_density
                } else {
                    let input2_density = ChunkNoiseFunctionComponent::sample_from_stack(
                        &mut component_stack[..=self.input2_index],
                        pos,
                        sample_options,
                    );

                    input1_density.min(input2_density)
                }
            }
            BinaryOperation::Max => {
                let input2_max = component_stack[self.input2_index].max();

                if input1_density > input2_max {
                    input1_density
                } else {
                    let input2_density = ChunkNoiseFunctionComponent::sample_from_stack(
                        &mut component_stack[..=self.input2_index],
                        pos,
                        sample_options,
                    );

                    input1_density.max(input2_density)
                }
            }
        }
    }

    fn fill(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        array: &mut [f64],
        mapper: &impl IndexToNoisePos,
        sample_options: &mut ChunkNoiseFunctionSampleOptions,
    ) {
        ChunkNoiseFunctionComponent::fill_from_stack(
            &mut component_stack[..=self.input1_index],
            array,
            mapper,
            sample_options,
        );

        match self.data.operation {
            BinaryOperation::Add => {
                // Batch fill argument2 into a temporary array, then add element-wise.
                // This matches vanilla's Ap2.fillArray for ADD, which is critical for
                // CacheOnce instances in argument2's subtree to get their batch cache
                // populated during cell cache filling passes.
                let mut array2 = vec![0.0; array.len()];
                ChunkNoiseFunctionComponent::fill_from_stack(
                    &mut component_stack[..=self.input2_index],
                    &mut array2,
                    mapper,
                    sample_options,
                );
                for (v1, v2) in array.iter_mut().zip(array2.iter()) {
                    *v1 += *v2;
                }
            }
            BinaryOperation::Mul => {
                array.iter_mut().enumerate().for_each(|(index, value)| {
                    if *value != 0.0 {
                        let pos = mapper.at(index, Some(sample_options));
                        let density2 = ChunkNoiseFunctionComponent::sample_from_stack(
                            &mut component_stack[..=self.input2_index],
                            &pos,
                            sample_options,
                        );
                        *value *= density2;
                    }
                });
            }
            BinaryOperation::Min => {
                let input2_min = component_stack[self.input2_index].min();
                array.iter_mut().enumerate().for_each(|(index, value)| {
                    if *value > input2_min {
                        let pos = mapper.at(index, Some(sample_options));
                        let density2 = ChunkNoiseFunctionComponent::sample_from_stack(
                            &mut component_stack[..=self.input2_index],
                            &pos,
                            sample_options,
                        );
                        *value = value.min(density2);
                    }
                });
            }
            BinaryOperation::Max => {
                let input2_max = component_stack[self.input2_index].max();
                array.iter_mut().enumerate().for_each(|(index, value)| {
                    if *value < input2_max {
                        let pos = mapper.at(index, Some(sample_options));
                        let density2 = ChunkNoiseFunctionComponent::sample_from_stack(
                            &mut component_stack[..=self.input2_index],
                            &pos,
                            sample_options,
                        );
                        *value = value.max(density2);
                    }
                });
            }
        }
    }
}

impl Binary {
    #[must_use]
    pub fn new(
        input1_index: usize,
        input2_index: usize,
        min_value: f64,
        max_value: f64,
        data: &'static BinaryData,
    ) -> Self {
        Self {
            input1_index,
            input2_index,
            min_value,
            max_value,
            data,
        }
    }

    /// Apply the binary operation to two input densities.
    #[inline]
    #[must_use]
    pub fn apply(&self, input1: f64, input2: f64) -> f64 {
        match self.data.operation {
            BinaryOperation::Add => input1 + input2,
            BinaryOperation::Mul => input1 * input2,
            BinaryOperation::Min => input1.min(input2),
            BinaryOperation::Max => input1.max(input2),
        }
    }

    /// Get the operation type.
    #[inline]
    #[must_use]
    pub fn operation(&self) -> BinaryOperation {
        self.data.operation
    }

    /// Get the data for this binary operation.
    #[inline]
    #[must_use]
    pub fn data(&self) -> &'static BinaryData {
        self.data
    }
}

/// A unary operation density function that transforms a single input.
///
/// Available operations:
/// - `Abs`: `|input|`
/// - `Square`: `input²`
/// - `Cube`: `input³`
/// - `HalfNegative`: `input < 0 ? input * 0.5 : input`
/// - `QuarterNegative`: `input < 0 ? input * 0.25 : input`
/// - `Squeeze`: `clamp(input, -1, 1)`
#[derive(Clone)]
pub struct Unary {
    /// Index of the input component in the stack.
    pub input_index: usize,
    /// Minimum possible output value.
    min_value: f64,
    /// Maximum possible output value.
    max_value: f64,
    /// Operation data.
    data: &'static UnaryData,
}

impl NoiseFunctionComponentRange for Unary {
    #[inline]
    fn min(&self) -> f64 {
        self.min_value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.max_value
    }
}

impl StaticChunkNoiseFunctionComponentImpl for Unary {
    fn sample(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> f64 {
        let input_density = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut component_stack[..=self.input_index],
            pos,
            sample_options,
        );
        self.data.apply_density(input_density)
    }

    fn fill(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        array: &mut [f64],
        mapper: &impl IndexToNoisePos,
        sample_options: &mut ChunkNoiseFunctionSampleOptions,
    ) {
        ChunkNoiseFunctionComponent::fill_from_stack(
            &mut component_stack[..=self.input_index],
            array,
            mapper,
            sample_options,
        );
        for value in array {
            *value = self.data.apply_density(*value);
        }
    }
}

impl Unary {
    #[must_use]
    pub fn new(
        input_index: usize,
        min_value: f64,
        max_value: f64,
        data: &'static UnaryData,
    ) -> Self {
        Self {
            input_index,
            min_value,
            max_value,
            data,
        }
    }

    /// Apply the unary operation to an input density.
    #[inline]
    #[must_use]
    pub fn apply(&self, input_density: f64) -> f64 {
        self.data.apply_density(input_density)
    }

    /// Get the data for this unary operation.
    #[inline]
    #[must_use]
    pub fn data(&self) -> &'static UnaryData {
        self.data
    }
}

/// A clamp density function that restricts values to a range.
///
/// Clamps the input to the range `[min_value, max_value]`:
/// ```text
/// sample(pos) = clamp(input.sample(pos), min_value, max_value)
/// ```
#[derive(Clone)]
pub struct Clamp {
    /// Index of the input component in the stack.
    pub input_index: usize,
    /// Clamp bounds.
    data: &'static ClampData,
}

impl Clamp {
    #[must_use]
    pub fn new(input_index: usize, data: &'static ClampData) -> Self {
        Self { input_index, data }
    }

    /// Apply the clamp operation to an input density.
    #[inline]
    #[must_use]
    pub fn apply(&self, input_density: f64) -> f64 {
        self.data.apply_density(input_density)
    }

    /// Get the data for this clamp operation.
    #[inline]
    #[must_use]
    pub fn data(&self) -> &'static ClampData {
        self.data
    }
}

impl NoiseFunctionComponentRange for Clamp {
    #[inline]
    fn min(&self) -> f64 {
        self.data.min_value
    }

    #[inline]
    fn max(&self) -> f64 {
        self.data.max_value
    }
}

impl StaticChunkNoiseFunctionComponentImpl for Clamp {
    fn sample(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        pos: &impl NoisePos,
        sample_options: &ChunkNoiseFunctionSampleOptions,
    ) -> f64 {
        let input_density = ChunkNoiseFunctionComponent::sample_from_stack(
            &mut component_stack[..=self.input_index],
            pos,
            sample_options,
        );
        self.data.apply_density(input_density)
    }

    fn fill(
        &self,
        component_stack: &mut [ChunkNoiseFunctionComponent],
        array: &mut [f64],
        mapper: &impl IndexToNoisePos,
        sample_options: &mut ChunkNoiseFunctionSampleOptions,
    ) {
        ChunkNoiseFunctionComponent::fill_from_stack(
            &mut component_stack[..=self.input_index],
            array,
            mapper,
            sample_options,
        );
        for value in array {
            *value = self.data.apply_density(*value);
        }
    }
}
