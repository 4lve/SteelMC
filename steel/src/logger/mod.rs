use crate::STEEL_CONFIG;
use crate::logger::selection::Selection;
#[cfg(feature = "spawn_chunk_display")]
use crate::logger::spawn_progress::{Grid, SpawnProgressDisplay};
use crate::{SERVER, logger::history::History};
use chrono::Utc;
use crossterm::{
    cursor::{
        MoveLeft, MoveRight, MoveUp,
        SetCursorStyle::{BlinkingBar, BlinkingBlock},
    },
    style::{
        Color::{DarkGrey, Yellow},
        ResetColor, SetForegroundColor,
    },
    terminal,
};
use std::time;
use std::{
    io::{Result, Stdout, Write, stdout},
    sync::Arc,
};
use steel_core::command::sender::CommandSender;
use steel_utils::locks::AsyncRwLock;
use steel_utils::logger::{Level, LogData, STEEL_LOGGER, SteelLogger};
use tokio::{sync::mpsc, task};
use tokio_util::sync::CancellationToken;
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

mod history;
mod input;
mod selection;
#[cfg(feature = "spawn_chunk_display")]
mod spawn_progress;

struct Input {
    pub text: String,
    pub length: usize,
    pub pos: usize,
    pub replace: bool,
    pub completion: Completer,
    pub history: History,
    pub selection: Selection,
    #[cfg(feature = "spawn_chunk_display")]
    pub spawn_display: SpawnProgressDisplay,
    pub out: Stdout,
    pub cancel_token: CancellationToken,
}
impl Input {
    async fn new(path: &'static str, cancel_token: CancellationToken) -> Self {
        Input {
            text: String::new(),
            length: 0,
            pos: 0,
            replace: false,
            completion: Completer::new(),
            history: History::new(path).await,
            #[cfg(feature = "spawn_chunk_display")]
            spawn_display: SpawnProgressDisplay::new(),
            out: stdout(),
            selection: Selection::new(),
            cancel_token,
        }
    }
    fn push(&mut self, string: String) -> Result<()> {
        let string_len = string.chars().count();
        if self.pos == 0 {
            self.text.insert_str(0, &string);
        } else {
            let Some((pos, char)) = self.text.char_indices().nth(self.pos - 1) else {
                return Ok(());
            };
            self.text.insert_str(pos + char.len_utf8(), &string);
        }
        let length = self.length + string_len;
        let pos = self.pos + string_len;
        self.update_suggestion_list(pos);
        self.rewrite_input(length, pos)?;
        Ok(())
    }
    fn replace(&mut self, string: String) -> Result<()> {
        let string_len = string.chars().count();
        if self.pos == 0 {
            if self.is_empty() {
                self.text = string;
            } else {
                self.text = format!("{}{}", string, &self.text[1..]);
            }
        } else {
            let Some((pos, char)) = self.text.char_indices().nth(self.pos - 1) else {
                return Ok(());
            };
            if self.is_at_end() {
                self.text.insert_str(pos + char.len_utf8(), &string);
            } else {
                self.text
                    .replace_range(pos + char.len_utf8()..=pos + char.len_utf8(), &string);
            }
        }
        let length = if self.is_at_end() {
            self.length + string_len
        } else {
            self.length + string_len.saturating_sub(1)
        };
        let pos = self.pos + string_len;
        self.update_suggestion_list(pos);
        self.rewrite_input(length, pos)?;
        Ok(())
    }
    fn pop_back(&mut self) -> Result<()> {
        if !self.is_at_start() {
            let Some((pos, _)) = self.text.char_indices().nth(self.pos - 1) else {
                return Ok(());
            };
            self.text.remove(pos);
            let length = self.length - 1;
            let pos = self.pos - 1;
            self.update_suggestion_list(pos);
            self.rewrite_input(length, pos)?;
        }
        Ok(())
    }
    fn pop_front(&mut self) -> Result<()> {
        if !self.is_at_end() {
            if self.pos == 0 {
                self.text.remove(0);
            } else {
                let Some((pos, char)) = self.text.char_indices().nth(self.pos - 1) else {
                    return Ok(());
                };
                self.text.remove(pos + char.len_utf8());
            }
            let length = self.length - 1;
            let pos = self.pos;
            self.update_suggestion_list(pos);
            self.rewrite_input(length, pos)?;
        }
        Ok(())
    }
    fn is_empty(&self) -> bool {
        self.length == 0
    }
    fn is_at_start(&self) -> bool {
        self.pos == 0
    }
    fn is_at_end(&self) -> bool {
        self.pos == self.length
    }
    fn reset(&mut self) -> Result<()> {
        self.text = String::new();
        self.rewrite_input(0, 0)?;
        self.completion.enabled = false;
        Ok(())
    }
    fn delete_selection(&mut self) -> Result<()> {
        if !self.selection.is_active() {
            return Ok(());
        }
        let range = self.selection.get_range();
        let start = range.start;
        let end = range.end;

        // Find byte positions for the character indices
        let char_indices: Vec<(usize, char)> = self.text.char_indices().collect();

        let byte_start = char_indices[start].0;
        let byte_end = if end < char_indices.len() {
            char_indices[end].0
        } else {
            self.text.len()
        };

        // Remove the selected text
        self.text.replace_range(byte_start..byte_end, "");

        // Update position and length
        let new_length = self.length - (end - start);
        let new_pos = start;

        // Update suggestions
        self.update_suggestion_list(new_pos);

        self.selection.clear();
        self.rewrite_input(new_length, new_pos)?;
        Ok(())
    }
}

