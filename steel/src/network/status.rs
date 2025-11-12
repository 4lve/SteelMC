use steel_protocol::packets::status::{
    CPongResponse, SPingRequest, {CStatusResponse, Players, Status, Version},
};
use steel_registry::packets::CURRENT_MC_PROTOCOL;

use crate::{MC_VERSION, STEEL_CONFIG, network::JavaTcpClient};

impl JavaTcpClient {
    /// Handles a status request from the client.
    pub async fn handle_status_request(&self) {
        let res_packet = CStatusResponse::new(Status {
            description: &STEEL_CONFIG.motd,
            players: Some(Players {
                max: STEEL_CONFIG.max_players.cast_signed(),
                //TODO: Get online players count
                online: 0,
                sample: vec![],
            }),
            enforce_secure_chat: STEEL_CONFIG.enforce_secure_chat,
            favicon: STEEL_CONFIG.load_favicon(),
            version: Some(Version {
                name: MC_VERSION,
                protocol: CURRENT_MC_PROTOCOL,
            }),
        });
        self.send_bare_packet_now(res_packet).await;
    }

    /// Handles a ping request from the client.
    pub async fn handle_ping_request(&self, packet: SPingRequest) {
        self.send_bare_packet_now(CPongResponse::new(packet.time))
            .await;
        self.close();
    }
}
