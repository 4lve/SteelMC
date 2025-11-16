use num_bigint::BigInt;
use rsa::Pkcs1v15Encrypt;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use steel_core::player::GameProfile;
use steel_protocol::{
    packets::login::{CHello, CLoginCompression, CLoginFinished, SHello, SKey},
    utils::ConnectionProtocol,
};
use steel_utils::{text::TextComponent, translations};
use uuid::Uuid;

use crate::{
    STEEL_CONFIG,
    network::{
        java_tcp_client::{ConnectionUpdate, JavaTcpClient},
        mojang_authentication::{AuthError, mojang_authenticate},
    },
};

/// Checks if a player name is valid.
#[must_use]
pub fn is_valid_player_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Generates an offline mode UUID for a player.
pub fn offline_uuid(username: &str) -> Result<Uuid, uuid::Error> {
    Uuid::from_slice(&Sha256::digest(username)[..16])
}

impl JavaTcpClient {
    /// Handles the hello packet during the login state.
    ///
    /// # Panics
    /// This function will panic if the player name converted to a UUID fails.
    pub async fn handle_hello(&self, packet: SHello) {
        if !is_valid_player_name(&packet.name) {
            self.kick(TextComponent::new().text("Invalid player name"))
                .await;
        }

        let id = if STEEL_CONFIG.online_mode {
            packet.profile_id
        } else {
            offline_uuid(&packet.name).expect("This is very not safe and bad")
        };

        {
            let mut gameprofile = self.gameprofile.lock().await;
            *gameprofile = Some(GameProfile {
                id,
                name: packet.name.clone(),
                properties: vec![],
                profile_actions: None,
            });
        }

        if STEEL_CONFIG.encryption {
            let challenge: [u8; 4] = rand::random();
            self.challenge.store(challenge);

            self.send_bare_packet_now(CHello::new(
                String::new(),
                &self.server.key_store.public_key_der,
                challenge,
                true,
            ))
            .await;
        } else {
            self.finish_login(&GameProfile {
                id,
                name: packet.name,
                properties: vec![],
                profile_actions: None,
            })
            .await;
        }
    }

    /// Handles the key packet during the login state, used for encryption.
    pub async fn handle_key(&self, packet: SKey) {
        let challenge = self.challenge.load();

        let Ok(challenge_response) = self
            .server
            .key_store
            .private_key
            .decrypt(Pkcs1v15Encrypt, &packet.challenge)
        else {
            self.kick(TextComponent::new().text("Invalid key")).await;
            return;
        };

        if challenge_response != challenge {
            self.kick(TextComponent::new().text("Invalid challenge response"))
                .await;
            return;
        }

        let Ok(secret_key) = self
            .server
            .key_store
            .private_key
            .decrypt(Pkcs1v15Encrypt, &packet.key)
        else {
            self.kick(TextComponent::new().text("Invalid key")).await;
            return;
        };

        let secret_key: [u8; 16] = if let Ok(secret_key) = secret_key.try_into() {
            secret_key
        } else {
            self.kick(TextComponent::new().text("Invalid key")).await;
            return;
        };

        let Ok(_) = self
            .connection_updates
            .send(ConnectionUpdate::EnableEncryption(secret_key))
        else {
            self.kick(TextComponent::new().text("Failed to send connection update"))
                .await;
            return;
        };

        self.connection_updated.notified().await;

        let mut gameprofile = self.gameprofile.lock().await;

        let Some(profile) = gameprofile.as_mut() else {
            self.kick(TextComponent::new().text("No GameProfile")).await;
            return;
        };

        if STEEL_CONFIG.online_mode {
            let server_hash = &Sha1::new()
                .chain_update(secret_key)
                .chain_update(&self.server.key_store.public_key_der)
                .finalize();

            // TODO! Remove BigInt & make our own util for that
            let server_hash = BigInt::from_signed_bytes_be(server_hash).to_str_radix(16);

            match mojang_authenticate(&profile.name, &server_hash).await {
                Ok(new_profile) => *profile = new_profile,
                Err(error) => {
                    self.kick(match error {
                        AuthError::FailedResponse => TextComponent::new()
                            .translate(translations::MULTIPLAYER_DISCONNECT_AUTHSERVERS_DOWN.msg()),
                        AuthError::UnverifiedUsername => TextComponent::new().translate(
                            translations::MULTIPLAYER_DISCONNECT_UNVERIFIED_USERNAME.msg(),
                        ),
                        e => TextComponent::new().text(e.to_string()),
                    })
                    .await;
                }
            }
        }

        //TODO: Check for duplicate player UUID or name

        self.finish_login(profile).await;
    }

    /// Finishes the login process and transitions to the configuration state.
    ///
    /// # Panics
    /// This function will panic if the compression threshold cannot be converted to an i32. Should never happen.
    pub async fn finish_login(&self, profile: &GameProfile) {
        if let Some(compression) = STEEL_CONFIG.compression {
            self.send_bare_packet_now(CLoginCompression::new(
                compression
                    .threshold
                    .get()
                    .try_into()
                    .expect("Failed to convert compression threshold to i32"),
            ))
            .await;
            self.compression.store(Some(compression));
            self.connection_updates
                .send(ConnectionUpdate::EnableCompression(compression))
                .expect("Failed to send connection update");
        }

        //TODO: Here compression isn't awaited, if this becomes a problem in the future look here.

        self.send_bare_packet_now(CLoginFinished::new(
            profile.id,
            &profile.name,
            &profile.properties,
        ))
        .await;
    }

    /// Handles the login acknowledged packet and transitions to the configuration state.
    pub async fn handle_login_acknowledged(&self) {
        self.protocol.store(ConnectionProtocol::Config);

        self.start_configuration().await;
    }
}
