#![feature(
    const_trait_impl,
    const_slice_make_iter,
    const_cmp,
    derive_const,
    core_intrinsics
)]
#![allow(internal_features)]

pub mod locks;
pub mod math;
pub mod text;
mod front_vec;
mod types;

pub use types::BlockPos;
pub use types::BlockStateId;
pub use types::ChunkPos;
pub use front_vec::FrontVec;
pub use types::ResourceLocation;