impl Input {
    fn get_pos(pos: usize) -> (usize, usize) {
        if let Ok((w, _)) = terminal::size() {
            let w = w as usize;
            let absolute_pos = pos + 2;
            let x = absolute_pos % w;
            let y = absolute_pos / w;
            // When x is at 0, we're actually at the last position of the previous y
            // if x == 0 {
            //     return (y - 1, w);
            // }
            return (y, x);
        }
        (0, pos + 2)
    }
    fn get_current_pos(&self) -> (usize, usize) {
        Self::get_pos(self.pos)
    }
    fn get_end(&self) -> (usize, usize) {
        Self::get_pos(self.length)
    }
    fn cursor_to(&mut self, from: (usize, usize), to: (usize, usize)) -> Result<()> {
        if from.0 > to.0 {
            write!(self.out, "\x1b[{}A", from.0 - to.0)?;
        } else if to.0 > from.0 {
            write!(self.out, "\x1b[{}B", to.0 - from.0)?;
        }
        if from.1 > to.1 {
            write!(self.out, "\x1b[{}D", from.1 - to.1)?;
        } else if to.1 > from.1 {
            write!(self.out, "\x1b[{}C", to.1 - from.1)?;
        }
        Ok(())
    }
}

struct Completer {
    pub enabled: bool,
    pub error: bool,
    pub selected: usize,
    pub completed: String,
    pub suggestions: Vec<String>,
}
impl Completer {
    fn new() -> Self {
        Completer {
            enabled: false,
            error: false,
            selected: 0,
            completed: String::new(),
            suggestions: vec![],
        }
    }
}
impl Input {
    pub fn update_suggestion_list(&mut self, pos: usize) {
        let char_start = if self.text.is_empty() {
            0
        } else {
            let (start, size) = self
                .text
                .char_indices()
                .nth(pos.saturating_sub(1))
                .expect("Input position out of range!");
            start + size.len_utf8()
        };
        // Gets the right chars
        let command = &self.text[..char_start];

        let Some(server) = SERVER.get() else {
            self.completion.completed = String::new();
            self.completion.selected = 0;
            self.completion.error = true;
            return;
        };
        // Gets the suggested commands
        self.completion.suggestions = server
            .command_dispatcher
            .read()
            .handle_suggestions(CommandSender::Console, command, server.clone())
            .0
            .into_iter()
            .map(|suggestion| suggestion.text)
            .collect();
        if self.completion.suggestions.is_empty() {
            self.completion.completed = String::new();
            self.completion.selected = 0;
            self.completion.error = true;
        } else {
            self.completion.error = false;
        }
    }
    fn update_completion(&mut self, update: i8) -> Result<()> {
        // Goes to the end
        self.cursor_to(self.get_current_pos(), self.get_end())?;
        // Clears
        write!(self.out, "\x1b[J")?;
        let text = if self.completion.suggestions.is_empty() {
            self.cursor_to(self.get_end(), self.get_current_pos())?;
            self.out.flush()?;
            return Ok(());
        } else {
            // Updates completion position
            self.completion.selected = if update < 0 {
                (self.completion.selected + self.completion.suggestions.len() - (-update) as usize)
                    % self.completion.suggestions.len()
            } else {
                (self.completion.selected + update as usize) % self.completion.suggestions.len()
            };
            let width = if let Ok((width, _)) = terminal::size() {
                width as usize / 20
            } else {
                1
            };
            let grid_size = width * 3;
            let start = (self.completion.selected / grid_size) * grid_size;
            let mut height = 0u16;
            'outer: for w in 0..width {
                for h in 0..3 {
                    let pos = start + w * 3 + h;
                    if pos >= self.completion.suggestions.len() {
                        break 'outer;
                    }

                    write!(self.out, "\n\r")?;
                    if w != 0 {
                        write!(self.out, "{}", MoveRight(w as u16 * 20))?;
                    }

                    let color = if pos == self.completion.selected {
                        Yellow
                    } else {
                        DarkGrey
                    };

                    write!(
                        self.out,
                        "{}{:<20}{}",
                        SetForegroundColor(color),
                        if self.completion.suggestions[pos].len() > 20 {
                            format!("{}...", &self.completion.suggestions[pos][..17])
                        } else {
                            self.completion.suggestions[pos].clone()
                        },
                        ResetColor
                    )?;
                    height += 1;
                }
                write!(self.out, "{}", MoveUp(3))?;
                height = 0;
            }
            let y = height + self.get_end().0 as u16;
            let x = self.get_current_pos().1;
            if y != 0 {
                write!(self.out, "{}", MoveUp(y))?;
            }
            write!(self.out, "\r{}", MoveRight(x as u16))?;

            if let Some(text) = self.text[..self.pos].split_whitespace().last()
                && let Some(striped) =
                    self.completion.suggestions[self.completion.selected].strip_prefix(text)
            {
                striped
            } else {
                &self.completion.suggestions[self.completion.selected]
            }
        };
        self.completion.completed = text.to_string();
        self.out.flush()?;

