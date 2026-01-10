//! This module contains the `World` struct, which represents a world.
use std::{
    io,
    sync::{Arc, Weak},
    time::Duration,
};

use scc::HashMap;
use steel_protocol::packet_traits::ClientPacket;
use steel_protocol::packets::game::{CPlayerChat, CSystemChat};
use steel_registry::vanilla_blocks;
use steel_registry::{REGISTRY, compat_traits::RegistryWorld, dimension_type::DimensionTypeRef};
use steel_utils::{BlockPos, BlockStateId, ChunkPos, SectionPos, types::UpdateFlags};
use tokio::{runtime::Runtime, time::Instant};
use uuid::Uuid;

use crate::{
    ChunkMap,
    chunk::chunk_access::ChunkAccess,
    player::{LastSeen, Player},
};

mod player_area_map;
mod world_entities;

pub use player_area_map::PlayerAreaMap;

/// A struct that represents a world.
pub struct World {
    /// The chunk map of the world.
    pub chunk_map: Arc<ChunkMap>,
    /// A map of all the players in the world.
    pub players: HashMap<Uuid, Arc<Player>>,
    /// Spatial index for player proximity queries.
    pub player_area_map: PlayerAreaMap,
    /// The dimension of the world.
    pub dimension: DimensionTypeRef,
}

