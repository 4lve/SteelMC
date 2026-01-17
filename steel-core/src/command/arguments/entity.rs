//! A entity argument.
use crate::command::arguments::SuggestionContext;
use crate::command::context::CommandContext;
use crate::{command::arguments::CommandArgument, entity::LivingEntity};
use std::sync::Arc;
use steel_protocol::packets::game::{ArgumentType, SuggestionEntry, SuggestionType};
use steel_utils::translations::ARGUMENT_ENTITY_SELECTOR_SELF;

/// A entity argument.
pub struct EntityArgument {
    /// If only accepts one entity
    one: bool,
}
impl EntityArgument {
    /// Creates a selector for multiple entities
    pub fn entities() -> Self {
        EntityArgument { one: false }
    }
    /// Creates a selector for one entity
    pub fn entity() -> Self {
        EntityArgument { one: true }
    }
}

impl CommandArgument for EntityArgument {
    type Output = Vec<Arc<dyn LivingEntity + Send + Sync>>;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let entities = match arg[0] {
            "@s" => {
                if let Some(player) = &context.player {
                    vec![player.clone() as Arc<dyn LivingEntity + Send + Sync>]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        };
        Some((&arg[1..], entities))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (
            ArgumentType::Entity {
                flags: self.one as u8,
            },
            Some(SuggestionType::AskServer),
        )
    }

    fn suggest(&self, _prefix: &str, _suggestion_ctx: &SuggestionContext) -> Vec<SuggestionEntry> {
        vec![SuggestionEntry::with_tooltip(
            "@s",
            ARGUMENT_ENTITY_SELECTOR_SELF,
        )]
    }
}
