//! Slot abstraction for inventory access.

use std::sync::Arc;

use enum_dispatch::enum_dispatch;
use steel_registry::data_components::vanilla_components::EquippableSlot;
use steel_registry::item_stack::ItemStack;
use steel_utils::locks::SyncMutex;

use crate::inventory::SyncPlayerInv;
use crate::inventory::crafting::{CraftingContainer, ResultContainer};
use crate::inventory::lock::{ContainerId, ContainerLockGuard, ContainerRef};
use crate::player::Player;

/// A synchronized crafting container.
pub type SyncCraftingContainer = Arc<SyncMutex<CraftingContainer>>;

/// A synchronized result container.
pub type SyncResultContainer = Arc<SyncMutex<ResultContainer>>;

/// A slot is a view into a single position in a container.
/// Slots require a `ContainerLockGuard` to access items, ensuring proper locking.
#[enum_dispatch]
pub trait Slot {
    /// Returns a reference to the item in this slot.
    fn get_item<'a>(&self, guard: &'a ContainerLockGuard) -> &'a ItemStack;

    /// Returns a mutable reference to the item in this slot.
    fn get_item_mut<'a>(&self, guard: &'a mut ContainerLockGuard) -> &'a mut ItemStack;

    /// Sets the item in this slot.
    fn set_item(&self, guard: &mut ContainerLockGuard, stack: ItemStack);

    /// Modifies the item in this slot in-place.
    fn modify_item<R>(
        &self,
        guard: &mut ContainerLockGuard,
        f: impl FnOnce(&mut ItemStack) -> R,
    ) -> R {
        let item = self.get_item_mut(guard);
        f(item)
    }

    /// Sets the item in this slot, triggered by a player action.
    ///
    /// This is called when a player directly places or swaps an item in a slot.
    /// The `previous` parameter contains the item that was in the slot before.
    ///
    /// Subclasses can override this to trigger events like equipment change sounds.
    /// The default implementation just calls `set_item`.
    fn set_by_player(
        &self,
        guard: &mut ContainerLockGuard,
        stack: ItemStack,
        _previous: &ItemStack,
    ) {
        self.set_item(guard, stack);
    }

    /// Returns true if this slot has an item.
    fn has_item(&self, guard: &ContainerLockGuard) -> bool {
        !self.get_item(guard).is_empty()
    }

    /// Returns true if the given item can be placed in this slot.
    fn may_place(&self, _stack: &ItemStack) -> bool {
        true
    }

    /// Returns true if items can be picked up from this slot.
    fn may_pickup(&self) -> bool {
        true
    }

    /// Returns true if partial removal is allowed from this slot.
    ///
    /// For normal slots: `may_pickup() && may_place(current_item)`
    /// For result slots: `false` (must take the full stack)
    fn allow_modification(&self, guard: &ContainerLockGuard) -> bool {
        self.may_pickup() && self.may_place(self.get_item(guard))
    }

    /// Returns the maximum stack size for this slot.
    fn get_max_stack_size(&self) -> i32 {
        99
    }

    /// Returns the maximum stack size for a specific item in this slot.
    fn get_max_stack_size_for_item(&self, stack: &ItemStack) -> i32 {
        self.get_max_stack_size().min(stack.max_stack_size())
    }

    /// Removes up to `amount` items from this slot and returns them.
    fn remove(&self, guard: &mut ContainerLockGuard, amount: i32) -> ItemStack {
        let item = self.get_item_mut(guard);
        if item.is_empty() || amount <= 0 {
            return ItemStack::empty();
        }

        let take_count = amount.min(item.count());
        let mut taken = item.clone();
        taken.set_count(take_count);

        let remaining = item.count() - take_count;
        if remaining <= 0 {
            *item = ItemStack::empty();
        } else {
            item.set_count(remaining);
        }

        taken
    }

    /// Tries to remove items from this slot with validation.
    ///
    /// Returns `Some(items)` if removal succeeded, `None` otherwise.
    /// If `allow_modification()` is false and `max_amount < item.count`,
    /// returns `None` (forcing full stack pickup for result slots).
    fn try_remove(
        &self,
        guard: &mut ContainerLockGuard,
        amount: i32,
        max_amount: i32,
    ) -> Option<ItemStack> {
        if !self.may_pickup() {
            return None;
        }

        let item_count = self.get_item(guard).count();

        // If modification not allowed (e.g., result slots), must take full stack
        if !self.allow_modification(guard) && max_amount < item_count {
            return None;
        }

        let take_amount = amount.min(max_amount);
        let result = self.remove(guard, take_amount);
        if result.is_empty() {
            return None;
        }

        Some(result)
    }

    /// Called when an item is taken from this slot.
    /// Returns any remainder items that couldn't be placed back (e.g., crafting remainders).
    fn on_take(
        &self,
        _guard: &mut ContainerLockGuard,
        _stack: &ItemStack,
        _player: &Player,
    ) -> Option<ItemStack> {
        None
    }

    /// Safely takes items from this slot with all checks and callbacks.
    ///
    /// This combines `try_remove` and `on_take` into a single operation.
    ///
    /// Returns the items taken (empty if nothing could be taken).
    fn safe_take(
        &self,
        guard: &mut ContainerLockGuard,
        amount: i32,
        max_amount: i32,
        player: &Player,
    ) -> ItemStack {
        if let Some(taken) = self.try_remove(guard, amount, max_amount) {
            if let Some(remainder) = self.on_take(guard, &taken, player) {
                // Try to add remainder to player inventory, or drop it
                player.add_item_or_drop_with_guard(guard, remainder);
            }
            taken
        } else {
            ItemStack::empty()
        }
    }

    /// Marks the slot's container as changed.
    fn set_changed(&self, guard: &mut ContainerLockGuard);

    /// Returns the container slot index.
    fn get_container_slot(&self) -> usize;

    /// Returns true if this is a "fake" slot (like crafting result).
    /// Fake slots don't persist items and are virtual views.
    fn is_fake(&self) -> bool {
        false
    }
}

