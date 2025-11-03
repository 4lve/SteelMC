pub mod game_profile;

use steel_protocol::{
    packet_traits::{ClientPacket, EncodedPacket},
    utils::{ConnectionProtocol, EnqueuedPacket},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::player::game_profile::GameProfile;

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

    pub fn enqueue_packet<P: ClientPacket>(&self, packet: P) {
        let buf = EncodedPacket::write_packet(packet, ConnectionProtocol::PLAY).unwrap();
        self.outgoing_packets
            .send(EnqueuedPacket::RawData(buf))
            .unwrap();
    }
}
