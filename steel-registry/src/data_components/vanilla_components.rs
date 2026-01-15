use std::str::FromStr;

use steel_utils::{BlockStateId, Identifier, types::Todo};

use crate::data_components::{DataComponentRegistry, DataComponentType};

/// Equipment slot for the equippable component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquippableSlot {
    Head,
    Chest,
    Legs,
    Feet,
    Body,
    Mainhand,
    Offhand,
    Saddle,
}

impl EquippableSlot {
    /// Parses an equipment slot from a string (as used in items.json).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "head" => Some(Self::Head),
            "chest" => Some(Self::Chest),
            "legs" => Some(Self::Legs),
            "feet" => Some(Self::Feet),
            "body" => Some(Self::Body),
            "mainhand" => Some(Self::Mainhand),
            "offhand" => Some(Self::Offhand),
            "saddle" => Some(Self::Saddle),
            _ => None,
        }
    }

    /// Returns the string representation of this slot.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Chest => "chest",
            Self::Legs => "legs",
            Self::Feet => "feet",
            Self::Body => "body",
            Self::Mainhand => "mainhand",
            Self::Offhand => "offhand",
            Self::Saddle => "saddle",
        }
    }

    /// Returns true if this is a humanoid armor slot.
    #[must_use]
    pub const fn is_humanoid_armor(&self) -> bool {
        matches!(self, Self::Head | Self::Chest | Self::Legs | Self::Feet)
    }
}

/// The equippable component data.
#[derive(Debug, Clone, PartialEq)]
pub struct Equippable {
    pub slot: EquippableSlot,
}

/// A single rule within a Tool component.
/// Rules are evaluated in order; the first matching rule determines the speed/drop behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolRule {
    /// The blocks this rule applies to (can be a tag like "#minecraft:mineable/pickaxe",
    /// a single block like "minecraft:cobweb", or a list of blocks).
    pub blocks: Vec<Identifier>,
    /// The mining speed for these blocks. If None, uses the tool's default_mining_speed.
    pub speed: Option<f32>,
    /// Whether the tool is "correct" for dropping items from these blocks.
    /// If None, falls back to the block's requiresCorrectToolForDrops property.
    pub correct_for_drops: Option<bool>,
}

impl ToolRule {
    /// Creates a rule that sets both mining speed and marks the tool as correct for drops.
    #[must_use]
    pub fn mines_and_drops(blocks: Vec<Identifier>, speed: f32) -> Self {
        Self {
            blocks,
            speed: Some(speed),
            correct_for_drops: Some(true),
        }
    }

    /// Creates a rule that explicitly denies drops (e.g., incorrect tool tier).
    #[must_use]
    pub fn denies_drops(blocks: Vec<Identifier>) -> Self {
        Self {
            blocks,
            speed: None,
            correct_for_drops: Some(false),
        }
    }

    /// Creates a rule that only overrides the mining speed.
    #[must_use]
    pub fn override_speed(blocks: Vec<Identifier>, speed: f32) -> Self {
        Self {
            blocks,
            speed: Some(speed),
            correct_for_drops: None,
        }
    }
}

/// The tool component data - defines mining speed and drop behavior for blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    /// Rules evaluated in order to determine mining speed and drop behavior.
    pub rules: Vec<ToolRule>,
    /// Default mining speed when no rule matches.
    pub default_mining_speed: f32,
    /// Damage to apply to the item per block mined.
    pub damage_per_block: i32,
    /// Whether the tool can destroy blocks in creative mode.
    pub can_destroy_blocks_in_creative: bool,
}

impl Default for Tool {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_mining_speed: 1.0,
            damage_per_block: 1,
            can_destroy_blocks_in_creative: true,
        }
    }
}

impl Tool {
    /// Returns the mining speed for a block state.
    /// Evaluates rules in order; returns the first matching rule's speed,
    /// or `default_mining_speed` if no rule matches.
    #[must_use]
    pub fn get_mining_speed(&self, block_state_id: BlockStateId) -> f32 {
        for rule in &self.rules {
            if let Some(speed) = rule.speed
                && rule.matches_block(block_state_id)
            {
                return speed;
            }
        }
        self.default_mining_speed
    }