/// A normal slot that references a container and index.
pub struct NormalSlot {
    container: SyncPlayerInv,
    index: usize,
}

impl NormalSlot {
    /// Creates a new normal slot.
    pub fn new(container: SyncPlayerInv, index: usize) -> Self {
        Self { container, index }
    }

    /// Returns a reference to the container.
    #[must_use]
    pub fn container_ref(&self) -> ContainerRef {
        ContainerRef::PlayerInventory(Arc::clone(&self.container))
    }
}

impl Slot for NormalSlot {
    fn get_item<'a>(&self, guard: &'a ContainerLockGuard) -> &'a ItemStack {
        guard
            .get(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .get_item(self.index)
    }

    fn get_item_mut<'a>(&self, guard: &'a mut ContainerLockGuard) -> &'a mut ItemStack {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .get_item_mut(self.index)
    }

    fn set_item(&self, guard: &mut ContainerLockGuard, stack: ItemStack) {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .set_item(self.index, stack);
    }

    fn set_changed(&self, guard: &mut ContainerLockGuard) {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .set_changed();
    }

    fn get_container_slot(&self) -> usize {
        self.index
    }
}

/// An armor slot that only accepts items equippable in the corresponding slot.
pub struct ArmorSlot {
    container: SyncPlayerInv,
    index: usize,
    /// The equipment slot this armor slot accepts.
    slot: EquippableSlot,
}

impl ArmorSlot {
    /// Creates a new armor slot.
    pub fn new(container: SyncPlayerInv, index: usize, slot: EquippableSlot) -> Self {
        Self {
            container,
            index,
            slot,
        }
    }

    /// Returns the equipment slot this armor slot accepts.
    #[must_use]
    pub fn equipment_slot(&self) -> EquippableSlot {
        self.slot
    }

    /// Returns a reference to the container.
    #[must_use]
    pub fn container_ref(&self) -> ContainerRef {
        ContainerRef::PlayerInventory(Arc::clone(&self.container))
    }
}

