//! This module contains types and utilities for parsing command arguments.
pub mod anchor;
pub mod gamemode;
pub mod rotation;
pub mod time;
pub mod vector2;
pub mod vector3;

use steel_protocol::packets::game::{ArgumentType, SuggestionType};

use crate::command::context::CommandContext;

/// A trait that defines a command argument parser.
pub trait CommandArgument: Send + Sync {
    /// The type of the parsed output.
    type Output;

    /// Parses from the given arguments the expected type and returns the remaining unconsumed arguments and the parsed output.
    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)>;

    /// Returns the parser ID associated with this argument.
    fn usage(&self) -> (ArgumentType, Option<SuggestionType>);
}

struct Helper;

impl Helper {
    pub fn parse_relative_coordinate<const IS_Y: bool>(
        s: &str,
        origin: Option<f64>,
    ) -> Option<f64> {
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
