pub mod chunk_sender;
mod game_profile;
pub mod networking;

pub use game_profile::GameProfile;

use steel_protocol::{
    packets::{common::SCustomPayload, game::SClientTickEnd},
    utils::EnqueuedPacket,
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::player::chunk_sender::ChunkSender;

#[derive(Debug, Clone)]
pub struct Player {
    pub game_profile: GameProfile,
    pub outgoing_packets: UnboundedSender<EnqueuedPacket>,
    pub cancel_token: CancellationToken,

    // Im still not sure if this is the right place but we can find out in the near future
    pub chunk_sender: ChunkSender,
}

impl Player {
    pub fn new(
        game_profile: GameProfile,
        outgoing_packets: UnboundedSender<EnqueuedPacket>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            game_profile,
            outgoing_packets,
            cancel_token,
            chunk_sender: ChunkSender::default(),
        }
    }

    pub fn handle_custom_payload(&self, packet: SCustomPayload) {
        log::info!("Hello from the other side! {packet:?}");
    }

    pub fn handle_client_tick_end(&self, packet: SClientTickEnd) {
        log::info!("Hello from the other side! {packet:?}");
    }
}
