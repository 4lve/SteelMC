//! Chunk storage abstraction.
//!
//! This module provides the `ChunkStorage` enum which abstracts chunk persistence.
//! Variants can store chunks on disk (via `RegionManager`) or in memory
//! (via `RamOnlyStorage`) for testing and minigames.

use std::{io, sync::Weak};

use steel_utils::ChunkPos;

use crate::chunk::chunk_access::{ChunkAccess, ChunkStatus};
use crate::world::World;

use super::ram_only::RamOnlyStorage;
use super::region_manager::RegionManager;
use super::PreparedChunkSave;

/// Chunk storage backend.
///
/// This enum provides persistence for chunks, either to disk (region files)
/// or in-memory (for testing/minigames).
pub enum ChunkStorage {
    /// Disk-based storage using region files.
    Disk(RegionManager),
    /// In-memory storage for testing and minigames.
    RamOnly(RamOnlyStorage),
}

impl ChunkStorage {
    /// Loads a chunk from storage.
    ///
    /// Returns `Ok(None)` if the chunk doesn't exist in storage.
    /// For `RamOnly` with `create_empty_on_miss=true`, this always
    /// returns an empty chunk (never `None`).
    pub async fn load_chunk(
        &self,
        pos: ChunkPos,
        min_y: i32,
        height: i32,
        level: Weak<World>,
    ) -> io::Result<Option<(ChunkAccess, ChunkStatus)>> {
        match self {
            Self::Disk(rm) => rm.load_chunk(pos, min_y, height, level).await,
            Self::RamOnly(ram) => ram.load_chunk(pos, min_y, height, level).await,
        }
    }

    /// Saves prepared chunk data to storage.
    ///
    /// Returns `Ok(true)` if the chunk was saved, `Ok(false)` if it was a no-op.
    pub async fn save_chunk_data(
        &self,
        prepared: PreparedChunkSave,
        status: ChunkStatus,
    ) -> io::Result<bool> {
        match self {
            Self::Disk(rm) => rm.save_chunk_data(prepared, status).await,
            Self::RamOnly(ram) => ram.save_chunk_data(prepared).await,
        }
    }

    /// Checks if a chunk exists in storage.
    pub async fn chunk_exists(&self, pos: ChunkPos) -> io::Result<bool> {
        match self {
            Self::Disk(rm) => rm.chunk_exists(pos).await,
            Self::RamOnly(ram) => ram.chunk_exists(pos).await,
        }
    }

    /// Acquires a chunk for loading, preparing any necessary resources.
    ///
    /// For disk storage, this opens/creates the region file and returns
    /// whether the chunk exists. For RAM storage, this just checks existence.
    pub async fn acquire_chunk(&self, pos: ChunkPos) -> io::Result<bool> {
        match self {
            Self::Disk(rm) => rm.acquire_chunk(pos).await,
            Self::RamOnly(ram) => ram.chunk_exists(pos).await,
        }
    }

    /// Releases a loaded chunk, allowing the storage to clean up resources.
    pub async fn release_chunk(&self, pos: ChunkPos) -> io::Result<()> {
        match self {
            Self::Disk(rm) => rm.release_chunk(pos).await,
            Self::RamOnly(_) => Ok(()), // No-op for RAM storage
        }
    }

    /// Flushes all dirty data to storage.
    pub async fn flush_all(&self) -> io::Result<()> {
        match self {
            Self::Disk(rm) => rm.flush_all().await,
            Self::RamOnly(_) => Ok(()), // No-op for RAM storage
        }
    }

    /// Closes all storage handles and flushes pending data.
    pub async fn close_all(&self) -> io::Result<()> {
        match self {
            Self::Disk(rm) => rm.close_all().await,
            Self::RamOnly(_) => Ok(()), // No-op for RAM storage
        }
    }
}
