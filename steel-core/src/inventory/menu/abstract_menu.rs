//! Abstract container menu implementation.

use rustc_hash::FxHashSet;
use steel_registry::item_stack::ItemStack;

use super::{ClickAction, ClickType, QuickCraftPhase, QuickCraftType};
use crate::inventory::{Container, Slot, slot_ops};

/// Special slot index indicating a click outside the inventory window.
pub const SLOT_CLICKED_OUTSIDE: i16 = -999;

/// The number of slots per row in standard inventory layouts.
pub const SLOTS_PER_ROW: i32 = 9;

/// Standard slot size in pixels (for client rendering).
pub const SLOT_SIZE: i32 = 18;

/// Menu type identifiers matching Minecraft's registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum MenuType {
    Generic9x1 = 0,
    Generic9x2 = 1,
    Generic9x3 = 2,
    Generic9x4 = 3,
    Generic9x5 = 4,
    Generic9x6 = 5,
    Generic3x3 = 6,
    Crafter3x3 = 7,
    Anvil = 8,
    Beacon = 9,
    BlastFurnace = 10,
    BrewingStand = 11,
    Crafting = 12,
    Enchantment = 13,
    Furnace = 14,
    Grindstone = 15,
    Hopper = 16,
    Lectern = 17,
    Loom = 18,
    Merchant = 19,
    ShulkerBox = 20,
    Smithing = 21,
    Smoker = 22,
    CartographyTable = 23,
    Stonecutter = 24,
}

impl MenuType {
    /// Returns the menu type ID for network serialization.
    #[must_use]
    pub const fn id(self) -> i32 {
        self as i32
    }

    /// Creates a MenuType from an ID.
    #[must_use]
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Generic9x1),
            1 => Some(Self::Generic9x2),
            2 => Some(Self::Generic9x3),
            3 => Some(Self::Generic9x4),
            4 => Some(Self::Generic9x5),
            5 => Some(Self::Generic9x6),
            6 => Some(Self::Generic3x3),
            7 => Some(Self::Crafter3x3),
            8 => Some(Self::Anvil),
            9 => Some(Self::Beacon),
            10 => Some(Self::BlastFurnace),
            11 => Some(Self::BrewingStand),
            12 => Some(Self::Crafting),
            13 => Some(Self::Enchantment),
            14 => Some(Self::Furnace),
            15 => Some(Self::Grindstone),
            16 => Some(Self::Hopper),
            17 => Some(Self::Lectern),
            18 => Some(Self::Loom),
            19 => Some(Self::Merchant),
            20 => Some(Self::ShulkerBox),
            21 => Some(Self::Smithing),
            22 => Some(Self::Smoker),
            23 => Some(Self::CartographyTable),
            24 => Some(Self::Stonecutter),
            _ => None,
        }
    }
}

/// A container menu that manages slots and handles player interactions.
///
/// This is the base structure for all container GUIs (chests, furnaces, etc.).
pub struct AbstractContainerMenu<C: Container> {
    /// The menu type for client display.
    pub menu_type: Option<MenuType>,
    /// The container ID for network synchronization.
    pub container_id: i32,
    /// The slots in this menu.
    pub slots: Vec<Slot>,
    /// The item currently being carried by the cursor.
    carried: ItemStack,
    /// State ID for optimistic locking with the client.
    state_id: i32,
    /// Quick-craft state: the distribution type being used.
    quick_craft_type: Option<QuickCraftType>,
    /// Quick-craft state: current phase.
    quick_craft_phase: QuickCraftPhase,
    /// Quick-craft state: slots being dragged over.
    quick_craft_slots: FxHashSet<usize>,
    /// The container this menu operates on.
    pub container: C,
}

impl<C: Container> AbstractContainerMenu<C> {
    /// Creates a new container menu.
    #[must_use]
    pub fn new(menu_type: Option<MenuType>, container_id: i32, container: C) -> Self {
        Self {
            menu_type,
            container_id,
            slots: Vec::new(),
            carried: ItemStack::empty(),
            state_id: 0,
            quick_craft_type: None,
            quick_craft_phase: QuickCraftPhase::Start,
            quick_craft_slots: FxHashSet::default(),
            container,
        }
    }

