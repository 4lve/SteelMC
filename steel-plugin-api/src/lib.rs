//! # Steel Plugin API
//!
//! This crate defines the FFI-safe interface for Steel plugins using `abi_stable`.
//!
//! Plugins implement the [`PluginChunkGenerator`] trait to provide custom world generation.

#![allow(missing_docs)]

use abi_stable::{
    StableAbi,
    declare_root_module_statics,
    library::RootModule,
    package_version_strings,
    sabi_trait,
    sabi_types::VersionStrings,
    std_types::{RBox, RStr, RString, RVec},
};

/// FFI-safe block state ID wrapper.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, StableAbi)]
pub struct FfiBlockStateId(pub u16);

/// FFI-safe chunk position.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, StableAbi)]
pub struct FfiChunkPos {
    pub x: i32,
    pub z: i32,
}

/// FFI-safe trait for chunk access.
///
/// This provides methods for plugins to read and write blocks in a chunk.
#[sabi_trait]
pub trait ChunkAccessTrait: Send + Sync {
    /// Sets a block at the given relative position.
    fn set_relative_block(&self, x: usize, y: usize, z: usize, block: FfiBlockStateId);

    /// Gets a block at the given relative position.
    fn get_relative_block(&self, x: usize, y: usize, z: usize) -> FfiBlockStateId;

    /// Gets the chunk position.
    fn pos(&self) -> FfiChunkPos;
}

/// Plugin metadata.
#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct PluginMetadata {
    /// The name of the plugin.
    pub name: RString,
    /// The version of the plugin.
    pub version: RString,
    /// A description of the plugin.
    pub description: RString,
}

/// FFI-safe trait for chunk generators provided by plugins.
#[sabi_trait]
pub trait PluginChunkGenerator: Send + Sync {
    /// Returns the name of this generator.
    fn name(&self) -> RStr<'_>;

    /// Creates structures in a chunk.
    fn create_structures(&self, chunk: ChunkAccessTrait_TO<'_, RBox<()>>);

    /// Creates biomes in a chunk.
    fn create_biomes(&self, chunk: ChunkAccessTrait_TO<'_, RBox<()>>);

    /// Fills the chunk with noise/terrain.
    fn fill_from_noise(&self, chunk: ChunkAccessTrait_TO<'_, RBox<()>>);

    /// Builds the surface of the chunk.
    fn build_surface(&self, chunk: ChunkAccessTrait_TO<'_, RBox<()>>);

    /// Applies carvers to the chunk.
    fn apply_carvers(&self, chunk: ChunkAccessTrait_TO<'_, RBox<()>>);

    /// Applies biome decorations to the chunk.
    fn apply_biome_decorations(&self, chunk: ChunkAccessTrait_TO<'_, RBox<()>>);
}

/// The root module for Steel plugins.
///
/// This is the entry point that the plugin loader looks for.
#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = PluginModule_Ref)))]
#[sabi(missing_field(panic))]
pub struct PluginModule {
    /// Returns the plugin metadata.
    pub get_metadata: extern "C" fn() -> PluginMetadata,

    /// Returns the chunk generators provided by this plugin.
    #[sabi(last_prefix_field)]
    pub get_chunk_generators: extern "C" fn() -> RVec<PluginChunkGenerator_TO<'static, RBox<()>>>,
}

impl RootModule for PluginModule_Ref {
    declare_root_module_statics! {PluginModule_Ref}

    const BASE_NAME: &'static str = "steel_plugin";
    const NAME: &'static str = "steel_plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}
