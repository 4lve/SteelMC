//! This module contains the implementation of the world's entity-related methods.
use std::sync::Arc;

use steel_protocol::packets::game::{CGameEvent, GameEventType};
use tokio::time::Instant;

use crate::{player::Player, world::World};

impl World {
    /// Removes a player from the world.
    pub async fn remove_player(self: &Arc<Self>, player: Arc<Player>) {
        let uuid = player.gameprofile.id;

        if self.players.remove_async(&uuid).await.is_some() {
            let self_clone = self.clone();
            let start = Instant::now();
            self_clone.chunk_map.remove_player(&player);
            player.cleanup();
            log::info!("Player {uuid} removed in {:?}", start.elapsed());
        }
    }

    /// Adds a player to the world.
    pub fn add_player(self: &Arc<Self>, player: Arc<Player>) {
        if self
            .players
            .insert_sync(player.gameprofile.id, player.clone())
            .is_err()
        {
            player.connection.close();
            return;
        }

        // Send existing players to the new player (ADD_PLAYER without chat sessions yet)
        // The chat sessions will be sent separately when they become available
        self.players.iter_sync(|_, existing_player| {
            if existing_player.gameprofile.id != player.gameprofile.id {
                let add_existing = steel_protocol::packets::game::CPlayerInfoUpdate::add_player(
                    existing_player.gameprofile.id,
                    existing_player.gameprofile.name.clone(),
                );
                player.connection.send_packet(add_existing);

                // If the existing player has a chat session, send it too
                if let Some(session) = existing_player.chat_session()
                    && let Ok(protocol_data) = session.as_data().to_protocol_data()
                {
                    let session_packet =
                        steel_protocol::packets::game::CPlayerInfoUpdate::update_chat_session(
                            existing_player.gameprofile.id,
                            protocol_data,
                        );
                    player.connection.send_packet(session_packet);
                }
            }
            true
        });

        // Broadcast new player to all existing players (ADD_PLAYER)
        let player_info_packet = steel_protocol::packets::game::CPlayerInfoUpdate::add_player(
            player.gameprofile.id,
            player.gameprofile.name.clone(),
        );

        self.players.iter_sync(|_, p| {
            p.connection.send_packet(player_info_packet.clone());
            true
        });

        player.connection.send_packet(CGameEvent {
            event: GameEventType::LevelChunksLoadStart,
            data: 0.0,
        });

        player.connection.send_packet(CGameEvent {
            event: GameEventType::ChangeGameMode,
            data: 1.0,
        });
    }
}
