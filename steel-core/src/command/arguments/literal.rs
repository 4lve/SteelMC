//! A literal argument.
use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;

/// A literal argument.
pub struct LiteralArgument {
    /// The expected literal value.
    pub expected: &'static str,
}

impl CommandArgument for LiteralArgument {
    type Output = ();

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        if *arg.first()? == self.expected {
            Some((&arg[1..], ()))
        } else {
            None
        }
    }
}
