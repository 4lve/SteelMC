//! Manual item behavior assignments for vanilla items.
//!
//! This module is for custom item behaviors that are too complex to auto-generate.
//! Block items are automatically assigned `BlockItemBehavior` by the build script.
//!
//! Add custom behaviors here for items like buckets, food, tools, etc.

use crate::items::ItemRegistry;

// Example: Custom behavior for bucket items
// pub static BUCKET_BEHAVIOR: BucketItemBehavior = BucketItemBehavior;

/// Assigns custom item behaviors that cannot be auto-generated.
/// This is called after the auto-generated `assign_item_behaviors`.
pub fn assign_custom_item_behaviors(_registry: &mut ItemRegistry) {
    // Example usage:
    // registry.set_behavior(&ITEMS.bucket, &BUCKET_BEHAVIOR);
    // registry.set_behavior(&ITEMS.water_bucket, &WATER_BUCKET_BEHAVIOR);

    // TODO: Add custom behavior assignments here
    // For now, all items use either BlockItemBehavior (auto-assigned) or DefaultItemBehavior
}
