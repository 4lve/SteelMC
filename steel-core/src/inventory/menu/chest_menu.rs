//! Chest menu implementation.

use steel_registry::item_stack::ItemStack;

use super::abstract_menu::{AbstractContainerMenu, MenuType, SLOT_SIZE};
use crate::inventory::{Container, SimpleContainer, Slot};

/// A chest menu with configurable rows (1-6).
pub struct ChestMenu {
    /// The underlying abstract menu.
    pub menu: AbstractContainerMenu<SimpleContainer>,
    /// The number of rows in the chest.
    rows: usize,
    /// The number of chest slots (rows * 9).
    chest_slot_count: usize,
}

impl ChestMenu {
    /// Creates a new chest menu with the specified number of rows.
    ///
    /// # Panics
    /// Panics if `rows` is not in the range 1-6.
    #[must_use]
    pub fn new(container_id: i32, rows: usize) -> Self {
        assert!((1..=6).contains(&rows), "Chest rows must be 1-6");

        let menu_type = match rows {
            1 => MenuType::Generic9x1,
            2 => MenuType::Generic9x2,
            3 => MenuType::Generic9x3,
            4 => MenuType::Generic9x4,
            5 => MenuType::Generic9x5,
            6 => MenuType::Generic9x6,
            _ => unreachable!(),
        };

        let chest_slot_count = rows * 9;
        let container = SimpleContainer::new(chest_slot_count);
        let mut menu = AbstractContainerMenu::new(Some(menu_type), container_id, container);

        // Add chest slots
        let chest_top = 18;
        for row in 0..rows {
            for col in 0..9 {
                let slot_index = col + row * 9;
                menu.add_slot(Slot::new(
                    slot_index,
                    8 + col as i32 * SLOT_SIZE,
                    chest_top + row as i32 * SLOT_SIZE,
                ));
            }
        }

        // Add player inventory slots
        let inventory_top = chest_top + rows as i32 * SLOT_SIZE + 13;
        menu.add_player_inventory_slots(8, inventory_top);

        Self {
            menu,
            rows,
            chest_slot_count,
        }
    }

    /// Creates a 1-row chest menu.
    #[must_use]
    pub fn one_row(container_id: i32) -> Self {
        Self::new(container_id, 1)
    }

    /// Creates a 2-row chest menu.
    #[must_use]
    pub fn two_rows(container_id: i32) -> Self {
        Self::new(container_id, 2)
    }

    /// Creates a 3-row chest menu (standard chest).
    #[must_use]
    pub fn three_rows(container_id: i32) -> Self {
        Self::new(container_id, 3)
    }

    /// Creates a 4-row chest menu.
    #[must_use]
    pub fn four_rows(container_id: i32) -> Self {
        Self::new(container_id, 4)
    }

    /// Creates a 5-row chest menu.
    #[must_use]
    pub fn five_rows(container_id: i32) -> Self {
        Self::new(container_id, 5)
    }

    /// Creates a 6-row chest menu (double chest).
    #[must_use]
    pub fn six_rows(container_id: i32) -> Self {
        Self::new(container_id, 6)
    }

    /// Returns the number of rows in the chest.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns the container ID.
    #[must_use]
    pub fn container_id(&self) -> i32 {
        self.menu.container_id
    }

    /// Returns the menu type.
    #[must_use]
    pub fn menu_type(&self) -> MenuType {
        self.menu.menu_type.unwrap()
    }

    /// Handles shift-click (quick move) for chest menus.
    ///
    /// - From chest slots: move to player inventory
    /// - From player inventory: move to chest
    #[must_use]
    pub fn quick_move_stack(&mut self, slot_index: usize) -> ItemStack {
        if slot_index >= self.menu.slots.len() {
            return ItemStack::empty();
        }

        // Get container slot index before any mutable borrows
        let container_slot = self.menu.slots[slot_index].container_slot();

        // Check if slot has item and get a copy
        let stack = self.menu.container.get_item(container_slot).clone();
        if stack.is_empty() {
            return ItemStack::empty();
        }

        let original = stack.clone();
        let mut remaining = stack;

        let moved = if slot_index < self.chest_slot_count {
            // Moving from chest to player inventory
            let player_slots_start = self.chest_slot_count;
            let player_slots_end = self.menu.slots.len();
            self.menu
                .move_item_stack_to(&mut remaining, player_slots_start, player_slots_end, true)
        } else {
            // Moving from player inventory to chest
            self.menu
                .move_item_stack_to(&mut remaining, 0, self.chest_slot_count, false)
        };

        if !moved {
            return ItemStack::empty();
        }

        // Update the source slot
        self.menu.container.set_item(container_slot, remaining);

        original
    }

    /// Gets the chest container.
    #[must_use]
    pub fn container(&self) -> &SimpleContainer {
        &self.menu.container
    }

    /// Gets the chest container mutably.
    #[must_use]
    pub fn container_mut(&mut self) -> &mut SimpleContainer {
        &mut self.menu.container
    }
}
