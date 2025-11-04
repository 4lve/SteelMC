use steel_protocol::packets::status::{
    CPongResponse, SPingRequest, SStatusRequest, {CStatusResponse, Players, Status, Version},
};

use crate::{MC_VERSION, STEEL_CONFIG, network::JavaTcpClient};

pub async fn handle_status_request(tcp_client: &JavaTcpClient, _packet: SStatusRequest) {
    let res_packet = CStatusResponse::new(Status {
        description: STEEL_CONFIG.motd.clone(),
        players: Some(Players {
            max: STEEL_CONFIG.max_players as i32,
            //TODO: Get online players count
            online: 0,
            sample: vec![],
        }),
        enforce_secure_chat: STEEL_CONFIG.enforce_secure_chat,
        favicon: STEEL_CONFIG.load_favicon(),
        version: Some(Version {
            name: MC_VERSION,
            protocol: steel_registry::packets::CURRENT_MC_PROTOCOL as i32,
        }),
    });
    tcp_client.send_bare_packet_now(res_packet).await;
}

pub async fn handle_ping_request(tcp_client: &JavaTcpClient, packet: SPingRequest) {
    tcp_client
        .send_bare_packet_now(CPongResponse::new(packet.time))
        .await;
    tcp_client.close();
}
