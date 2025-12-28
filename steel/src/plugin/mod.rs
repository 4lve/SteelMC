//! Plugin loading and management.
//!
//! This module handles discovering, loading, and managing plugins from dynamic libraries.

use std::path::Path;
use std::sync::Arc;

use abi_stable::{
    library::{LibraryError, RootModule},
    std_types::RBox,
};
use steel_core::chunk::{
    plugin_chunk_generator::PluginChunkGeneratorWrapper,
    world_gen_context::ChunkGeneratorType,
};
use steel_plugin_api::{PluginChunkGenerator_TO, PluginMetadata, PluginModule_Ref};

/// A loaded plugin.
pub struct LoadedPlugin {
    /// The plugin metadata.
    pub metadata: PluginMetadata,
    /// The chunk generators provided by this plugin, wrapped in Arc for sharing.
    pub generators: Vec<Arc<PluginChunkGenerator_TO<'static, RBox<()>>>>,
}

/// Manages loaded plugins.
pub struct PluginManager {
    /// List of loaded plugins.
    pub plugins: Vec<LoadedPlugin>,
}

impl PluginManager {
    /// Creates a new empty plugin manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Loads all plugins from the specified directory.
    ///
    /// Plugins are expected to be dynamic libraries (`.so` on Linux, `.dll` on Windows, `.dylib` on macOS).
    pub fn load_plugins_from_directory(&mut self, plugins_dir: &Path) -> Result<(), String> {
        if !plugins_dir.exists() {
            log::info!("Plugins directory does not exist: {:?}", plugins_dir);
            // Create the directory for convenience
            if let Err(e) = std::fs::create_dir_all(plugins_dir) {
                log::warn!("Failed to create plugins directory: {e}");
            }
            return Ok(());
        }

        let entries = std::fs::read_dir(plugins_dir).map_err(|e| format!("Failed to read plugins directory: {e}"))?;

        for entry in entries.flatten() {
            let path = entry.path();

            // Check if it's a dynamic library
            let extension = path.extension().and_then(|e| e.to_str());
            let is_plugin = matches!(extension, Some("so") | Some("dll") | Some("dylib"));

            if is_plugin {
                match self.load_plugin(&path) {
                    Ok(()) => log::info!("Loaded plugin: {:?}", path),
                    Err(e) => log::error!("Failed to load plugin {:?}: {e}", path),
                }
            }
        }

        Ok(())
    }

    /// Loads a single plugin from a dynamic library path.
    fn load_plugin(&mut self, path: &Path) -> Result<(), String> {
        log::debug!("Loading plugin from: {:?}", path);

        // Load the root module
        let module: PluginModule_Ref = PluginModule_Ref::load_from_file(path)
            .map_err(|e| format_library_error(&e))?;

        // Get metadata
        let get_metadata = module.get_metadata();
        let metadata = get_metadata();

        log::info!(
            "Plugin '{}' v{}: {}",
            metadata.name,
            metadata.version,
            metadata.description
        );

        // Get generators
        let get_generators = module.get_chunk_generators();
        let generators = get_generators();

        log::info!(
            "  Provides {} chunk generator(s)",
            generators.len()
        );

        for generator in &generators {
            log::info!("    - {}", generator.name());
        }

        self.plugins.push(LoadedPlugin {
            metadata,
            generators: generators.into_iter().map(Arc::new).collect(),
        });

        Ok(())
    }

    /// Gets all chunk generators from all loaded plugins as wrapped types.
    /// 
    /// Note: This consumes the generators from the plugin manager since 
    /// `PluginChunkGenerator_TO` doesn't implement Clone.
    pub fn take_all_chunk_generators(&mut self) -> Vec<ChunkGeneratorType> {
        self.plugins
            .iter_mut()
            .flat_map(|plugin| {
                std::mem::take(&mut plugin.generators)
                    .into_iter()
                    .filter_map(|arc_gen| {
                        // Try to unwrap the Arc, skip if there are multiple references
                        Arc::try_unwrap(arc_gen).ok().map(|generator| {
                            ChunkGeneratorType::Plugin(PluginChunkGeneratorWrapper::new(generator))
                        })
                    })
            })
            .collect()
    }

    /// Gets the first plugin generator, if any.
    /// 
    /// Note: This consumes the generator since `PluginChunkGenerator_TO` doesn't implement Clone.
    pub fn take_first_generator(&mut self) -> Option<ChunkGeneratorType> {
        self.plugins
            .first_mut()
            .and_then(|p| p.generators.pop())
            .and_then(|arc_gen| {
                Arc::try_unwrap(arc_gen).ok().map(|generator| {
                    ChunkGeneratorType::Plugin(PluginChunkGeneratorWrapper::new(generator))
                })
            })
    }
}


impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Formats a library error for display.
fn format_library_error(error: &LibraryError) -> String {
    match error {
        LibraryError::OpenError { path, err, .. } => {
            format!("Failed to open library at {path:?}: {err}")
        }
        LibraryError::GetSymbolError { err, .. } => {
            format!("Failed to get symbol from library: {err}")
        }
        LibraryError::InvalidAbiHeader(e) => {
            format!("Invalid ABI header: {e:?}")
        }
        LibraryError::ParseVersionError(e) => {
            format!("Failed to parse version: {e}")
        }
        LibraryError::IncompatibleVersionNumber { library_name, expected_version, actual_version } => {
            format!(
                "Incompatible version for {}: expected {expected_version:?}, got {actual_version:?}",
                &*library_name
            )
        }
        LibraryError::RootModule { err, module_name, .. } => {
            format!("Root module error for {}: {err}", &*module_name)
        }
        LibraryError::Many(errors) => {
            errors.iter().map(format_library_error).collect::<Vec<_>>().join("; ")
        }
        _ => format!("Unknown library error: {error:?}"),
    }
}