    /// Adds a slot to this menu.
    pub fn add_slot(&mut self, mut slot: Slot) {
        slot.index = self.slots.len();
        self.slots.push(slot);
    }

    /// Adds standard player inventory slots (main inventory + hotbar).
    ///
    /// Call this after adding the container's own slots.
    pub fn add_player_inventory_slots(&mut self, left: i32, top: i32) {
        // Main inventory (3 rows of 9, slots 9-35 in player inventory)
        for row in 0..3 {
            for col in 0..9 {
                let container_slot = col + (row + 1) * 9;
                self.add_slot(Slot::new(
                    container_slot,
                    left + col as i32 * SLOT_SIZE,
                    top + row as i32 * SLOT_SIZE,
                ));
            }
        }

        // Hotbar (slots 0-8 in player inventory)
        let hotbar_top = top + 3 * SLOT_SIZE + 4; // 4 pixel gap
        for col in 0..9 {
            self.add_slot(Slot::new(col, left + col as i32 * SLOT_SIZE, hotbar_top));
        }
    }

    /// Returns the slot at the given index.
    #[must_use]
    pub fn get_slot(&self, index: usize) -> Option<&Slot> {
        self.slots.get(index)
    }

    /// Returns the item currently being carried by the cursor.
    #[must_use]
    pub fn get_carried(&self) -> &ItemStack {
        &self.carried
    }

    /// Sets the item being carried by the cursor.
    pub fn set_carried(&mut self, item: ItemStack) {
        self.carried = item;
    }

    /// Returns the current state ID.
    #[must_use]
    pub fn state_id(&self) -> i32 {
        self.state_id
    }

    /// Increments and returns the state ID.
    pub fn increment_state_id(&mut self) -> i32 {
        self.state_id = (self.state_id + 1) & 0x7FFF;
        self.state_id
    }

    /// Returns all items in the menu slots.
    #[must_use]
    pub fn get_items(&self) -> Vec<ItemStack> {
        self.slots
            .iter()
            .map(|slot| slot_ops::get_item(slot, &self.container).clone())
            .collect()
    }

    /// Checks if a slot index is valid.
    #[must_use]
    pub fn is_valid_slot_index(&self, slot: i16) -> bool {
        slot == -1
            || slot == SLOT_CLICKED_OUTSIDE
            || (slot >= 0 && (slot as usize) < self.slots.len())
    }

    /// Resets the quick-craft state.
    fn reset_quick_craft(&mut self) {
        self.quick_craft_phase = QuickCraftPhase::Start;
        self.quick_craft_slots.clear();
        self.quick_craft_type = None;
    }

    /// Handles a click on the container.
    #[allow(clippy::too_many_lines)]
    pub fn click(
        &mut self,
        slot_index: i16,
        button: i8,
        click_type: ClickType,
        has_infinite_materials: bool,
    ) {
        match click_type {
            ClickType::QuickCraft => {
                self.handle_quick_craft(slot_index, button, has_infinite_materials);
            }
            _ if self.quick_craft_phase != QuickCraftPhase::Start => {
                // Reset if we get a non-quick-craft click during quick-craft
                self.reset_quick_craft();
            }
            ClickType::Pickup | ClickType::QuickMove => {
                let action = ClickAction::from_button(button);

                if slot_index == SLOT_CLICKED_OUTSIDE {
                    // Drop items outside window
                    if !self.carried.is_empty() {
                        if action == ClickAction::Primary {
                            // Drop all
                            self.carried = ItemStack::empty();
                        } else {
                            // Drop one
                            self.carried.shrink(1);
                        }
                    }
                } else if click_type == ClickType::QuickMove {
                    self.handle_quick_move(slot_index as usize);
                } else {
                    self.handle_pickup(slot_index as usize, action);
                }
            }
            ClickType::Swap => {
                self.handle_swap(slot_index as usize, button as usize);
            }
            ClickType::Clone => {
                if has_infinite_materials && self.carried.is_empty() && slot_index >= 0 {
                    self.handle_clone(slot_index as usize);
                }
            }
            ClickType::Throw => {
                if self.carried.is_empty() && slot_index >= 0 {
                    self.handle_throw(slot_index as usize, button == 1);
                }
            }
            ClickType::PickupAll => {
                if slot_index >= 0 {
                    self.handle_pickup_all(button == 0);
                }
            }
        }
    }

