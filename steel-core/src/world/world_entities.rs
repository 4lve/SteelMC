//! This module contains the implementation of the world's entity-related methods.
use std::sync::Arc;

use steel_protocol::packets::game::{
    CGameEvent, CPlayerInfoUpdate, CRemovePlayerInfo, CSystemChat, GameEventType,
};
use tokio::time::Instant;

use crate::{entity::PlayerEntity, player::Player, world::World};
use steel_registry::vanilla_entities;
use steel_utils::text::TextComponent;
use steel_utils::text::color::NamedColor;
use steel_utils::translations;

impl World {
    /// Removes a player from the world.
    pub async fn remove_player(self: &Arc<Self>, player: Arc<Player>) {
        let uuid = player.gameprofile.id;

        if self.players.remove_async(&uuid).await.is_some() {
            let self_clone = self.clone();
            let start = Instant::now();

            // Remove from entity tracker if they have an entity
            if let Some(entity_id) = *player.entity_id.lock() {
                self_clone.entity_tracker.remove_entity(entity_id);
                self_clone.entity_tracker.remove_player(uuid);
            }

            // Broadcast player removal from tab list to all players
            let remove_info_packet = CRemovePlayerInfo { uuids: vec![uuid] };
            self.players.iter_sync(|_, p| {
                p.connection.send_packet(remove_info_packet.clone());
                true
            });

            // Broadcast leave message
            let leave_message = TextComponent::from(
                translations::MULTIPLAYER_PLAYER_LEFT.message([player.gameprofile.name.clone()]),
            )
            .color(NamedColor::Yellow);
            self.broadcast_system_chat(CSystemChat::new(leave_message, false));

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
                let add_existing = CPlayerInfoUpdate::add_player(
                    existing_player.gameprofile.id,
                    existing_player.gameprofile.name.clone(),
                    existing_player.gameprofile.properties.clone(),
                );
                player.connection.send_packet(add_existing);

                // If the existing player has a chat session, send it too
                if let Some(session) = existing_player.chat_session()
                    && let Ok(protocol_data) = session.as_data().to_protocol_data()
                {
                    let session_packet = CPlayerInfoUpdate::update_chat_session(
                        existing_player.gameprofile.id,
                        protocol_data,
                    );
                    player.connection.send_packet(session_packet);
                }
            }
            true
        });

        // Broadcast new player to all existing players (ADD_PLAYER)
        let player_info_packet = CPlayerInfoUpdate::add_player(
            player.gameprofile.id,
            player.gameprofile.name.clone(),
            player.gameprofile.properties.clone(),
        );

        self.players.iter_sync(|_, p| {
            p.connection.send_packet(player_info_packet.clone());
            true
        });

        // Broadcast join message
        let join_message = TextComponent::from(
            translations::MULTIPLAYER_PLAYER_JOINED.message([player.gameprofile.name.clone()]),
        )
        .color(NamedColor::Yellow);
        self.broadcast_system_chat(CSystemChat::new(join_message, false));

        player.connection.send_packet(CGameEvent {
            event: GameEventType::LevelChunksLoadStart,
            data: 0.0,
        });

        player.connection.send_packet(CGameEvent {
            event: GameEventType::ChangeGameMode,
            data: player.game_mode.load().into(),
        });

        // Register player as an entity for visibility tracking
        // Entity ID should already be assigned in server.add_player()
        let entity_id = player
            .entity_id
            .lock()
            .expect("Player should have entity ID by now");
        let player_entity = Arc::new(PlayerEntity::new(entity_id, player.clone()));

        // Add to entity tracker so other players can see them
        self.entity_tracker.add_entity(
            player_entity,
            Some(vanilla_entities::PLAYER.tracking_range_blocks()),
        );

        // Immediately update visibility (matches vanilla's updatePlayers() call)
        // This pairs the new player with existing players without waiting for next tick
        self.entity_tracker.update_player_visibility(&player);

        // Also update visibility for all existing players to see the new player
        self.players.iter_sync(|_, existing_player| {
            if existing_player.gameprofile.id != player.gameprofile.id {
                self.entity_tracker
                    .update_player_visibility(existing_player);
            }
            true
        });
    }
}
