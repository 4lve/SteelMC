use steel_protocol::packets::common::c_custom_payload_packet::CCustomPayloadPacket;
use steel_protocol::packets::common::{
    s_client_information_packet::SClientInformationPacket,
    s_custom_payload_packet::SCustomPayloadPacket,
};
use steel_protocol::packets::configuration::c_finish_configuration_packet::CFinishConfigurationPacket;

use steel_protocol::packets::configuration::c_select_known_packs::CSelectKnownPacks;
use steel_protocol::packets::configuration::s_finish_configuration_packet::SFinishConfigurationPacket;
use steel_protocol::packets::configuration::s_select_known_packs::SSelectKnownPacks;
use steel_protocol::packets::shared_implementation::KnownPack;
use steel_protocol::utils::ConnectionProtocol;

use steel_utils::ResourceLocation;
use steel_world::player::player::Player;
use steel_world::server::server::WorldServer;

use crate::network::java_tcp_client::JavaTcpClient;

pub async fn handle_custom_payload(_tcp_client: &JavaTcpClient, packet: &SCustomPayloadPacket) {
    println!("Custom payload packet: {:?}", packet);
}

pub async fn handle_client_information(
    _tcp_client: &JavaTcpClient,
    packet: &SClientInformationPacket,
) {
    println!("Client information packet: {:?}", packet);
}

const BRAND_PAYLOAD: &[u8; 5] = b"Steel";

pub async fn start_configuration(tcp_client: &JavaTcpClient) {
    tcp_client
        .send_packet_now(CCustomPayloadPacket::new(
            ResourceLocation::vanilla_static("brand"),
            Box::new(*BRAND_PAYLOAD),
        ))
        .await;

    tcp_client
        .send_packet_now(CSelectKnownPacks::new(vec![KnownPack::new(
            "minecraft".to_string(),
            "core".to_string(),
            "1.21.10".to_string(),
        )]))
        .await;
}

pub async fn handle_select_known_packs(tcp_client: &JavaTcpClient, packet: &SSelectKnownPacks) {
    println!("Select known packs packet: {:?}", packet);

    let registry_cache = tcp_client.server.registry_cache.clone();
    for encoded_packet in registry_cache.compressed_registry_packets.iter() {
        tcp_client
            .send_encoded_packet_now(encoded_packet)
            .await;
    }

    // Send the packet for tags
    tcp_client
        .send_encoded_packet_now(&registry_cache.compressed_tags_packet)
        .await;

    // Finish configuration with CFinishConfigurationPacket
    tcp_client
        .send_packet_now(CFinishConfigurationPacket::new())
        .await;
}

pub async fn handle_finish_configuration(
    tcp_client: &JavaTcpClient,
    _packet: &SFinishConfigurationPacket,
) {
    tcp_client
        .connection_protocol
        .store(ConnectionProtocol::PLAY);

    tcp_client.server.add_player(Player::new(
        tcp_client.gameprofile.lock().await.clone().unwrap(),
        tcp_client.outgoing_queue.clone(),
        tcp_client.cancel_token.clone(),
    ));
}
