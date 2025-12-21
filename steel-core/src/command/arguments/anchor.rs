//! A anchor argument.
use steel_protocol::packets::game::{ArgumentType, SuggestionType};

use crate::command::arguments::CommandArgument;
use crate::command::context::{CommandContext, EntityAnchor};

/// A anchor argument.
pub struct AnchorArgument;

impl CommandArgument for AnchorArgument {
    type Output = EntityAnchor;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let anchor = match *arg.first()? {
            "feet" => EntityAnchor::Feet,
            "eyes" => EntityAnchor::Eyes,
            _ => return None,
        };

        Some((&arg[1..], anchor))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (ArgumentType::EntityAnchor, None)
    }
}
