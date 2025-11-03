mod game_profile;
pub mod networking;

pub use game_profile::GameProfile;

use steel_protocol::utils::EnqueuedPacket;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct Player {
    pub game_profile: GameProfile,
    pub outgoing_packets: mpsc::UnboundedSender<EnqueuedPacket>,
    pub cancel_token: CancellationToken,
}

impl Player {
    pub fn new(
        game_profile: GameProfile,
        outgoing_packets: mpsc::UnboundedSender<EnqueuedPacket>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            game_profile,
            outgoing_packets,
            cancel_token,
        }
    }
}