        if !self.is_at_end() {
            return Ok(());
        }
        write!(
            self.out,
            "\x1b[s{}{}\x1b[u",
            SetForegroundColor(DarkGrey),
            &self.completion.completed
        )?;
        self.out.flush()?;
        Ok(())
    }

    pub fn rewrite_current_input(&mut self) -> Result<()> {
        let length = self.length;
        let pos = self.pos;
        self.rewrite_input(length, pos)
    }

    pub fn rewrite_input(&mut self, length: usize, pos: usize) -> Result<()> {
        self.cursor_to(self.get_current_pos(), (0, 0))?;
        if self.replace {
            write!(self.out, "{BlinkingBlock}")?;
        } else {
            write!(self.out, "{BlinkingBar}")?;
        }

        // Build the output string with selection highlighting
        let output = if self.selection.is_active() {
            let range = self.selection.get_range();
            let start = range.start;
            let end = range.end;

            let chars: Vec<char> = self.text.chars().collect();
            let mut result = String::new();
            let mut ended = false;

            for (i, ch) in chars.iter().enumerate() {
                if i == start {
                    result.push_str("\x1b[7m"); // Start inverse video
                }
                if i == end {
                    ended = true;
                    result.push_str("\x1b[27m"); // End inverse video
                }
                result.push(*ch);
            }
            if !ended {
                result.push_str("\x1b[27m"); // End inverse video
            }
            result
        } else {
            self.text.clone()
        };

        write!(
            self.out,
            "\x1b[J\x1b[27m> {}{}{}\x1b[0m",
            if self.completion.error {
                "\x1b[0;31m"
            } else {
                ""
            },
            output,
            if let Ok((w, _)) = terminal::size()
                && (length + 2).is_multiple_of(w as usize)
            {
                format!(" {}", MoveLeft(1))
            } else {
                String::new()
            }
        )?;
        self.length = length;
        self.pos = pos;
        self.cursor_to(self.get_end(), self.get_current_pos())?;
        self.out.flush()?;
        if self.completion.enabled {
            self.update_completion(0)?;
        }
        Ok(())
    }
}

