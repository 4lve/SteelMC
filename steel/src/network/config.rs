use std::sync::Arc;

use steel_protocol::packets::common::CCustomPayload;
use steel_protocol::packets::common::{SClientInformation, SCustomPayload};
use steel_protocol::packets::config::CFinishConfiguration;

use steel_protocol::packets::config::CSelectKnownPacks;
use steel_protocol::packets::config::SSelectKnownPacks;
use steel_protocol::packets::shared_implementation::KnownPack;
use steel_protocol::utils::ConnectionProtocol;

use steel_core::player::Player;
use steel_core::player::networking::JavaConnection;
use steel_utils::Identifier;

use crate::MC_VERSION;
use crate::network::JavaTcpClient;
use crate::network::java_tcp_client::ConnectionUpdate;

const BRAND_PAYLOAD: [u8; 5] = *b"Steel";

impl JavaTcpClient {
    /// Handles a custom payload packet during the configuration state.
    pub fn handle_config_custom_payload(&self, packet: SCustomPayload) {
        println!("Custom payload packet: {packet:?}");
    }

    /// Handles the client information packet during the configuration state.
    pub fn handle_client_information(&self, packet: SClientInformation) {
        println!("Client information packet: {packet:?}");
    }

    /// Starts the configuration process by sending initial packets.
    pub async fn start_configuration(&self) {
        self.send_bare_packet_now(CCustomPayload::new(
            Identifier::vanilla_static("brand"),
            Box::new(BRAND_PAYLOAD),
        ))
        .await;

        self.send_bare_packet_now(CSelectKnownPacks::new(vec![KnownPack::new(
            "minecraft".to_string(),
            "core".to_string(),
            MC_VERSION.to_string(),
        )]))
        .await;
    }

    /// Handles the select known packs packet during the configuration state.
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

    /// Finishes the configuration process and transitions to the play state.
    ///
    /// # Panics
    /// This function will panic if the game profile is empty, should be impossible at this point.
    pub async fn finish_configuration(&self) {
        self.protocol.store(ConnectionProtocol::Play);

        let gameprofile = self
            .gameprofile
            .lock()
            .await
            .clone()
            .expect("Game profile is empty");

        let world = self.server.worlds[0].clone();

        let player = Arc::new_cyclic(|player| {
            Player::new(
                gameprofile,
                JavaConnection::new(
                    self.outgoing_queue.clone(),
                    self.cancel_token.clone(),
                    self.compression.load(),
                    self.network_writer.clone(),
                    self.id,
                    player.clone(),
                )
                .into(),
                world,
            )
        });

        self.connection_updates
            .send(ConnectionUpdate::Upgrade(player.connection.clone()))
            .expect("Failed to send connection update");

        self.server.add_player(player);
    }
}