impl Slot for ArmorSlot {
    fn get_item<'a>(&self, guard: &'a ContainerLockGuard) -> &'a ItemStack {
        guard
            .get(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .get_item(self.index)
    }

    fn get_item_mut<'a>(&self, guard: &'a mut ContainerLockGuard) -> &'a mut ItemStack {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .get_item_mut(self.index)
    }

    fn set_item(&self, guard: &mut ContainerLockGuard, stack: ItemStack) {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .set_item(self.index, stack);
    }

    /// Sets the armor item in this slot, triggered by a player action.
    fn set_by_player(
        &self,
        guard: &mut ContainerLockGuard,
        stack: ItemStack,
        previous: &ItemStack,
    ) {
        // TODO: Call player.onEquipItem(equipmentSlot, previous, stack) here
        let _ = previous;
        self.set_item(guard, stack);
    }

    fn may_place(&self, stack: &ItemStack) -> bool {
        stack.is_equippable_in_slot(self.slot)
    }

    fn get_max_stack_size(&self) -> i32 {
        1
    }

    fn set_changed(&self, guard: &mut ContainerLockGuard) {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .set_changed();
    }

    fn get_container_slot(&self) -> usize {
        self.index
    }
}

/// A slot in a crafting grid.
///
/// This slot holds items placed in the crafting grid and triggers
/// recipe recalculation when changed.
pub struct CraftingGridSlot {
    container: SyncCraftingContainer,
    result_container: SyncResultContainer,
    index: usize,
}

impl CraftingGridSlot {
    /// Creates a new crafting grid slot.
    pub fn new(
        container: SyncCraftingContainer,
        result_container: SyncResultContainer,
        index: usize,
    ) -> Self {
        Self {
            container,
            result_container,
            index,
        }
    }

    /// Returns a reference to the crafting container.
    #[must_use]
    pub fn container_ref(&self) -> ContainerRef {
        ContainerRef::CraftingContainer(Arc::clone(&self.container))
    }

    /// Returns a reference to the result container.
    #[must_use]
    pub fn result_container_ref(&self) -> ContainerRef {
        ContainerRef::ResultContainer(Arc::clone(&self.result_container))
    }

    /// Updates the crafting result based on current grid contents.
    ///
    /// This recalculates the recipe and updates the result slot, matching
    /// Java's `slotsChanged` -> `slotChangedCraftingGrid` callback pattern.
    fn update_result(&self, guard: &mut ContainerLockGuard) {
        let container_id = ContainerId::from_arc(&self.container);
        let result_container_id = ContainerId::from_arc(&self.result_container);

        // Get the crafting container to read its contents
        let crafting = guard
            .get(container_id)
            .expect("crafting container not locked");

        // Build the crafting input from the container
        let items: Vec<ItemStack> = (0..4).map(|i| crafting.get_item(i).clone()).collect();
        let positioned = steel_registry::recipe::CraftingInput::positioned(2, 2, items);

        // Find matching recipe
        let result_stack = steel_registry::REGISTRY
            .recipes
            .find_crafting_recipe_2x2(&positioned.input)
            .map_or_else(ItemStack::empty, |r| r.assemble());

        // Update the result container
        guard
            .get_mut(result_container_id)
            .expect("result container not locked")
            .set_item(0, result_stack);
    }
}

impl Slot for CraftingGridSlot {
    fn get_item<'a>(&self, guard: &'a ContainerLockGuard) -> &'a ItemStack {
        guard
            .get(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .get_item(self.index)
    }

    fn get_item_mut<'a>(&self, guard: &'a mut ContainerLockGuard) -> &'a mut ItemStack {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .get_item_mut(self.index)
    }

    fn set_item(&self, guard: &mut ContainerLockGuard, stack: ItemStack) {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .set_item(self.index, stack);
        self.update_result(guard);
    }

    fn set_changed(&self, guard: &mut ContainerLockGuard) {
        guard
            .get_mut(ContainerId::from_arc(&self.container))
            .expect("container not locked")
            .set_changed();
        self.update_result(guard);
    }

    fn get_container_slot(&self) -> usize {
        self.index
    }
}

/// A slot that displays the crafting result.
///
/// This slot shows what can be crafted from the current grid contents.
/// When an item is taken from this slot, it consumes ingredients from the grid
/// and handles crafting remainders (e.g., buckets from milk buckets).
pub struct CraftingResultSlot {
    result_container: SyncResultContainer,
    crafting_container: SyncCraftingContainer,
}

