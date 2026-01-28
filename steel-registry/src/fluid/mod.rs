pub mod fluid;
pub mod registry;
pub mod vanilla;

pub use fluid::*;
pub use registry::*;

// TODO: Consider moving FluidId constants to vanilla.rs for consistency with other vanilla definitions
// TODO: Add FluidTag support when tag system is fully implemented

impl FluidId {
    pub const Empty: FluidId = FluidId(0);
    pub const Flowing_Water: FluidId = FluidId(1);
    pub const Water: FluidId = FluidId(2);
    pub const Flowing_Lava: FluidId = FluidId(3);
    pub const Lava: FluidId = FluidId(4);
}
