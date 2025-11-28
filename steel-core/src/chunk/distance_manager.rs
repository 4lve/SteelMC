use steel_utils::ChunkPos;

use crate::chunk::{
    chunk_tracker::{ChunkTracker, MAX_LEVEL},
    ticket::{Ticket, TicketStorage, TicketType},
};

/// Manages chunk distances and tickets.
pub struct DistanceManager {
    /// Storage for all chunk tickets.
    pub ticket_storage: TicketStorage,
    /// Tracker for propagating chunk levels.
    pub tracker: ChunkTracker,
}

impl Default for DistanceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DistanceManager {
    /// Creates a new distance manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ticket_storage: TicketStorage::new(),
            tracker: ChunkTracker::new(),
        }
    }

    /// Adds a ticket for a specific chunk.
    pub fn add_ticket(&mut self, pos: ChunkPos, ticket: Ticket) {
        self.ticket_storage.add_ticket(pos, ticket);
        self.update_chunk_tracker(pos);
    }

    /// Removes a ticket from a specific chunk.
    pub fn remove_ticket(&mut self, pos: ChunkPos, ticket: &Ticket) {
        self.ticket_storage.remove_ticket(pos, ticket);
        self.update_chunk_tracker(pos);
    }

    /// Adds a player ticket for chunk loading.
    ///
    /// The ticket level is calculated as `31 - view_distance`, which propagates
    /// outward such that chunks at `view_distance` away reach level 31.
    /// All chunks within view distance are both loaded and ticked (simplified approach).
    pub fn add_player(&mut self, pos: ChunkPos, view_distance: u8) {
        // Level 31 is entity ticking (full chunk).
        // By setting the player's chunk to level (31 - view_distance),
        // chunks at view_distance away will be at level 31.
        let level = 31_u8.saturating_sub(view_distance);
        self.add_ticket(
            pos,
            Ticket {
                ticket_type: TicketType::Player,
                level,
                expiration: None,
            },
        );
    }

    /// Removes a player ticket.
    pub fn remove_player(&mut self, pos: ChunkPos, view_distance: u8) {
        let level = 31_u8.saturating_sub(view_distance);
        let ticket = Ticket {
            ticket_type: TicketType::Player,
            level,
            expiration: None,
        };
        self.remove_ticket(pos, &ticket);
    }

    #[inline]
    fn update_chunk_tracker(&mut self, pos: ChunkPos) {
        let ticket_level = ticket_level_or_max(&self.ticket_storage, pos);
        let ticket_storage = &self.ticket_storage;
        self.tracker.update(pos, ticket_level, |p| {
            ticket_level_or_max(ticket_storage, p)
        });
    }

    /// Runs pending updates and returns a list of changes.
    pub fn run_updates(&mut self) -> Vec<(ChunkPos, u8, u8)> {
        let ticket_storage = &self.ticket_storage;
        self.tracker
            .process_all_updates(|p| ticket_level_or_max(ticket_storage, p))
    }

    /// Purges expired tickets and updates the tracker.
    pub fn purge_tickets(&mut self, current_tick: u64) {
        let changed_chunks = self.ticket_storage.purge_expired(current_tick);
        for pos in changed_chunks {
            self.update_chunk_tracker(pos);
        }
    }
}

#[inline]
fn ticket_level_or_max(storage: &TicketStorage, pos: ChunkPos) -> u8 {
    storage
        .get_cached_level(pos)
        .or_else(|| storage.get_level(pos))
        .unwrap_or(MAX_LEVEL + 1)
}
