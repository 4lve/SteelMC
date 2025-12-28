//! Click type definitions for container interactions.

/// The type of click action performed on a container slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ClickType {
    /// Normal left or right click to pick up or place items.
    Pickup = 0,
    /// Shift-click to quickly move items between container sections.
    QuickMove = 1,
    /// Number key (1-9) or offhand key (F) to swap with hotbar/offhand.
    Swap = 2,
    /// Middle-click in creative mode to clone the full stack.
    Clone = 3,
    /// Q key to throw items out of the inventory.
    Throw = 4,
    /// Drag across multiple slots to distribute items.
    QuickCraft = 5,
    /// Double-click to collect all matching items to cursor.
    PickupAll = 6,
}

impl ClickType {
    /// Converts a byte value to a ClickType.
    #[must_use]
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Pickup),
            1 => Some(Self::QuickMove),
            2 => Some(Self::Swap),
            3 => Some(Self::Clone),
            4 => Some(Self::Throw),
            5 => Some(Self::QuickCraft),
            6 => Some(Self::PickupAll),
            _ => None,
        }
    }
}

/// The mouse button or action used in a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAction {
    /// Left mouse button or primary action.
    Primary,
    /// Right mouse button or secondary action.
    Secondary,
}

impl ClickAction {
    /// Converts a button number to a ClickAction.
    #[must_use]
    pub fn from_button(button: i8) -> Self {
        if button == 0 {
            Self::Primary
        } else {
            Self::Secondary
        }
    }
}

/// Quick-craft (drag) state machine phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCraftPhase {
    /// Starting a quick-craft operation.
    Start,
    /// Adding a slot to the quick-craft operation.
    Continue,
    /// Finishing the quick-craft operation.
    End,
}

impl QuickCraftPhase {
    /// Extracts the phase from the button mask.
    #[must_use]
    pub fn from_header(header: i32) -> Option<Self> {
        match header & 3 {
            0 => Some(Self::Start),
            1 => Some(Self::Continue),
            2 => Some(Self::End),
            _ => None,
        }
    }
}

/// Quick-craft distribution type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCraftType {
    /// Distribute items evenly across slots (left-click drag).
    Charitable,
    /// Place one item per slot (right-click drag).
    Greedy,
    /// Clone full stacks in creative mode (middle-click drag).
    Clone,
}

impl QuickCraftType {
    /// Extracts the type from the button mask.
    #[must_use]
    pub fn from_header(header: i32) -> Option<Self> {
        match (header >> 2) & 3 {
            0 => Some(Self::Charitable),
            1 => Some(Self::Greedy),
            2 => Some(Self::Clone),
            _ => None,
        }
    }

    /// Returns true if this quick-craft type is valid for the player.
    #[must_use]
    pub fn is_valid_for_player(&self, has_infinite_materials: bool) -> bool {
        match self {
            Self::Charitable | Self::Greedy => true,
            Self::Clone => has_infinite_materials,
        }
    }
}

/// Constructs a quick-craft button mask from phase and type.
#[must_use]
pub fn make_quick_craft_mask(phase: QuickCraftPhase, craft_type: QuickCraftType) -> i32 {
    let phase_bits = match phase {
        QuickCraftPhase::Start => 0,
        QuickCraftPhase::Continue => 1,
        QuickCraftPhase::End => 2,
    };
    let type_bits = match craft_type {
        QuickCraftType::Charitable => 0,
        QuickCraftType::Greedy => 1,
        QuickCraftType::Clone => 2,
    };
    (phase_bits & 3) | ((type_bits & 3) << 2)
}
