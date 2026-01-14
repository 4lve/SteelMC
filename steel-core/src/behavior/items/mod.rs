//! Item behavior implementations.
//!
//! The actual behavior registration is auto-generated from classes.json.
//! See `src/behavior/generated/items.rs` for the generated registration code.

mod block_item;
mod default;

pub use block_item::BlockItemBehavior;
pub use default::DefaultItemBehavior;
