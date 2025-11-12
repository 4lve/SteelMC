pub mod chunk_sender;
mod game_profile;
pub mod networking;

use std::sync::Arc;

pub use game_profile::GameProfile;

use steel_protocol::packets::common::SCustomPayload;

use crate::{player::networking::JavaConnection, world::World};

pub struct Player {
    pub gameprofile: GameProfile,
    pub connection: Arc<JavaConnection>,

    pub world: Arc<World>,
}

impl Player {
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

    pub fn handle_custom_payload(&self, packet: SCustomPayload) {
        log::info!("Hello from the other side! {packet:?}");
    }

    pub fn handle_client_tick_end(&self) {
        log::info!("Hello from the other side!");
    }
}
