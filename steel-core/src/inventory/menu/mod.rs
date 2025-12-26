//! Container menu system.

mod abstract_menu;
mod chest_menu;
mod click_type;
mod inventory_menu;

pub use abstract_menu::{AbstractContainerMenu, MenuType, SLOT_SIZE, SLOTS_PER_ROW};
pub use chest_menu::ChestMenu;
pub use click_type::{
    ClickAction, ClickType, QuickCraftPhase, QuickCraftType, make_quick_craft_mask,
};
pub use inventory_menu::{InventoryMenu, InventoryMenuContainer, INVENTORY_MENU_CONTAINER_ID, slots as inv_slots};

