//! Chat message signature tracking for secure chat validation.

use std::collections::VecDeque;
use steel_utils::codec::VarInt;

/// Maximum number of cached signatures (Vanilla: 128)
const MAX_CACHED_SIGNATURES: usize = 128;

/// Maximum number of previous messages to track (Vanilla: 20)
const MAX_PREVIOUS_MESSAGES: usize = 20;

/// Tracks the last seen message signatures by a player
#[derive(Debug, Clone, Default)]
pub struct LastSeen(Vec<Box<[u8]>>);

impl LastSeen {
    /// Gets the underlying vector of signatures
    #[must_use]
    pub fn as_slice(&self) -> &[Box<[u8]>] {
        &self.0
    }

    /// Gets the number of tracked signatures
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Checks if there are no tracked signatures
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Message signature cache for a player
#[derive(Debug)]
pub struct MessageCache {
    /// Max 128 cached message signatures. Most recent FIRST.
    /// Server should (when possible) reference indexes in this (recipient's) cache
    /// instead of sending full signatures in last seen.
    /// Must be 1:1 with client's signature cache.
    full_cache: VecDeque<Box<[u8]>>,

    /// Max 20 last seen messages by the sender. Most Recent LAST
    pub last_seen: LastSeen,
}

impl Default for MessageCache {
    fn default() -> Self {
        Self {
            full_cache: VecDeque::with_capacity(MAX_CACHED_SIGNATURES),
            last_seen: LastSeen::default(),
        }
    }
}

impl MessageCache {
    /// Creates a new message cache
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cache signatures from senders that the recipient hasn't seen yet.
    /// Not used for caching seen messages. Only for non-indexed signatures from senders.
    pub fn cache_signatures(&mut self, signatures: &[Box<[u8]>]) {
        for sig in signatures.iter().rev() {
            if self.full_cache.contains(sig) {
                continue;
            }
            // If the cache is maxed, and someone sends a signature older than the oldest in cache, ignore it
            if self.full_cache.len() < MAX_CACHED_SIGNATURES {
                self.full_cache.push_back(sig.clone()); // Recipient never saw this message so it must be older than the oldest in cache
            }
        }
    }

    /// Adds a seen signature to `last_seen` and `full_cache`.
    pub fn add_seen_signature(&mut self, signature: &[u8]) {
        if self.last_seen.0.len() >= MAX_PREVIOUS_MESSAGES {
            self.last_seen.0.remove(0);
        }
        self.last_seen.0.push(signature.into());

        // This probably doesn't need to be a loop, but better safe than sorry
        while self.full_cache.len() >= MAX_CACHED_SIGNATURES {
            self.full_cache.pop_back();
        }
        self.full_cache.push_front(signature.into()); // Since recipient saw this message it will be most recent in cache
    }

    /// Convert the sender's `last_seen` signatures to IDs if the recipient has them in their cache.
    /// Otherwise, the full signature is sent. (ID:0 indicates full signature is being sent)
    #[must_use]
    pub fn index_previous_messages(
        &self,
        sender_last_seen: &LastSeen,
    ) -> Box<[crate::player::PreviousMessageEntry]> {
        let mut indexed = Vec::new();
        for signature in sender_last_seen.as_slice() {
            let index = self.full_cache.iter().position(|s| s == signature);

            if let Some(index) = index {
                indexed.push(crate::player::PreviousMessageEntry {
                    // Send ID reference to recipient's cache (index + 1 because 0 is reserved for full signature)
                    id: VarInt(1 + index as i32),
                    signature: None,
                });
            } else {
                indexed.push(crate::player::PreviousMessageEntry {
                    // Send ID as 0 for full signature
                    id: VarInt(0),
                    signature: Some(signature.clone()),
                });
            }
        }
        indexed.into_boxed_slice()
    }
}
