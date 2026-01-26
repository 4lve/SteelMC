//! Fluid behavior system.
//!
//! This module handles fluid mechanics: spreading, flowing, waterlogging.
//! Based on vanilla Minecraft's FlowingFluid system.

pub mod flowing;
mod water;

pub use flowing::{FluidBehaviour, FluidType, FluidState, get_fluid_state};
pub use water::WaterFluid;
