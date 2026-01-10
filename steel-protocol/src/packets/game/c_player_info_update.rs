use steel_macros::ClientPacket;
use steel_registry::packets::play::C_PLAYER_INFO_UPDATE;
use steel_utils::codec::VarInt;
use steel_utils::serial::PrefixedWrite;
use uuid::Uuid;

// Import RemoteChatSessionData for chat session transmission
use super::chat_session_data::ProtocolRemoteChatSessionData;
use crate::packets::login::GameProfileProperty;

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
    pub properties: Vec<GameProfileProperty>,
    pub chat_session: Option<ProtocolRemoteChatSessionData>,
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
            actions: PlayerInfoAction::AddPlayer as u8 | PlayerInfoAction::InitializeChat as u8,
            entries: vec![PlayerInfoEntry {
                uuid,
                name: Some(name),
                properties,
                chat_session: None,
                game_mode: None,
                listed: None,
                latency: None,
            }],
        }
    }

    pub fn update_chat_session(uuid: Uuid, chat_session: ProtocolRemoteChatSessionData) -> Self {
        Self {
            actions: PlayerInfoAction::InitializeChat as u8,
            entries: vec![PlayerInfoEntry {
                uuid,
                name: None,
                properties: Vec::new(),
                chat_session: Some(chat_session),
                game_mode: None,
                listed: None,
                latency: None,
            }],
        }
    }
}

impl steel_utils::serial::WriteTo for CPlayerInfoUpdate {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.actions.write(writer)?;
        VarInt(self.entries.len() as i32).write(writer)?;

        for entry in &self.entries {
            entry.uuid.write(writer)?;

            if self.actions & (PlayerInfoAction::AddPlayer as u8) != 0
                && let Some(ref name) = entry.name
            {
                name.write_prefixed::<VarInt>(writer)?;
                // Write properties (including skin textures)
                VarInt(entry.properties.len() as i32).write(writer)?;
                for prop in &entry.properties {
                    prop.write(writer)?;
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
