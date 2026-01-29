//! Fluid tag identifiers for checking fluid types.
//!
//! These match vanilla's FluidTags.WATER and FluidTags.LAVA.

use steel_utils::Identifier;

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
