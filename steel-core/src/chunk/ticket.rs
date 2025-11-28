//! `Ticket` module manages chunk tickets and their storage.
use rustc_hash::FxHashMap;
use std::cmp::Ordering;

use steel_utils::ChunkPos;

/// The type of a ticket.
///
/// Variants are ordered by priority (lowest value = highest priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TicketType {
    /// A ticket created by a player. Level is calculated as `31 - view_distance`.
    /// All chunks within view distance are loaded and ticked.
    Player,
    /// A forced ticket.
    Forced,
    /// A light update ticket.
    Light,
    /// A portal ticket.
    Portal,
    /// A ticket created after teleportation.
    PostTeleport,
    /// An unknown ticket type.
    Unknown,
}

/// A ticket that keeps a chunk loaded at a certain level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    /// The type of the ticket.
    pub ticket_type: TicketType,
    /// The level of the ticket.
    pub level: u8,
    /// Expiration time in ticks. `None` means never expires.
    pub expiration: Option<u64>,
}

impl PartialOrd for Ticket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ticket {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower level = Higher priority
        self.level
            .cmp(&other.level)
            .then_with(|| self.ticket_type.cmp(&other.ticket_type))
            .then_with(|| self.expiration.cmp(&other.expiration))
    }
}

/// Manages tickets for chunks.
pub struct TicketStorage {
    tickets: FxHashMap<ChunkPos, Vec<Ticket>>,
    min_cache: FxHashMap<ChunkPos, u8>,
}

impl Default for TicketStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TicketStorage {
    /// Creates a new ticket storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tickets: FxHashMap::default(),
            min_cache: FxHashMap::default(),
        }
    }

    /// Adds a ticket.
    pub fn add_ticket(&mut self, pos: ChunkPos, ticket: Ticket) {
        let tickets = self.tickets.entry(pos).or_default();

        // Match Java behavior: update existing ticket if type and level match
        if let Some(existing) = tickets
            .iter_mut()
            .find(|t| t.ticket_type == ticket.ticket_type && t.level == ticket.level)
        {
            existing.expiration = ticket.expiration;
        } else {
            tickets.push(ticket);
        }

        self.refresh_min_cache(pos);
    }

    /// Removes a ticket.
    pub fn remove_ticket(&mut self, pos: ChunkPos, ticket: &Ticket) {
        if let Some(tickets) = self.tickets.get_mut(&pos) {
            // Match Java behavior: remove based on type and level
            if let Some(index) = tickets
                .iter()
                .position(|t| t.ticket_type == ticket.ticket_type && t.level == ticket.level)
            {
                tickets.swap_remove(index);
            }
            if tickets.is_empty() {
                self.tickets.remove(&pos);
                self.min_cache.remove(&pos);
            } else {
                self.refresh_min_cache(pos);
            }
        }
    }

    /// Gets the lowest (best) level for a chunk.
    #[must_use]
    pub fn get_level(&self, pos: ChunkPos) -> Option<u8> {
        self.min_cache.get(&pos).copied().or_else(|| {
            self.tickets
                .get(&pos)
                .and_then(|t| t.iter().map(|ticket| ticket.level).min())
        })
    }

    /// Gets the cached level without falling back to scanning.
    #[must_use]
    pub fn get_cached_level(&self, pos: ChunkPos) -> Option<u8> {
        self.min_cache.get(&pos).copied()
    }

    /// Purges expired tickets and returns chunks that changed levels.
    pub fn purge_expired(&mut self, current_tick: u64) -> Vec<ChunkPos> {
        let mut changed_chunks = Vec::new();
        let mut empty_entries = Vec::new();

        for (pos, tickets) in &mut self.tickets {
            let original_min = tickets.iter().map(|t| t.level).min();
            let len_before = tickets.len();

            tickets.retain(|t| t.expiration.is_none_or(|exp| exp > current_tick));

            if tickets.len() != len_before {
                let new_min = tickets.iter().map(|t| t.level).min();
                if original_min != new_min {
                    changed_chunks.push(*pos);
                }
            }

            if tickets.is_empty() {
                empty_entries.push(*pos);
                self.min_cache.remove(pos);
            } else if let Some(min_level) = tickets.iter().map(|t| t.level).min() {
                self.min_cache.insert(*pos, min_level);
            }
        }

        for pos in empty_entries {
            self.tickets.remove(&pos);
        }

        changed_chunks
    }

    fn refresh_min_cache(&mut self, pos: ChunkPos) {
        if let Some(tickets) = self.tickets.get(&pos) {
            if let Some(min_level) = tickets.iter().map(|t| t.level).min() {
                self.min_cache.insert(pos, min_level);
            } else {
                self.min_cache.remove(&pos);
            }
        } else {
            self.min_cache.remove(&pos);
        }
    }
}
