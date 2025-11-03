use std::collections::HashMap;
use steel_utils::ResourceLocation;
use steel_utils::text::TextComponent;

use crate::RegistryExt;

/// Represents a jukebox song definition from a data pack JSON file.
#[derive(Debug)]
pub struct JukeboxSong {
    pub key: ResourceLocation,
    pub sound_event: ResourceLocation,
    pub description: TextComponent,
    pub length_in_seconds: f32,
    pub comparator_output: i32,
}

pub type JukeboxSongRef = &'static JukeboxSong;

pub struct JukeboxSongRegistry {
    jukebox_songs_by_id: Vec<JukeboxSongRef>,
    jukebox_songs_by_key: HashMap<ResourceLocation, usize>,
    allows_registering: bool,
}

impl JukeboxSongRegistry {
    pub fn new() -> Self {
        Self {
            jukebox_songs_by_id: Vec::new(),
            jukebox_songs_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, jukebox_song: JukeboxSongRef) -> usize {
        if !self.allows_registering {
            panic!("Cannot register jukebox songs after the registry has been frozen");
        }

        let id = self.jukebox_songs_by_id.len();
        self.jukebox_songs_by_key
            .insert(jukebox_song.key.clone(), id);
        self.jukebox_songs_by_id.push(jukebox_song);
        id
    }

    pub fn by_id(&self, id: usize) -> Option<JukeboxSongRef> {
        self.jukebox_songs_by_id.get(id).copied()
    }

    pub fn get_id(&self, jukebox_song: JukeboxSongRef) -> &usize {
        self.jukebox_songs_by_key
            .get(&jukebox_song.key)
            .expect("Jukebox song not found")
    }

    pub fn by_key(&self, key: &ResourceLocation) -> Option<JukeboxSongRef> {
        self.jukebox_songs_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, JukeboxSongRef)> + '_ {
        self.jukebox_songs_by_id
            .iter()
            .enumerate()
            .map(|(id, &song)| (id, song))
    }

    pub fn len(&self) -> usize {
        self.jukebox_songs_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jukebox_songs_by_id.is_empty()
    }
}

impl RegistryExt for JukeboxSongRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for JukeboxSongRegistry {
    fn default() -> Self {
        Self::new()
    }
}
