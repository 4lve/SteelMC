use steel_macros::ClientPacket;
use steel_registry::packets::play::C_PLAYER_CHAT;
use steel_utils::{
    codec::{BitSet, VarInt},
    serial::PrefixedWrite,
    text::TextComponent,
};
use uuid::Uuid;

/// Represents Minecraft's ChatType.Bound structure
/// Contains a registry holder + sender name + optional target name
#[derive(Clone, Debug)]
pub struct ChatTypeBound {
    pub registry_id: VarInt,
    pub sender_name: TextComponent,
    pub target_name: Option<TextComponent>,
}

impl steel_utils::serial::WriteTo for ChatTypeBound {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        // Registry holder: ID + 1 for REFERENCE holders (ByteBufCodecs.holder pattern)
        VarInt(self.registry_id.0 + 1).write(writer)?;

        // Sender name as NBT Component
        let sender_encoded = self.sender_name.encode();
        writer.write_all(&sender_encoded)?;

        // Optional target name
        match &self.target_name {
            Some(name) => {
                true.write(writer)?;
                let target_encoded = name.encode();
                writer.write_all(&target_encoded)?;
            }
            None => false.write(writer)?,
        }

        Ok(())
    }
}

#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_PLAYER_CHAT)]
pub struct CPlayerChat {
    pub global_index: VarInt,
    pub sender: Uuid,
    pub index: VarInt,
    pub message_signature: Option<Box<[u8]>>,
    pub message: String,
    pub timestamp: i64,
    pub salt: i64,
    pub previous_messages: Box<[PreviousMessage]>,
    pub unsigned_content: Option<TextComponent>,
    pub filter_type: FilterType,
    pub chat_type: ChatTypeBound,
}

impl CPlayerChat {
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        global_index: VarInt,
        sender: Uuid,
        index: VarInt,
        message_signature: Option<Box<[u8]>>,
        message: String,
        timestamp: i64,
        salt: i64,
        previous_messages: Box<[PreviousMessage]>,
        unsigned_content: Option<TextComponent>,
        filter_type: FilterType,
        chat_type: ChatTypeBound,
    ) -> Self {
        Self {
            global_index,
            sender,
            index,
            message_signature,
            message,
            timestamp,
            salt,
            previous_messages,
            unsigned_content,
            filter_type,
            chat_type,
        }
    }
}

impl steel_utils::serial::WriteTo for CPlayerChat {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.global_index.write(writer)?;
        self.sender.write(writer)?;
        self.index.write(writer)?;

        match &self.message_signature {
            Some(sig) => {
                true.write(writer)?;
                writer.write_all(sig)?;
            }
            None => false.write(writer)?,
        }

        self.message.write_prefixed::<VarInt>(writer)?;
        self.timestamp.write(writer)?;
        self.salt.write(writer)?;

        VarInt(self.previous_messages.len() as i32).write(writer)?;
        for msg in self.previous_messages.iter() {
            msg.id.write(writer)?;
            if let Some(sig) = &msg.signature {
                writer.write_all(sig)?;
            }
        }

        match &self.unsigned_content {
            Some(content) => {
                true.write(writer)?;
                let encoded = content.encode();
                writer.write_all(&encoded)?;
            }
            None => false.write(writer)?,
        }

        VarInt(match self.filter_type {
            FilterType::PassThrough => 0,
            FilterType::FullyFiltered => 1,
            FilterType::PartiallyFiltered(_) => 2,
        })
        .write(writer)?;

        self.chat_type.write(writer)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PreviousMessage {
    pub id: VarInt,
    pub signature: Option<Box<[u8]>>,
}

#[derive(Clone, Debug)]
pub enum FilterType {
    PassThrough,
    FullyFiltered,
    PartiallyFiltered(BitSet),
}
