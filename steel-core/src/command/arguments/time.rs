//! A time argument.
use steel_protocol::packets::game::{ArgumentType, SuggestionType};

use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;

/// A time argument.
pub struct TimeArgument;

impl CommandArgument for TimeArgument {
    type Output = i32;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let s = arg.first()?;

        let (number, unit) = s
            .find(|c: char| c.is_alphabetic())
            .map_or((*s, "t"), |pos| (&s[..pos], &s[pos..]));

        let number = number.parse::<f32>().ok()?;
        if number < 0.0 {
            return None;
        }

        let ticks = match unit {
            "d" => number * 24000.0,
            "s" => number * 20.0,
            "t" => number,
            _ => return None,
        };

        Some((&arg[1..], ticks.round() as i32))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (ArgumentType::Time { min: 0 }, None)
    }
}
