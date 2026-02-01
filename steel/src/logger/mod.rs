use crate::logger::history::History;
use crate::logger::selection::Selection;
#[cfg(feature = "spawn_chunk_display")]
use crate::logger::spawn_progress::{Grid, SpawnProgressDisplay};
use crate::logger::suggestions::Completer;
use crate::{STEEL_CONFIG, logger::output::Output};
use chrono::Utc;
use crossterm::{
    cursor::{MoveLeft, MoveUp},
    style::{
        Attribute,
        Color::{self, DarkGrey},
        ResetColor, SetAttribute, SetForegroundColor,
    },
    terminal::{self, Clear, ClearType},
};
use std::time;
use std::{
    fmt::Write as _,
    io::{Result, Write},
    sync::Arc,
};
use steel_utils::locks::AsyncRwLock;
use steel_utils::logger::{Level, LogData, STEEL_LOGGER, SteelLogger};
use tokio::{sync::mpsc, task};
use tokio_util::sync::CancellationToken;
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

mod history;
mod input;
mod output;
mod selection;
#[cfg(feature = "spawn_chunk_display")]
mod spawn_progress;
mod suggestions;

enum Move {
    None,
    Up,
    Down,
}

struct LogState {
    pub out: Output,
    pub completion: Completer,
    pub history: History,
    pub selection: Selection,
    #[cfg(feature = "spawn_chunk_display")]
    pub spawn_display: SpawnProgressDisplay,
    pub cancel_token: CancellationToken,
}
impl LogState {
    async fn new(path: &'static str, cancel_token: CancellationToken) -> Self {
        LogState {
            out: Output::new(),
            completion: Completer::new(),
            history: History::new(path).await,
            #[cfg(feature = "spawn_chunk_display")]
            spawn_display: SpawnProgressDisplay::new(),
            selection: Selection::new(),
            cancel_token,
        }
    }
}

/// Modify input
impl LogState {
    fn push(&mut self, string: String) -> Result<()> {
        if self.out.is_at_start() {
            self.out.text.insert_str(0, &string);
        } else {
            let (pos, char) = self.out.char_pos(self.out.pos.saturating_sub(1));
            self.out.text.insert_str(pos + char, &string);
        }
        let string_len = string.chars().count();
        let length = self.out.length + string_len;
        let pos = self.out.pos + string_len;
        self.completion.update(&mut self.out, pos);
        self.rewrite_input(length, pos)
    }
    fn replace_push(&mut self, string: String) -> Result<()> {
        if self.out.is_at_end() {
            let (pos, char) = self.out.char_pos(self.out.pos.saturating_sub(1));
            self.out.text.insert_str(pos + char, &string);
        } else {
            let (pos, char) = self.out.char_pos(self.out.pos);
            self.out.text.replace_range(pos..pos + char, &string);
        }
        let string_len = string.chars().count();
        let length = if self.out.is_at_end() {
            self.out.length + string_len
        } else {
            self.out.length + string_len.saturating_sub(1)
        };
        let pos = self.out.pos + string_len;
        self.completion.update(&mut self.out, pos);
        self.rewrite_input(length, pos)
    }
    fn pop_before(&mut self) -> Result<()> {
        if self.out.is_at_start() {
            return Ok(());
        }
        let (pos, _) = self.out.char_pos(self.out.pos.saturating_sub(1));
        self.out.text.remove(pos);
        let length = self.out.length - 1;
        let pos = self.out.pos - 1;
        self.completion.update(&mut self.out, pos);
        self.rewrite_input(length, pos)
    }
    fn pop_after(&mut self) -> Result<()> {
        if self.out.is_at_end() {
            return Ok(());
        }
        let (pos, _) = self.out.char_pos(self.out.pos);
        self.out.text.remove(pos);
        let length = self.out.length - 1;
        let pos = self.out.pos;
        self.completion.update(&mut self.out, pos);
        self.rewrite_input(length, pos)
    }
    fn delete_selection(&mut self) -> Result<()> {
        if !self.selection.is_active() {
            return Ok(());
        }
        let range = self.selection.get_range();
        let start = range.start;
        let end = range.end;

        // Find byte positions for the character indices
        let byte_start = self.out.char_pos(start).0;
        let char_end = self.out.char_pos(end.saturating_sub(1));
        let byte_end = char_end.0 + char_end.1;

        // Remove the selected text
        self.out.text.replace_range(byte_start..byte_end, "");

        // Update position and length
        let new_length = self.out.length - (end - start);
        let new_pos = start;
        self.selection.clear();

        // Update suggestions
        self.completion.update(&mut self.out, new_pos);
        self.rewrite_input(new_length, new_pos)
    }
    fn reset(&mut self) -> Result<()> {
        self.out.text = String::new();
        self.completion.enabled = false;
        self.completion.selected = 0;
        self.completion.update(&mut self.out, 0);
        self.history.pos = 0;
        self.rewrite_input(0, 0)
    }
}

