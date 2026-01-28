// TODO: Consider adding fluid properties to FluidEntry when FluidProperty system is designed
//       (e.g., viscosity, temperature, etc.)

/// Fluid ID - uses raw registry ID (u16) to match vanilla
/// Vanilla IDs:
///   0 = Empty
///   1 = Flowing_Water
///   2 = Water
///   3 = Flowing_Lava
///   4 = Lava
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FluidId(pub u16);

impl FluidId {
    /// Returns true if this is the empty fluid
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

pub struct FluidEntry {
    pub id: FluidId,
    pub name: &'static str,
}