/// A logger implementation with commands suggestions
pub struct CommandLogger {
    input: Arc<AsyncRwLock<Input>>,
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
                Input::new(history_path, cancel_token).await,
            )),
            sender,
            cancel_token: log_cancel_token.clone(),
        });
        task::spawn(log.clone().log_loop(receiver));
        task::spawn(log.clone().input_main(log_cancel_token));
        STEEL_LOGGER.set(log.clone()).ok()?;
        Some(log)
    }
    /// Stops the logger and it's subprocesses
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
    async fn log_loop(self: Arc<Self>, mut receiver: mpsc::UnboundedReceiver<(Level, LogData)>) {
        loop {
            tokio::select! {
                biased;
                Some((lvl, data)) = receiver.recv() => {
                    #[cfg(feature = "spawn_chunk_display")]
                    if self.input.read().await.spawn_display.rendered {
                        continue;
                    }
                    let mut input = self.input.write().await;
                    let pos = input.get_current_pos();
                    if let Err(err) = input.cursor_to(pos, (0, 0)) {
                        log::error!("{err}");
                    }
                    if let Err(err) = write!(input.out,
                        "\x1b[J{}{} {}{}{}\n\r",
                        if STEEL_CONFIG.log.as_ref().is_some_and(|l| l.time) {
                            let time: chrono::DateTime<Utc> = time::SystemTime::now().into();
                            format!("{} ", time.format("%T:%3f"))
                        } else {
                            String::new()
                        },
                        lvl,
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
                    if let Err(err) = input.cursor_to((0, 0), pos) {
                        log::error!("{err}");
                    }
                    let length = input.length;
                    let pos = input.pos;
                    if let Err(err) = input.rewrite_input(length, pos) {
                        log::error!("{err}");
                    }
                    input.out.flush().ok();
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
        use crossterm::terminal::{Clear, ClearType};

        let mut input = self.input.write().await;
        input.spawn_display.rendered = true;
        let pos = input.get_current_pos();
        input.cursor_to(pos, (0, 0))?;
        write!(input.out, "\r{}", Clear(ClearType::FromCursorDown))?;
        for _ in 0..=DISPLAY_RADIUS {
            writeln!(input.out)?;
        }
        input.cursor_to((0, 0), pos)?;
        input.out.flush()?;
        input.rewrite_current_input()?;
        Ok(())
    }
    /// Ends the spawn display cleaning the screen
    pub async fn deactivate_spawn_display(&self) {
        use crate::spawn_progress::DISPLAY_RADIUS;
        use crossterm::terminal::{Clear, ClearType};

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
    pub async fn update_spawn_grid(&self, grid: &Grid, should_render: bool) {
        let mut input = self.input.write().await;
        input.spawn_display.set_grid(grid);
        if should_render {
            let _ = input.render_current_spawn();
        }
    }
}

impl<S: Subscriber> Layer<S> for LoggerLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut data = LogData::new();
        event.record(&mut data);

        self.0.log(Level::Tracing(*event.metadata().level()), data);
    }
}
