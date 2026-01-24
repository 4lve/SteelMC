//! Noise generation utilities for vanilla-accurate world generation.
//!
//! This module implements the same noise algorithms used by Minecraft
//! to ensure identical terrain generation for the same seed.

mod blended_noise;
mod double_perlin;
mod improved_noise;
mod math;
mod normal_noise;
mod perlin_noise;
mod simplex_noise;

pub use blended_noise::BlendedNoise;
pub use double_perlin::DoublePerlinNoise;
pub use improved_noise::ImprovedNoise;
pub use math::{
    clamp, clamped_lerp, clamped_map, floor, floor_div, floor_mod, floor_mod_usize, lerp, lerp_f32,
    lerp2, lerp3, lfloor, map, smoothstep, smoothstep_derivative,
};
pub use normal_noise::{NoiseParameters, NormalNoise};
pub use perlin_noise::{PerlinNoise, wrap};
pub use simplex_noise::SimplexNoise;
