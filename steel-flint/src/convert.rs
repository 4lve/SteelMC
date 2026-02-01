//! Conversion utilities between Flint types and `SteelMC` types.

use flint_core::test_spec::{Block as FlintBlock, BlockFace};
use flint_core::traits::BlockData;
use rustc_hash::FxHashMap;
use steel_registry::REGISTRY;
use steel_registry::blocks::properties::Direction;
use steel_utils::{BlockPos as SteelBlockPos, BlockStateId, Identifier};

/// Convert a Flint block specification to a `SteelMC` `BlockStateId`.
///
/// Returns `None` if the block ID is unknown or if any property is invalid.
pub fn flint_block_to_state_id(block: &FlintBlock) -> Option<BlockStateId> {
    // Parse the block ID - may have "minecraft:" prefix
    let block_id = if block.id.starts_with("minecraft:") {
        &block.id[10..]
    } else {
        &block.id
    };

    let identifier = Identifier::vanilla(block_id.to_string());

    // Convert properties from serde_json::Value to (&str, &str) pairs
    let properties: Vec<(String, String)> = block
        .properties
        .iter()
        .filter_map(|(key, value)| {
            // Skip the "properties" key (nested format from flint-core)
            if key == "properties" {
                return None;
            }

            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => return None,
            };

            Some((key.clone(), value_str))
        })
        .collect();

    // If no properties specified, return the block's default state
    if properties.is_empty() {
        let block_ref = REGISTRY.blocks.by_key(&identifier)?;
        return Some(REGISTRY.blocks.get_default_state_id(block_ref));
    }

    // Convert to the format expected by the registry
    let props_refs: Vec<(&str, &str)> = properties
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    REGISTRY
        .blocks
        .state_id_from_properties(&identifier, &props_refs)
}

/// Convert a `SteelMC` `BlockStateId` to Flint `BlockData`.
pub fn state_id_to_block_data(state_id: BlockStateId) -> BlockData {
    let Some(block) = REGISTRY.blocks.by_state_id(state_id) else {
        return BlockData::new("minecraft:air");
    };

    let id = format!("minecraft:{}", block.key.path);

    // Get properties from the registry
    // Note: Using std HashMap because flint-steel's API requires it
    let props = REGISTRY.blocks.get_properties(state_id);
    #[allow(clippy::disallowed_types)]
    let properties: FxHashMap<String, String> = props
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    BlockData::with_properties(id, properties)
}

/// Convert Flint `BlockPos` to `SteelMC` `BlockPos`.
#[allow(dead_code)]
pub fn flint_pos_to_steel(pos: flint_core::BlockPos) -> SteelBlockPos {
    SteelBlockPos::new(pos[0], pos[1], pos[2])
}

/// Convert `SteelMC` `BlockPos` to Flint `BlockPos`.
#[allow(dead_code)]
pub fn steel_pos_to_flint(pos: &SteelBlockPos) -> flint_core::BlockPos {
    [pos.x(), pos.y(), pos.z()]
}

/// Convert Flint `BlockFace` to `SteelMC` Direction.
#[allow(dead_code)]
pub fn flint_face_to_direction(face: BlockFace) -> Direction {
    match face {
        BlockFace::Top => Direction::Up,
        BlockFace::Bottom => Direction::Down,
        BlockFace::North => Direction::North,
        BlockFace::South => Direction::South,
        BlockFace::East => Direction::East,
        BlockFace::West => Direction::West,
    }
}

/// Convert `SteelMC` Direction to Flint `BlockFace`.
#[allow(dead_code)]
pub fn direction_to_flint_face(dir: Direction) -> BlockFace {
    match dir {
        Direction::Up => BlockFace::Top,
        Direction::Down => BlockFace::Bottom,
        Direction::North => BlockFace::North,
        Direction::South => BlockFace::South,
        Direction::East => BlockFace::East,
        Direction::West => BlockFace::West,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_registries;

    #[test]
    fn test_simple_block_conversion() {
        init_test_registries();
        let block = FlintBlock {
            id: "minecraft:stone".to_string(),
            properties: FxHashMap::default(),
        };

        let state_id = flint_block_to_state_id(&block);
        assert!(state_id.is_some(), "Stone should convert to valid state ID");

        let block_data = state_id_to_block_data(state_id.expect("Valid state ID"));
        assert_eq!(block_data.id, "minecraft:stone");
    }

    #[test]
    fn test_air_block() {
        init_test_registries();
        let block = FlintBlock {
            id: "minecraft:air".to_string(),
            properties: FxHashMap::default(),
        };

        let state_id = flint_block_to_state_id(&block);
        assert!(state_id.is_some(), "Air should convert to valid state ID");
    }

    #[test]
    fn test_block_without_prefix() {
        init_test_registries();
        let block = FlintBlock {
            id: "stone".to_string(),
            properties: FxHashMap::default(),
        };

        let state_id = flint_block_to_state_id(&block);
        assert!(state_id.is_some(), "Block without prefix should still work");
    }
}
