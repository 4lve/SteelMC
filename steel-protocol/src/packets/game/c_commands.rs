use std::io::{Result, Write};

use steel_macros::{ClientPacket, WriteTo};
#[allow(unused_imports)]
use steel_registry::packets::play::C_COMMANDS;
use steel_utils::{
    codec::VarInt,
    serial::{PrefixedWrite, WriteTo},
};

#[derive(ClientPacket, WriteTo)]
#[packet_id(Play = C_COMMANDS)]
pub struct CCommands {
    #[write(as = "vec")]
    pub nodes: Vec<CommandNode>,
    #[write(as = "var_int")]
    pub root_index: i32,
}

pub enum CommandNode {
    Root {
        children: Vec<i32>,
    },
    Literal {
        children: Vec<i32>,
        redirects_to: Option<i32>,
        name: &'static str,
        is_executable: bool,
    },
    Argument {
        children: Vec<i32>,
        redirects_to: Option<i32>,
        name: &'static str,
        is_executable: bool,
        parser: ArgumentType,
        suggestions_type: Option<SuggestionType>,
    },
}

impl CommandNode {
    const FLAG_IS_EXECUTABLE: u8 = 4;
    const FLAG_HAS_REDIRECT: u8 = 8;
    const FLAG_HAS_SUGGESTION_TYPE: u8 = 16;

    pub fn new_root(children: Vec<i32>) -> Self {
        Self::Root { children }
    }

    pub fn new_literal(
        info: CommandNodeInfo,
        name: &'static str,
        redirects_to: Option<i32>,
    ) -> Self {
        Self::Literal {
            children: info.children,
            name,
            is_executable: info.is_executable,
            redirects_to,
        }
    }

    pub fn new_argument(
        info: CommandNodeInfo,
        argument: (&'static str, ArgumentType, Option<SuggestionType>),
        redirects_to: Option<i32>,
    ) -> Self {
        Self::Argument {
            children: info.children,
            name: argument.0,
            is_executable: info.is_executable,
            redirects_to,
            parser: argument.1,
            suggestions_type: argument.2,
        }
    }

    pub fn make_executable(mut self) -> Self {
        if let CommandNode::Literal { is_executable, .. }
        | CommandNode::Argument { is_executable, .. } = &mut self
        {
            *is_executable = true;
        }
        self
    }

    fn flags(&self) -> u8 {
        let (mut flags, is_executable, has_redirect, has_suggestions_type) = match self {
            CommandNode::Root { .. } => (0, false, false, false),
            CommandNode::Literal {
                is_executable,
                redirects_to,
                ..
            } => (1, *is_executable, redirects_to.is_some(), false),
            CommandNode::Argument {
                is_executable,
                redirects_to: r,
                suggestions_type,
                ..
            } => (2, *is_executable, r.is_some(), suggestions_type.is_some()),
        };

        if is_executable {
            flags |= Self::FLAG_IS_EXECUTABLE
        }
        if has_redirect {
            flags |= Self::FLAG_HAS_REDIRECT
        }
        if has_suggestions_type {
            flags |= Self::FLAG_HAS_SUGGESTION_TYPE
        }
        flags
    }

    fn children(&self) -> &[i32] {
        match self {
            CommandNode::Root { children } => children,
            CommandNode::Literal { children, .. } => children,
            CommandNode::Argument { children, .. } => children,
        }
    }

    fn redirects_to(&self) -> &Option<i32> {
        match self {
            CommandNode::Root { .. } => &None,
            CommandNode::Literal { redirects_to, .. } => redirects_to,
            CommandNode::Argument { redirects_to, .. } => redirects_to,
        }
    }

    fn name(&self) -> Option<&'static str> {
        match self {
            CommandNode::Root { .. } => None,
            CommandNode::Literal { name, .. } => Some(*name),
            CommandNode::Argument { name, .. } => Some(*name),
        }
    }
}

impl WriteTo for CommandNode {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&self.flags().to_be_bytes())?;
        self.children().write_prefixed::<VarInt>(writer)?;

        if let Some(redirects_to) = self.redirects_to() {
            VarInt(*redirects_to).write(writer)?;
        }

        if let Some(name) = self.name() {
            name.write_prefixed::<VarInt>(writer)?;
        }

        if let CommandNode::Argument {
            parser,
            suggestions_type,
            ..
        } = self
        {
            parser.write(writer)?;

            if let Some(suggestions_type) = suggestions_type {
                suggestions_type.as_str().write_prefixed::<VarInt>(writer)?;
            }
        }

        Ok(())
    }
}

