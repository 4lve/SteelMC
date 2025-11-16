//! # Steel Utils
//!
//! This crate contains a collection of utilities used by the Steel Minecraft server.
#![warn(clippy::all, clippy::pedantic, clippy::cargo, missing_docs)]
#![allow(
    clippy::single_call_fn,
    clippy::multiple_inherent_impl,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::cargo_common_metadata
)]
#![feature(
    const_trait_impl,
    const_slice_make_iter,
    const_cmp,
    derive_const,
    core_intrinsics
)]
#![allow(internal_features)]

pub mod codec;
mod front_vec;
/// A module for custom locks.
pub mod locks;
pub mod math;
pub mod random;
pub mod serial;
pub mod text;
/// A module for common types.
pub mod types;

#[rustfmt::skip]
#[path = "generated/vanilla_translations.rs"]
#[allow(missing_docs, warnings)]
pub mod translations;

pub use front_vec::FrontVec;
pub use types::BlockPos;
pub use types::BlockStateId;
pub use types::ChunkPos;
pub use types::Identifier;