impl CraftingResultSlot {
    /// Creates a new crafting result slot.
    pub fn new(
        result_container: SyncResultContainer,
        crafting_container: SyncCraftingContainer,
    ) -> Self {
        Self {
            result_container,
            crafting_container,
        }
    }

    /// Returns a reference to the crafting container.
    #[must_use]
    pub fn crafting_container(&self) -> &SyncCraftingContainer {
        &self.crafting_container
    }

    /// Returns a reference to the result container.
    #[must_use]
    pub fn result_container_ref(&self) -> ContainerRef {
        ContainerRef::ResultContainer(Arc::clone(&self.result_container))
    }

    /// Returns a reference to the crafting container.
    #[must_use]
    pub fn crafting_container_ref(&self) -> ContainerRef {
        ContainerRef::CraftingContainer(Arc::clone(&self.crafting_container))
    }
}

impl Slot for CraftingResultSlot {
    fn get_item<'a>(&self, guard: &'a ContainerLockGuard) -> &'a ItemStack {
        guard
            .get(ContainerId::from_arc(&self.result_container))
            .expect("container not locked")
            .get_item(0)
    }

    fn get_item_mut<'a>(&self, guard: &'a mut ContainerLockGuard) -> &'a mut ItemStack {
        guard
            .get_mut(ContainerId::from_arc(&self.result_container))
            .expect("container not locked")
            .get_item_mut(0)
    }

    fn set_item(&self, guard: &mut ContainerLockGuard, stack: ItemStack) {
        guard
            .get_mut(ContainerId::from_arc(&self.result_container))
            .expect("container not locked")
            .set_item(0, stack);
    }

    /// Cannot place items directly in the result slot.
    fn may_place(&self, _stack: &ItemStack) -> bool {
        false
    }

    /// Result slots don't allow partial removal.
    fn allow_modification(&self, _guard: &ContainerLockGuard) -> bool {
        false
    }

    /// Removes items from the crafting result slot.
    ///
    /// Unlike normal slots, this **always takes the entire stack** regardless
    /// of the `amount` parameter.
    fn remove(&self, guard: &mut ContainerLockGuard, _amount: i32) -> ItemStack {
        std::mem::take(self.get_item_mut(guard))
    }

    fn set_changed(&self, guard: &mut ContainerLockGuard) {
        guard
            .get_mut(ContainerId::from_arc(&self.result_container))
            .expect("container not locked")
            .set_changed();
    }

    fn get_container_slot(&self) -> usize {
        0
    }

    /// Called when an item is taken from the result slot.
    /// This consumes ingredients, handles remainders, and updates the result.
    ///
    /// Based on Java's `ResultSlot::onTake`. Uses positioned crafting input to
    /// correctly map recipe slots back to the original crafting grid, and gets
    /// remainders from the recipe rather than individual items.
    fn on_take(
        &self,
        guard: &mut ContainerLockGuard,
        _stack: &ItemStack,
        _player: &Player,
    ) -> Option<ItemStack> {
        // TODO: Add statistics/achievement tracking here.
        // Java calls checkTakeAchievements(carried) which triggers:
        // - carried.onCraftedBy(player, removeCount) for achievements
        // - recipeCraftingHolder.awardUsedRecipes(player, items) for recipe unlocks

        let mut remainder_overflow: Vec<ItemStack> = Vec::new();

        // First, build the positioned input and get remainders from recipe while holding immutable borrow
        let crafting_container_id = ContainerId::from_arc(&self.crafting_container);
        let result_container_id = ContainerId::from_arc(&self.result_container);
        let (remainders_and_positioned, grid_width) = {
            let crafting = guard
                .get(crafting_container_id)
                .expect("crafting container not locked");

            // Build crafting input
            let width = 2; // 2x2 player crafting grid
            let items: Vec<ItemStack> = (0..4).map(|i| crafting.get_item(i).clone()).collect();
            let positioned = steel_registry::recipe::CraftingInput::positioned(width, 2, items);

            // Get remainders from recipe
            let remainders = steel_registry::REGISTRY
                .recipes
                .find_crafting_recipe_2x2(&positioned.input)
                .map(|r| r.get_remaining_items(&positioned.input))
                .unwrap_or_default();

            (Some((remainders, positioned)), width)
        };

        // Now apply the changes with mutable borrow
        let crafting = guard
            .get_mut(crafting_container_id)
            .expect("crafting container not locked");

        if let Some((remainders, positioned)) = remainders_and_positioned {
            let input = &positioned.input;

            // Iterate over the bounded recipe area, not the whole grid
            for y in 0..input.height {
                for x in 0..input.width {
                    // Calculate the actual slot index in the original crafting grid
                    let grid_slot = positioned.to_grid_slot(x, y, grid_width);

                    // Get the remainder for this position in the trimmed input
                    let remainder_idx = x + y * input.width;
                    let replacement = if remainder_idx < remainders.len() {
                        remainders[remainder_idx].clone()
                    } else {
                        ItemStack::empty()
                    };

                    // Consume one item from the grid slot
                    {
                        let item = crafting.get_item_mut(grid_slot);
                        if !item.is_empty() {
                            if item.count() > 1 {
                                item.set_count(item.count() - 1);
                            } else {
                                *item = ItemStack::empty();
                            }
                        }
                    }

                    // Handle remainder placement
                    if !replacement.is_empty() {
                        let current_item = crafting.get_item(grid_slot).clone();

                        if current_item.is_empty() {
                            // Slot is now empty, place remainder there
                            crafting.set_item(grid_slot, replacement);
                        } else if ItemStack::is_same_item_same_components(
                            &current_item,
                            &replacement,
                        ) {
                            // Same item type, try to stack
                            let new_count = current_item.count() + replacement.count();
                            crafting.get_item_mut(grid_slot).set_count(new_count);
                        } else {
                            // Different item type - need to return to player inventory
                            remainder_overflow.push(replacement);
                        }
                    }
                }
            }
        }

        crafting.set_changed();

        // Update the crafting result based on remaining ingredients
        // Build new input after consuming ingredients
        let items: Vec<ItemStack> = (0..4).map(|i| crafting.get_item(i).clone()).collect();
        let positioned = steel_registry::recipe::CraftingInput::positioned(grid_width, 2, items);
        let result_stack = steel_registry::REGISTRY
            .recipes
            .find_crafting_recipe_2x2(&positioned.input)
            .map_or_else(ItemStack::empty, |r| r.assemble());

        // Update the result container
        guard
            .get_mut(result_container_id)
            .expect("result container not locked")
            .set_item(0, result_stack);

        // Return overflow remainders
        if remainder_overflow.is_empty() {
            None
        } else if remainder_overflow.len() == 1 {
            Some(remainder_overflow.remove(0))
        } else {
            // Multiple different remainders - return the first one
            // The caller should ideally handle multiple remainders, but this is rare
            Some(remainder_overflow.remove(0))
        }
    }

    /// Crafting result slots are "fake" - they don't persist items.
    fn is_fake(&self) -> bool {
        true
    }
}

/// Enum of all slot types that implement the Slot trait.
#[enum_dispatch(Slot)]
pub enum SlotType {
    /// Normal inventory slot with no restrictions.
    Normal(NormalSlot),
    /// Armor slot that only accepts armor items.
    Armor(ArmorSlot),
    /// Crafting grid slot for crafting input.
    CraftingGrid(CraftingGridSlot),
    /// Crafting result slot (fake, doesn't persist items).
    CraftingResult(CraftingResultSlot),
}

impl SlotType {
    /// Returns all container references for this slot.
    /// For most slots this is just one container, but crafting slots
    /// reference both the crafting grid and result containers.
    #[must_use]
    pub fn all_container_refs(&self) -> Vec<ContainerRef> {
        match self {
            SlotType::Normal(s) => vec![s.container_ref()],
            SlotType::Armor(s) => vec![s.container_ref()],
            SlotType::CraftingGrid(s) => vec![s.container_ref(), s.result_container_ref()],
            SlotType::CraftingResult(s) => {
                vec![s.result_container_ref(), s.crafting_container_ref()]
            }
        }
    }
}
