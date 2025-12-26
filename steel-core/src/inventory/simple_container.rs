//! A simple container implementation backed by a Vec.

use steel_registry::item_stack::ItemStack;

use super::Container;

/// A simple container that stores items in a fixed-size vector.
#[derive(Debug)]
pub struct SimpleContainer {
    items: Vec<ItemStack>,
    changed: bool,
}

impl SimpleContainer {
    /// Creates a new container with the given number of slots.
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self {
            items: (0..size).map(|_| ItemStack::empty()).collect(),
            changed: false,
        }
    }

    /// Returns whether the container has been modified since the last check.
    #[must_use]
    pub fn has_changed(&self) -> bool {
        self.changed
    }

    /// Clears the changed flag.
    pub fn clear_changed(&mut self) {
        self.changed = false;
    }
}

impl Container for SimpleContainer {
    fn size(&self) -> usize {
        self.items.len()
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        &self.items[slot]
    }

    fn get_item_mut(&mut self, slot: usize) -> &mut ItemStack {
        &mut self.items[slot]
    }

    fn set_item(&mut self, slot: usize, item: ItemStack) {
        self.items[slot] = item;
        self.set_changed();
    }

    fn set_changed(&mut self) {
        self.changed = true;
    }

    fn clear(&mut self) {
        for item in &mut self.items {
            *item = ItemStack::empty();
        }
        self.set_changed();
    }
}

