use num_bigint::BigInt;
use rsa::Pkcs1v15Encrypt;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use steel_protocol::{
    packets::login::{
        c_hello::CHelloPacket, c_login_compression::CLoginCompressionPacket,
        c_login_finished::CLoginFinishedPacket, s_hello::SHelloPacket, s_key::SKeyPacket,
        s_login_acknowledged::SLoginAcknowledgedPacket,
    },
    utils::ConnectionProtocol,
};
use steel_utils::text::TextComponent;
use steel_world::player::GameProfile;
use uuid::Uuid;

use crate::{
    STEEL_CONFIG,
    network::{
        config,
        java_tcp_client::{ConnectionUpdate, JavaTcpClient},
        mojang_authentication::{AuthError, mojang_authenticate},
    },
};

pub fn is_valid_player_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn offline_uuid(username: &str) -> Result<Uuid, uuid::Error> {
    Uuid::from_slice(&Sha256::digest(username)[..16])
}
pub async fn handle_hello(tcp_client: &JavaTcpClient, packet: &SHelloPacket) {
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

    if STEEL_CONFIG.encryption {
        let challenge: [u8; 4] = rand::random();
        tcp_client.challenge.store(Some(challenge));

        tcp_client
            .send_packet_now(CHelloPacket::new(
                "".to_string(),
                tcp_client.server.key_store.public_key_der.clone(),
                challenge,
                true,
            ))
            .await;
    } else {
        finish_login(
            tcp_client,
            &GameProfile {
                id,
                name: packet.name.clone(),
                properties: vec![],
                profile_actions: None,
            },
        )
        .await;
    }
}

pub async fn handle_key(tcp_client: &JavaTcpClient, packet: &SKeyPacket) {
    let challenge = tcp_client.challenge.load();
    if challenge.is_none() {
        tcp_client
            .kick(TextComponent::text("No challenge found"))
            .await;
    }
    let challenge = challenge.unwrap();

    let Ok(challenge_response) = tcp_client
        .server
        .key_store
        .private_key
        .decrypt(Pkcs1v15Encrypt, &packet.challenge)
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

    let Ok(secret_key) = tcp_client
        .server
        .key_store
        .private_key
        .decrypt(Pkcs1v15Encrypt, &packet.key)
    else {
        tcp_client.kick(TextComponent::text("Invalid key")).await;
        return;
    };

    let secret_key: [u8; 16] = match secret_key.try_into() {
        Ok(secret_key) => secret_key,
        Err(_) => {
            tcp_client.kick(TextComponent::text("Invalid key")).await;
            return;
        }
    };

    let Ok(_) = tcp_client
        .connection_updates
        .send(ConnectionUpdate::EnableEncryption(secret_key))
    else {
        tcp_client
            .kick(TextComponent::text("Failed to send connection update"))
            .await;
        return;
    };

    tcp_client.connection_updated.notified().await;

    let mut gameprofile = tcp_client.gameprofile.lock().await;

    let Some(profile) = gameprofile.as_mut() else {
        tcp_client.kick(TextComponent::text("No GameProfile")).await;
        return;
    };

    if STEEL_CONFIG.online_mode {
        let server_hash = &Sha1::new()
            .chain_update(secret_key)
            .chain_update(&tcp_client.server.key_store.public_key_der)
            .finalize();

        let server_hash = BigInt::from_signed_bytes_be(server_hash).to_str_radix(16);

        match mojang_authenticate(&profile.name, &server_hash).await {
            Ok(new_profile) => *profile = new_profile,
            Err(error) => {
                tcp_client
                    .kick(match error {
                        AuthError::FailedResponse => {
                            TextComponent::translate("multiplayer.disconnect.authservers_down", [])
                        }
                        AuthError::UnverifiedUsername => TextComponent::translate(
                            "multiplayer.disconnect.unverified_username",
                            [],
                        ),
                        e => TextComponent::text(e.to_string()),
                    })
                    .await;
            }
        }
    }

    //TODO: Check for duplicate player UUID or name

    finish_login(tcp_client, profile).await;
}

pub async fn finish_login(tcp_client: &JavaTcpClient, profile: &GameProfile) {
    if let Some(compression) = STEEL_CONFIG.compression {
        tcp_client
            .send_packet_now(CLoginCompressionPacket::new(compression.threshold as i32))
            .await;
        tcp_client.compression_info.store(Some(compression));
        tcp_client
            .connection_updates
            .send(ConnectionUpdate::EnableCompression(compression))
            .unwrap();
    }
    tcp_client.can_process_next_packet.notify_waiters();
    tcp_client.connection_updated.notified().await;

    tcp_client
        .send_packet_now(CLoginFinishedPacket::new(
            profile.id,
            profile.name.clone(),
            profile.properties.clone(),
        ))
        .await;
}

pub async fn handle_login_acknowledged(
    tcp_client: &JavaTcpClient,
    _packet: &SLoginAcknowledgedPacket,
) {
    tcp_client
        .connection_protocol
        .store(ConnectionProtocol::CONFIGURATION);

    config::start_configuration(tcp_client).await;
}
