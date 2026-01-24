//! RAM-only chunk storage.
//!
//! This module provides an in-memory chunk storage implementation that
//! doesn't persist chunks to disk. Useful for:
//! - Testing frameworks (like Flint)
//! - Minigame worlds
//! - Temporary worlds that don't need persistence

use std::{io, sync::Weak};

use rustc_hash::FxHashSet;
use steel_utils::{ChunkPos, locks::AsyncRwLock};

use crate::chunk::{
    chunk_access::{ChunkAccess, ChunkStatus},
    level_chunk::LevelChunk,
    section::{ChunkSection, Sections},
};
use crate::world::World;

use super::PreparedChunkSave;

/// In-memory chunk storage.
///
/// This storage implementation creates empty chunks on demand and doesn't
/// persist any data to disk. It's designed for test worlds where:
/// - All chunks start as empty (all air)
/// - No actual data needs to be saved between loads
/// - Chunk generation is bypassed (chunks are instantly available)
pub struct RamOnlyStorage {
    /// Positions of chunks that have been "saved" (for tracking purposes).
    /// We don't actually store the data since tests typically work with
    /// the live chunk data in memory.
    saved_chunks: AsyncRwLock<FxHashSet<ChunkPos>>,
    /// If true, create empty chunks on first access instead of returning None.
    create_empty_on_miss: bool,
}

impl RamOnlyStorage {
    /// Creates a new RAM-only storage that returns empty chunks on demand.
    ///
    /// This is suitable for test worlds where you want an "infinite" world
    /// of empty (air) chunks without any chunk generation.
    #[must_use]
    pub fn empty_world() -> Self {
        Self {
            saved_chunks: AsyncRwLock::new(FxHashSet::default()),
            create_empty_on_miss: true,
        }
    }

    /// Creates a new RAM-only storage that only returns previously saved chunks.
    ///
    /// Useful for scenarios where you want to control exactly which chunks exist.
    #[must_use]
    #[allow(dead_code)]
    pub fn preloaded() -> Self {
        Self {
            saved_chunks: AsyncRwLock::new(FxHashSet::default()),
            create_empty_on_miss: false,
        }
    }

    /// Creates an empty chunk at the given position.
    fn create_empty_chunk(
        pos: ChunkPos,
        min_y: i32,
        height: i32,
        level: Weak<World>,
    ) -> (ChunkAccess, ChunkStatus) {
        let section_count = (height / 16) as usize;
        let sections: Vec<ChunkSection> = (0..section_count)
            .map(|_| ChunkSection::new_empty())
            .collect();

        let sections = Sections::from_owned(sections.into_boxed_slice());

        // Create as Full status since empty chunks don't need generation
        let chunk = ChunkAccess::Full(LevelChunk::from_disk(sections, pos, min_y, height, level));

        (chunk, ChunkStatus::Full)
    }

    /// Loads a chunk from storage.
    pub async fn load_chunk(
        &self,
        pos: ChunkPos,
        min_y: i32,
        height: i32,
        level: Weak<World>,
    ) -> io::Result<Option<(ChunkAccess, ChunkStatus)>> {
        // For RAM storage with create_empty_on_miss, we always create empty chunks
        // The actual block data is stored in the live ChunkAccess/World
        if self.create_empty_on_miss {
            Ok(Some(Self::create_empty_chunk(pos, min_y, height, level)))
        } else {
            // In preloaded mode, only return chunks that have been "saved"
            if self.saved_chunks.read().await.contains(&pos) {
                Ok(Some(Self::create_empty_chunk(pos, min_y, height, level)))
            } else {
                Ok(None)
            }
        }
    }

    /// Saves prepared chunk data to storage.
    pub async fn save_chunk_data(&self, prepared: PreparedChunkSave) -> io::Result<bool> {
        // Just track that this chunk has been saved
        // The actual data is in the live World/ChunkAccess, not persisted
        self.saved_chunks.write().await.insert(prepared.pos);
        Ok(true)
    }

    /// Checks if a chunk exists in storage.
    pub async fn chunk_exists(&self, pos: ChunkPos) -> io::Result<bool> {
        if self.create_empty_on_miss {
            // All chunks "exist" since we create them on demand
            Ok(true)
        } else {
            Ok(self.saved_chunks.read().await.contains(&pos))
        }
    }
}
