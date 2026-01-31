/// Base path for builtin datapacks
pub const DATAPACK_BASE: &str =
    "../steel-registry/build_assets/builtin_datapacks/minecraft/data/minecraft";

pub fn strip_minecraft_prefix(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}
