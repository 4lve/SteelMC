use steel_utils::locks::SteelRwLock;

use crate::chunk::section::Sections;

#[derive(Debug)]
pub struct LevelChunk {
    pub data: SteelRwLock<Sections>,
}
