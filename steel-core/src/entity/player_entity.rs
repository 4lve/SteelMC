//! Player entity wrapper that reads position dynamically from the Player

use std::sync::Arc;
use steel_utils::math::Vector3;
use uuid::Uuid;

use super::{BaseEntity, Entity, EntityData};
use crate::player::Player;

/// Wrapper around a Player that implements the Entity trait
/// This reads the player's position/rotation dynamically on each call
pub struct PlayerEntity {
    player: Arc<Player>,
    base: BaseEntity,
}

impl PlayerEntity {
    /// Creates a new `PlayerEntity` wrapper
    pub fn new(entity_id: i32, player: Arc<Player>) -> Self {
        let base = BaseEntity::new(entity_id, player.gameprofile.id, *player.position.lock());
        Self { player, base }
    }
}

impl Entity for PlayerEntity {
    fn entity_id(&self) -> i32 {
        self.base.entity_id()
    }

    fn uuid(&self) -> Uuid {
        self.base.uuid()
    }

    fn position(&self) -> Vector3<f64> {
        *self.player.position.lock()
    }

    fn rotation(&self) -> (f32, f32) {
        *self.player.rotation.lock()
    }

    fn delta_movement(&self) -> Vector3<f64> {
        // Players don't have physics yet, so return zero velocity
        // TODO: Implement player velocity tracking
        Vector3::default()
    }

    fn entity_data(&self) -> &EntityData {
        &self.base.entity_data
    }
}
