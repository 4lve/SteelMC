//! Equipment slot types for entities.
//!
//! This module defines the different equipment slots that entities can have.

/// The type of equipment slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlotType {
    /// Hand slots (mainhand, offhand).
    Hand,
    /// Humanoid armor slots (head, chest, legs, feet).
    HumanoidArmor,
    /// Animal armor slot (body).
    AnimalArmor,
    /// Saddle slot.
    Saddle,
}

/// Equipment slots for entities.
///
/// Each slot has a type, an index within that type, and a unique ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    /// Main hand slot.
    Mainhand,
    /// Off hand slot.
    Offhand,
    /// Feet armor slot (boots).
    Feet,
    /// Legs armor slot (leggings).
    Legs,
    /// Chest armor slot (chestplate).
    Chest,
    /// Head armor slot (helmet).
    Head,
    /// Body armor slot (for animals like horses).
    Body,
    /// Saddle slot.
    Saddle,
}

impl EquipmentSlot {
    /// All equipment slot values.
    pub const VALUES: [EquipmentSlot; 8] = [
        EquipmentSlot::Mainhand,
        EquipmentSlot::Offhand,
        EquipmentSlot::Feet,
        EquipmentSlot::Legs,
        EquipmentSlot::Chest,
        EquipmentSlot::Head,
        EquipmentSlot::Body,
        EquipmentSlot::Saddle,
    ];

    /// Returns the slot type.
    #[must_use]
    pub const fn slot_type(self) -> EquipmentSlotType {
        match self {
            EquipmentSlot::Mainhand | EquipmentSlot::Offhand => EquipmentSlotType::Hand,
            EquipmentSlot::Feet
            | EquipmentSlot::Legs
            | EquipmentSlot::Chest
            | EquipmentSlot::Head => EquipmentSlotType::HumanoidArmor,
            EquipmentSlot::Body => EquipmentSlotType::AnimalArmor,
            EquipmentSlot::Saddle => EquipmentSlotType::Saddle,
        }
    }

    /// Returns the index within the slot type.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            EquipmentSlot::Mainhand => 0,
            EquipmentSlot::Offhand => 1,
            EquipmentSlot::Feet => 0,
            EquipmentSlot::Legs => 1,
            EquipmentSlot::Chest => 2,
            EquipmentSlot::Head => 3,
            EquipmentSlot::Body => 0,
            EquipmentSlot::Saddle => 0,
        }
    }

    /// Returns the index with a base offset (for inventory slot mapping).
    ///
    /// In vanilla, armor slots are at base 36:
    /// - Feet: 36
    /// - Legs: 37
    /// - Chest: 38
    /// - Head: 39
    #[must_use]
    pub const fn index_with_base(self, base: usize) -> usize {
        base + self.index()
    }

    /// Returns the unique ID for this slot.
    #[must_use]
    pub const fn id(self) -> usize {
        match self {
            EquipmentSlot::Mainhand => 0,
            EquipmentSlot::Offhand => 5,
            EquipmentSlot::Feet => 1,
            EquipmentSlot::Legs => 2,
            EquipmentSlot::Chest => 3,
            EquipmentSlot::Head => 4,
            EquipmentSlot::Body => 6,
            EquipmentSlot::Saddle => 7,
        }
    }

    /// Returns the count limit for this slot (0 = no limit).
    ///
    /// Most armor slots limit to 1 item.
    #[must_use]
    pub const fn count_limit(self) -> i32 {
        match self {
            EquipmentSlot::Mainhand | EquipmentSlot::Offhand => 0, // No limit
            _ => 1,                                                // Armor slots limit to 1
        }
    }

    /// Returns the slot name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            EquipmentSlot::Mainhand => "mainhand",
            EquipmentSlot::Offhand => "offhand",
            EquipmentSlot::Feet => "feet",
            EquipmentSlot::Legs => "legs",
            EquipmentSlot::Chest => "chest",
            EquipmentSlot::Head => "head",
            EquipmentSlot::Body => "body",
            EquipmentSlot::Saddle => "saddle",
        }
    }

    /// Returns whether this slot is an armor slot.
    #[must_use]
    pub const fn is_armor(self) -> bool {
        matches!(
            self.slot_type(),
            EquipmentSlotType::HumanoidArmor | EquipmentSlotType::AnimalArmor
        )
    }

    /// Parses an equipment slot from its name.
    #[must_use]
    pub fn by_name(name: &str) -> Option<EquipmentSlot> {
        match name {
            "mainhand" => Some(EquipmentSlot::Mainhand),
            "offhand" => Some(EquipmentSlot::Offhand),
            "feet" => Some(EquipmentSlot::Feet),
            "legs" => Some(EquipmentSlot::Legs),
            "chest" => Some(EquipmentSlot::Chest),
            "head" => Some(EquipmentSlot::Head),
            "body" => Some(EquipmentSlot::Body),
            "saddle" => Some(EquipmentSlot::Saddle),
            _ => None,
        }
    }

    /// Creates an equipment slot from its ID.
    #[must_use]
    pub const fn from_id(id: usize) -> Option<EquipmentSlot> {
        match id {
            0 => Some(EquipmentSlot::Mainhand),
            1 => Some(EquipmentSlot::Feet),
            2 => Some(EquipmentSlot::Legs),
            3 => Some(EquipmentSlot::Chest),
            4 => Some(EquipmentSlot::Head),
            5 => Some(EquipmentSlot::Offhand),
            6 => Some(EquipmentSlot::Body),
            7 => Some(EquipmentSlot::Saddle),
            _ => None,
        }
    }
}