    /// Handles quick-craft (drag) operations.
    fn handle_quick_craft(&mut self, slot_index: i16, button: i8, has_infinite_materials: bool) {
        let Some(phase) = QuickCraftPhase::from_header(button as i32) else {
            self.reset_quick_craft();
            return;
        };

        match phase {
            QuickCraftPhase::Start => {
                if self.carried.is_empty() {
                    self.reset_quick_craft();
                    return;
                }

                let Some(craft_type) = QuickCraftType::from_header(button as i32) else {
                    self.reset_quick_craft();
                    return;
                };

                if !craft_type.is_valid_for_player(has_infinite_materials) {
                    self.reset_quick_craft();
                    return;
                }

                self.quick_craft_type = Some(craft_type);
                self.quick_craft_phase = QuickCraftPhase::Continue;
                self.quick_craft_slots.clear();
            }
            QuickCraftPhase::Continue => {
                if self.quick_craft_phase != QuickCraftPhase::Continue {
                    self.reset_quick_craft();
                    return;
                }

                if slot_index >= 0 && (slot_index as usize) < self.slots.len() {
                    let slot = &self.slots[slot_index as usize];
                    if self.can_item_quick_replace(slot, &self.carried.clone(), true)
                        && slot.may_place(&self.carried)
                    {
                        self.quick_craft_slots.insert(slot_index as usize);
                    }
                }
            }
            QuickCraftPhase::End => {
                if self.quick_craft_phase != QuickCraftPhase::Continue {
                    self.reset_quick_craft();
                    return;
                }

                if self.quick_craft_slots.is_empty() || self.carried.is_empty() {
                    self.reset_quick_craft();
                    return;
                }

                // If only one slot, treat as normal pickup
                if self.quick_craft_slots.len() == 1 {
                    let slot_idx = *self.quick_craft_slots.iter().next().unwrap();
                    self.reset_quick_craft();
                    let action = match self.quick_craft_type {
                        Some(QuickCraftType::Greedy) => ClickAction::Secondary,
                        _ => ClickAction::Primary,
                    };
                    self.handle_pickup(slot_idx, action);
                    return;
                }

                // Distribute items
                self.distribute_quick_craft(has_infinite_materials);
                self.reset_quick_craft();
            }
        }
    }

    /// Distributes items across quick-craft slots.
    fn distribute_quick_craft(&mut self, _has_infinite_materials: bool) {
        let Some(craft_type) = self.quick_craft_type else {
            return;
        };

        let source = self.carried.clone();
        if source.is_empty() {
            return;
        }

        let slot_count = self.quick_craft_slots.len();
        let per_slot = match craft_type {
            QuickCraftType::Charitable => source.count() / slot_count as i32,
            QuickCraftType::Greedy => 1,
            QuickCraftType::Clone => source.max_stack_size(),
        };

        let mut remaining = source.count();
        let slots: Vec<_> = self.quick_craft_slots.iter().copied().collect();

        for slot_idx in slots {
            let slot = &self.slots[slot_idx];
            if !slot.may_place(&source) {
                continue;
            }

            let current = slot_ops::get_item(slot, &self.container);
            let current_count = current.count();
            let max_size = slot
                .max_stack_size_for(&source)
                .min(source.max_stack_size());
            let space = max_size - current_count;
            let to_add = per_slot.min(space).min(remaining);

            if to_add <= 0 {
                continue;
            }

            if current.is_empty() {
                slot_ops::set_item(slot, &mut self.container, source.copy_with_count(to_add));
            } else {
                let current_mut = slot_ops::get_item_mut(slot, &mut self.container);
                current_mut.grow(to_add);
            }

            if craft_type != QuickCraftType::Clone {
                remaining -= to_add;
            }
        }

        if craft_type != QuickCraftType::Clone {
            self.carried.set_count(remaining);
        }
    }

