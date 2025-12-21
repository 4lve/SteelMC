//! Handler for the "seed" command.
use std::sync::Arc;

use steel_utils::text::TextComponent;
use steel_utils::text::color::NamedColor;
use steel_utils::text::interactivity::{ClickEvent, HoverEvent, Interactivity};
use steel_utils::text::translation::TranslatedMessage;
use steel_utils::translations;

use crate::command::commands::{CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::config::STEEL_CONFIG;
use crate::server::Server;

/// Handler for the "seed" command.
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["seed"],
        "Displays the world seed.",
        "minecraft:command.seed",
    )
    .executes(SeedCommandExecutor)
}

struct SeedCommandExecutor;

impl CommandExecutor<()> for SeedCommandExecutor {
    fn execute(
        &self,
        _args: (),
        context: &mut CommandContext,
        _server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        context.sender.send_message(
            TranslatedMessage::new(
                "commands.seed.success",
                Some(Box::new([TextComponent::new()
                    .text(&STEEL_CONFIG.seed)
                    .color(NamedColor::Green)
                    .interactivity(
                        Interactivity::new()
                            .hover_event(HoverEvent::show_text(
                                translations::CHAT_COPY_CLICK.msg().into(),
                            ))
                            .click_event(ClickEvent::CopyToClipboard {
                                value: (&STEEL_CONFIG.seed).into(),
                            }),
                    )])),
            )
            .into(),
        );
        Ok(())
    }
}
