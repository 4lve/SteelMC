#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(
    clippy::single_call_fn,
    clippy::multiple_inherent_impl,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::cargo_common_metadata,
    clippy::missing_panics_doc
)]
#![feature(
    const_trait_impl,
    const_slice_make_iter,
    const_cmp,
    derive_const,
    core_intrinsics
)]
#![allow(internal_features)]

use steel_utils::Identifier;

use crate::{
    banner_pattern::BannerPatternRegistry,
    biome::BiomeRegistry,
    blocks::BlockRegistry,
    cat_variant::CatVariantRegistry,
    chat_type::ChatTypeRegistry,
    chicken_variant::ChickenVariantRegistry,
    cow_variant::CowVariantRegistry,
    damage_type::DamageTypeRegistry,
    data_components::{DataComponentRegistry, vanilla_components},
    dialog::DialogRegistry,
    dimension_type::DimensionTypeRegistry,
    frog_variant::FrogVariantRegistry,
    instrument::InstrumentRegistry,
    items::ItemRegistry,
    jukebox_song::JukeboxSongRegistry,
    painting_variant::PaintingVariantRegistry,
    pig_variant::PigVariantRegistry,
    trim_material::TrimMaterialRegistry,
    trim_pattern::TrimPatternRegistry,
    wolf_sound_variant::WolfSoundVariantRegistry,
    wolf_variant::WolfVariantRegistry,
};
pub mod banner_pattern;
pub mod biome;
pub mod blocks;
pub mod cat_variant;
pub mod chat_type;
pub mod chicken_variant;
pub mod cow_variant;
pub mod damage_type;
pub mod data_components;
pub mod dialog;
pub mod dimension_type;
pub mod frog_variant;
pub mod instrument;
pub mod items;
pub mod jukebox_song;
pub mod painting_variant;
pub mod pig_variant;
pub mod trim_material;
pub mod trim_pattern;
pub mod wolf_sound_variant;
pub mod wolf_variant;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_blocks.rs"]
pub mod vanilla_blocks;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_block_tags.rs"]
pub mod vanilla_block_tags;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_banner_patterns.rs"]
pub mod vanilla_banner_patterns;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_items.rs"]
pub mod vanilla_items;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_item_tags.rs"]
pub mod vanilla_item_tags;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_biomes.rs"]
pub mod vanilla_biomes;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_chat_types.rs"]
pub mod vanilla_chat_types;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_trim_patterns.rs"]
pub mod vanilla_trim_patterns;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_trim_materials.rs"]
pub mod vanilla_trim_materials;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_wolf_variants.rs"]
pub mod vanilla_wolf_variants;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_wolf_sound_variants.rs"]
pub mod vanilla_wolf_sound_variants;

#[rustfmt::skip]
#[path = "generated/vanilla_pig_variants.rs"]
pub mod vanilla_pig_variants;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_frog_variants.rs"]
pub mod vanilla_frog_variants;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_cat_variants.rs"]
pub mod vanilla_cat_variants;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_cow_variants.rs"]
pub mod vanilla_cow_variants;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_chicken_variants.rs"]
pub mod vanilla_chicken_variants;

#[rustfmt::skip]
#[path = "generated/vanilla_painting_variants.rs"]
pub mod vanilla_painting_variants;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_dimension_types.rs"]
pub mod vanilla_dimension_types;

#[rustfmt::skip]
#[path = "generated/vanilla_damage_types.rs"]
pub mod vanilla_damage_types;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_jukebox_songs.rs"]
pub mod vanilla_jukebox_songs;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_instruments.rs"]
pub mod vanilla_instruments;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_dialogs.rs"]
pub mod vanilla_dialogs;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/vanilla_packets.rs"]
pub mod packets;

pub trait RegistryExt {
    fn freeze(&mut self);
}