    /// Returns true if this tool is "correct" for getting drops from the block.
    /// Evaluates rules in order; returns the first matching rule's `correct_for_drops`,
    /// or `false` if no rule explicitly matches.
    #[must_use]
    pub fn is_correct_for_drops(&self, block_state_id: BlockStateId) -> bool {
        for rule in &self.rules {
            if let Some(correct) = rule.correct_for_drops
                && rule.matches_block(block_state_id)
            {
                return correct;
            }
        }
        false
    }
}

impl ToolRule {
    /// Checks if this rule matches a block state.
    /// Handles both direct block identifiers and block tags (prefixed with #).
    #[must_use]
    pub fn matches_block(&self, block_state_id: BlockStateId) -> bool {
        use crate::REGISTRY;

        let Some(block) = REGISTRY.blocks.by_state_id(block_state_id) else {
            return false;
        };

        for block_id in &self.blocks {
            let id_str = format!("{}:{}", block_id.namespace, block_id.path);

            // Check if it's a tag reference (starts with #)
            if let Some(tag_str) = id_str.strip_prefix('#') {
                if let Ok(tag_id) = Identifier::from_str(tag_str)
                    && REGISTRY.blocks.is_in_tag(block, &tag_id)
                {
                    return true;
                }
            } else {
                // Direct block match
                if block.key == *block_id {
                    return true;
                }
            }
        }

        false
    }
}

// Basic data components
pub const CUSTOM_DATA: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("custom_data"));

pub const MAX_STACK_SIZE: DataComponentType<i32> =
    DataComponentType::new(Identifier::vanilla_static("max_stack_size"));

pub const MAX_DAMAGE: DataComponentType<i32> =
    DataComponentType::new(Identifier::vanilla_static("max_damage"));

pub const DAMAGE: DataComponentType<i32> =
    DataComponentType::new(Identifier::vanilla_static("damage"));

pub const UNBREAKABLE: DataComponentType<()> =
    DataComponentType::new(Identifier::vanilla_static("unbreakable"));

pub const CUSTOM_NAME: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("custom_name"));

pub const ITEM_NAME: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("item_name"));

pub const ITEM_MODEL: DataComponentType<Identifier> =
    DataComponentType::new(Identifier::vanilla_static("item_model"));

pub const LORE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("lore"));

pub const RARITY: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("rarity"));

pub const ENCHANTMENTS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("enchantments"));

pub const CAN_PLACE_ON: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("can_place_on"));

pub const CAN_BREAK: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("can_break"));

pub const ATTRIBUTE_MODIFIERS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("attribute_modifiers"));

pub const CUSTOM_MODEL_DATA: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("custom_model_data"));

pub const TOOLTIP_DISPLAY: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("tooltip_display"));

pub const REPAIR_COST: DataComponentType<i32> =
    DataComponentType::new(Identifier::vanilla_static("repair_cost"));

pub const CREATIVE_SLOT_LOCK: DataComponentType<()> =
    DataComponentType::new(Identifier::vanilla_static("creative_slot_lock"));

pub const ENCHANTMENT_GLINT_OVERRIDE: DataComponentType<bool> =
    DataComponentType::new(Identifier::vanilla_static("enchantment_glint_override"));

pub const INTANGIBLE_PROJECTILE: DataComponentType<()> =
    DataComponentType::new(Identifier::vanilla_static("intangible_projectile"));

pub const FOOD: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("food"));

pub const CONSUMABLE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("consumable"));

pub const USE_REMAINDER: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("use_remainder"));

pub const USE_COOLDOWN: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("use_cooldown"));

pub const DAMAGE_RESISTANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("damage_resistant"));

pub const TOOL: DataComponentType<Tool> =
    DataComponentType::new(Identifier::vanilla_static("tool"));

pub const WEAPON: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("weapon"));

pub const ENCHANTABLE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("enchantable"));

pub const EQUIPPABLE: DataComponentType<Equippable> =
    DataComponentType::new(Identifier::vanilla_static("equippable"));

pub const REPAIRABLE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("repairable"));

pub const GLIDER: DataComponentType<()> =
    DataComponentType::new(Identifier::vanilla_static("glider"));

pub const TOOLTIP_STYLE: DataComponentType<Identifier> =
    DataComponentType::new(Identifier::vanilla_static("tooltip_style"));

pub const DEATH_PROTECTION: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("death_protection"));

pub const BLOCKS_ATTACKS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("blocks_attacks"));

pub const STORED_ENCHANTMENTS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("stored_enchantments"));

