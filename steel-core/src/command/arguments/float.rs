//! A float argument for damage amounts.
use steel_protocol::packets::game::{ArgumentType, SuggestionType};

use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;

/// A float argument that accepts positive floating point numbers.
pub struct FloatArgument {
    /// The minimum value allowed for this float argument.
    pub min: f32,
}

impl CommandArgument for FloatArgument {
    type Output = f32;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let s = arg.first()?;
        let value = s.parse::<f32>().ok()?;

        if value < self.min {
            return None;
        }

        Some((&arg[1..], value))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (
            ArgumentType::Float {
                min: Some(self.min),
                max: None,
            },
            None,
        )
    }
}
