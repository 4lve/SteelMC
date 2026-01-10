use std::time::{SystemTime, UNIX_EPOCH};

use steel_utils::codec::VarInt;
use steel_utils::serial::WriteTo;
use uuid::Uuid;

/// Network-serializable chat session data.
///
/// This is a simplified version that holds raw byte data for transmission.
/// The full version with validated keys lives in steel-core.
#[derive(Clone, Debug)]
pub struct ProtocolRemoteChatSessionData {
    /// The session ID
    pub session_id: Uuid,
    /// When the key expires (as milliseconds since UNIX epoch)
    pub expires_at_millis: i64,
    /// The public key bytes
    pub public_key_bytes: Vec<u8>,
    /// The key signature bytes
    pub key_signature: Vec<u8>,
}

impl ProtocolRemoteChatSessionData {
    /// Creates new chat session data from raw components
    #[must_use]
    pub fn new(
        session_id: Uuid,
        expires_at: SystemTime,
        public_key_bytes: Vec<u8>,
        key_signature: Vec<u8>,
    ) -> Self {
        let expires_at_millis = expires_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        Self {
            session_id,
            expires_at_millis,
            public_key_bytes,
            key_signature,
        }
    }
}

impl WriteTo for ProtocolRemoteChatSessionData {
    fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.session_id.write(writer)?;

        // Write expires_at (i64 millis)
        self.expires_at_millis.write(writer)?;

        // Write public key (length as VarInt, then bytes)
        VarInt(self.public_key_bytes.len() as i32).write(writer)?;
        writer.write_all(&self.public_key_bytes)?;

        // Write key signature (length as VarInt, then bytes)
        VarInt(self.key_signature.len() as i32).write(writer)?;
        writer.write_all(&self.key_signature)?;

        Ok(())
    }
}