pub const BLOCKS_REGISTRY: Identifier = Identifier::vanilla_static("block");
pub const ITEMS_REGISTRY: Identifier = Identifier::vanilla_static("item");
pub const BIOMES_REGISTRY: Identifier = Identifier::vanilla_static("worldgen/biome");
pub const CHAT_TYPE_REGISTRY: Identifier = Identifier::vanilla_static("chat_type");
pub const TRIM_PATTERN_REGISTRY: Identifier = Identifier::vanilla_static("trim_pattern");
pub const TRIM_MATERIAL_REGISTRY: Identifier = Identifier::vanilla_static("trim_material");
pub const WOLF_VARIANT_REGISTRY: Identifier = Identifier::vanilla_static("wolf_variant");
pub const WOLF_SOUND_VARIANT_REGISTRY: Identifier =
    Identifier::vanilla_static("wolf_sound_variant");
pub const PIG_VARIANT_REGISTRY: Identifier = Identifier::vanilla_static("pig_variant");
pub const FROG_VARIANT_REGISTRY: Identifier = Identifier::vanilla_static("frog_variant");
pub const CAT_VARIANT_REGISTRY: Identifier = Identifier::vanilla_static("cat_variant");
pub const COW_VARIANT_REGISTRY: Identifier = Identifier::vanilla_static("cow_variant");
pub const CHICKEN_VARIANT_REGISTRY: Identifier = Identifier::vanilla_static("chicken_variant");
pub const PAINTING_VARIANT_REGISTRY: Identifier = Identifier::vanilla_static("painting_variant");
pub const DIMENSION_TYPE_REGISTRY: Identifier = Identifier::vanilla_static("dimension_type");
pub const DAMAGE_TYPE_REGISTRY: Identifier = Identifier::vanilla_static("damage_type");
pub const BANNER_PATTERN_REGISTRY: Identifier = Identifier::vanilla_static("banner_pattern");
//TODO: Add enchantments
//pub const ENCHANTMENT_REGISTRY: Identifier = Identifier::vanilla_static("enchantment");
pub const JUKEBOX_SONG_REGISTRY: Identifier = Identifier::vanilla_static("jukebox_song");
pub const INSTRUMENT_REGISTRY: Identifier = Identifier::vanilla_static("instrument");
pub const DIALOG_REGISTRY: Identifier = Identifier::vanilla_static("dialog");

pub struct Registry {
    pub blocks: BlockRegistry,
    pub items: ItemRegistry,
    pub data_components: DataComponentRegistry,
    pub biomes: BiomeRegistry,
    pub chat_types: ChatTypeRegistry,
    pub trim_patterns: TrimPatternRegistry,
    pub trim_materials: TrimMaterialRegistry,
    pub wolf_variants: WolfVariantRegistry,
    pub wolf_sound_variants: WolfSoundVariantRegistry,
    pub pig_variants: PigVariantRegistry,
    pub frog_variants: FrogVariantRegistry,
    pub cat_variants: CatVariantRegistry,
    pub cow_variants: CowVariantRegistry,
    pub chicken_variants: ChickenVariantRegistry,
    pub painting_variants: PaintingVariantRegistry,
    pub dimension_types: DimensionTypeRegistry,
    pub damage_types: DamageTypeRegistry,
    pub banner_patterns: BannerPatternRegistry,
    pub jukebox_songs: JukeboxSongRegistry,
    pub instruments: InstrumentRegistry,
    pub dialogs: DialogRegistry,
}

