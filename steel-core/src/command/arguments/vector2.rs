//! A vector2 argument.
use steel_protocol::packets::game::{ArgumentType, SuggestionType};
use steel_utils::math::Vector2;

use crate::command::arguments::{CommandArgument, Helper};
use crate::command::context::CommandContext;

/// A vector2 argument.
pub struct Vector2Argument;

impl CommandArgument for Vector2Argument {
    type Output = Vector2<f64>;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        let x = Helper::parse_relative_coordinate::<false>(
            arg.first()?,
            context.position.map(|o| o.x),
        )?;
        let z =
            Helper::parse_relative_coordinate::<false>(arg.get(1)?, context.position.map(|o| o.z))?;

        Some((&arg[2..], Vector2::new(x, z)))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (ArgumentType::Vec2, None)
    }
}
