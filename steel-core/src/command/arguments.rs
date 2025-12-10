//! This module contains types and utilities for parsing command arguments.
use std::sync::Arc;

use steel_registry::damage_type::DamageType;
use steel_utils::BlockPos;
use steel_utils::math::{Vector2, Vector3};
use steel_utils::text::TextComponent;

use crate::command::context::CommandContext;
use crate::player::Player;

// TODO: https://minecraft.wiki/w/Argument_types

/// An argument parsed from a command.
pub enum Arg<'a> {
    // Entities(Vec<Arc<dyn EntityBase>>),
    // Entity(Arc<dyn EntityBase>),
    Players(Vec<Arc<Player>>),
    BlockPos(BlockPos),
    Pos3D(Vector3<f64>),
    Pos2D(Vector2<f64>),
    Rotation(f32, f32),
    GameMode(()),
    // GameMode(GameMode),
    // Difficulty(Difficulty),
    // CommandTree(CommandTree),
    Item(&'a str),
    ResourceLocation(&'a str),
    Block(&'a str),
    BlockPredicate(&'a str),
    // BossbarColor(BossbarColor),
    // BossbarStyle(BossbarDivisions),
    // Particle(Particle),
    Msg(String),
    TextComponent(TextComponent),
    Time(i32),
    // Num(Result<Number, NotInBounds>),
    Bool(bool),
    #[allow(unused)]
    Simple(&'a str),
    // SoundCategory(SoundCategory),
    DamageType(DamageType),
    // Effect(&'static StatusEffect),
    // Enchantment(&'static Enchantment),
}

/// An argument that can be passed to a command.
pub enum CommandParserArgument {
    Angle,
    BlockPos,
    BlockPredicate,
    BossbarStyle,
    BossbarColor,
    Color,
    TextComponent,
    Dimension,
    Entities,
    Entity,
    Players,
    EntityAnchor,
    FloatRange,
    Function,
    GameProfile,
    Gamemode,
    Heightmap,
    IntRange,
    ItemPredicate,
    ItemSlot,
    ItemSlots,
    ItemStack,
    Literal(&'static str),
    LootModifier,
    LootPredicate,
    LootTable,
    Message,
    NbtCompoundTag,
    NbtPath,
    NbtTag,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Resource,
    ResourceKey,
    ResourceLocation,
    ResourceOrTag,
    ResourceOrTagKey,
    ResourceSelector,
    Rotation,
    ScoreHolder,
    ScoreboardSlot,
    Style,
    Swizzle,
    Team,
    TemplateMirror,
    TemplateRotation,
    Time,
    Pos2D,
    Pos3D,
    Block,
    Bool,
    DamageType,
    Difficulty,
    Effect,
    Enchantment,
    Item,
    Num,
    SoundCategory,
}

impl CommandParserArgument {
    /// Parses an argument from a slice of strings.
    ///
    /// Returns the parsed argument and the remaining unparsed strings slice.
    pub fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        context: &mut CommandContext,
    ) -> Option<(Arg<'a>, &'a [&'a str])> {
        match self {
            Self::Angle => unimplemented!(),
            Self::BlockPos => unimplemented!(),
            Self::BlockPredicate => unimplemented!(),
            Self::BossbarStyle => unimplemented!(),
            Self::BossbarColor => unimplemented!(),
            Self::Color => unimplemented!(),
            Self::TextComponent => unimplemented!(),
            Self::Dimension => unimplemented!(),
            Self::Entities => unimplemented!(),
            Self::Entity => unimplemented!(),
            Self::Players => unimplemented!(),
            Self::EntityAnchor => unimplemented!(),
            Self::FloatRange => unimplemented!(),
            Self::Function => unimplemented!(),
            Self::GameProfile => unimplemented!(),
            Self::Gamemode => unimplemented!(),
            Self::Heightmap => unimplemented!(),
            Self::IntRange => unimplemented!(),
            Self::ItemPredicate => unimplemented!(),
            Self::ItemSlot => unimplemented!(),
            Self::ItemSlots => unimplemented!(),
            Self::ItemStack => unimplemented!(),
            Self::Literal(literal) => {
                if arg.get(0)? == literal {
                    Some((Arg::Simple(literal), &arg[1..]))
                } else {
                    None
                }
            }
            Self::LootModifier => unimplemented!(),
            Self::LootPredicate => unimplemented!(),
            Self::LootTable => unimplemented!(),
            Self::Message => unimplemented!(),
            Self::NbtCompoundTag => unimplemented!(),
            Self::NbtPath => unimplemented!(),
            Self::NbtTag => unimplemented!(),
            Self::Objective => unimplemented!(),
            Self::ObjectiveCriteria => unimplemented!(),
            Self::Operation => unimplemented!(),
            Self::Particle => unimplemented!(),
            Self::Resource => unimplemented!(),
            Self::ResourceKey => unimplemented!(),
            Self::ResourceLocation => {
                let s = arg.get(0)?;
                Some((Arg::ResourceLocation(s), &arg[1..]))
            }
            Self::ResourceOrTag => unimplemented!(),
            Self::ResourceOrTagKey => unimplemented!(),
            Self::ResourceSelector => unimplemented!(),
            Self::Rotation => {
                let mut yaw = arg.get(0)?.parse::<f32>().ok()?;
                let mut pitch = arg.get(1)?.parse::<f32>().ok()?;

                yaw %= 360.0;
                if yaw >= 180.0 {
                    yaw -= 360.0;
                }
                pitch %= 360.0;
                if pitch >= 180.0 {
                    pitch -= 360.0;
                }

                Some((Arg::Rotation(yaw, pitch), &arg[2..]))
            }
            Self::ScoreHolder => unimplemented!(),
            Self::ScoreboardSlot => unimplemented!(),
            Self::Style => unimplemented!(),
            Self::Swizzle => unimplemented!(),
            Self::Team => unimplemented!(),
            Self::TemplateMirror => unimplemented!(),
            Self::TemplateRotation => unimplemented!(),
            Self::Time => unimplemented!(),
            Self::Pos2D => {
                let x =
                    Self::parse_coordinate::<false>(arg.get(0)?, context.position.map(|o| o.x))?;
                let z =
                    Self::parse_coordinate::<false>(arg.get(1)?, context.position.map(|o| o.z))?;
                Some((Arg::Pos2D(Vector2::new(x, z)), &arg[2..]))
            }
            Self::Pos3D => {
                let x =
                    Self::parse_coordinate::<false>(arg.get(0)?, context.position.map(|o| o.x))?;
                let y = Self::parse_coordinate::<true>(arg.get(1)?, context.position.map(|o| o.y))?;
                let z =
                    Self::parse_coordinate::<false>(arg.get(2)?, context.position.map(|o| o.z))?;
                Some((Arg::Pos3D(Vector3::new(x, y, z)), &arg[3..]))
            }
            _ => unimplemented!(),
        }
    }

    fn parse_coordinate<const IS_Y: bool>(s: &str, origin: Option<f64>) -> Option<f64> {
        if let Some(s) = s.strip_prefix('~') {
            let origin = origin?;
            let offset = if s.is_empty() { 0.0 } else { s.parse().ok()? };
            Some(origin + offset)
        } else {
            let mut v = s.parse().ok()?;

            // set position to block center if no decimal place is given
            if !IS_Y && !s.contains('.') {
                v += 0.5;
            }

            Some(v)
        }
    }
}
