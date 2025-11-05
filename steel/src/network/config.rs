use steel_protocol::packets::common::CCustomPayload;
use steel_protocol::packets::common::{SClientInformation, SCustomPayload};
use steel_protocol::packets::config::CFinishConfiguration;

use steel_protocol::packets::config::CSelectKnownPacks;
use steel_protocol::packets::config::SFinishConfiguration;
use steel_protocol::packets::config::SSelectKnownPacks;
use steel_protocol::packets::shared_implementation::KnownPack;
use steel_protocol::utils::ConnectionProtocol;

use steel_utils::Identifier;
use steel_world::player::Player;
use steel_world::server::WorldServer;

use crate::MC_VERSION;
use crate::network::JavaTcpClient;

const BRAND_PAYLOAD: &[u8; 5] = b"Steel";

impl JavaTcpClient {
    pub fn handle_config_custom_payload(&self, packet: SCustomPayload) {
        println!("Custom payload packet: {packet:?}");
    }

    pub fn handle_client_information(&self, packet: SClientInformation) {
        println!("Client information packet: {packet:?}");
    }

    pub async fn start_configuration(&self) {
        self.send_bare_packet_now(CCustomPayload::new(
            Identifier::vanilla_static("brand"),
            Box::new(*BRAND_PAYLOAD),
        ))
        .await;

        self.send_bare_packet_now(CSelectKnownPacks::new(vec![KnownPack::new(
            "minecraft".to_string(),
            "core".to_string(),
            MC_VERSION.to_string(),
        )]))
        .await;
    }

    pub async fn handle_select_known_packs(&self, packet: SSelectKnownPacks) {
        println!("Select known packs packet: {packet:?}");

        let registry_cache = self.server.registry_cache.registry_packets.clone();
        for encoded_packet in registry_cache.iter() {
            self.send_packet_now(encoded_packet).await;
        }

        // Send the packet for tags
        self.send_packet_now(&self.server.registry_cache.tags_packet)
            .await;

        // Finish configuration with CFinishConfigurationPacket
        self.send_bare_packet_now(CFinishConfiguration {}).await;
    }

    /// # Panics
    /// This function will panic if the game profile is empty, should be impossible at this point.
    pub async fn handle_finish_configuration(&self, _packet: SFinishConfiguration) {
        self.protocol.store(ConnectionProtocol::Play);

        self.server.add_player(Player::new(
            self.gameprofile
                .lock()
                .await
                .clone()
                .expect("Game profile is empty"),
            self.outgoing_queue.clone(),
            self.cancel_token.clone(),
        ));
    }
}
