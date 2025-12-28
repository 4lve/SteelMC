# Steel Plugin Example

An example plugin for SteelMC that demonstrates custom world generation using the `abi_stable` plugin system.

## Description

This plugin provides a custom chunk generator that creates a world made of stone blocks instead of the default flat world (bedrock, dirt, grass).

## Building

From the workspace root:

```bash
cargo build -p steel-plugin-example --release
```

The compiled plugin will be at:
- **Linux**: `target/release/libsteel_plugin_example.so`
- **Windows**: `target/release/steel_plugin_example.dll`
- **macOS**: `target/release/libsteel_plugin_example.dylib`

## Installation

1. Create a `plugins` directory in your SteelMC server folder (if it doesn't exist)
2. Copy the compiled plugin to the `plugins` directory:

```bash
mkdir -p plugins
cp target/release/libsteel_plugin_example.so plugins/
```

## Usage

Simply start the SteelMC server:

```bash
cargo run -p steel
```

The server will automatically:
1. Discover and load the plugin from the `plugins/` directory
2. Use the plugin's chunk generator for world generation

You should see output like:
```
INFO  Loaded 1 plugin(s)
INFO  Plugin 'Stone World' v0.1.0: A plugin that generates a world made of stone blocks
INFO    Provides 1 chunk generator(s)
INFO      - stone_world
INFO  Using chunk generator from plugin
```

## Creating Your Own Plugin

See the source code in `src/lib.rs` for an example of how to:

1. Implement the `PluginChunkGenerator` trait
2. Export plugin metadata via `get_metadata()`
3. Export generators via `get_chunk_generators()`
4. Set up the plugin entry point with `#[export_root_module]`

### Key Dependencies

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
abi_stable = "0.11"
steel-plugin-api = { path = "../steel-plugin-api" }
```

### Block State IDs

Block state IDs are protocol-level identifiers. Currently, plugins must use hardcoded IDs. Common values:
- Stone: `1`
- Air: `0`

For other blocks, check the SteelMC registry or Minecraft protocol documentation.
