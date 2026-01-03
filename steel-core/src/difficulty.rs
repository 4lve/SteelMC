//! Server difficulty settings.

use serde::Deserialize;

/// The server difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    /// Peaceful - no hostile mobs spawn, health regenerates.
    Peaceful = 0,
    /// Easy - hostile mobs deal less damage.
    Easy = 1,
    /// Normal - default difficulty.
    #[default]
    Normal = 2,
    /// Hard - hostile mobs deal more damage, can break doors.
    Hard = 3,
}

impl Difficulty {
    /// Returns true if this is peaceful difficulty.
    #[inline]
    #[must_use]
    pub const fn is_peaceful(self) -> bool {
        matches!(self, Self::Peaceful)
    }
}
