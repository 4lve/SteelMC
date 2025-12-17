use steel_macros::ClientPacket;
use steel_registry::packets::play::C_PLAYER_INFO_UPDATE;
use steel_utils::codec::VarInt;
use steel_utils::serial::{PrefixedWrite, WriteTo};
use uuid::Uuid;

use crate::packets::login::GameProfileProperty;

// Import RemoteChatSessionData for chat session transmission
use super::chat_session_data::RemoteChatSessionData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerInfoAction {
    AddPlayer = 0x01,
    InitializeChat = 0x02,
    UpdateGameMode = 0x04,
    UpdateListed = 0x08,
    UpdateLatency = 0x10,
}

#[derive(Debug, Clone)]
pub struct PlayerInfoEntry {
    pub uuid: Uuid,
    pub name: Option<String>,
    pub properties: Option<Vec<GameProfileProperty>>,
    pub chat_session: Option<RemoteChatSessionData>,
    pub game_mode: Option<VarInt>,
    pub listed: Option<bool>,
    pub latency: Option<VarInt>,
}

#[derive(ClientPacket, Debug, Clone)]
#[packet_id(Play = C_PLAYER_INFO_UPDATE)]
pub struct CPlayerInfoUpdate {
    pub actions: u8, // Bitmask of PlayerInfoAction
    pub entries: Vec<PlayerInfoEntry>,
}

impl CPlayerInfoUpdate {
    pub fn add_player(uuid: Uuid, name: String, properties: Vec<GameProfileProperty>) -> Self {
        Self {
            actions: PlayerInfoAction::AddPlayer as u8
                | PlayerInfoAction::InitializeChat as u8
                | PlayerInfoAction::UpdateGameMode as u8
                | PlayerInfoAction::UpdateListed as u8
                | PlayerInfoAction::UpdateLatency as u8,
            entries: vec![PlayerInfoEntry {
                uuid,
                name: Some(name),
                properties: Some(properties),
                chat_session: None,
                game_mode: Some(VarInt(1)), // Creative mode
                listed: Some(true),         // Show in tab list
                latency: Some(VarInt(0)),   // 0ms latency
            }],
        }
    }

    pub fn update_chat_session(uuid: Uuid, chat_session: RemoteChatSessionData) -> Self {
        Self {
            actions: PlayerInfoAction::InitializeChat as u8,
            entries: vec![PlayerInfoEntry {
                uuid,
                name: None,
                properties: None,
                chat_session: Some(chat_session),
                game_mode: None,
                listed: None,
                latency: None,
            }],
        }
    }
}

impl WriteTo for CPlayerInfoUpdate {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.actions.write(writer)?;
        VarInt(self.entries.len() as i32).write(writer)?;

        for entry in &self.entries {
            entry.uuid.write(writer)?;

            if self.actions & (PlayerInfoAction::AddPlayer as u8) != 0 {
                if let Some(ref name) = entry.name {
                    name.write_prefixed::<VarInt>(writer)?;
                }
                if let Some(ref properties) = entry.properties {
                    VarInt(properties.len() as i32).write(writer)?;
                    for prop in properties {
                        prop.name.write_prefixed::<VarInt>(writer)?;
                        prop.value.write_prefixed::<VarInt>(writer)?;
                        if let Some(ref signature) = prop.signature {
                            true.write(writer)?;
                            signature.write_prefixed::<VarInt>(writer)?;
                        } else {
                            false.write(writer)?;
                        }
                    }
                } else {
                    VarInt(0).write(writer)?;
                }
            }

            if self.actions & (PlayerInfoAction::InitializeChat as u8) != 0 {
                // Write nullable chat session data
                if let Some(ref session_data) = entry.chat_session {
                    true.write(writer)?;
                    session_data.write(writer)?;
                } else {
                    false.write(writer)?;
                }
            }

            if self.actions & (PlayerInfoAction::UpdateGameMode as u8) != 0
                && let Some(game_mode) = entry.game_mode
            {
                game_mode.write(writer)?;
            }

            if self.actions & (PlayerInfoAction::UpdateListed as u8) != 0
                && let Some(listed) = entry.listed
            {
                listed.write(writer)?;
            }

            if self.actions & (PlayerInfoAction::UpdateLatency as u8) != 0
                && let Some(latency) = entry.latency
            {
                latency.write(writer)?;
            }
        }

        Ok(())
    }
}
