use super::{FluidEntry, FluidId};
use crate::RegistryExt;
use rustc_hash::FxHashMap;

// TODO: Consider adding fluid tag support when tag system is ready
// TODO: Consider adding iterator methods for fluids (e.g., all_fluids(), filter by property)

pub struct FluidRegistry {
    by_id: FxHashMap<FluidId, FluidEntry>,
    by_name: FxHashMap<&'static str, FluidId>,
    allows_registering: bool,
}

impl FluidRegistry {
    pub fn new() -> Self {
        Self {
            by_id: FxHashMap::default(),
            by_name: FxHashMap::default(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, entry: FluidEntry) {
        if !self.allows_registering {
            panic!("Cannot register fluid after registry is frozen");
        }
        self.by_name.insert(entry.name, entry.id);
        self.by_id.insert(entry.id, entry);
    }

    pub fn get(&self, id: FluidId) -> Option<&FluidEntry> {
        self.by_id.get(&id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<FluidId> {
        self.by_name.get(name).copied()
    }
}

impl RegistryExt for FluidRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}
