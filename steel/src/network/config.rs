use steel_protocol::packets::common::CCustomPayload;
use steel_protocol::packets::common::{SClientInformation, SCustomPayload};
use steel_protocol::packets::config::CFinishConfiguration;

use steel_protocol::packets::config::CSelectKnownPacks;
use steel_protocol::packets::config::SFinishConfiguration;
use steel_protocol::packets::config::SSelectKnownPacks;
use steel_protocol::packets::shared_implementation::KnownPack;
use steel_protocol::utils::ConnectionProtocol;

use steel_utils::ResourceLocation;
use steel_world::player::Player;
use steel_world::server::WorldServer;

use crate::MC_VERSION;
use crate::network::JavaTcpClient;

pub async fn handle_custom_payload(_tcp_client: &JavaTcpClient, packet: SCustomPayload) {
    println!("Custom payload packet: {:?}", packet);
}

pub async fn handle_client_information(_tcp_client: &JavaTcpClient, packet: SClientInformation) {
    println!("Client information packet: {:?}", packet);
}

const BRAND_PAYLOAD: &[u8; 5] = b"Steel";

pub async fn start_configuration(tcp_client: &JavaTcpClient) {
    tcp_client
        .send_packet_now(CCustomPayload::new(
            ResourceLocation::vanilla_static("brand"),
            Box::new(*BRAND_PAYLOAD),
        ))
        .await;

    tcp_client
        .send_packet_now(CSelectKnownPacks::new(vec![KnownPack::new(
            "minecraft".to_string(),
            "core".to_string(),
            MC_VERSION.to_string(),
        )]))
        .await;
}

pub async fn handle_select_known_packs(tcp_client: &JavaTcpClient, packet: SSelectKnownPacks) {
    println!("Select known packs packet: {:?}", packet);

    let registry_cache = tcp_client.server.registry_cache.registry_packets.clone();
    for encoded_packet in registry_cache.iter() {
        tcp_client.send_encoded_packet_now(encoded_packet).await;
    }

    // Send the packet for tags
    tcp_client
        .send_encoded_packet_now(&tcp_client.server.registry_cache.tags_packet)
        .await;

    // Finish configuration with CFinishConfigurationPacket
    tcp_client.send_packet_now(CFinishConfiguration {}).await;
}

pub async fn handle_finish_configuration(
    tcp_client: &JavaTcpClient,
    _packet: SFinishConfiguration,
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
