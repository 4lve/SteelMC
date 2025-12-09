//! This module contains the `World` struct, which represents a world.
use std::sync::Arc;

use scc::HashMap;
use steel_registry::Registry;
use steel_protocol::packets::game::{CPlayerChat, CSystemChat};
use steel_utils::codec::VarInt;
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
        self.players.iter_sync(|_uuid, player| {
            player.tick();

            true
        });
    }

    /// Broadcasts a signed chat message to all players in the world.
    pub fn broadcast_chat(
        &self,
        mut packet: CPlayerChat,
        sender: Arc<Player>,
        sender_last_seen: LastSeen,
        message_signature: Option<[u8; 256]>,
    ) {
        self.players.iter_sync(|_, recipient| {
            let messages_received = recipient.get_and_increment_messages_received();
            packet.global_index = VarInt(messages_received);

            let previous_messages = {
                let recipient_cache = recipient.signature_cache.lock();
                recipient_cache.index_previous_messages(&sender_last_seen)
            };
            packet.previous_messages = previous_messages;

            recipient.connection.send_packet(packet.clone());

            if let Some(signature) = &message_signature {
                recipient
                    .message_validator
                    .lock()
                    .add_pending(Some(Box::new(*signature) as Box<[u8]>));
            } else {
                recipient.message_validator.lock().add_pending(None);
            }

            if let Some(signature) = &message_signature {
                recipient
                    .signature_cache
                    .lock()
                    .add_seen_signature(signature);

                if recipient.gameprofile.id != sender.gameprofile.id {
                    recipient
                        .signature_cache
                        .lock()
                        .cache_signatures(sender_last_seen.as_slice());
                }
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
            packet.global_index = VarInt(messages_received);

            recipient.connection.send_packet(packet.clone());
            true
        });
    }
}
