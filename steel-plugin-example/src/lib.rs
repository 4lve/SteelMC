//! # Steel Example Plugin
//!
//! An example plugin that demonstrates custom world generation.
//! This plugin creates a world made of stone instead of the normal flat world layers.

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    std_types::{RBox, RStr, RString, RVec},
};
use steel_plugin_api::{
    ChunkAccessTrait_TO, FfiBlockStateId, PluginChunkGenerator, PluginChunkGenerator_TO,
    PluginMetadata, PluginModule, PluginModule_Ref,
};

// Block state IDs - these are raw protocol IDs
// Stone has block ID 1 and no properties, so state ID should be low
// We use conservative IDs here - stone is typically ID 1 in all Minecraft versions
const STONE: FfiBlockStateId = FfiBlockStateId(1);

/// A chunk generator that creates a flat world of stone.
#[derive(Debug, Clone)]
pub struct StoneWorldGenerator;

impl PluginChunkGenerator for StoneWorldGenerator {
    fn name(&self) -> RStr<'_> {
        RStr::from_str("stone_world")
    }

    fn create_structures(&self, _chunk: ChunkAccessTrait_TO<'_, RBox<()>>) {
        // No structures in this simple generator
    }

    fn create_biomes(&self, _chunk: ChunkAccessTrait_TO<'_, RBox<()>>) {
        // Use default biomes
    }

    fn fill_from_noise(&self, chunk: ChunkAccessTrait_TO<'_, RBox<()>>) {
        // Create a flat world with sine wave terrain
        for x in 0..16 {
            for z in 0..16 {
                let y = (f64::sin(x as f64 / 16.0 * std::f64::consts::PI)
                    * f64::sin(z as f64 / 16.0 * std::f64::consts::PI)
                    * 8.0
                    + 7.0) as usize;
                assert!(y < 16);
                chunk.set_relative_block(x, y, z, STONE);
            }
        }
    }

    fn build_surface(&self, _chunk: ChunkAccessTrait_TO<'_, RBox<()>>) {
        // Surface is already stone blocks
    }

    fn apply_carvers(&self, _chunk: ChunkAccessTrait_TO<'_, RBox<()>>) {
        // No carvers
    }

    fn apply_biome_decorations(&self, _chunk: ChunkAccessTrait_TO<'_, RBox<()>>) {
        // No decorations
    }
}

/// Returns the plugin metadata.
#[sabi_extern_fn]
fn get_metadata() -> PluginMetadata {
    PluginMetadata {
        name: RString::from("Stone World"),
        version: RString::from("0.1.0"),
        description: RString::from("A plugin that generates a world made of stone blocks"),
    }
}

/// Returns the chunk generators provided by this plugin.
#[sabi_extern_fn]
fn get_chunk_generators() -> RVec<PluginChunkGenerator_TO<'static, RBox<()>>> {
    let generator = StoneWorldGenerator;
    let boxed: PluginChunkGenerator_TO<'static, RBox<()>> =
        PluginChunkGenerator_TO::from_value(generator, abi_stable::sabi_trait::TD_Opaque);
    RVec::from(vec![boxed])
}

/// Exports the root module for the plugin loader.
#[export_root_module]
fn instantiate_root_module() -> PluginModule_Ref {
    PluginModule {
        get_metadata,
        get_chunk_generators,
    }
    .leak_into_prefix()
}
