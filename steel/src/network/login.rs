use rsa::Pkcs1v15Encrypt;
use sha2::{Digest, Sha256};
use steel_protocol::packets::{
    clientbound::{CBoundLogin, CBoundPacket},
    login::{
        c_hello_packet::CHelloPacket, c_login_compression_packet::CLoginCompressionPacket,
        c_login_finished_packet::CLoginFinishedPacket, s_hello_packet::SHelloPacket,
        s_key_packet::SKeyPacket,
    },
};
use steel_utils::text::TextComponent;
use uuid::Uuid;

use crate::{
    STEEL_CONFIG,
    network::{game_profile::GameProfile, java_tcp_client::JavaTcpClient},
};

pub fn is_valid_player_name(name: &str) -> bool {
    name.len() >= 3
        && name.len() <= 16
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn offline_uuid(username: &str) -> Result<Uuid, uuid::Error> {
    Uuid::from_slice(&Sha256::digest(username)[..16])
}
pub async fn handle_hello(tcp_client: &JavaTcpClient, packet: &SHelloPacket) {
    println!("Hello packet: {:?}", packet);
    if !is_valid_player_name(&packet.name) {
        tcp_client
            .kick(TextComponent::text("Invalid player name"))
            .await;
    }

    let id = if STEEL_CONFIG.online_mode {
        packet.profile_id
    } else {
        offline_uuid(&packet.name).expect("This is very not safe and bad")
    };

    {
        let mut gameprofile = tcp_client.gameprofile.lock().await;
        *gameprofile = Some(GameProfile {
            id,
            name: packet.name.clone(),
            properties: vec![],
            profile_actions: None,
        });
    }

    if let Some(compression) = STEEL_CONFIG.compression {
        tcp_client
            .send_packet_now(CBoundPacket::Login(CBoundLogin::LoginCompression(
                CLoginCompressionPacket::new(compression.threshold as i32),
            )))
            .await;
        tcp_client.set_compression(compression).await;
        println!("Set compression to: {:?}", compression);
    }

    if STEEL_CONFIG.encryption {
        let challenge: [u8; 4] = rand::random();
        tcp_client.challenge.store(Some(challenge));

        tcp_client
            .send_packet_now(CBoundPacket::Login(CBoundLogin::Hello(CHelloPacket::new(
                "".to_string(),
                tcp_client.server.key_store.public_key_der.to_vec(),
                challenge.to_vec(),
                true,
            ))))
            .await;
    } else {
        tcp_client
            .send_packet_now(CBoundPacket::Login(CBoundLogin::LoginFinished(
                CLoginFinishedPacket::new(id, packet.name.clone(), vec![]),
            )))
            .await;
    }
}

pub async fn handle_key(tcp_client: &JavaTcpClient, packet: &SKeyPacket) {
    println!("Key packet: {:?}", packet);
    let challenge = tcp_client.challenge.load();
    if challenge.is_none() {
        tcp_client
            .kick(TextComponent::text("No challenge found"))
            .await;
    }
    let challenge = challenge.unwrap();
    let challenge = challenge.to_vec();

    let Ok(challenge_response) = tcp_client
        .server
        .key_store
        .private_key
        .decrypt(Pkcs1v15Encrypt, &packet.key)
    else {
        tcp_client.kick(TextComponent::text("Invalid key")).await;
        return;
    };

    if challenge_response != challenge {
        tcp_client
            .kick(TextComponent::text("Invalid challenge response"))
            .await;
        return;
    }
}
