//! A text argument.
use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;
use steel_protocol::packets::game::{ArgumentType, SuggestionType};

/// A text argument.
pub struct TextArgument;

impl CommandArgument for TextArgument {
    type Output = String;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        Some((&[], arg.join(" ")))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (ArgumentType::NbtTag, None)
    }
}