impl LogState {
    pub fn rewrite_current_input(&mut self) -> Result<()> {
        self.rewrite_input(self.out.length, self.out.pos)
    }
    pub fn rewrite_input(&mut self, length: usize, pos: usize) -> Result<()> {
        self.out.cursor_to(self.out.get_current_pos(), (0, 0))?;

        // Build the output string with selection highlighting
        let output = if self.selection.is_active() {
            let range = self.selection.get_range();
            let start = range.start;
            let end = range.end;

            let mut result = String::new();
            let mut ended = false;
            for (i, ch) in self.out.text.chars().enumerate() {
                if i == start {
                    write!(result, "{}", SetAttribute(Attribute::Reverse)).ok();
                }
                if i == end {
                    ended = true;
                    write!(result, "{}", SetAttribute(Attribute::NoReverse)).ok();
                }
                result.push(ch);
            }
            if !ended {
                write!(result, "{}", SetAttribute(Attribute::NoReverse)).ok();
            }
            result
        } else {
            self.out.text.clone()
        };

        let end_correction = if let Ok((w, _)) = terminal::size()
            && (length + 2).is_multiple_of(w as usize)
        {
            format!(" {}", MoveLeft(1))
        } else {
            String::new()
        };
        let input_color = if self.completion.error {
            SetForegroundColor(Color::Red)
        } else {
            SetForegroundColor(Color::White)
        };
        write!(
            self.out,
            "{}> {input_color}{}{end_correction}{ResetColor}",
            Clear(ClearType::FromCursorDown),
            output,
        )?;

        self.out.length = length;
        self.out.pos = pos;
        self.out
            .cursor_to(self.out.get_end(), self.out.get_current_pos())?;
        self.out.flush()?;
        if self.completion.enabled {
            self.completion.rewrite(&mut self.out, Move::None)?;
        }
        Ok(())
    }
}

/// A logger implementation with commands suggestions
pub struct CommandLogger {
    input: Arc<AsyncRwLock<LogState>>,
    sender: mpsc::UnboundedSender<(Level, LogData)>,
    cancel_token: CancellationToken,
}
impl CommandLogger {
    /// Initializes the `CommandLogger`
    pub async fn init(
        history_path: &'static str,
        cancel_token: CancellationToken,
    ) -> Option<Arc<Self>> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let log_cancel_token = CancellationToken::new();