pub const DYED_COLOR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("dyed_color"));

pub const MAP_COLOR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("map_color"));

pub const MAP_ID: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("map_id"));

pub const MAP_DECORATIONS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("map_decorations"));

pub const MAP_POST_PROCESSING: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("map_post_processing"));

pub const CHARGED_PROJECTILES: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("charged_projectiles"));

pub const BUNDLE_CONTENTS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("bundle_contents"));

pub const POTION_CONTENTS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("potion_contents"));

pub const POTION_DURATION_SCALE: DataComponentType<f32> =
    DataComponentType::new(Identifier::vanilla_static("potion_duration_scale"));

pub const SUSPICIOUS_STEW_EFFECTS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("suspicious_stew_effects"));

pub const WRITABLE_BOOK_CONTENT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("writable_book_content"));

pub const WRITTEN_BOOK_CONTENT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("written_book_content"));

pub const TRIM: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("trim"));

pub const DEBUG_STICK_STATE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("debug_stick_state"));

pub const ENTITY_DATA: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("entity_data"));

pub const BUCKET_ENTITY_DATA: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("bucket_entity_data"));

pub const BLOCK_ENTITY_DATA: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("block_entity_data"));

pub const INSTRUMENT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("instrument"));

pub const PROVIDES_TRIM_MATERIAL: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("provides_trim_material"));

pub const OMINOUS_BOTTLE_AMPLIFIER: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("ominous_bottle_amplifier"));

pub const JUKEBOX_PLAYABLE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("jukebox_playable"));

pub const PROVIDES_BANNER_PATTERNS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("provides_banner_patterns"));

pub const RECIPES: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("recipes"));

pub const LODESTONE_TRACKER: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("lodestone_tracker"));

pub const FIREWORK_EXPLOSION: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("firework_explosion"));

pub const FIREWORKS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("fireworks"));

pub const PROFILE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("profile"));

pub const NOTE_BLOCK_SOUND: DataComponentType<Identifier> =
    DataComponentType::new(Identifier::vanilla_static("note_block_sound"));

pub const BANNER_PATTERNS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("banner_patterns"));

pub const BASE_COLOR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("base_color"));

pub const POT_DECORATIONS: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("pot_decorations"));

pub const CONTAINER: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("container"));

pub const BLOCK_STATE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("block_state"));

pub const BEES: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("bees"));

pub const LOCK: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("lock"));

pub const CONTAINER_LOOT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("container_loot"));

pub const BREAK_SOUND: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("break_sound"));

// Entity variant components
pub const VILLAGER_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("villager/variant"));

pub const WOLF_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("wolf/variant"));

pub const WOLF_SOUND_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("wolf/sound_variant"));

pub const WOLF_COLLAR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("wolf/collar"));

pub const FOX_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("fox/variant"));

pub const SALMON_SIZE: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("salmon/size"));

pub const PARROT_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("parrot/variant"));

pub const TROPICAL_FISH_PATTERN: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("tropical_fish/pattern"));

pub const TROPICAL_FISH_BASE_COLOR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("tropical_fish/base_color"));

pub const TROPICAL_FISH_PATTERN_COLOR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("tropical_fish/pattern_color"));

pub const MOOSHROOM_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("mooshroom/variant"));

pub const RABBIT_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("rabbit/variant"));

pub const PIG_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("pig/variant"));

pub const COW_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("cow/variant"));

pub const CHICKEN_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("chicken/variant"));

pub const FROG_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("frog/variant"));

pub const HORSE_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("horse/variant"));

pub const PAINTING_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("painting/variant"));

pub const LLAMA_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("llama/variant"));

pub const AXOLOTL_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("axolotl/variant"));

pub const CAT_VARIANT: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("cat/variant"));

pub const CAT_COLLAR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("cat/collar"));

pub const SHEEP_COLOR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("sheep/color"));

pub const SHULKER_COLOR: DataComponentType<Todo> =
    DataComponentType::new(Identifier::vanilla_static("shulker/color"));