impl World {
    /// Creates a new world.
    ///
    /// Uses `Arc::new_cyclic` to create a cyclic reference between
    /// the World and its `ChunkMap`'s `WorldGenContext`.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new(chunk_runtime: Arc<Runtime>, dimension: DimensionTypeRef) -> Arc<Self> {
        Arc::new_cyclic(|weak_self: &Weak<World>| Self {
            chunk_map: Arc::new(ChunkMap::new(chunk_runtime, weak_self.clone(), &dimension)),
            players: HashMap::new(),
            player_area_map: PlayerAreaMap::new(),
            dimension,
        })
    }

    /// Returns the total height of the world in blocks.
    pub fn get_height(&self) -> i32 {
        self.dimension.height
    }

    /// Returns the minimum Y coordinate of the world.
    pub fn get_min_y(&self) -> i32 {
        self.dimension.min_y
    }

    /// Returns the maximum Y coordinate of the world.
    pub fn get_max_y(&self) -> i32 {
        self.get_min_y() + self.get_height() - 1
    }

    /// Returns whether the given Y coordinate is outside the build height.
    pub fn is_outside_build_height(&self, block_y: i32) -> bool {
        block_y < self.get_min_y() || block_y > self.get_max_y()
    }

    /// Returns whether the block position is within valid horizontal bounds.
    pub fn is_in_valid_bounds_horizontal(&self, block_pos: &BlockPos) -> bool {
        let chunk_x = SectionPos::block_to_section_coord(block_pos.0.x);
        let chunk_z = SectionPos::block_to_section_coord(block_pos.0.z);
        ChunkPos::is_valid(chunk_x, chunk_z)
    }

    /// Returns whether the block position is within valid world bounds.
    pub fn is_in_valid_bounds(&self, block_pos: &BlockPos) -> bool {
        !self.is_outside_build_height(block_pos.0.y)
            && self.is_in_valid_bounds_horizontal(block_pos)
    }

    /// Returns the maximum build height (one above the highest placeable block).
    /// This is `min_y + height`.
    #[must_use]
    pub fn max_build_height(&self) -> i32 {
        self.get_min_y() + self.get_height()
    }

    /// Checks if a player may interact with the world at the given position.
    /// Currently only checks if position is within world bounds.
    #[must_use]
    pub fn may_interact(&self, _player: &Player, pos: &BlockPos) -> bool {
        self.is_in_valid_bounds(pos)
    }

    /// Gets the block state at the given position.
    ///
    /// Returns the default block state (void air) if the position is out of bounds or the chunk is not loaded.
    #[must_use]
    pub fn get_block_state(&self, pos: &BlockPos) -> BlockStateId {
        if !self.is_in_valid_bounds(pos) {
            return REGISTRY.blocks.get_base_state_id(vanilla_blocks::AIR);
        }

        let Some(chunk) = self.get_chunk_at(pos) else {
            return REGISTRY.blocks.get_base_state_id(vanilla_blocks::AIR);
        };

        chunk.get_block_state(*pos)
    }

    /// Sets a block at the given position.
    ///
    /// Returns `true` if the block was successfully set, `false` otherwise.
    pub fn set_block(&self, pos: BlockPos, block_state: BlockStateId, flags: UpdateFlags) -> bool {
        if !self.is_in_valid_bounds(&pos) {
            return false;
        }

        let Some(chunk) = self.get_chunk_at(&pos) else {
            return false;
        };

        let Some(_old_state) = chunk.set_block_state(pos, block_state, flags) else {
            return false;
        };

        // Record the block change for broadcasting to clients
        self.chunk_map.block_changed(&pos);

        //TODO: Neighbor updates and stuff like that

        true
    }

    fn get_chunk_at(&self, pos: &BlockPos) -> Option<Arc<ChunkAccess>> {
        let chunk_pos = ChunkPos::new(
            SectionPos::block_to_section_coord(pos.0.x),
            SectionPos::block_to_section_coord(pos.0.z),
        );
        self.chunk_map.get_full_chunk(&chunk_pos)
    }

    /// Ticks the world.
    pub fn tick_b(&self, tick_count: u64) {
        self.chunk_map.tick_b(tick_count);

        // Tick players
        let start = Instant::now();
        self.players.iter_sync(|_uuid, player| {
            player.tick();

            true
        });
        let player_tick_elapsed = start.elapsed();
        if player_tick_elapsed >= Duration::from_millis(100) {
            log::warn!("Player tick slow: {player_tick_elapsed:?}");
        }
    }

    /// Broadcasts a signed chat message to all players in the world.
    ///
    /// # Panics
    /// Panics if `message_signature` is `None` after checking `is_some()` (should never happen).
    pub fn broadcast_chat(
        &self,
        mut packet: CPlayerChat,
        _sender: Arc<Player>,
        sender_last_seen: LastSeen,
        message_signature: Option<[u8; 256]>,
    ) {
        log::debug!(
            "broadcast_chat: sender_last_seen has {} signatures, message_signature present: {}",
            sender_last_seen.len(),
            message_signature.is_some()
        );

        self.players.iter_sync(|_, recipient| {
            let messages_received = recipient.get_and_increment_messages_received();
            packet.global_index = messages_received;

            log::debug!(
                "Broadcasting to player {} (UUID: {}), global_index={}",
                recipient.gameprofile.name,
                recipient.gameprofile.id,
                messages_received
            );

            // IMPORTANT: Index previous messages BEFORE updating the cache
            // This matches vanilla's order: pack() then push()
            let previous_messages = {
                let recipient_cache = recipient.signature_cache.lock();
                recipient_cache.index_previous_messages(&sender_last_seen)
            };

            log::debug!(
                "  Indexed {} previous messages for recipient",
                previous_messages.len()
            );

            packet.previous_messages.clone_from(&previous_messages);

            // Send the packet
            recipient.connection.send_packet(packet.clone());

            // AFTER sending, update the recipient's cache using vanilla's push algorithm
            // This adds all lastSeen signatures + current signature to the cache
            if let Some(signature) = message_signature {
                recipient
                    .signature_cache
                    .lock()
                    .push(&sender_last_seen, Some(&signature));

                log::debug!("  Added signature to recipient's cache and pending list");

                // Add to pending messages for acknowledgment tracking
                recipient
                    .message_validator
                    .lock()
                    .add_pending(Some(Box::new(signature) as Box<[u8]>));
            } else {
                // Even unsigned messages update the pending tracker
                recipient.message_validator.lock().add_pending(None);
                log::debug!("  Added unsigned message to pending list");
            }

            true
        });
    }

    /// Broadcasts a system chat message to all players.
    pub fn broadcast_system_chat(&self, packet: CSystemChat) {
        self.players.iter_sync(|_, player| {
            player.connection.send_packet(packet.clone());
            true
        });
    }

    /// Broadcasts an unsigned player chat message to all players.
    pub fn broadcast_unsigned_chat(
        &self,
        mut packet: CPlayerChat,
        sender_name: &str,
        message: &str,
    ) {
        log::info!("<{sender_name}> {message}");

        self.players.iter_sync(|_, recipient| {
            let messages_received = recipient.get_and_increment_messages_received();
            packet.global_index = messages_received;

            recipient.connection.send_packet(packet.clone());
            true
        });
    }

    /// Broadcasts a packet to all players tracking the given chunk.
    /// TODO: Look into sending `EncodedPacket` instead
    pub fn broadcast_to_nearby<P: ClientPacket + Clone>(
        &self,
        chunk: ChunkPos,
        packet: P,
        exclude: Option<Uuid>,
    ) {
        let tracking_players = self.player_area_map.get_tracking_players(chunk);
        for uuid in tracking_players {
            if Some(uuid) == exclude {
                continue;
            }
            if let Some(player) = self.players.read_sync(&uuid, |_, p| p.clone()) {
                player.connection.send_packet(packet.clone());
            }
        }
    }

    /// Saves all dirty chunks in this world to disk.
    ///
    /// This should be called during graceful shutdown.
    /// Returns the number of chunks saved.
    pub async fn save_all_chunks(&self) -> io::Result<usize> {
        self.chunk_map.save_all_chunks().await
    }
}

impl RegistryWorld for World {
    fn get_block_state(&self, pos: &BlockPos) -> BlockStateId {
        Self::get_block_state(self, pos)
    }

    fn set_block(&self, pos: BlockPos, block_state: BlockStateId, flags: UpdateFlags) -> bool {
        Self::set_block(self, pos, block_state, flags)
    }

    fn is_in_valid_bounds(&self, block_pos: &BlockPos) -> bool {
        Self::is_in_valid_bounds(self, block_pos)
    }
}