        let log = Arc::new(Self {
            input: Arc::new(AsyncRwLock::const_new(
                LogState::new(history_path, cancel_token).await,
            )),
            sender,
            cancel_token: log_cancel_token.clone(),
        });
        task::spawn(log.clone().log_loop(receiver));
        task::spawn(log.clone().input_main());
        STEEL_LOGGER.set(log.clone()).ok()?;
        Some(log)
    }
    /// Stops the logger and it's subprocesses
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
    async fn log_loop(self: Arc<Self>, mut receiver: mpsc::UnboundedReceiver<(Level, LogData)>) {
        loop {
            #[cfg(feature = "spawn_chunk_display")]
            if self.input.read().await.spawn_display.rendered {
                continue;
            }
            tokio::select! {
                biased;
                Some((lvl, data)) = receiver.recv() => {
                    let mut input = self.input.write().await;
                    let pos = input.out.get_current_pos();
                    if let Err(err) = input.out.cursor_to(pos, (0, 0)) {
                        log::error!("{err}");
                    }
                    if let Err(err) = writeln!(input.out,
                        "{}{}{lvl} {}{}{}\r",
                        Clear(ClearType::FromCursorDown),
                        if STEEL_CONFIG.log.as_ref().is_some_and(|l| l.time) {
                            let time: chrono::DateTime<Utc> = time::SystemTime::now().into();
                            format!("{} ", time.format("%T:%3f"))
                        } else {
                            String::new()
                        },
                        if STEEL_CONFIG.log.as_ref().is_some_and(|l| l.module_path) {
                            format!(" {}{}{}",
                                SetForegroundColor(DarkGrey),
                                data.module_path,
                                ResetColor
                            )
                        } else {
                            String::new()
                        },
                        data.message,
                        if STEEL_CONFIG.log.as_ref().is_some_and(|l| l.extra) {
                            format!("{}{}{}",
                                SetForegroundColor(DarkGrey),
                                data.extra,
                                ResetColor
                            )
                        } else {
                            String::new()
                        },
                    ) {
                        log::error!("{err}");
                    }
                    if let Err(err) = input.out.cursor_to((0, 0), pos) {
                        log::error!("{err}");
                    }
                    if let Err(err) = input.rewrite_current_input() {
                        log::error!("{err}");
                    }
                }
                () = self.cancel_token.cancelled() => break,
            }
        }
    }
}

impl SteelLogger for CommandLogger {
    fn log(&self, lvl: Level, data: LogData) {
        self.sender.send((lvl, data)).ok();
    }
}

/// A logger layer for tracing
pub struct LoggerLayer(pub Arc<CommandLogger>);
impl LoggerLayer {
    /// Creates a new logger
    pub async fn new(history_path: &'static str, cancel_token: CancellationToken) -> Option<Self> {
        Some(Self(CommandLogger::init(history_path, cancel_token).await?))
    }
}
#[cfg(feature = "spawn_chunk_display")]
impl CommandLogger {
    /// Initializes the display of the spawn chunks
    pub async fn activate_spawn_display(&self) -> Result<()> {
        use crate::spawn_progress::DISPLAY_RADIUS;

        let mut input = self.input.write().await;
        input.spawn_display.rendered = true;
        let pos = input.out.get_current_pos();
        input.out.cursor_to(pos, (0, 0))?;
        write!(input.out, "\r{}", Clear(ClearType::FromCursorDown))?;
        for _ in 0..DISPLAY_RADIUS {
            writeln!(input.out)?;
        }
        input.out.cursor_to((0, 0), pos)?;
        input.out.flush()?;
        input.rewrite_current_input()?;
        Ok(())
    }
    /// Ends the spawn display cleaning the screen
    pub async fn deactivate_spawn_display(&self) {
        use crate::spawn_progress::DISPLAY_RADIUS;

        let mut input = self.input.write().await;
        write!(
            input.out,
            "{}\n{}",
            MoveUp(DISPLAY_RADIUS as u16 + 2),
            Clear(ClearType::FromCursorDown)
        )
        .ok();
        input.rewrite_current_input().ok();
        input.spawn_display.rendered = false;
    }
    /// Updates the spawn grid, and displays it if required
    pub async fn update_spawn_grid(&self, grid: &Grid, should_render: bool) -> Result<()> {
        let mut state = self.input.write().await;
        state.spawn_display.set_grid(grid);
        if !should_render {
            return Ok(());
        }
        {
            let state = &mut state as &mut LogState;
            state.spawn_display.rewrite(&mut state.out)?;
        }
        state.rewrite_current_input()
    }
}

impl<S: Subscriber> Layer<S> for LoggerLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut data = LogData::new();
        event.record(&mut data);

        self.0.log(Level::Tracing(*event.metadata().level()), data);
    }
}
