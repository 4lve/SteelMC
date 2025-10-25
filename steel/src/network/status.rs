use std::sync::atomic::Ordering;

use steel_protocol::packets::{
    clientbound::{CBoundPacket, CBoundStatus},
    status::{
        c_pong_response_packet::CPongResponsePacket,
        c_status_response_packet::{CStatusResponsePacket, Players, Status, Version},
        s_ping_request_packet::SPingRequestPacket,
        s_status_request_packet::SStatusRequestPacket,
    },
};

use crate::{MC_VERSION, STEEL_CONFIG, network::java_tcp_client::JavaTcpClient};

pub async fn handle_status_request(tcp_client: &JavaTcpClient, _packet: &SStatusRequestPacket) {
    // Checks if this funciton has already been called this connection. If not it sets has_requested_status to true. If it has been called before compare_exchange fails.
    if tcp_client
        .has_requested_status
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        tcp_client.close();
        return;
    }

    let res_packet = CStatusResponsePacket::new(Status {
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
    tcp_client
        .send_packet_now(CBoundPacket::Status(CBoundStatus::StatusResponse(
            res_packet,
        )))
        .await;
}

pub async fn handle_ping_request(tcp_client: &JavaTcpClient, packet: &SPingRequestPacket) {
    let res_packet = CPongResponsePacket::new(packet.time);
    tcp_client
        .send_packet_now(CBoundPacket::Status(CBoundStatus::Pong(res_packet)))
        .await;
    tcp_client.close();
}
