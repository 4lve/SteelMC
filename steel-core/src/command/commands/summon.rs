//! Handler for the "summon" command.
use std::sync::Arc;

use simdnbt::owned::NbtCompound;
use steel_registry::vanilla_entities::EntityType;
use steel_utils::math::Vector3;
use steel_utils::text::TextComponent;

use crate::command::arguments::entity_type::EntityTypeArgument;
use crate::command::arguments::nbt::NbtArgument;
use crate::command::arguments::vector3::Vector3Argument;
use crate::command::commands::{
    CommandExecutor, CommandHandlerBuilder, CommandHandlerDyn, argument,
};
use crate::command::context::CommandContext;
use crate::command::error::CommandError;
use crate::config::STEEL_CONFIG;
use crate::entity::MobEntity;
use crate::server::Server;

/// Handler for the "summon" command.
///
/// Syntax:
/// - `/summon <entity_type>` - summon at player position
/// - `/summon <entity_type> <x> <y> <z>` - summon at specified position
/// - `/summon <entity_type> <x> <y> <z> <nbt>` - summon with NBT data
#[must_use]
pub fn command_handler() -> impl CommandHandlerDyn {
    CommandHandlerBuilder::new(
        &["summon"],
        "Summons an entity.",
        "minecraft:command.summon",
    )
    .then(
        argument("entity", EntityTypeArgument)
            // /summon <entity> - at player position
            .executes(SummonAtSenderExecutor)
            // /summon <entity> <pos>
            .then(
                argument("pos", Vector3Argument)
                    // /summon <entity> <pos> - at specified position
                    .executes(SummonAtPosExecutor)
                    // /summon <entity> <pos> <nbt> - with NBT data
                    .then(argument("nbt", NbtArgument).executes(SummonWithNbtExecutor)),
            ),
    )
}

type EntityTypeRef = &'static EntityType;

/// Executor for `/summon <entity>` - summons at sender's position
struct SummonAtSenderExecutor;

impl CommandExecutor<((), EntityTypeRef)> for SummonAtSenderExecutor {
    fn execute(
        &self,
        args: ((), EntityTypeRef),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let ((), entity_type) = args;

        // Get position from context (player position)
        let position = context.position.ok_or_else(|| {
            CommandError::CommandFailed(Box::new(TextComponent::const_text(
                "No position available (must be run by a player)",
            )))
        })?;

        summon_entity(entity_type, position, None, context, server)
    }
}

/// Executor for `/summon <entity> <pos>` - summons at specified position
struct SummonAtPosExecutor;

impl CommandExecutor<(((), EntityTypeRef), Vector3<f64>)> for SummonAtPosExecutor {
    fn execute(
        &self,
        args: (((), EntityTypeRef), Vector3<f64>),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let (((), entity_type), position) = args;
        summon_entity(entity_type, position, None, context, server)
    }
}

/// Executor for `/summon <entity> <pos> <nbt>` - summons with NBT data
struct SummonWithNbtExecutor;

impl CommandExecutor<((((), EntityTypeRef), Vector3<f64>), NbtCompound)> for SummonWithNbtExecutor {
    fn execute(
        &self,
        args: ((((), EntityTypeRef), Vector3<f64>), NbtCompound),
        context: &mut CommandContext,
        server: &Arc<Server>,
    ) -> Result<(), CommandError> {
        let ((((), entity_type), position), nbt) = args;
        summon_entity(entity_type, position, Some(&nbt), context, server)
    }
}

/// Common function to summon an entity
fn summon_entity(
    entity_type: EntityTypeRef,
    position: Vector3<f64>,
    nbt: Option<&NbtCompound>,
    context: &mut CommandContext,
    _server: &Arc<Server>,
) -> Result<(), CommandError> {
    // X/Z bounds check (vanilla world border default is +/- 29999984)
    const WORLD_BORDER: f64 = 29_999_984.0;

    // Validate entity is summonable
    if !entity_type.can_summon() {
        return Err(CommandError::CommandFailed(Box::new(
            TextComponent::new().text(format!("Entity {} cannot be summoned", entity_type.key)),
        )));
    }

    // Check peaceful mode - hostile mobs can't spawn in peaceful
    if STEEL_CONFIG.difficulty.is_peaceful() && !entity_type.is_allowed_in_peaceful() {
        return Err(CommandError::CommandFailed(Box::new(
            TextComponent::new().text("Monsters cannot be summoned in Peaceful difficulty"),
        )));
    }

    // Get the player's world
    let player = context
        .sender
        .get_player()
        .ok_or(CommandError::InvalidRequirement)?;

    // Validate position is within world bounds
    // Minecraft Y bounds: -64 to 320 for overworld
    if position.y < -64.0 || position.y > 320.0 {
        return Err(CommandError::CommandFailed(Box::new(
            TextComponent::new().text("Cannot summon entity outside of world bounds"),
        )));
    }

    if position.x.abs() > WORLD_BORDER || position.z.abs() > WORLD_BORDER {
        return Err(CommandError::CommandFailed(Box::new(
            TextComponent::new().text("Cannot summon entity outside of world border"),
        )));
    }

    let world = &player.world;

    // Allocate entity ID
    let entity_id = world.entity_tracker.allocate_entity_id();

    // Create the mob entity
    let mut mob = if let Some(nbt_data) = nbt {
        MobEntity::new_with_nbt(entity_id, entity_type.id, position, nbt_data)
    } else {
        MobEntity::new(entity_id, entity_type.id, position)
    };

    // Set default rotation based on player facing (optional)
    if let Some((yaw, _pitch)) = context.rotation {
        mob.set_rotation(yaw, 0.0);
    }

    // Add to entity tracker
    let mob = Arc::new(mob);
    world
        .entity_tracker
        .add_entity(mob, Some(entity_type.tracking_range_blocks()));

    // Update visibility immediately so nearby players can see it
    world.entity_tracker.update_player_visibility(player);

    // Send success message
    let entity_name = entity_type
        .key
        .strip_prefix("minecraft:")
        .unwrap_or(entity_type.key);

    context
        .sender
        .send_message(TextComponent::new().text(format!("Summoned new {entity_name}")));

    Ok(())
}