    /// Handles normal pickup (left/right click).
    fn handle_pickup(&mut self, slot_index: usize, action: ClickAction) {
        if slot_index >= self.slots.len() {
            return;
        }

        let slot = &self.slots[slot_index];
        let slot_item = slot_ops::get_item(slot, &self.container).clone();
        let carried = self.carried.clone();

        if slot_item.is_empty() {
            // Slot is empty: place items from cursor
            if !carried.is_empty() && slot.may_place(&carried) {
                let amount = if action == ClickAction::Primary {
                    carried.count()
                } else {
                    1
                };
                let remaining = slot_ops::safe_insert(slot, &mut self.container, carried, amount);
                self.carried = remaining;
            }
        } else if slot.may_pickup() {
            // Slot has items
            if carried.is_empty() {
                // Pick up items
                let amount = if action == ClickAction::Primary {
                    slot_item.count()
                } else {
                    (slot_item.count() + 1) / 2
                };
                let taken = slot_ops::safe_take(slot, &mut self.container, amount, i32::MAX);
                self.carried = taken;
            } else if slot.may_place(&carried) {
                // Try to place or swap
                if ItemStack::is_same_item_same_components(&slot_item, &carried) {
                    // Same item: merge
                    let amount = if action == ClickAction::Primary {
                        carried.count()
                    } else {
                        1
                    };
                    let remaining =
                        slot_ops::safe_insert(slot, &mut self.container, carried, amount);
                    self.carried = remaining;
                } else if carried.count() <= slot.max_stack_size_for(&carried) {
                    // Different item: swap
                    slot_ops::set_item(slot, &mut self.container, carried);
                    self.carried = slot_item;
                }
            } else if ItemStack::is_same_item_same_components(&slot_item, &carried) {
                // Can't place but same item: try to pick up more
                let space = carried.max_stack_size() - carried.count();
                let taken =
                    slot_ops::safe_take(slot, &mut self.container, slot_item.count(), space);
                self.carried.grow(taken.count());
            }
        }
    }

    /// Handles shift-click (quick move).
    ///
    /// This is a virtual method that should be overridden by specific menu implementations.
    fn handle_quick_move(&mut self, slot_index: usize) {
        if slot_index >= self.slots.len() {
            return;
        }

        // Default implementation: do nothing
        // Specific menus override quick_move_stack
        let _ = self.quick_move_stack(slot_index);
    }

    /// Moves a stack from one slot to another section of the menu.
    ///
    /// Override this in specific menu implementations.
    #[must_use]
    pub fn quick_move_stack(&mut self, _slot_index: usize) -> ItemStack {
        ItemStack::empty()
    }

    /// Tries to move an item stack to the given slot range.
    ///
    /// Returns true if any items were moved.
    pub fn move_item_stack_to(
        &mut self,
        item: &mut ItemStack,
        start_slot: usize,
        end_slot: usize,
        backwards: bool,
    ) -> bool {
        let mut moved = false;

        let range: Box<dyn Iterator<Item = usize>> = if backwards {
            Box::new((start_slot..end_slot).rev())
        } else {
            Box::new(start_slot..end_slot)
        };

        // First pass: try to merge with existing stacks
        if item.is_stackable() {
            for slot_idx in range {
                if item.is_empty() {
                    break;
                }

                let slot = &self.slots[slot_idx];
                let target = slot_ops::get_item(slot, &self.container);

                if !target.is_empty() && ItemStack::is_same_item_same_components(item, target) {
                    let max_size = slot.max_stack_size_for(target);
                    let space = max_size - target.count();
                    let to_transfer = item.count().min(space);

                    if to_transfer > 0 {
                        item.shrink(to_transfer);
                        let target_mut = slot_ops::get_item_mut(slot, &mut self.container);
                        target_mut.grow(to_transfer);
                        self.container.set_changed();
                        moved = true;
                    }
                }
            }
        }

        // Second pass: try to place in empty slots
        if !item.is_empty() {
            let range: Box<dyn Iterator<Item = usize>> = if backwards {
                Box::new((start_slot..end_slot).rev())
            } else {
                Box::new(start_slot..end_slot)
            };

            for slot_idx in range {
                if item.is_empty() {
                    break;
                }

                let slot = &self.slots[slot_idx];
                let target = slot_ops::get_item(slot, &self.container);

                if target.is_empty() && slot.may_place(item) {
                    let max_size = slot.max_stack_size_for(item);
                    let to_transfer = item.count().min(max_size);
                    let to_place = item.split(to_transfer);
                    slot_ops::set_item(slot, &mut self.container, to_place);
                    moved = true;
                }
            }
        }

        moved
    }

