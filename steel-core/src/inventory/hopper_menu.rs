//! Hopper menu implementation.
//!
//! Based on: [HopperMenu.java](file:///home/coco/Documents/GitHub/SteelMC/minecraft-src/minecraft/src/net/minecraft/world/inventory/HopperMenu.java)

use std::mem;

use steel_registry::item_stack::ItemStack;
use steel_registry::menu_type::MenuTypeRef;
use steel_registry::vanilla_menu_types;
use steel_utils::text::TextComponent;

use crate::inventory::{
    SyncPlayerInv,
    lock::{ContainerLockGuard, ContainerRef},
    menu::{Menu, MenuBehavior},
    menu_provider::{MenuInstance, MenuProvider},
    slot::{NormalSlot, Slot, SlotType, add_standard_inventory_slots},
};
use crate::player::Player;

/// The hopper menu (5 slots).
pub struct HopperMenu {
    behavior: MenuBehavior,
    container: ContainerRef,
}

impl HopperMenu {
    /// Creates a new hopper menu.
    #[must_use]
    pub fn new(inventory: SyncPlayerInv, container_id: u8, container: ContainerRef) -> Self {
        // Hopper has 5 slots
        let mut menu_slots = Vec::with_capacity(5 + 36);

        // Add container slots (0-4)
        // In vanilla, these are simply added.
        for i in 0..5 {
            menu_slots.push(SlotType::Normal(NormalSlot::new(container.clone(), i)));
        }

        // Add standard inventory slots
        add_standard_inventory_slots(&mut menu_slots, &inventory);

        Self {
            behavior: MenuBehavior::new(menu_slots, container_id, Some(vanilla_menu_types::HOPPER)),
            container,
        }
    }
}

impl Menu for HopperMenu {
    fn behavior(&self) -> &MenuBehavior {
        &self.behavior
    }

    fn behavior_mut(&mut self) -> &mut MenuBehavior {
        &mut self.behavior
    }

    fn quick_move_stack(
        &mut self,
        guard: &mut ContainerLockGuard,
        slot_index: usize,
        _player: &Player,
    ) -> ItemStack {
        // Based on HopperMenu.quickMoveStack

        if slot_index >= self.behavior.slots.len() {
            return ItemStack::empty();
        }

        let slot = &self.behavior.slots[slot_index];
        let stack = slot.get_item(guard).clone();
        if stack.is_empty() {
            return ItemStack::empty();
        }

        let clicked = stack.clone();
        let mut stack_mut = stack;

        // Slot ID 0-4 = Hopper
        // Slot ID 5-31 = Inventory
        // Slot ID 32-40 = Hotbar

        // If in hopper (0-4), move to inventory
        if slot_index < 5 {
            if !self
                .behavior
                .move_item_stack_to(guard, &mut stack_mut, 5, 41, true)
            {
                return ItemStack::empty();
            }
        } else {
            // In inventory, move to hopper
            if !self
                .behavior
                .move_item_stack_to(guard, &mut stack_mut, 0, 5, false)
            {
                return ItemStack::empty();
            }
        }

        if stack_mut.is_empty() {
            self.behavior.slots[slot_index].set_item(guard, ItemStack::empty());
        } else {
            self.behavior.slots[slot_index].set_changed(guard);
        }

        if stack_mut.count == clicked.count {
            return ItemStack::empty();
        }

        clicked
    }

    fn still_valid(&self) -> bool {
        let guard = self.behavior.lock_all_containers();
        guard
            .get(self.container.container_id())
            .is_some_and(super::container::Container::still_valid)
    }

    fn removed(&mut self, player: &Player) {
        let carried = mem::take(&mut self.behavior.carried);
        if !carried.is_empty() {
            player.drop_item(carried, false);
        }
    }
}

impl MenuInstance for HopperMenu {
    fn menu_type(&self) -> MenuTypeRef {
        vanilla_menu_types::HOPPER
    }

    fn container_id(&self) -> u8 {
        self.behavior.container_id
    }
}

/// Provider for creating hopper menus.
pub struct HopperMenuProvider {
    inventory: SyncPlayerInv,
    container: ContainerRef,
    title: TextComponent,
}

impl HopperMenuProvider {
    /// Creates a new hopper menu provider.
    pub fn new(inventory: SyncPlayerInv, container: ContainerRef, title: TextComponent) -> Self {
        Self {
            inventory,
            container,
            title,
        }
    }
}

impl MenuProvider for HopperMenuProvider {
    fn title(&self) -> TextComponent {
        self.title.clone()
    }

    fn create(&self, container_id: u8) -> Box<dyn MenuInstance> {
        Box::new(HopperMenu::new(
            self.inventory.clone(),
            container_id,
            self.container.clone(),
        ))
    }
}
