use crate::SERVER;
use crate::logger::{CommandLogger, Input};
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use std::{
    fmt::Write as _,
    io::{self, Result, Write},
    os::fd::AsFd,
    sync::Arc,
};
use steel_core::command::sender::CommandSender;
use termion::cursor::{BlinkingBar, BlinkingBlock};
use termion::{
    event::{Event, Key},
    input::TermRead,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::spawn_blocking;
use tokio_util::sync::CancellationToken;

pub enum ExtendedKey {
    Termion(Key),
    ShiftHome,
    ShiftEnd,
    String(String),
}

impl CommandLogger {
    pub async fn input_main(self: Arc<Self>, token: CancellationToken) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        Self::input_receiver(tx, token.clone());
        self.input_key(rx, token).await?;
        Ok(())
    }

    fn input_receiver(tx: UnboundedSender<ExtendedKey>, token: CancellationToken) {
        spawn_blocking(move || {
            let stdin = io::stdin();
            let stdin_fd = stdin.as_fd();

            // Set stdin to non-blocking mode
            let mut flags =
                fcntl(stdin_fd, FcntlArg::F_GETFL).expect("Couldn't set non-blocking mode");
            flags |= OFlag::O_NONBLOCK.bits();
            fcntl(
                stdin_fd,
                FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags)),
            )
            .expect("Couldn't set non-blocking mode");
            let mut reader = stdin.events();
            let special_chars = ['\n', '\t'];
            let mut string = String::new();

            loop {
                let mut added_char = false;
                if token.is_cancelled() {
                    break;
                }
                if let Some(Ok(event)) = reader.next() {
                    match event {
                        Event::Key(Key::Char(char)) if !special_chars.contains(&char) => {
                            added_char = true;
                            write!(string, "{char}").ok();
                        }
                        Event::Key(key) => {
                            tx.send(ExtendedKey::Termion(key)).ok();
                        }
                        Event::Unsupported(bytes) => {
                            match bytes.as_slice() {
                                [27, 91, 49, 59, 50, 72] => {
                                    tx.send(ExtendedKey::ShiftHome).ok(); // \x1b[1;2H
                                }
                                [27, 91, 49, 59, 50, 70] => {
                                    tx.send(ExtendedKey::ShiftEnd).ok(); // \x1b[1;2F
                                }
                                _ => (),
                            }
                        }
                        Event::Mouse(_) => (),
                    }
                }
                if !added_char && !string.is_empty() {
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
                    let mut input = self.input.write().await;
                    match key {
                        ExtendedKey::Termion(key) => match key {
                            Key::Char('\n') => {
                                if input.is_empty() {
                                    continue;
                                }
                                let message = input.text.clone();
                                input.add_history();
                                input.reset()?;
                                input.history.pos = 0;
                                drop(input);
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
                            Key::Char('\t') => {
                                if input.completion.enabled {
                                    input.completion.enabled = false;
                                    input.completion.selected = 0;
                                    let completion = input.completion.completed.clone();
                                    if input.replace {
                                        input.replace(completion)?;
                                    } else {
                                        input.push(completion)?;
                                    }
                                    input.completion.completed = String::new();
                                } else {
                                    input.completion.enabled = true;
                                    let pos = input.pos;
                                    input.update_suggestion_list(pos);
                                    input.rewrite_current_input()?;
                                }
                                continue;
                            }
                            Key::Backspace => {
                                if input.selection.is_active() {
                                    input.delete_selection()?;
                                    continue;
                                }
                                input.pop_back()?;
                            }
                            Key::Delete => {
                                if input.selection.is_active() {
                                    input.delete_selection()?;
                                    continue;
                                }
                                input.pop_front()?;
                            }
                            Key::Left => {
                                if input.selection.is_active() {
                                    let length = input.length;
                                    let pos = input.selection.get_range().start;
                                    input.selection.clear();
                                    input.update_suggestion_list(pos);
                                    input.rewrite_input(length, pos)?;
                                    continue;
                                }
                                if !input.is_at_start() {
                                    let from = input.get_current_pos();
                                    let pos = input.pos - 1;
                                    let to = Input::get_pos(pos);
                                    input.cursor_to(from, to)?;
                                    input.pos -= 1;
                                    input.update_suggestion_list(pos);
                                }
                            }
                            Key::Right => {
                                if input.selection.is_active() {
                                    let length = input.length;
                                    let pos = input.selection.get_range().end;
                                    input.selection.clear();
                                    input.update_suggestion_list(pos);
                                    input.rewrite_input(length, pos)?;
                                    continue;
                                }
                                if !input.is_at_end() {
                                    let from = input.get_current_pos();
                                    let pos = input.pos + 1;
                                    let to = Input::get_pos(pos);
                                    input.cursor_to(from, to)?;
                                    input.pos += 1;
                                    input.update_suggestion_list(pos);
                                }
                            }
                            Key::Up | Key::Ctrl('p') => {
                                if input.completion.enabled {
                                    input.update_completion(-1)?;
                                } else {
                                    input.selection.clear();
                                    input.move_history(1)?;
                                }
                                continue;
                            }
                            Key::Down | Key::Ctrl('n') => {
                                if input.completion.enabled {
                                    input.update_completion(1)?;
                                } else if input.history.pos != 0 {
                                    input.selection.clear();
                                    input.move_history(-1)?;
                                }
                                continue;
                            }
                            Key::Ctrl('c') => {
                                if input.selection.is_active() {
                                    copy_to_clipboard(&mut input);
                                    continue;
                                }
                                input.cancel_token.cancel();
                            }
                            Key::Ctrl('q') => {
                                input.cancel_token.cancel();
                            }
                            Key::Ctrl('x') => {
                                if input.selection.is_active() {
                                    copy_to_clipboard(&mut input);
                                    input.delete_selection()?;
                                }
                                continue;
                            }
                            Key::End => {
                                if input.selection.is_active() {
                                    let length = input.length;
                                    input.selection.clear();
                                    input.update_suggestion_list(length);
                                    input.rewrite_input(length, length)?;
                                    continue;
                                }
                                if !input.is_at_end() {
                                    let from = input.get_current_pos();
                                    let to = input.get_end();
                                    input.cursor_to(from, to)?;
                                    input.pos = input.length;
                                    let pos = input.length;
                                    input.update_suggestion_list(pos);
                                }
                            }
                            Key::Home => {
                                if input.selection.is_active() {
                                    let length = input.length;
                                    input.selection.clear();
                                    input.update_suggestion_list(0);
                                    input.rewrite_input(length, 0)?;
                                    continue;
                                }
                                if !input.is_at_start() {
                                    let from = input.get_current_pos();
                                    input.cursor_to(from, (0, 2))?;
                                    input.pos = 0;
                                    input.update_suggestion_list(0);
                                }
                            }
                            Key::ShiftLeft => {
                                if !input.is_at_start() {
                                    if !input.selection.is_active() {
                                        let current_pos = input.pos;
                                        input.selection.start_at(current_pos);
                                    }
                                    let from = input.get_current_pos();
                                    let to = Input::get_pos(input.pos - 1);
                                    input.cursor_to(from, to)?;
                                    input.pos -= 1;
                                    let new_pos = input.pos;
                                    input.selection.extend(new_pos);
                                    let length = input.length;
                                    input.update_suggestion_list(new_pos);
                                    input.rewrite_input(length, new_pos)?;
                                }
                                continue;
                            }
                            Key::ShiftRight => {
                                if !input.is_at_end() {
                                    if !input.selection.is_active() {
                                        let current_pos = input.pos;
                                        input.selection.start_at(current_pos);
                                    }
                                    let from = input.get_current_pos();
                                    let to = Input::get_pos(input.pos + 1);
                                    input.cursor_to(from, to)?;
                                    input.pos += 1;
                                    let new_pos = input.pos;
                                    input.selection.extend(new_pos);
                                    let length = input.length;
                                    input.update_suggestion_list(new_pos);
                                    input.rewrite_input(length, new_pos)?;
                                }
                            }
                            Key::Ctrl('a') => {
                                // Select all text
                                if input.length > 0 {
                                    let len = input.length;
                                    input.selection.set(0, len);
                                    input.update_suggestion_list(len);
                                    input.rewrite_input(len, len)?;
                                }
                            }
                            Key::Insert => {
                                input.replace = !input.replace;
                                if input.replace {
                                    write!(input.out, "{BlinkingBlock}")?;
                                } else {
                                    write!(input.out, "{BlinkingBar}")?;
                                }
                            }
                            _ => (),
                        },
                        ExtendedKey::ShiftHome => {
                            // Select all previous text
                            if !input.is_at_start() {
                                let len = input.length;
                                let end = if input.selection.is_active() {
                                    input.selection.get_range().end
                                } else {
                                    input.pos
                                };
                                input.selection.set(0, end + 1);
                                input.update_suggestion_list(0);
                                input.rewrite_input(len, 0)?;
                                continue;
                            }
                        }
                        ExtendedKey::ShiftEnd => {
                            // Select all text next
                            if !input.is_at_end() {
                                let len = input.length;
                                let start = if input.selection.is_active() {
                                    input.selection.get_range().start
                                } else {
                                    input.pos
                                };
                                input.selection.set(start, len);
                                input.update_suggestion_list(len);
                                input.rewrite_input(len, len)?;
                                continue;
                            }
                        }
                        ExtendedKey::String(string) => {
                            if string.chars().any(|c| c == ' ') {
                                input.completion.selected = 0;
                            }
                            // Delete selection if active
                            if input.selection.is_active() {
                                input.delete_selection()?;
                            }
                            if input.replace {
                                input.replace(string)?;
                            } else {
                                input.push(string)?;
                            }
                            continue;
                        }
                    }
                    if input.completion.enabled {
                        input.update_completion(0)?;
                    }
                    input.out.flush()?;
                }
                () = token.cancelled() => {
                    let mut input = self.input.write().await;
                    input.completion.enabled = false;
                    if !input.is_at_end() {
                        let from = input.get_current_pos();
                        let to = input.get_end();
                        input.cursor_to(from, to)?;
                    }
                    write!(input.out, "\x1b[J")?;
                    input.save_history().await?;
                    input.out.suspend_raw_mode()?;
                    input.out.flush()?;
                    break;
                },
            }
        }
        Ok(())
    }
}

fn copy_to_clipboard(input: &mut Input) -> Option<()> {
    let range = input.selection.get_range();
    let start = range.start;
    let end = range.end;

    // Find byte positions for the character indices
    let char_indices: Vec<(usize, char)> = input.text.char_indices().collect();

    let byte_start = char_indices[start].0;
    let byte_end = if end < char_indices.len() {
        char_indices[end].0
    } else {
        input.text.len()
    };
    let text = &input.text[byte_start..byte_end];
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(cb) => cb,
        Err(err) => {
            log::error!("{err}");
            return None;
        }
    };
    clipboard.set_text(text).ok()?;
    Some(())
}