pub struct CommandNodeInfo {
    children: Vec<i32>,
    is_executable: bool,
}

impl CommandNodeInfo {
    pub fn new(children: Vec<i32>, is_executable: bool) -> Self {
        Self {
            children,
            is_executable,
        }
    }

    pub fn chain(mut self, mut other: Self) -> Self {
        self.children.append(&mut other.children);
        self.is_executable |= other.is_executable;
        self
    }
}

#[repr(u32)]
pub enum ArgumentType {
    Bool,
    Float {
        min: Option<f32>,
        max: Option<f32>,
    },
    Double {
        min: Option<f64>,
        max: Option<f64>,
    },
    Integer {
        min: Option<i32>,
        max: Option<i32>,
    },
    Long {
        min: Option<i64>,
        max: Option<i64>,
    },
    String {
        behavior: ArgumentStringTypeBehavior,
    },
    Entity {
        flags: u8,
    },
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    HexColor,
    Component,
    Style,
    Message,
    Nbt,
    NbtTag,
    NbtPath,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Angle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder {
        flags: u8,
    },
    Swizzle,
    Team,
    ItemSlot,
    ItemSlots,
    ResourceLocation,
    Function,
    EntityAnchor,
    IntRange,
    FloatRange,
    Dimension,
    Gamemode,
    Time {
        min: i32,
    },
    ResourceOrTag {
        identifier: &'static str,
    },
    ResourceOrTagKey {
        identifier: &'static str,
    },
    Resource {
        identifier: &'static str,
    },
    ResourceKey {
        identifier: &'static str,
    },
    TemplateMirror,
    TemplateRotation,
    Heightmap,
    LootTable,
    LootPredicate,
    LootModifier,
    Dialog,
    Uuid,
}

#[derive(Debug, Clone, Copy)]
pub enum ArgumentStringTypeBehavior {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

impl WriteTo for ArgumentType {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Safety: Since Self is repr(u32), it is guaranteed to hold the discriminant in the first 4 bytes
        // See https://doc.rust-lang.org/reference/items/enumerations.html#pointer-casting
        let id = unsafe { *(self as *const Self as *const i32) };
        VarInt(id).write(writer)?;

        match self {
            // Self::Float { min, max } => Self::write_number_arg(*min, *max, write),
            // Self::Double { min, max } => Self::write_number_arg(*min, *max, write),
            // Self::Integer { min, max } => Self::write_number_arg(*min, *max, write),
            // Self::Long { min, max } => Self::write_number_arg(*min, *max, write),
            Self::String { behavior } => {
                let i = match behavior {
                    ArgumentStringTypeBehavior::SingleWord => 0,
                    ArgumentStringTypeBehavior::QuotablePhrase => 1,
                    ArgumentStringTypeBehavior::GreedyPhrase => 2,
                };
                VarInt(i).write(writer)
            }
            // Self::Entity { flags } => Self::write_with_flags(*flags, write),
            // Self::ScoreHolder { flags } => Self::write_with_flags(*flags, write),
            Self::Time { min } => min.write(writer),
            // Self::ResourceOrTag { identifier } => Self::write_with_identifier(identifier, write),
            // Self::ResourceOrTagKey { identifier } => Self::write_with_identifier(identifier, write),
            // Self::Resource { identifier } => Self::write_with_identifier(identifier, write),
            // Self::ResourceKey { identifier } => Self::write_with_identifier(identifier, write),
            _ => Ok(()),
        }
    }
}

pub enum SuggestionType {
    AskServer,
    AllRecipes,
    AvailableSounds,
    SummonableEntities,
}

impl SuggestionType {
    fn as_str(&self) -> &str {
        match self {
            SuggestionType::AskServer => "minecraft:ask_server",
            SuggestionType::AllRecipes => "minecraft:all_recipes",
            SuggestionType::AvailableSounds => "minecraft:available_sounds",
            SuggestionType::SummonableEntities => "minecraft:summonable_entities",
        }
    }
}
