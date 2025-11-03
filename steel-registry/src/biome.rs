use std::collections::HashMap;
use steel_utils::ResourceLocation;

use crate::RegistryExt;

#[derive(Debug)]
pub struct Biome {
    pub key: ResourceLocation,
    pub has_precipitation: bool,
    pub temperature: f32,
    pub downfall: f32,
    pub temperature_modifier: TemperatureModifier,
    pub effects: BiomeEffects,
    pub creature_spawn_probability: f32,
    pub spawners: HashMap<String, Vec<SpawnerData>>,
    pub spawn_costs: HashMap<ResourceLocation, SpawnCost>,
    pub carvers: Vec<ResourceLocation>,
    pub features: Vec<Vec<ResourceLocation>>,
}

#[derive(Debug)]
pub struct BiomeEffects {
    pub fog_color: i32,
    pub sky_color: i32,
    pub water_color: i32,
    pub water_fog_color: i32,
    pub foliage_color: Option<i32>,
    pub grass_color: Option<i32>,
    pub grass_color_modifier: GrassColorModifier,
    pub music: Option<Vec<WeightedMusic>>,
    pub ambient_sound: Option<ResourceLocation>,
    pub additions_sound: Option<AdditionsSound>,
    pub mood_sound: Option<MoodSound>,
    pub particle: Option<Particle>,
}

#[derive(Debug)]
pub struct SpawnerData {
    pub entity_type: ResourceLocation,
    pub weight: i32,
    pub min_count: i32,
    pub max_count: i32,
}

#[derive(Debug)]
pub struct SpawnCost {
    pub energy_budget: f64,
    pub charge: f64,
}

#[derive(Debug, Default)]
pub enum TemperatureModifier {
    #[default]
    None,
    Frozen,
}

#[derive(Debug)]
pub enum GrassColorModifier {
    None,
    DarkForest,
    Swamp,
}

#[derive(Debug)]
pub struct WeightedMusic {
    pub data: Music,
    pub weight: i32,
}

#[derive(Debug)]
pub struct Music {
    pub replace_current_music: bool,
    pub max_delay: i32,
    pub min_delay: i32,
    pub sound: ResourceLocation,
}

#[derive(Debug)]
pub struct AdditionsSound {
    pub sound: ResourceLocation,
    pub tick_chance: f64,
}

#[derive(Debug)]
pub struct MoodSound {
    pub sound: ResourceLocation,
    pub tick_delay: i32,
    pub block_search_extent: i32,
    pub offset: f64,
}

#[derive(Debug)]
pub struct Particle {
    pub options: ParticleOptions,
    pub probability: f32,
}

#[derive(Debug)]
pub struct ParticleOptions {
    pub particle_type: ResourceLocation,
}

pub type BiomeRef = &'static Biome;

pub struct BiomeRegistry {
    biomes_by_id: Vec<BiomeRef>,
    biomes_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl BiomeRegistry {
    pub fn new() -> Self {
        Self {
            biomes_by_id: Vec::new(),
            biomes_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, biome: BiomeRef, key: ResourceLocation) -> usize {
        if !self.allows_registering {
            panic!("Cannot register biomes after the registry has been frozen");
        }

        let id = self.biomes_by_id.len();
        self.biomes_by_key.insert(key, id);
        self.biomes_by_id.push(biome);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<BiomeRef> {
        self.biomes_by_id.get(id).copied()
    }

    pub fn get_id(&self, biome: BiomeRef) -> &usize {
        self.biomes_by_key.get(&biome.key).expect("Biome not found")
    }

    pub fn get(&self, key: &ResourceLocation) -> Option<BiomeRef> {
        self.biomes_by_key.get(key).and_then(|id| self.by_id(*id))
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<BiomeRef> {
        self.get(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, BiomeRef)> + '_ {
        self.biomes_by_id
            .iter()
            .enumerate()
            .map(|(id, &biome)| (id, biome))
    }

    pub fn len(&self) -> usize {
        self.biomes_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.biomes_by_id.is_empty()
    }
}

impl RegistryExt for BiomeRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for BiomeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
