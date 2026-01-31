//! Fluid tag identifiers for checking fluid types.
//!
//! These match vanilla's FluidTags.WATER and FluidTags.LAVA.

use steel_utils::Identifier;

/// Empty fluid ID (0) - represents the absence of fluid.
/// Note: This is not a real fluid tag, but a convention for empty blocks.
pub const EMPTY: u8 = 0;

/// Returns the water fluid tag identifier.
#[must_use]
pub fn water() -> Identifier {
    Identifier::vanilla_static("water")
}

/// Returns the lava fluid tag identifier.
#[must_use]
pub fn lava() -> Identifier {
    Identifier::vanilla_static("lava")
}
