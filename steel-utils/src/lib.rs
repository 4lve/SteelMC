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
pub mod locks;
pub mod math;
pub mod random;
pub mod serial;
pub mod text;
pub mod types;

#[rustfmt::skip]
#[path = "generated/vanilla_translations.rs"]
pub mod translations;

pub use front_vec::FrontVec;
pub use types::BlockPos;
pub use types::BlockStateId;
pub use types::ChunkPos;
pub use types::Identifier;
