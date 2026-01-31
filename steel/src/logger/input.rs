use crate::SERVER;
use crate::logger::history::History;
use crate::logger::output::Output;
use crate::logger::{CommandLogger, LogState};
use crossterm::{
    clipboard::CopyToClipboard,
    cursor::SetCursorStyle::{BlinkingBar, BlinkingBlock, DefaultUserShape},
    event::{Event, KeyCode, KeyEvent, KeyModifiers, poll, read},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::time::Duration;
use std::{
    fmt::Write as _,
    io::{Result, Write},
    sync::Arc,
};
use steel_core::command::sender::CommandSender;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::spawn_blocking;
use tokio_util::sync::CancellationToken;

enum ExtendedKey {
    Generic(KeyEvent),
    Ctrl(char),
    String(String),
}

impl CommandLogger {
    /// Main entry of the input process
    pub async fn input_main(self: Arc<Self>, token: CancellationToken) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        enable_raw_mode()?;
        Self::input_receiver(tx, token.clone());
        self.input_key(rx, token).await?;
        Ok(())
    }

    fn input_receiver(tx: UnboundedSender<ExtendedKey>, token: CancellationToken) {
        spawn_blocking(move || {
            let mut string = String::new();
            loop {
                if token.is_cancelled() {
                    break;
                }

                if let Ok(true) = poll(Duration::from_secs(0)) {
                    let event = read().expect("Event bug; Cannot read event.");
                    if let Event::Key(key) = event {
                        if let KeyCode::Char(char) = key.code {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                tx.send(ExtendedKey::Ctrl(char)).ok();
                            } else {
                                write!(string, "{char}").ok();
                            }
                            continue;
                        }
                        tx.send(ExtendedKey::Generic(key)).ok();
                    }
                }
                if !string.is_empty() {
                    tx.send(ExtendedKey::String(string.clone())).ok();
                    string = String::new();
                }
            }
        });
    }

    #[allow(clippy::too_many_lines)]
    async fn input_key(
        self: Arc<Self>,
        mut rx: UnboundedReceiver<ExtendedKey>,
        token: CancellationToken,
    ) -> Result<()> {
        loop {
            tokio::select! {
                Some(key) = rx.recv() => {
                    let mut lock = self.input.write().await;
                    let mut state = &mut lock as &mut LogState;
                    match key {
                        ExtendedKey::Generic(key) => match key.code {
                            KeyCode::Enter => {
                                if state.out.is_empty() {
                                    continue;
                                }
                                let message = state.out.text.clone();
                                state.history.push(&mut state.out);
                                state.reset()?;
                                state.history.pos = 0;
                                drop(lock);
                                steel_utils::console!("{}", message);
                                if let Some(server) = SERVER.get() {
                                    server.command_dispatcher.read().handle_command(
                                        CommandSender::Console,
                                        message,
                                        server,
                                    );
                                }
                                continue;
                            }
                            KeyCode::Tab => {
                                if state.completion.enabled {
                                    state.completion.enabled = false;
                                    state.completion.selected = 0;
                                    let completion = state.completion.completed.clone();
                                    if state.out.replace {
                                        state.replace(completion)?;
                                    } else {
                                        state.push(completion)?;
                                    }
                                    state.completion.completed = String::new();
                                } else {
                                    state.completion.enabled = true;
                                    let pos = state.out.pos;
                                    state.completion.update(&mut state.out, pos);
                                    state.rewrite_current_input()?;
                                }
                                continue;
                            }
                            KeyCode::Backspace => {
                                if state.selection.is_active() {
                                    state.delete_selection()?;
                                    continue;
                                }
                                state.pop_back()?;
                            }
                            KeyCode::Delete => {
                                if state.selection.is_active() {
                                    state.delete_selection()?;
                                    continue;
                                }
                                state.pop_front()?;
                            }
                            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                                if !state.out.is_at_start() {
                                    if !state.selection.is_active() {
                                        let current_pos = state.out.pos;
                                        state.selection.start_at(current_pos);
                                    }
                                    let from = state.out.get_current_pos();
                                    let to = Output::get_pos(state.out.pos - 1);
                                    state.out.cursor_to(from, to)?;
                                    state.out.pos -= 1;
                                    let new_pos = state.out.pos;
                                    state.selection.extend(new_pos);
                                    let length = state.out.length;
                                    state.completion.update(&mut state.out, new_pos);
                                    state.rewrite_input(length, new_pos)?;
                                }
                                continue;
                            }
                            KeyCode::Left => {
                                if state.selection.is_active() {
                                    let length = state.out.length;
                                    let pos = state.selection.get_range().start;
                                    state.selection.clear();
                                    state.completion.update(&mut state.out, pos);
                                    state.rewrite_input(length, pos)?;
                                    continue;
                                }
                                if !state.out.is_at_start() {
                                    let from = state.out.get_current_pos();
                                    let pos = state.out.pos - 1;
                                    let to = Output::get_pos(pos);
                                    state.out.cursor_to(from, to)?;
                                    state.out.pos -= 1;
                                    state.completion.update(&mut state.out, pos);
                                }
                            }
                            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                                if !state.out.is_at_end() {
                                    if !state.selection.is_active() {
                                        let current_pos = state.out.pos;
                                        state.selection.start_at(current_pos);
                                    }
                                    let from = state.out.get_current_pos();
                                    let to = Output::get_pos(state.out.pos + 1);
                                    state.out.cursor_to(from, to)?;
                                    state.out.pos += 1;
                                    let new_pos = state.out.pos;
                                    state.selection.extend(new_pos);
                                    let length = state.out.length;
                                    state.completion.update(&mut state.out, new_pos);
                                    state.rewrite_input(length, new_pos)?;
                                }
                            }
                            KeyCode::Right => {
                                if state.selection.is_active() {
                                    let length = state.out.length;
                                    let pos = state.selection.get_range().end;
                                    state.selection.clear();
                                    state.completion.update(&mut state.out, pos);
                                    state.rewrite_input(length, pos)?;
                                    continue;
                                }
                                if !state.out.is_at_end() {
                                    let from = state.out.get_current_pos();
                                    let pos = state.out.pos + 1;
                                    let to = Output::get_pos(pos);
                                    state.out.cursor_to(from, to)?;
                                    state.out.pos += 1;
                                    state.completion.update(&mut state.out, pos);
                                }
                            }
                            KeyCode::Up => {
                                previous(&mut state)?;
                                continue;
                            }
                            KeyCode::Down => {
                                next(&mut state)?;
                                continue;
                            }
                            KeyCode::End if key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Select all text next
                                if state.out.is_at_end() {
                                    continue;
                                }
                                let len = state.out.length;
                                let start = if state.selection.is_active() {
                                    state.selection.get_range().start
                                } else {
                                    state.out.pos
                                };
                                state.selection.set(start, len);
                                state.completion.update(&mut state.out, len);
                                state.rewrite_input(len, len)?;
                                continue;
                            }
                            KeyCode::End => {
                                if state.selection.is_active() {
                                    let length = state.out.length;
                                    state.selection.clear();
                                    state.completion.update(&mut state.out, length);
                                    state.rewrite_input(length, length)?;
                                    continue;
                                }
                                if !state.out.is_at_end() {
                                    let from = state.out.get_current_pos();
                                    let to = state.out.get_end();
                                    state.out.cursor_to(from, to)?;
                                    state.out.pos = state.out.length;
                                    let pos = state.out.length;
                                    state.completion.update(&mut state.out, pos);
                                }
                            }
                            KeyCode::Home if key.modifiers.contains(KeyModifiers::SHIFT) =>{
                                // Select all previous text
                                if state.out.is_at_start() {
                                    continue;
                                }
                                let len = state.out.length;
                                let end = if state.selection.is_active() {
                                    state.selection.get_range().end
                                } else {
                                    state.out.pos
                                };
                                state.selection.set(0, end + 1);
                                state.completion.update(&mut state.out, 0);
                                state.rewrite_input(len, 0)?;
                                continue;
                            }
                            KeyCode::Home => {
                                if state.selection.is_active() {
                                    let length = state.out.length;
                                    state.selection.clear();
                                    state.completion.update(&mut state.out, 0);
                                    state.rewrite_input(length, 0)?;
                                    continue;
                                }
                                if !state.out.is_at_start() {
                                    let from = state.out.get_current_pos();
                                    state.out.cursor_to(from, (0, 2))?;
                                    state.out.pos = 0;
                                    state.completion.update(&mut state.out, 0);
                                }
                            }
                            KeyCode::Insert => {
                                state.out.replace = !state.out.replace;
                                if state.out.replace {
                                    write!(state.out, "{BlinkingBlock}")?;
                                } else {
                                    write!(state.out, "{BlinkingBar}")?;
                                }
                                continue;
                            }
                            _ => (),
                        },
                        ExtendedKey::Ctrl(char) => {
                            match char {
                                'c' => {
                                    if state.selection.is_active() {
                                        copy_to_clipboard(&mut state);
                                        continue;
                                    }
                                    state.cancel_token.cancel();
                                }
                                'q' => {
                                    state.cancel_token.cancel();
                                }
                                'x' => {
                                    if state.selection.is_active() {
                                        copy_to_clipboard(&mut state);
                                        state.delete_selection()?;
                                    }
                                    continue;
                                }
                                'a' => {
                                    // Select all text
                                    if state.out.length > 0 {
                                        let len = state.out.length;
                                        state.selection.set(0, len);
                                        state.completion.update(&mut state.out, len);
                                        state.rewrite_input(len, len)?;
                                    }
                                    continue;
                                }
                                'p' => {
                                    previous(&mut state)?;
                                    continue;
                                }
                                'n' => {
                                    next(&mut state)?;
                                    continue;
                                }
                                _ => ()
                            }
                        }
                        ExtendedKey::String(string) => {
                            if string.chars().any(|c| c == ' ') {
                                state.completion.selected = 0;
                            }
                            // Delete selection if active
                            if state.selection.is_active() {
                                state.delete_selection()?;
                            }
                            if state.out.replace {
                                state.replace(string)?;
                            } else {
                                state.push(string)?;
                            }
                            continue;
                        }
                    }
                    if state.completion.enabled {
                        state.completion.rewrite(&mut state.out, 0)?;
                    }
                    state.out.flush()?;
                }
                () = token.cancelled() => {
                    let mut state = self.input.write().await;
                    state.completion.enabled = false;
                    if !state.out.is_at_end() {
                        let from = state.out.get_current_pos();
                        let to = state.out.get_end();
                        state.out.cursor_to(from, to)?;
                    }
                    write!(state.out, "{}{DefaultUserShape}", Clear(ClearType::FromCursorDown))?;
                    state.history.save().await?;
                    state.out.flush()?;
                    disable_raw_mode()?;
                    break;
                },
            }
        }
        Ok(())
    }
}

fn copy_to_clipboard(input: &mut LogState) -> Option<()> {
    let range = input.selection.get_range();
    let start = range.start;
    let end = range.end;

    // Find byte positions for the character indices
    let char_indices: Vec<(usize, char)> = input.out.text.char_indices().collect();

    let byte_start = char_indices[start].0;
    let byte_end = if end < char_indices.len() {
        char_indices[end].0
    } else {
        input.out.text.len()
    };
    let text = input.out.text[byte_start..byte_end].to_string();
    if let Err(err) = execute!(input.out, CopyToClipboard::to_clipboard_from(text)) {
        log::error!("{err}");
        return None;
    }
    Some(())
}

fn previous(state: &mut LogState) -> Result<()> {
    if state.completion.enabled {
        state.completion.rewrite(&mut state.out, -1)?;
    } else {
        state.selection.clear();
        History::update(state, 1)?;
    }
    Ok(())
}
fn next(state: &mut LogState) -> Result<()> {
    if state.completion.enabled {
        state.completion.rewrite(&mut state.out, 1)?;
    } else if state.history.pos != 0 {
        state.selection.clear();
        History::update(state, -1)?;
    }
    Ok(())
}