pub fn register_vanilla_data_components(registry: &mut DataComponentRegistry) {
    // Basic components
    registry.register(CUSTOM_DATA);
    registry.register(MAX_STACK_SIZE);
    registry.register(MAX_DAMAGE);
    registry.register(DAMAGE);
    registry.register(UNBREAKABLE);
    registry.register(CUSTOM_NAME);
    registry.register(ITEM_NAME);
    registry.register(ITEM_MODEL);
    registry.register(LORE);
    registry.register(RARITY);
    registry.register(ENCHANTMENTS);
    registry.register(CAN_PLACE_ON);
    registry.register(CAN_BREAK);
    registry.register(ATTRIBUTE_MODIFIERS);
    registry.register(CUSTOM_MODEL_DATA);
    registry.register(TOOLTIP_DISPLAY);
    registry.register(REPAIR_COST);
    registry.register(CREATIVE_SLOT_LOCK);
    registry.register(ENCHANTMENT_GLINT_OVERRIDE);
    registry.register(INTANGIBLE_PROJECTILE);
    registry.register(FOOD);
    registry.register(CONSUMABLE);
    registry.register(USE_REMAINDER);
    registry.register(USE_COOLDOWN);
    registry.register(DAMAGE_RESISTANT);
    registry.register(TOOL);
    registry.register(WEAPON);
    registry.register(ENCHANTABLE);
    registry.register(EQUIPPABLE);
    registry.register(REPAIRABLE);
    registry.register(GLIDER);
    registry.register(TOOLTIP_STYLE);
    registry.register(DEATH_PROTECTION);
    registry.register(BLOCKS_ATTACKS);
    registry.register(STORED_ENCHANTMENTS);
    registry.register(DYED_COLOR);
    registry.register(MAP_COLOR);
    registry.register(MAP_ID);
    registry.register(MAP_DECORATIONS);
    registry.register(MAP_POST_PROCESSING);
    registry.register(CHARGED_PROJECTILES);
    registry.register(BUNDLE_CONTENTS);
    registry.register(POTION_CONTENTS);
    registry.register(POTION_DURATION_SCALE);
    registry.register(SUSPICIOUS_STEW_EFFECTS);
    registry.register(WRITABLE_BOOK_CONTENT);
    registry.register(WRITTEN_BOOK_CONTENT);
    registry.register(TRIM);
    registry.register(DEBUG_STICK_STATE);
    registry.register(ENTITY_DATA);
    registry.register(BUCKET_ENTITY_DATA);
    registry.register(BLOCK_ENTITY_DATA);
    registry.register(INSTRUMENT);
    registry.register(PROVIDES_TRIM_MATERIAL);
    registry.register(OMINOUS_BOTTLE_AMPLIFIER);
    registry.register(JUKEBOX_PLAYABLE);
    registry.register(PROVIDES_BANNER_PATTERNS);
    registry.register(RECIPES);
    registry.register(LODESTONE_TRACKER);
    registry.register(FIREWORK_EXPLOSION);
    registry.register(FIREWORKS);
    registry.register(PROFILE);
    registry.register(NOTE_BLOCK_SOUND);
    registry.register(BANNER_PATTERNS);
    registry.register(BASE_COLOR);
    registry.register(POT_DECORATIONS);
    registry.register(CONTAINER);
    registry.register(BLOCK_STATE);
    registry.register(BEES);
    registry.register(LOCK);
    registry.register(CONTAINER_LOOT);
    registry.register(BREAK_SOUND);

    // Entity variant components
    registry.register(VILLAGER_VARIANT);
    registry.register(WOLF_VARIANT);
    registry.register(WOLF_SOUND_VARIANT);
    registry.register(WOLF_COLLAR);
    registry.register(FOX_VARIANT);
    registry.register(SALMON_SIZE);
    registry.register(PARROT_VARIANT);
    registry.register(TROPICAL_FISH_PATTERN);
    registry.register(TROPICAL_FISH_BASE_COLOR);
    registry.register(TROPICAL_FISH_PATTERN_COLOR);
    registry.register(MOOSHROOM_VARIANT);
    registry.register(RABBIT_VARIANT);
    registry.register(PIG_VARIANT);
    registry.register(COW_VARIANT);
    registry.register(CHICKEN_VARIANT);
    registry.register(FROG_VARIANT);
    registry.register(HORSE_VARIANT);
    registry.register(PAINTING_VARIANT);
    registry.register(LLAMA_VARIANT);
    registry.register(AXOLOTL_VARIANT);
    registry.register(CAT_VARIANT);
    registry.register(CAT_COLLAR);
    registry.register(SHEEP_COLOR);
    registry.register(SHULKER_COLOR);
}
