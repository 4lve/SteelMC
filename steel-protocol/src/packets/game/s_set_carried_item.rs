//! Set carried item (hotbar selection) packet.

use std::io::{Read, Result};

use steel_macros::ServerPacket;
use steel_utils::serial::ReadFrom;

/// Client changes their selected hotbar slot.
#[derive(ServerPacket, Debug, Clone)]
pub struct SSetCarriedItem {
    /// The new selected slot (0-8).
    pub slot: i16,
}

impl ReadFrom for SSetCarriedItem {
    fn read(data: &mut impl Read) -> Result<Self> {
        let slot = i16::read(data)?;
        Ok(Self { slot })
    }
}

