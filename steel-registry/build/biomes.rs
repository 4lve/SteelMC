use std::{collections::HashMap, fs};

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::{Deserialize, Serialize};
use steel_utils::ResourceLocation;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum VecOrSingle<T>
where
    T: Clone,
{
    Vec(Vec<T>),
    Single(T),
}

impl<T> VecOrSingle<T>
where
    T: Clone,
{
    fn into_vec(self) -> Vec<T> {
        match self {
            VecOrSingle::Vec(v) => v,
            VecOrSingle::Single(s) => vec![s],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BiomeJson {
    has_precipitation: bool,
    temperature: f32,
    downfall: f32,

    #[serde(default)]
    temperature_modifier: TemperatureModifier,

    effects: BiomeEffects,

    #[serde(default)]
    creature_spawn_probability: f32,
    #[serde(default)]
    spawners: HashMap<String, Vec<SpawnerData>>,
    #[serde(default)]
    spawn_costs: HashMap<ResourceLocation, SpawnCost>,

    carvers: VecOrSingle<ResourceLocation>,
    features: Vec<Vec<ResourceLocation>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BiomeEffects {
    fog_color: i32,
    sky_color: i32,
    water_color: i32,
    water_fog_color: i32,

    #[serde(default)]
    foliage_color: Option<i32>,
    #[serde(default)]
    grass_color: Option<i32>,

    #[serde(default)]
    grass_color_modifier: GrassColorModifier,

    #[serde(default)]
    music: Option<Vec<WeightedMusic>>,

    #[serde(default)]
    ambient_sound: Option<ResourceLocation>,
    #[serde(default)]
    additions_sound: Option<AdditionsSound>,
    #[serde(default)]
    mood_sound: Option<MoodSound>,
    #[serde(default)]
    particle: Option<Particle>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpawnerData {
    #[serde(rename = "type")]
    entity_type: ResourceLocation,
    weight: i32,
    #[serde(rename = "minCount")]
    min_count: i32,
    #[serde(rename = "maxCount")]
    max_count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpawnCost {
    energy_budget: f64,
    charge: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureModifier {
    #[default]
    None,
    Frozen,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum GrassColorModifier {
    #[default]
    None,
    DarkForest,
    Swamp,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WeightedMusic {
    data: Music,
    weight: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Music {
    replace_current_music: bool,
    max_delay: i32,
    min_delay: i32,
    sound: ResourceLocation,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdditionsSound {
    sound: ResourceLocation,
    tick_chance: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MoodSound {
    sound: ResourceLocation,
    tick_delay: i32,
    block_search_extent: i32,
    offset: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Particle {
    options: ParticleOptions,
    probability: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ParticleOptions {
    #[serde(rename = "type")]
    particle_type: ResourceLocation,
}

fn generate_temperature_modifier(modifier: &TemperatureModifier) -> TokenStream {
    match modifier {
        TemperatureModifier::None => quote! { TemperatureModifier::None },
        TemperatureModifier::Frozen => quote! { TemperatureModifier::Frozen },
    }
}

fn generate_grass_color_modifier(modifier: &GrassColorModifier) -> TokenStream {
    match modifier {
        GrassColorModifier::None => quote! { GrassColorModifier::None },
        GrassColorModifier::DarkForest => quote! { GrassColorModifier::DarkForest },
        GrassColorModifier::Swamp => quote! { GrassColorModifier::Swamp },
    }
}

fn generate_resource_location(resource: &ResourceLocation) -> TokenStream {
    let namespace = resource.namespace.as_ref();
    let path = resource.path.as_ref();
    quote! { ResourceLocation { namespace: Cow::Borrowed(#namespace), path: Cow::Borrowed(#path) } }
}

fn generate_option<T, F>(opt: &Option<T>, f: F) -> TokenStream
where
    F: FnOnce(&T) -> TokenStream,
{
    match opt {
        Some(val) => {
            let inner = f(val);
            quote! { Some(#inner) }
        }
        None => quote! { None },
    }
}

fn generate_vec<T, F>(vec: &[T], f: F) -> TokenStream
where
    F: Fn(&T) -> TokenStream,
{
    let items: Vec<_> = vec.iter().map(f).collect();
    quote! { vec![#(#items),*] }
}

fn generate_hashmap_string<T, F>(map: &HashMap<String, T>, f: F) -> TokenStream
where
    F: Fn(&T) -> TokenStream,
{
    let entries: Vec<_> = map
        .iter()
        .map(|(k, v)| {
            let val = f(v);
            quote! { (#k.to_string(), #val) }
        })
        .collect();
    quote! { HashMap::from([#(#entries),*]) }
}

fn generate_hashmap_resource<T, F>(map: &HashMap<ResourceLocation, T>, f: F) -> TokenStream
where
    F: Fn(&T) -> TokenStream,
{
    let entries: Vec<_> = map
        .iter()
        .map(|(k, v)| {
            let key = generate_resource_location(k);
            let val = f(v);
            quote! { (#key, #val) }
        })
        .collect();
    quote! { HashMap::from([#(#entries),*]) }
}

fn generate_spawner_data(data: &SpawnerData) -> TokenStream {
    let entity_type = generate_resource_location(&data.entity_type);
    let weight = data.weight;
    let min_count = data.min_count;
    let max_count = data.max_count;

    quote! {
        SpawnerData {
            entity_type: #entity_type,
            weight: #weight,
            min_count: #min_count,
            max_count: #max_count,
        }
    }
}

fn generate_spawn_cost(cost: &SpawnCost) -> TokenStream {
    let energy_budget = cost.energy_budget;
    let charge = cost.charge;

    quote! {
        SpawnCost {
            energy_budget: #energy_budget,
            charge: #charge,
        }
    }
}

fn generate_particle(particle: &Particle) -> TokenStream {
    let particle_type = generate_resource_location(&particle.options.particle_type);
    let probability = particle.probability;

    quote! {
        Particle {
            options: ParticleOptions {
                particle_type: #particle_type,
            },
            probability: #probability,
        }
    }
}

fn generate_mood_sound(mood: &MoodSound) -> TokenStream {
    let sound = generate_resource_location(&mood.sound);
    let tick_delay = mood.tick_delay;
    let block_search_extent = mood.block_search_extent;
    let offset = mood.offset;

    quote! {
        MoodSound {
            sound: #sound,
            tick_delay: #tick_delay,
            block_search_extent: #block_search_extent,
            offset: #offset,
        }
    }
}

fn generate_additions_sound(additions: &AdditionsSound) -> TokenStream {
    let sound = generate_resource_location(&additions.sound);
    let tick_chance = additions.tick_chance;

    quote! {
        AdditionsSound {
            sound: #sound,
            tick_chance: #tick_chance,
        }
    }
}

fn generate_music(music: &Music) -> TokenStream {
    let replace_current_music = music.replace_current_music;
    let max_delay = music.max_delay;
    let min_delay = music.min_delay;
    let sound = generate_resource_location(&music.sound);

    quote! {
        Music {
            replace_current_music: #replace_current_music,
            max_delay: #max_delay,
            min_delay: #min_delay,
            sound: #sound,
        }
    }
}

fn generate_weighted_music(weighted: &WeightedMusic) -> TokenStream {
    let data = generate_music(&weighted.data);
    let weight = weighted.weight;

    quote! {
        WeightedMusic {
            data: #data,
            weight: #weight,
        }
    }
}

fn generate_biome_effects(effects: &BiomeEffects) -> TokenStream {
    let fog_color = effects.fog_color;
    let sky_color = effects.sky_color;
    let water_color = effects.water_color;
    let water_fog_color = effects.water_fog_color;
    let foliage_color = generate_option(&effects.foliage_color, |&v| quote! { #v });
    let grass_color = generate_option(&effects.grass_color, |&v| quote! { #v });
    let grass_color_modifier = generate_grass_color_modifier(&effects.grass_color_modifier);
    let music = generate_option(&effects.music, |m| generate_vec(m, generate_weighted_music));
    let ambient_sound = generate_option(&effects.ambient_sound, generate_resource_location);
    let additions_sound = generate_option(&effects.additions_sound, generate_additions_sound);
    let mood_sound = generate_option(&effects.mood_sound, generate_mood_sound);
    let particle = generate_option(&effects.particle, generate_particle);

    quote! {
        BiomeEffects {
            fog_color: #fog_color,
            sky_color: #sky_color,
            water_color: #water_color,
            water_fog_color: #water_fog_color,
            foliage_color: #foliage_color,
            grass_color: #grass_color,
            grass_color_modifier: #grass_color_modifier,
            music: #music,
            ambient_sound: #ambient_sound,
            additions_sound: #additions_sound,
            mood_sound: #mood_sound,
            particle: #particle,
        }
    }
}

pub(crate) fn build() -> TokenStream {
    println!(
        "cargo:rerun-if-changed=build_assets/builtin_datapacks/minecraft/data/minecraft/worldgen/biome/"
    );

    let biome_dir = "build_assets/builtin_datapacks/minecraft/data/minecraft/worldgen/biome";
    let mut biomes = Vec::new();

    // Read all biome JSON files
    for entry in fs::read_dir(biome_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let biome_name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let content = fs::read_to_string(&path).unwrap();
            let biome: BiomeJson = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", biome_name, e));

            biomes.push((biome_name, biome));
        }
    }

    // Sort biomes by name for consistent generation
    biomes.sort_by(|a, b| a.0.cmp(&b.0));

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::biome::{
            Biome, BiomeEffects, BiomeRegistry, TemperatureModifier, GrassColorModifier,
            SpawnerData, SpawnCost, WeightedMusic, Music, AdditionsSound, MoodSound,
            Particle, ParticleOptions,
        };
        use steel_utils::ResourceLocation;
        use std::borrow::Cow;
        use std::sync::LazyLock;
        use std::collections::HashMap;
    });

    // Generate static biome definitions
    for (biome_name, biome) in &biomes {
        let biome_ident = Ident::new(&biome_name.to_shouty_snake_case(), Span::call_site());
        let biome_name_str = biome_name.clone();

        let key = quote! { ResourceLocation::vanilla_static(#biome_name_str) };
        let has_precipitation = biome.has_precipitation;
        let temperature = biome.temperature;
        let downfall = biome.downfall;
        let temperature_modifier = generate_temperature_modifier(&biome.temperature_modifier);
        let effects = generate_biome_effects(&biome.effects);
        let creature_spawn_probability = biome.creature_spawn_probability;
        let spawners =
            generate_hashmap_string(&biome.spawners, |v| generate_vec(v, generate_spawner_data));
        let spawn_costs = generate_hashmap_resource(&biome.spawn_costs, generate_spawn_cost);
        let carvers = generate_vec(
            &biome.carvers.clone().into_vec(),
            generate_resource_location,
        );
        let features = generate_vec(&biome.features, |inner_vec| {
            generate_vec(inner_vec, generate_resource_location)
        });

        stream.extend(quote! {
            pub static #biome_ident: LazyLock<Biome> = LazyLock::new(|| Biome {
                key: #key,
                has_precipitation: #has_precipitation,
                temperature: #temperature,
                downfall: #downfall,
                temperature_modifier: #temperature_modifier,
                effects: #effects,
                creature_spawn_probability: #creature_spawn_probability,
                spawners: #spawners,
                spawn_costs: #spawn_costs,
                carvers: #carvers,
                features: #features,
            });
        });
    }

    // Generate registration function
    let mut register_stream = TokenStream::new();
    for (biome_name, _) in &biomes {
        let biome_ident = Ident::new(&biome_name.to_shouty_snake_case(), Span::call_site());
        register_stream.extend(quote! {
            registry.register(&#biome_ident, #biome_ident.key.clone());
        });
    }

    stream.extend(quote! {
        pub fn register_biomes(registry: &mut BiomeRegistry) {
            #register_stream
        }
    });

    stream
}
