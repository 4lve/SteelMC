//! Spawn chunk generation with optional terminal progress display.
//!
//! During server startup, generates chunks around the spawn position until
//! the 7×7 Full area is complete. When the `spawn_chunk_display` feature is
//! enabled, a colored ANSI grid shows real-time progress including the
//! surrounding dependency rings.
//!
//! The [`SwitchableWriter`] (display-only) replaces the default tracing
//! writer so that log lines appear above the grid without disturbing it.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::sleep;

#[cfg(feature = "slow_chunk_gen")]
use std::sync::atomic::Ordering;
use steel_core::chunk::chunk_access::ChunkStatus;
#[cfg(feature = "slow_chunk_gen")]
use steel_core::chunk::chunk_holder::SLOW_CHUNK_GEN;
use steel_core::chunk::chunk_ticket_manager::MAX_VIEW_DISTANCE;
use steel_core::server::Server;
use steel_utils::{ChunkPos, SectionPos};

#[cfg(feature = "spawn_chunk_display")]
use std::io::{self, IsTerminal, Write};

#[cfg(feature = "spawn_chunk_display")]
use steel_core::chunk::chunk_pyramid::GENERATION_PYRAMID;
#[cfg(feature = "spawn_chunk_display")]
use steel_utils::locks::SyncMutex;
#[cfg(feature = "spawn_chunk_display")]
use tracing_subscriber::fmt::MakeWriter;

/// Vanilla spawn chunk radius — chunks within this radius reach Full status.
const SPAWN_RADIUS: i32 = 3;
/// Number of chunks that must reach Full status (7×7).
const TOTAL_SPAWN_CHUNKS: usize = ((SPAWN_RADIUS * 2 + 1) * (SPAWN_RADIUS * 2 + 1)) as usize;

/// Dependency margin: how many extra chunk rings are needed around the Full
/// area for the generation pipeline (structure refs, features, lighting, etc.).
#[cfg(feature = "spawn_chunk_display")]
const MARGIN: i32 = GENERATION_PYRAMID
    .get_step_to(ChunkStatus::Full)
    .accumulated_dependencies
    .get_radius_of(ChunkStatus::Empty) as i32;
/// Display radius: Full radius + dependency margin.
#[cfg(feature = "spawn_chunk_display")]
const DISPLAY_RADIUS: i32 = SPAWN_RADIUS + MARGIN;
/// Display grid diameter (covers Full chunks + all dependency chunks).
#[cfg(feature = "spawn_chunk_display")]
const DISPLAY_DIAMETER: usize = (DISPLAY_RADIUS * 2 + 1) as usize;

// ---------------------------------------------------------------------------
// SwitchableWriter — tracing writer with progress display (feature-gated)
// ---------------------------------------------------------------------------

/// A tracing writer that can redirect output through a [`SpawnProgressDisplay`].
///
/// When the display is not activated, output goes directly to stderr.
/// When activated, log lines are rendered above the progress grid.
///
/// Internally reference-counted — cloning is cheap and shares the same state.
#[cfg(feature = "spawn_chunk_display")]
#[derive(Clone)]
pub struct SwitchableWriter {
    inner: Arc<SyncMutex<Option<SpawnProgressDisplay>>>,
}

#[cfg(feature = "spawn_chunk_display")]
impl Default for SwitchableWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "spawn_chunk_display")]
impl SwitchableWriter {
    /// Creates a new writer in normal (stderr) mode.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SyncMutex::new(None)),
        }
    }

    /// Activates the progress display. Log output will be routed through it.
    fn activate_display(&self, display: SpawnProgressDisplay) {
        *self.inner.lock() = Some(display);
    }

    /// Deactivates the progress display, erasing the grid from the terminal.
    fn deactivate_display(&self) {
        if let Some(mut display) = self.inner.lock().take() {
            display.erase_final();
        }
    }

    /// Updates the grid and re-renders (only if display is active).
    fn update_grid(&self, grid: &[[Option<ChunkStatus>; DISPLAY_DIAMETER]; DISPLAY_DIAMETER]) {
        if let Some(display) = self.inner.lock().as_mut() {
            display.update_grid(grid);
        }
    }
}

#[cfg(feature = "spawn_chunk_display")]
impl<'a> MakeWriter<'a> for SwitchableWriter {
    type Writer = SwitchableWriteTarget;

    fn make_writer(&'a self) -> Self::Writer {
        SwitchableWriteTarget {
            inner: Arc::clone(&self.inner),
            buffer: Vec::with_capacity(256),
        }
    }
}

/// Per-log-event writer that buffers the formatted line and flushes on drop.
#[cfg(feature = "spawn_chunk_display")]
pub struct SwitchableWriteTarget {
    inner: Arc<SyncMutex<Option<SpawnProgressDisplay>>>,
    buffer: Vec<u8>,
}

#[cfg(feature = "spawn_chunk_display")]
impl Write for SwitchableWriteTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "spawn_chunk_display")]
impl Drop for SwitchableWriteTarget {
    fn drop(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let mut inner = self.inner.lock();
        if let Some(display) = inner.as_mut() {
            display.write_log_line(&self.buffer);
        } else {
            drop(inner);
            let _ = io::stderr().write_all(&self.buffer);
        }
    }
}

