//! This module contains the `World` struct, which represents a world.
use std::sync::Arc;
use std::time::Duration;

use scc::HashMap;
use steel_protocol::packets::game::{CPlayerChat, CSystemChat};
use steel_registry::Registry;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::{
    ChunkMap,
    player::{LastSeen, Player},
};

mod world_entities;

/// A struct that represents a world.
pub struct World {
    /// The chunk map of the world.
    pub chunk_map: Arc<ChunkMap>,
    /// A map of all the players in the world.
    pub players: HashMap<Uuid, Arc<Player>>,
}

impl World {
    /// Creates a new world.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new(registry: &Arc<Registry>, chunk_runtime: Arc<Runtime>) -> Self {
        Self {
            chunk_map: Arc::new(ChunkMap::new(registry, chunk_runtime)),
            players: HashMap::new(),
        }
    }

    /// Ticks the world.
    pub fn tick_b(&self, tick_count: u64) {
        self.chunk_map.tick_b(tick_count);

        // Tick players
        let start = tokio::time::Instant::now();
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

    /// Saves all dirty chunks in this world to disk.
    ///
    /// This should be called during graceful shutdown.
    /// Returns the number of chunks saved.
    pub async fn save_all_chunks(&self) -> std::io::Result<usize> {
        self.chunk_map.save_all_chunks().await
    }
}
