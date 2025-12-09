use std::collections::HashMap;
use steel_utils::Identifier;

use crate::RegistryExt;

/// Represents a timeline definition from a data pack JSON file.
#[derive(Debug)]
pub struct Timeline {
    pub key: Identifier,
}

pub type TimelineRef = &'static Timeline;

pub struct TimelineRegistry {
    timelines_by_id: Vec<TimelineRef>,
    timelines_by_key: HashMap<Identifier, usize>,
    tags: HashMap<Identifier, Vec<TimelineRef>>,
    allows_registering: bool,
}

impl TimelineRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            timelines_by_id: Vec::new(),
            timelines_by_key: HashMap::new(),
            tags: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, timeline: TimelineRef) -> usize {
        assert!(
            self.allows_registering,
            "Cannot register timelines after the registry has been frozen"
        );

        let id = self.timelines_by_id.len();
        self.timelines_by_key.insert(timeline.key.clone(), id);
        self.timelines_by_id.push(timeline);
        id
    }

    #[must_use]
    pub fn by_id(&self, id: usize) -> Option<TimelineRef> {
        self.timelines_by_id.get(id).copied()
    }

    #[must_use]
    pub fn get_id(&self, timeline: TimelineRef) -> &usize {
        self.timelines_by_key
            .get(&timeline.key)
            .expect("Timeline not found")
    }

    #[must_use]
    pub fn by_key(&self, key: &Identifier) -> Option<TimelineRef> {
        self.timelines_by_key
            .get(key)
            .and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, TimelineRef)> + '_ {
        self.timelines_by_id
            .iter()
            .enumerate()
            .map(|(id, &timeline)| (id, timeline))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.timelines_by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.timelines_by_id.is_empty()
    }

    // Tag-related methods

    /// Registers a tag with a list of timeline keys.
    /// Timeline keys that don't exist in the registry are silently skipped.
    pub fn register_tag(&mut self, tag: Identifier, timeline_keys: &[&'static str]) {
        assert!(
            self.allows_registering,
            "Cannot register tags after registry has been frozen"
        );

        let timelines: Vec<TimelineRef> = timeline_keys
            .iter()
            .filter_map(|key| self.by_key(&Identifier::vanilla_static(key)))
            .collect();

        self.tags.insert(tag, timelines);
    }

    /// Gets all timelines in a tag.
    #[must_use]
    pub fn get_tag(&self, tag: &Identifier) -> Option<&[TimelineRef]> {
        self.tags.get(tag).map(std::vec::Vec::as_slice)
    }

    /// Iterates over all timelines in a tag.
    pub fn iter_tag(&self, tag: &Identifier) -> impl Iterator<Item = TimelineRef> + '_ {
        self.get_tag(tag)
            .map(|timelines| timelines.iter().copied())
            .into_iter()
            .flatten()
    }

    /// Returns an iterator over all tag keys.
    pub fn tag_keys(&self) -> impl Iterator<Item = &Identifier> {
        self.tags.keys()
    }
}

impl RegistryExt for TimelineRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for TimelineRegistry {
    fn default() -> Self {
        Self::new()
    }
}
