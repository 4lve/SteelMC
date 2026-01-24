//! # Steel Utils
//!
//! This crate contains a collection of utilities used by the Steel Minecraft server.

#![feature(
    const_trait_impl,
    const_slice_make_iter,
    const_cmp,
    derive_const,
    core_intrinsics
)]
#![allow(internal_features)]

pub mod codec;
/// Density function system for vanilla-accurate terrain generation.
pub mod density;
mod front_vec;
/// CRC32C hashing for component validation.
pub mod hash;
/// A module for custom locks.
pub mod locks;
pub mod math;
/// Noise generation for vanilla-accurate world generation.
pub mod noise;
/// Pumpkin-style noise router for vanilla-accurate terrain generation.
pub mod noise_router;
pub mod random;
pub mod serial;
pub mod text;
/// A module for common types.
pub mod types;

#[rustfmt::skip]
#[path = "generated/vanilla_translations.rs"]
#[allow(missing_docs, warnings)]
pub mod translations;

#[rustfmt::skip]
#[path = "generated/noise_params.rs"]
#[allow(missing_docs, warnings)]
pub mod generated_noise_params;

#[rustfmt::skip]
#[path = "generated/density_functions.rs"]
#[allow(missing_docs, warnings)]
pub mod generated_density_functions;

pub use front_vec::FrontVec;
pub use types::BlockPos;
pub use types::BlockStateId;
pub use types::ChunkPos;
pub use types::Identifier;
pub use types::SectionPos;
