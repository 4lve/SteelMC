use crate::player::Player;
use steel_protocol::{
    packet_traits::{ClientPacket, EncodedPacket},
    utils::{ConnectionProtocol, EnqueuedPacket},
};

impl Player {
    pub fn enqueue_packet<P: ClientPacket>(&self, packet: P) {
        let buf = EncodedPacket::write_vec(packet, ConnectionProtocol::Play).unwrap();
        self.outgoing_packets
            .send(EnqueuedPacket::RawData(buf))
            .unwrap();
    }
}