    /// Handles swap with hotbar slot (number keys).
    fn handle_swap(&mut self, slot_index: usize, hotbar_slot: usize) {
        if slot_index >= self.slots.len() {
            return;
        }

        // For a proper swap, we need access to the player's inventory
        // This is a simplified version that doesn't have that context
        let _ = hotbar_slot;
        // TODO: Implement proper swap with player hotbar
    }

    /// Handles middle-click clone in creative mode.
    fn handle_clone(&mut self, slot_index: usize) {
        if slot_index >= self.slots.len() {
            return;
        }

        let slot = &self.slots[slot_index];
        let item = slot_ops::get_item(slot, &self.container);

        if !item.is_empty() {
            self.carried = item.copy_with_count(item.max_stack_size());
        }
    }

    /// Handles Q key throw.
    fn handle_throw(&mut self, slot_index: usize, throw_all: bool) {
        if slot_index >= self.slots.len() {
            return;
        }

        let slot = &self.slots[slot_index];
        let item = slot_ops::get_item(slot, &self.container);

        if !item.is_empty() && slot.may_pickup() {
            let amount = if throw_all { item.count() } else { 1 };
            let _dropped = slot_ops::safe_take(slot, &mut self.container, amount, i32::MAX);
            // The caller should handle the actual dropping
        }
    }

    /// Handles double-click to collect matching items.
    fn handle_pickup_all(&mut self, forward: bool) {
        if self.carried.is_empty() {
            return;
        }

        let max_size = self.carried.max_stack_size();
        if self.carried.count() >= max_size {
            return;
        }

        let slots_len = self.slots.len();

        // Two passes: first non-full stacks, then full stacks
        for pass in 0..2 {
            let indices: Vec<usize> = if forward {
                (0..slots_len).collect()
            } else {
                (0..slots_len).rev().collect()
            };

            for slot_idx in indices {
                if self.carried.count() >= max_size {
                    break;
                }

                let slot = &self.slots[slot_idx];
                let item = slot_ops::get_item(slot, &self.container);

                // Extract info before mutable borrow
                let item_empty = item.is_empty();
                let item_count = item.count();
                let item_max = item.max_stack_size();
                let matches = ItemStack::is_same_item_same_components(&self.carried, item);
                let can_pickup = slot.may_pickup();

                if !item_empty && matches && can_pickup {
                    let is_full = item_count == item_max;
                    if (pass == 0 && !is_full) || (pass == 1 && is_full) {
                        let space = max_size - self.carried.count();
                        let taken =
                            slot_ops::safe_take(slot, &mut self.container, item_count, space);
                        self.carried.grow(taken.count());
                    }
                }
            }
        }
    }

    /// Checks if an item can be quick-replaced into a slot.
    fn can_item_quick_replace(&self, slot: &Slot, item: &ItemStack, ignore_size: bool) -> bool {
        let slot_item = slot_ops::get_item(slot, &self.container);
        let slot_is_empty = slot_item.is_empty();

        if slot_is_empty {
            return true;
        }

        if !ItemStack::is_same_item_same_components(item, slot_item) {
            return false;
        }

        let additional = if ignore_size { 0 } else { item.count() };
        slot_item.count() + additional <= item.max_stack_size()
    }

    /// Called when the menu is closed.
    pub fn removed(&mut self) {
        // Drop carried item back or into inventory
        if !self.carried.is_empty() {
            // The caller should handle this
            self.carried = ItemStack::empty();
        }
    }

    /// Sets a slot's contents directly (for syncing from network).
    pub fn set_slot(&mut self, slot_index: usize, item: ItemStack) {
        if slot_index < self.slots.len() {
            let slot = &self.slots[slot_index];
            slot_ops::set_item(slot, &mut self.container, item);
        }
    }

    /// Initializes the menu contents from network data.
    pub fn initialize_contents(
        &mut self,
        state_id: i32,
        items: Vec<ItemStack>,
        carried: ItemStack,
    ) {
        self.state_id = state_id;
        for (i, item) in items.into_iter().enumerate() {
            if i < self.slots.len() {
                let slot = &self.slots[i];
                slot_ops::set_item(slot, &mut self.container, item);
            }
        }
        self.carried = carried;
    }
}
