use super::{FluidEntry, FluidId};

// TODO: Consider generating this file from vanilla data files instead of hardcoding
// TODO: Add modded fluid constants here when mod support is implemented

pub const EMPTY: FluidEntry = FluidEntry {
    id: FluidId::Empty,
    name: "empty",
};

pub const FLOWING_WATER: FluidEntry = FluidEntry {
    id: FluidId::Flowing_Water,
    name: "flowing_water",
};

pub const WATER: FluidEntry = FluidEntry {
    id: FluidId::Water,
    name: "water",
};

pub const FLOWING_LAVA: FluidEntry = FluidEntry {
    id: FluidId::Flowing_Lava,
    name: "flowing_lava",
};

pub const LAVA: FluidEntry = FluidEntry {
    id: FluidId::Lava,
    name: "lava",
};