impl Registry {
    #[must_use]
    pub fn new_vanilla() -> Self {
        let mut block_registry = BlockRegistry::new();
        vanilla_blocks::register_blocks(&mut block_registry);
        vanilla_block_tags::register_block_tags(&mut block_registry);

        let mut data_component_registry = DataComponentRegistry::new();
        vanilla_components::register_vanilla_data_components(&mut data_component_registry);

        let mut item_registry = ItemRegistry::new();
        vanilla_items::register_items(&mut item_registry);
        vanilla_item_tags::register_item_tags(&mut item_registry);

        let mut biome_registry = BiomeRegistry::new();
        vanilla_biomes::register_biomes(&mut biome_registry);

        let mut chat_type_registry = ChatTypeRegistry::new();
        vanilla_chat_types::register_chat_types(&mut chat_type_registry);

        let mut trim_pattern_registry = TrimPatternRegistry::new();
        vanilla_trim_patterns::register_trim_patterns(&mut trim_pattern_registry);

        let mut trim_material_registry = TrimMaterialRegistry::new();
        vanilla_trim_materials::register_trim_materials(&mut trim_material_registry);

        let mut wolf_variant_registry = WolfVariantRegistry::new();
        vanilla_wolf_variants::register_wolf_variants(&mut wolf_variant_registry);

        let mut wolf_sound_variant_registry = WolfSoundVariantRegistry::new();
        vanilla_wolf_sound_variants::register_wolf_sound_variants(&mut wolf_sound_variant_registry);

        let mut pig_variant_registry = PigVariantRegistry::new();
        vanilla_pig_variants::register_pig_variants(&mut pig_variant_registry);

        let mut frog_variant_registry = FrogVariantRegistry::new();
        vanilla_frog_variants::register_frog_variants(&mut frog_variant_registry);

        let mut cat_variant_registry = CatVariantRegistry::new();
        vanilla_cat_variants::register_cat_variants(&mut cat_variant_registry);

        let mut cow_variant_registry = CowVariantRegistry::new();
        vanilla_cow_variants::register_cow_variants(&mut cow_variant_registry);

        let mut chicken_variant_registry = ChickenVariantRegistry::new();
        vanilla_chicken_variants::register_chicken_variants(&mut chicken_variant_registry);

        let mut painting_variant_registry = PaintingVariantRegistry::new();
        vanilla_painting_variants::register_painting_variants(&mut painting_variant_registry);

        let mut dimension_type_registry = DimensionTypeRegistry::new();
        vanilla_dimension_types::register_dimension_types(&mut dimension_type_registry);

        let mut damage_type_registry = DamageTypeRegistry::new();
        vanilla_damage_types::register_damage_types(&mut damage_type_registry);

        let mut banner_pattern_registry = BannerPatternRegistry::new();
        vanilla_banner_patterns::register_banner_patterns(&mut banner_pattern_registry);

        let mut jukebox_song_registry = JukeboxSongRegistry::new();
        vanilla_jukebox_songs::register_jukebox_songs(&mut jukebox_song_registry);

        let mut instrument_registry = InstrumentRegistry::new();
        vanilla_instruments::register_instruments(&mut instrument_registry);

        let mut dialog_registry = DialogRegistry::new();
        vanilla_dialogs::register_dialogs(&mut dialog_registry);

        Self {
            blocks: block_registry,
            data_components: data_component_registry,
            items: item_registry,
            biomes: biome_registry,
            chat_types: chat_type_registry,
            trim_patterns: trim_pattern_registry,
            trim_materials: trim_material_registry,
            wolf_variants: wolf_variant_registry,
            wolf_sound_variants: wolf_sound_variant_registry,
            pig_variants: pig_variant_registry,
            frog_variants: frog_variant_registry,
            cat_variants: cat_variant_registry,
            cow_variants: cow_variant_registry,
            chicken_variants: chicken_variant_registry,
            painting_variants: painting_variant_registry,
            dimension_types: dimension_type_registry,
            damage_types: damage_type_registry,
            banner_patterns: banner_pattern_registry,
            jukebox_songs: jukebox_song_registry,
            instruments: instrument_registry,
            dialogs: dialog_registry,
        }
    }

    pub fn freeze(&mut self) {
        self.blocks.freeze();
        self.data_components.freeze();
        self.items.freeze();
        self.biomes.freeze();
        self.chat_types.freeze();
        self.trim_patterns.freeze();
        self.trim_materials.freeze();
        self.wolf_variants.freeze();
        self.wolf_sound_variants.freeze();
        self.pig_variants.freeze();
        self.frog_variants.freeze();
        self.cat_variants.freeze();
        self.cow_variants.freeze();
        self.chicken_variants.freeze();
        self.painting_variants.freeze();
        self.dimension_types.freeze();
        self.damage_types.freeze();
        self.banner_patterns.freeze();
        self.jukebox_songs.freeze();
        self.instruments.freeze();
        self.dialogs.freeze();
    }
}
