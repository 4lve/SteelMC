//! Terminal progress display for spawn chunk generation.
//!
//! Shows a colored ANSI grid with real-time chunk generation progress.

use std::io::{Result, Write};

use crate::logger::Input;
use crate::spawn_progress::{DISPLAY_DIAMETER, DISPLAY_RADIUS};
use crossterm::{
    cursor::{MoveRight, MoveUp},
    style::{Color::Rgb, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use steel_core::chunk::chunk_access::ChunkStatus;

/// Grid type alias for convenience.
pub type Grid = [[Option<ChunkStatus>; DISPLAY_DIAMETER]; DISPLAY_DIAMETER];

// ---------------------------------------------------------------------------
// SpawnProgressDisplay
// ---------------------------------------------------------------------------

/// Returns the vanilla RGB color for a chunk status.
/// Colors are taken from `LevelLoadingScreen.COLORS` in the vanilla client.
const fn status_color(status: Option<ChunkStatus>) -> (u8, u8, u8) {
    match status {
        None | Some(ChunkStatus::Empty) => (84, 84, 84),
        Some(ChunkStatus::StructureStarts) => (153, 153, 153),
        Some(ChunkStatus::StructureReferences) => (95, 97, 145),
        Some(ChunkStatus::Biomes) => (128, 178, 82),
        Some(ChunkStatus::Noise) => (209, 209, 209),
        Some(ChunkStatus::Surface) => (114, 104, 9),
        Some(ChunkStatus::Carvers) => (48, 53, 114),
        Some(ChunkStatus::Features) => (33, 198, 0),
        Some(ChunkStatus::InitializeLight) => (204, 204, 204),
        Some(ChunkStatus::Light) => (255, 224, 160),
        Some(ChunkStatus::Spawn) => (242, 96, 96),
        Some(ChunkStatus::Full) => (255, 255, 255),
    }
}

/// Terminal progress display showing a colored grid of chunk generation statuses.
pub struct SpawnProgressDisplay {
    grid: Grid,
    /// If the progress is being displayed
    pub rendered: bool,
}

impl SpawnProgressDisplay {
    /// Creates a new display with all cells unloaded (black).
    pub fn new() -> Self {
        Self {
            grid: [[None; DISPLAY_DIAMETER]; DISPLAY_DIAMETER],
            rendered: false,
        }
    }

    /// Updates the internal grid state.
    pub fn set_grid(&mut self, new_grid: &Grid) {
        self.grid = *new_grid;
    }
}
impl Input {
    pub fn render_current_spawn(&mut self) -> Result<()> {
        write!(
            self.out,
            "{}\n{}",
            MoveUp(DISPLAY_RADIUS as u16 + 2),
            Clear(ClearType::FromCursorDown)
        )?;
        let w = if let Ok((w, _)) = terminal::size() {
            w / 2 - DISPLAY_RADIUS as u16 - 1
        } else {
            0
        };
        for z in (0..DISPLAY_DIAMETER).step_by(2) {
            writeln!(self.out)?;
            if w != 0 {
                write!(self.out, "{}", MoveRight(w))?;
            }
            for x in 0..DISPLAY_DIAMETER {
                let (tr, tg, tb) = status_color(self.spawn_display.grid[z][x]);
                if z + 1 < DISPLAY_DIAMETER {
                    let (br, bg, bb) = status_color(self.spawn_display.grid[z + 1][x]);
                    write!(
                        self.out,
                        "{}{}▀",
                        SetForegroundColor(Rgb {
                            r: tr,
                            g: tg,
                            b: tb
                        }),
                        SetBackgroundColor(Rgb {
                            r: br,
                            g: bg,
                            b: bb
                        })
                    )?;
                } else {
                    write!(
                        self.out,
                        "{}▀",
                        SetForegroundColor(Rgb {
                            r: tr,
                            g: tg,
                            b: tb
                        }),
                    )?;
                }
            }
            writeln!(self.out, "{ResetColor}")?;
            self.out.flush()?;
        }
        write!(self.out, "\r")?;
        let pos = self.get_current_pos();
        self.cursor_to((0, 0), pos)?;
        self.rewrite_current_input()?;
        Ok(())
    }
}