// ---------------------------------------------------------------------------
// SpawnProgressDisplay — terminal grid rendering (feature-gated)
// ---------------------------------------------------------------------------

/// Returns the vanilla RGB color for a chunk status.
/// Colors are taken from `LevelLoadingScreen.COLORS` in the vanilla client.
#[cfg(feature = "spawn_chunk_display")]
const fn status_color(status: Option<ChunkStatus>) -> (u8, u8, u8) {
    match status {
        None => (0, 0, 0),
        Some(ChunkStatus::Empty) => (84, 84, 84),
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
#[cfg(feature = "spawn_chunk_display")]
struct SpawnProgressDisplay {
    grid: [[Option<ChunkStatus>; DISPLAY_DIAMETER]; DISPLAY_DIAMETER],
    rendered: bool,
}

#[cfg(feature = "spawn_chunk_display")]
impl SpawnProgressDisplay {
    /// Creates a new display with all cells unloaded (black).
    fn new() -> Self {
        Self {
            grid: [[None; DISPLAY_DIAMETER]; DISPLAY_DIAMETER],
            rendered: false,
        }
    }

    /// Erases the grid from the terminal by moving the cursor up and clearing lines.
    fn erase(&self, out: &mut impl Write) {
        if !self.rendered {
            return;
        }
        let term_lines = (DISPLAY_DIAMETER + 1) / 2;
        for _ in 0..term_lines {
            let _ = write!(out, "\x1b[1A\x1b[2K");
        }
    }

    /// Renders the grid to the given writer (appends new lines).
    /// Uses half-block characters to render 2 rows per terminal line.
    fn render(&self, out: &mut impl Write) {
        for z in (0..DISPLAY_DIAMETER).step_by(2) {
            for x in 0..DISPLAY_DIAMETER {
                let (tr, tg, tb) = status_color(self.grid[z][x]);
                let (br, bg, bb) = if z + 1 < DISPLAY_DIAMETER {
                    status_color(self.grid[z + 1][x])
                } else {
                    (0, 0, 0)
                };
                // ▀ = upper half block: foreground is top row, background is bottom row
                let _ = write!(out, "\x1b[38;2;{tr};{tg};{tb}m\x1b[48;2;{br};{bg};{bb}m▀");
            }
            let _ = writeln!(out, "\x1b[0m");
        }
    }

    /// Overwrites the grid in-place (moves cursor up, rewrites each line).
    /// Uses half-block characters to render 2 rows per terminal line.
    fn render_overwrite(&self, out: &mut impl Write) {
        let term_lines = (DISPLAY_DIAMETER + 1) / 2;
        let _ = write!(out, "\x1b[{term_lines}A");
        for z in (0..DISPLAY_DIAMETER).step_by(2) {
            let _ = write!(out, "\r");
            for x in 0..DISPLAY_DIAMETER {
                let (tr, tg, tb) = status_color(self.grid[z][x]);
                let (br, bg, bb) = if z + 1 < DISPLAY_DIAMETER {
                    status_color(self.grid[z + 1][x])
                } else {
                    (0, 0, 0)
                };
                let _ = write!(out, "\x1b[38;2;{tr};{tg};{tb}m\x1b[48;2;{br};{bg};{bb}m▀");
            }
            let _ = writeln!(out, "\x1b[0m\x1b[K");
        }
    }

    /// Updates the grid state and re-renders if anything changed.
    fn update_grid(
        &mut self,
        new_grid: &[[Option<ChunkStatus>; DISPLAY_DIAMETER]; DISPLAY_DIAMETER],
    ) {
        if self.grid == *new_grid && self.rendered {
            return;
        }
        self.grid = *new_grid;
        let mut out = io::stderr().lock();
        if self.rendered {
            self.render_overwrite(&mut out);
        } else {
            self.render(&mut out);
        }
        let _ = out.flush();
        self.rendered = true;
    }

    /// Erases the grid, writes a log line, then re-renders the grid.
    fn write_log_line(&mut self, line: &[u8]) {
        let mut out = io::stderr().lock();
        self.erase(&mut out);
        let _ = out.write_all(line);
        self.render(&mut out);
        let _ = out.flush();
        self.rendered = true;
    }

    /// Fully erases the grid from the terminal (for cleanup).
    fn erase_final(&mut self) {
        if !self.rendered {
            return;
        }
        let mut out = io::stderr().lock();
        self.erase(&mut out);
        let _ = out.flush();
        self.rendered = false;
    }
}

// ---------------------------------------------------------------------------
// Spawn chunk generation
// ---------------------------------------------------------------------------

/// Generates spawn chunks, optionally displaying progress in the terminal.
///
/// Adds a ticket at the world spawn position so that a 7×7 area of chunks
/// reaches `Full` status. The generation system is pumped in a loop until
/// completion. With the `spawn_chunk_display` feature, progress is shown as
/// a colored terminal grid that includes the surrounding dependency chunks.
pub async fn generate_spawn_chunks(
    server: &Arc<Server>,
    #[cfg(feature = "spawn_chunk_display")] writer: &SwitchableWriter,
) {
    let world = &server.worlds[0];

    let spawn_pos = world.level_data.read().data().spawn_pos();
    let center_chunk = ChunkPos::new(
        SectionPos::block_to_section_coord(spawn_pos.0.x),
        SectionPos::block_to_section_coord(spawn_pos.0.z),
    );

    log::info!(
        "Preparing spawn area: {TOTAL_SPAWN_CHUNKS} chunks around chunk ({}, {})",
        center_chunk.0.x,
        center_chunk.0.y,
    );

    #[cfg(feature = "spawn_chunk_display")]
    let use_display = io::stderr().is_terminal();

    #[cfg(feature = "spawn_chunk_display")]
    if use_display {
        writer.activate_display(SpawnProgressDisplay::new());
    }

    // Add a ticket at the center chunk. Ticket level MAX_VIEW_DISTANCE - SPAWN_RADIUS
    // ensures that chunks within radius SPAWN_RADIUS reach Full status:
    //   center: level 29, is_full(29) = true
    //   distance 3: level 32, is_full(32) = true (32 <= MAX_VIEW_DISTANCE)
    //   distance 4: level 33, is_full(33) = false
    let ticket_level = MAX_VIEW_DISTANCE - SPAWN_RADIUS as u8;
    {
        let mut tickets = world.chunk_map.chunk_tickets.lock();
        tickets.add_ticket(center_chunk, ticket_level);
    }

    #[cfg(feature = "slow_chunk_gen")]
    SLOW_CHUNK_GEN.store(true, Ordering::Relaxed);

    let start = Instant::now();
    let mut tick_count: u64 = 1; // Start at 1 to avoid 0 % N == 0 triggering debug logs

    #[cfg(feature = "spawn_chunk_display")]
    let mut prev_grid = [[None; DISPLAY_DIAMETER]; DISPLAY_DIAMETER];
    #[cfg(feature = "spawn_chunk_display")]
    let mut last_render = Instant::now();

    loop {
        // Drive chunk ticket propagation and generation task scheduling
        world.chunk_map.tick_b(tick_count, 0, false);

        let mut completed = 0;

        // With the display feature, poll the full display area (spawn + dependencies)
        // and update the terminal grid. Without it, only poll the spawn area.
        #[cfg(feature = "spawn_chunk_display")]
        {
            let mut grid = [[None; DISPLAY_DIAMETER]; DISPLAY_DIAMETER];

            for dz in -DISPLAY_RADIUS..=DISPLAY_RADIUS {
                for dx in -DISPLAY_RADIUS..=DISPLAY_RADIUS {
                    let pos = ChunkPos::new(center_chunk.0.x + dx, center_chunk.0.y + dz);
                    let status = world
                        .chunk_map
                        .chunks
                        .read_sync(&pos, |_, holder| holder.persisted_status())
                        .flatten();

                    let gx = (dx + DISPLAY_RADIUS) as usize;
                    let gz = (dz + DISPLAY_RADIUS) as usize;
                    grid[gz][gx] = status;

                    if dx.abs() <= SPAWN_RADIUS
                        && dz.abs() <= SPAWN_RADIUS
                        && status == Some(ChunkStatus::Full)
                    {
                        completed += 1;
                    }
                }
            }

            // Throttle display updates to ~20fps to avoid terminal glitching
            if use_display && grid != prev_grid {
                prev_grid = grid;
                if last_render.elapsed() >= Duration::from_millis(50) {
                    writer.update_grid(&grid);
                    last_render = Instant::now();
                }
            }
        }

        #[cfg(not(feature = "spawn_chunk_display"))]
        {
            for dz in -SPAWN_RADIUS..=SPAWN_RADIUS {
                for dx in -SPAWN_RADIUS..=SPAWN_RADIUS {
                    let pos = ChunkPos::new(center_chunk.0.x + dx, center_chunk.0.y + dz);
                    let status = world
                        .chunk_map
                        .chunks
                        .read_sync(&pos, |_, holder| holder.persisted_status())
                        .flatten();

                    if status == Some(ChunkStatus::Full) {
                        completed += 1;
                    }
                }
            }
        }

        if completed == TOTAL_SPAWN_CHUNKS {
            break;
        }

        // Yield to allow async chunk generation tasks to make progress
        sleep(Duration::from_millis(10)).await;
        tick_count += 1;
    }

    #[cfg(feature = "slow_chunk_gen")]
    SLOW_CHUNK_GEN.store(false, Ordering::Relaxed);

    let elapsed = start.elapsed();

    #[cfg(feature = "spawn_chunk_display")]
    if use_display {
        // Render final state in case the last update was throttled
        writer.update_grid(&prev_grid);
        writer.deactivate_display();
    }

    log::info!(
        "Spawn area prepared: {TOTAL_SPAWN_CHUNKS} chunks in {:.2}s",
        elapsed.as_secs_f64(),
    );
}
