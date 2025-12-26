//! Container click packet (serverbound).

use std::io::{Read, Result, Write};

use steel_macros::ServerPacket;
use steel_utils::{
    codec::VarInt,
    serial::{ReadFrom, WriteTo},
};

use super::slot_data::SlotData;

/// The type of click action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ClickType {
    /// Normal left or right click.
    Pickup = 0,
    /// Shift-click.
    QuickMove = 1,
    /// Number key or offhand swap.
    Swap = 2,
    /// Middle-click clone (creative).
    Clone = 3,
    /// Q key throw.
    Throw = 4,
    /// Drag operation.
    QuickCraft = 5,
    /// Double-click collect.
    PickupAll = 6,
}

impl ReadFrom for ClickType {
    fn read(data: &mut impl Read) -> Result<Self> {
        let value = VarInt::read(data)?.0;
        Ok(match value {
            0 => Self::Pickup,
            1 => Self::QuickMove,
            2 => Self::Swap,
            3 => Self::Clone,
            4 => Self::Throw,
            5 => Self::QuickCraft,
            6 => Self::PickupAll,
            _ => Self::Pickup, // Default to pickup for unknown
        })
    }
}

impl WriteTo for ClickType {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        VarInt(*self as i32).write(writer)
    }
}

/// A slot change entry in a container click packet.
#[derive(Debug, Clone)]
pub struct SlotChange {
    /// The slot index.
    pub slot: i16,
    /// The new slot contents.
    pub data: SlotData,
}

impl ReadFrom for SlotChange {
    fn read(data: &mut impl Read) -> Result<Self> {
        let slot = i16::read(data)?;
        let slot_data = SlotData::read(data)?;
        Ok(Self {
            slot,
            data: slot_data,
        })
    }
}

impl WriteTo for SlotChange {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.slot.write(writer)?;
        self.data.write(writer)
    }
}

/// Client click on a container slot.
#[derive(ServerPacket, Debug, Clone)]
pub struct SContainerClick {
    /// The container ID.
    pub container_id: i8,
    /// State ID for synchronization.
    pub state_id: i32,
    /// The slot that was clicked.
    pub slot: i16,
    /// The mouse button used.
    pub button: i8,
    /// The type of click action.
    pub click_type: ClickType,
    /// Slots that changed as a result of this click.
    pub changed_slots: Vec<SlotChange>,
    /// The item now on the cursor.
    pub carried_item: SlotData,
}

impl ReadFrom for SContainerClick {
    fn read(data: &mut impl Read) -> Result<Self> {
        let container_id = i8::read(data)?;
        let state_id = VarInt::read(data)?.0;
        let slot = i16::read(data)?;
        let button = i8::read(data)?;
        let click_type = ClickType::read(data)?;

        let changed_count = VarInt::read(data)?.0 as usize;
        let mut changed_slots = Vec::with_capacity(changed_count.min(128));
        for _ in 0..changed_count.min(128) {
            changed_slots.push(SlotChange::read(data)?);
        }

        let carried_item = SlotData::read(data)?;

        Ok(Self {
            container_id,
            state_id,
            slot,
            button,
            click_type,
            changed_slots,
            carried_item,
        })
    }
}
