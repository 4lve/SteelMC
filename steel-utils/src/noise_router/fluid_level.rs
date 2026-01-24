//! Fluid level system for terrain generation.
//!
//! This module defines fluid levels and samplers used by the aquifer system
//! to determine water and lava placement.

use enum_dispatch::enum_dispatch;

use crate::BlockStateId;

/// A fluid level at a specific Y threshold.
#[derive(Debug, Clone)]
pub struct FluidLevel {
    /// The maximum Y coordinate (exclusive) where this fluid exists.
    max_y: i32,
    /// The block state for this fluid (water or lava).
    block: BlockStateId,
}

impl FluidLevel {
    /// Creates a new fluid level.
    #[must_use]
    pub const fn new(max_y: i32, block: BlockStateId) -> Self {
        Self { max_y, block }
    }

    /// Returns the maximum Y coordinate (exclusive) for this fluid.
    #[must_use]
    pub const fn max_y_exclusive(&self) -> i32 {
        self.max_y
    }

    /// Returns the block at the given Y coordinate.
    ///
    /// Returns the fluid block if `y < max_y`, otherwise returns air.
    #[must_use]
    pub fn get_block(&self, y: i32, air: BlockStateId) -> BlockStateId {
        if y < self.max_y {
            self.block
        } else {
            air
        }
    }

    /// Returns the fluid block state.
    #[must_use]
    pub const fn block(&self) -> BlockStateId {
        self.block
    }
}

/// Trait for sampling fluid levels at positions.
#[enum_dispatch]
pub trait FluidLevelSamplerImpl {
    /// Gets the fluid level at the given position.
    fn get_fluid_level(&self, x: i32, y: i32, z: i32) -> FluidLevel;
}

/// A sampler that returns fluid levels based on position.
#[enum_dispatch(FluidLevelSamplerImpl)]
#[derive(Clone)]
pub enum FluidLevelSampler {
    /// A static fluid level sampler that returns the same level everywhere.
    Static(StaticFluidLevelSampler),
    /// A standard chunk fluid level sampler with top and bottom fluids.
    Standard(StandardChunkFluidLevelSampler),
}

/// A static fluid level sampler that returns the same level everywhere.
#[derive(Clone)]
pub struct StaticFluidLevelSampler {
    level: FluidLevel,
}

impl StaticFluidLevelSampler {
    /// Creates a new static fluid level sampler.
    #[must_use]
    pub const fn new(level: FluidLevel) -> Self {
        Self { level }
    }
}

impl FluidLevelSamplerImpl for StaticFluidLevelSampler {
    fn get_fluid_level(&self, _x: i32, _y: i32, _z: i32) -> FluidLevel {
        self.level.clone()
    }
}

/// A standard fluid level sampler with different fluids above and below a threshold.
#[derive(Clone)]
pub struct StandardChunkFluidLevelSampler {
    /// The fluid above the threshold (typically water at sea level).
    top_fluid: FluidLevel,
    /// The fluid below the threshold (typically lava at depth).
    bottom_fluid: FluidLevel,
    /// The Y threshold below which bottom_fluid is used.
    bottom_y: i32,
}

impl StandardChunkFluidLevelSampler {
    /// Creates a new standard chunk fluid level sampler.
    #[must_use]
    pub const fn new(top_fluid: FluidLevel, bottom_fluid: FluidLevel, bottom_y: i32) -> Self {
        Self {
            top_fluid,
            bottom_fluid,
            bottom_y,
        }
    }

    /// Creates the default overworld fluid level sampler.
    ///
    /// - Top fluid: water at sea level (63)
    /// - Bottom fluid: lava at Y=-54
    /// - Bottom threshold: Y=-54
    #[must_use]
    pub fn overworld(water: BlockStateId, lava: BlockStateId) -> Self {
        Self {
            top_fluid: FluidLevel::new(63, water),
            bottom_fluid: FluidLevel::new(-54, lava),
            bottom_y: -54,
        }
    }
}

impl FluidLevelSamplerImpl for StandardChunkFluidLevelSampler {
    fn get_fluid_level(&self, _x: i32, y: i32, _z: i32) -> FluidLevel {
        if y < self.bottom_y {
            self.bottom_fluid.clone()
        } else {
            self.top_fluid.clone()
        }
    }
}
