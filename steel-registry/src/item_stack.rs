//! Item stack implementation.

use steel_utils::Identifier;

use crate::{
    data_components::{ComponentPatchEntry, ComponentValue, DataComponentMap, DataComponentPatch},
    items::ItemRef,
    vanilla_items::ITEMS,
};

/// A stack of items with a count and component modifications.
#[derive(Debug)]
pub struct ItemStack {
    /// The item type. AIR represents an empty stack.
    item: ItemRef,
    /// The number of items in this stack.
    count: i32,
    /// Modifications to the prototype components.
    patch: DataComponentPatch,
}

impl Default for ItemStack {
    fn default() -> Self {
        Self::empty()
    }
}

impl ItemStack {
    /// Creates an empty item stack (using AIR).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            item: &ITEMS.air,
            count: 0,
            patch: DataComponentPatch::new(),
        }
    }

    /// Creates a new item stack with count 1.
    #[must_use]
    pub fn new(item: ItemRef) -> Self {
        Self::with_count(item, 1)
    }

    /// Creates a new item stack with the specified count.
    #[must_use]
    pub fn with_count(item: ItemRef, count: i32) -> Self {
        Self {
            item,
            count,
            patch: DataComponentPatch::new(),
        }
    }

    #[must_use]
    pub fn prototype(&self) -> &'static DataComponentMap {
        &self.item.components
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        std::ptr::eq(self.item, &ITEMS.air) || self.count <= 0
    }

    #[must_use]
    pub fn item(&self) -> ItemRef {
        if self.is_empty() {
            &ITEMS.air
        } else {
            self.item
        }
    }

    #[must_use]
    pub fn count(&self) -> i32 {
        if self.is_empty() { 0 } else { self.count }
    }

    pub fn set_count(&mut self, count: i32) {
        self.count = count;
    }

    /// Increases the stack count by the given amount.
    pub fn grow(&mut self, amount: i32) {
        self.count += amount;
    }

    /// Decreases the stack count by the given amount.
    pub fn shrink(&mut self, amount: i32) {
        self.count -= amount;
    }

    /// Splits off the given amount from this stack and returns it as a new stack.
    /// This stack's count is reduced by the split amount.
    #[must_use]
    pub fn split(&mut self, amount: i32) -> Self {
        let to_take = amount.min(self.count);
        let result = self.copy_with_count(to_take);
        self.shrink(to_take);
        result
    }

    /// Creates a copy of this stack with the specified count.
    #[must_use]
    pub fn copy_with_count(&self, count: i32) -> Self {
        if self.is_empty() {
            return Self::empty();
        }
        Self {
            item: self.item,
            count,
            patch: self.patch.clone(),
        }
    }

    /// Clears this stack, making it empty. Returns the original stack contents.
    pub fn copy_and_clear(&mut self) -> Self {
        let copy = self.clone();
        *self = Self::empty();
        copy
    }

    /// Returns the maximum stack size for this item type.
    #[must_use]
    pub fn max_stack_size(&self) -> i32 {
        use crate::data_components::vanilla_components::MAX_STACK_SIZE;
        self.prototype().get(MAX_STACK_SIZE).copied().unwrap_or(64)
    }

    /// Returns whether this item can stack with others.
    #[must_use]
    pub fn is_stackable(&self) -> bool {
        self.max_stack_size() > 1
    }

    #[must_use]
    pub fn is_same_item(a: &Self, b: &Self) -> bool {
        a.item().key == b.item().key
    }

    /// Checks if two stacks have the same item and components.
    #[must_use]
    pub fn is_same_item_same_components(a: &Self, b: &Self) -> bool {
        if !Self::is_same_item(a, b) {
            return false;
        }
        if a.is_empty() && b.is_empty() {
            return true;
        }
        a.components_equal(b)
    }

    #[must_use]
    pub fn matches(a: &Self, b: &Self) -> bool {
        a.count() == b.count() && Self::is_same_item_same_components(a, b)
    }

    #[must_use]
    pub fn is(&self, item: ItemRef) -> bool {
        self.item().key == item.key
    }

    pub fn get_effective_value(&self, key: &Identifier) -> Option<&dyn ComponentValue> {
        match self.patch.get_entry(key) {
            Some(ComponentPatchEntry::Set(v)) => Some(v.as_ref()),
            Some(ComponentPatchEntry::Removed) => None,
            None => self.prototype().get_raw(key),
        }
    }

    pub fn components_equal(&self, other: &Self) -> bool {
        let mut all_keys = rustc_hash::FxHashSet::default();

        for key in self.prototype().keys() {
            if !self.patch.is_removed(key) {
                all_keys.insert(key);
            }
        }
        for (key, entry) in self.patch.iter() {
            if matches!(entry, ComponentPatchEntry::Set(_)) {
                all_keys.insert(key);
            }
        }
        for key in other.prototype().keys() {
            if !other.patch.is_removed(key) {
                all_keys.insert(key);
            }
        }
        for (key, entry) in other.patch.iter() {
            if matches!(entry, ComponentPatchEntry::Set(_)) {
                all_keys.insert(key);
            }
        }
        for key in all_keys {
            let val_a = self.get_effective_value(key);
            let val_b = other.get_effective_value(key);

            match (val_a, val_b) {
                (Some(a), Some(b)) => {
                    if !a.eq_value(b) {
                        return false;
                    }
                }
                (None, None) => {}
                _ => return false,
            }
        }

        true
    }
}

impl Clone for ItemStack {
    fn clone(&self) -> Self {
        Self {
            item: self.item,
            count: self.count,
            patch: self.patch.clone(),
        }
    }
}

impl std::fmt::Display for ItemStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "Empty")
        } else {
            write!(f, "{} {}", self.count, self.item.key)
        }
    }
}
