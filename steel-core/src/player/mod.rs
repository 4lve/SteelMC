//! This module contains all things player-related.
pub mod chunk_sender;
mod game_profile;
/// This module contains the networking implementation for the player.
pub mod networking;

use std::sync::Arc;

pub use game_profile::GameProfile;

use steel_protocol::packets::common::SCustomPayload;

use crate::{player::networking::JavaConnection, world::World};

/// A struct representing a player.
pub struct Player {
    /// The player's game profile.
    pub gameprofile: GameProfile,
    /// The player's connection.
    pub connection: Arc<JavaConnection>,

    /// The world the player is in.
    pub world: Arc<World>,
}

impl Player {
    /// Creates a new player.
    pub fn new(
        gameprofile: GameProfile,
        connection: Arc<JavaConnection>,
        world: Arc<World>,
    ) -> Self {
        Self {
            gameprofile,
            connection,

            world,
        }
    }

    /// Handles a custom payload packet.
    pub fn handle_custom_payload(&self, packet: SCustomPayload) {
        log::info!("Hello from the other side! {packet:?}");
    }

    /// Handles the end of a client tick.
    pub fn handle_client_tick_end(&self) {
        log::info!("Hello from the other side!");
    }
}
