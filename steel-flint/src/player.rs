//! Test player implementation for SteelMC.

use std::collections::HashMap;
use std::sync::Arc;

use flint_core::test_spec::{BlockFace, PlayerSlot};
use flint_steel::{BlockPos, FlintPlayer, Item};
use steel_core::world::World;

/// Test player implementation that stores state in memory.
pub struct SteelTestPlayer {
    /// Reference to the world
    #[allow(dead_code)]
    world: Arc<World>,
    /// Inventory storage
    inventory: HashMap<PlayerSlot, Item>,
    /// Selected hotbar slot (1-9)
    selected_hotbar: u8,
}

impl SteelTestPlayer {
    /// Creates a new test player.
    pub fn new(world: Arc<World>) -> Self {
        Self {
            world,
            inventory: HashMap::new(),
            selected_hotbar: 1,
        }
    }
}

impl FlintPlayer for SteelTestPlayer {
    fn set_slot(&mut self, slot: PlayerSlot, item: Option<&Item>) {
        if let Some(item) = item {
            // Clone the item to store it
            self.inventory.insert(slot, Item {
                id: item.id.clone(),
                count: item.count,
            });
        } else {
            self.inventory.remove(&slot);
        }
    }

    fn get_slot(&self, slot: PlayerSlot) -> Option<Item> {
        self.inventory.get(&slot).map(|item| Item {
            id: item.id.clone(),
            count: item.count,
        })
    }

    fn select_hotbar(&mut self, slot: u8) {
        if (1..=9).contains(&slot) {
            self.selected_hotbar = slot;
        }
    }

    fn selected_hotbar(&self) -> u8 {
        self.selected_hotbar
    }

    fn use_item_on(&mut self, pos: BlockPos, face: &BlockFace) {
        log::warn!("Stub: use_item_on({:?}, {:?}) - Requires decoupling Player from Networking in steel-core", pos, face);
        
        // Note: Real implementation needs to call block behaviors.
        // But behaviors require &Player, which requires connection.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_registries;
    use crate::world::SteelTestWorld;
    use flint_steel::FlintWorld;

    #[test]
    fn test_inventory() {
        init_test_registries();
        let mut world = SteelTestWorld::new();
        let mut player = world.create_player();

        let item = Item::new("minecraft:stone");
        player.set_slot(PlayerSlot::Hotbar1, Some(&item));
        
        let retrieved = player.get_slot(PlayerSlot::Hotbar1).unwrap();
        assert_eq!(retrieved.id, "minecraft:stone");
    }
}
