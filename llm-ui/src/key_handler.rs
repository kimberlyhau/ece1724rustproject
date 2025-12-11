use crate::app::{App, ChatOutcome, Screen, InputMode};
use crossterm::{
    event::{self, Event, KeyCode,KeyEventKind},
};
use anyhow::{Result};
use tokio::time::{ Duration};
use tokio::sync::mpsc;
use ratatui::widgets::ListState;

use crate::app::OPTIONS as options;

pub async fn key_handler(app: &mut App, tx: mpsc::Sender<String>, rx: &mut mpsc::Receiver<String>) -> Result<ChatOutcome> {
    process_incoming_message(app, rx);
    
    if event::poll(Duration::from_millis(1))? {
        if let Event::Key(key) = event::read()? {
            match app.screen {
                Screen::SignIn => {
                    if key.code == KeyCode::Char('q') {
                        return Ok(ChatOutcome::Quit);
                    }
                    match key.code {
                        KeyCode::Enter => {
                            app.set_profile(tx.clone());
                        },
                        KeyCode::Char(to_insert) => app.enter_char(to_insert),
                        KeyCode::Backspace => app.delete_char(),
                        KeyCode::Left => app.move_cursor_left(),
                        KeyCode::Right => app.move_cursor_right(),
                        KeyCode::Esc => return Ok(ChatOutcome::Quit),
                        _ => {}
                    }
                }
                Screen::History => {
                    if key.code == KeyCode::Char('q') {
                        return Ok(ChatOutcome::Quit);
                    }
                    match key.code {
                        KeyCode::Enter => {
                            if app.input == "0" {
                                app.start_new_chat();
                            } else {
                                app.fetch_chat(tx.clone());
                            }
                        },
                        KeyCode::Char(to_insert) => app.enter_char(to_insert),
                        KeyCode::Backspace => app.delete_char(),
                        KeyCode::Left => app.move_cursor_left(),
                        KeyCode::Right => app.move_cursor_right(),
                        _ => {}
                    }
                }
                Screen::Chat | Screen::ColourSelection => {
                    let mut scroll_offset = app.scroll_offset.clone();
                    if key.code == KeyCode::Char('q') {
                        return Ok(ChatOutcome::Quit);
                    }
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('e') => {
                                app.input_mode = InputMode::Editing;
                            }
                            KeyCode::Char('q') => {
                                return Ok(ChatOutcome::Quit);
                            }
                            KeyCode::Char('c') => {
                                app.selected_flags = vec![false; 14];
                                app.state = ListState::default();
                                app.state.select(Some(0));
                                app.user_colour_pick = None;
                                app.input_mode = InputMode::ColourSelection;
                                app.screen = crate::app::Screen::ColourSelection;
                            }
                            KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                            KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                            KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                            KeyCode::PageDown => scroll_offset = scroll_offset.saturating_add(5),
                            _ => {}
                        },
                        InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => {app.submit_message(tx.clone());
                            },
                            KeyCode::Char(to_insert) => app.enter_char(to_insert),
                            KeyCode::Backspace => app.delete_char(),
                            KeyCode::Left => app.move_cursor_left(),
                            KeyCode::Right => app.move_cursor_right(),
                            KeyCode::Esc => app.input_mode = InputMode::Normal,
                            _ => {}
                        },
                        InputMode::Editing => {},
                        InputMode::Processing => match key.code {
                            KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                            KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                            KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                            KeyCode::PageDown => scroll_offset = scroll_offset.saturating_add(5),
                            KeyCode::Char('q') => {
                                return Ok(ChatOutcome::Quit);
                            }   
                            _ => {}
                        },
                        InputMode::ColourSelection => match key.code {
                            KeyCode::Char('q') => {
                                return Ok(ChatOutcome::Quit);
                            },
                            KeyCode::Up => {
                                if let Some(mut i) = app.state.selected() {
                                    i = previous_selectable(&app.selected_flags, i);
                                    app.state.select(Some(i));
                                }
                            }
                            KeyCode::Down => {
                                if let Some(mut i) = app.state.selected() {
                                    i = next_selectable(&app.selected_flags, i);
                                    app.state.select(Some(i));
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(i) = app.state.selected() {
                                    if !app.selected_flags[i] {
                                        // println!("Selected: {}", options[i]);
                                        app.selected_flags[i] = true;
                                        let count = app.selected_flags.iter().filter(|&n| *n == true).count();
                                        if count==2{
                                            app.llm_colour = options[i];
                                            if let Some(user_colour_picked) = app.user_colour_pick {
                                                app.user_colour=user_colour_picked;
                                            }
                                            app.input_mode = InputMode::Normal;
                                            app.screen = crate::app::Screen::Chat;
                                        }else if count==1{
                                            app.user_colour_pick=Some(options[i]);
                                        }
                                        // Move to next selectable
                                        let next = next_selectable(&app.selected_flags, i);
                                        app.state.select(Some(next));
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                                app.screen = crate::app::Screen::Chat;
                            }
                            _ => {}
                        }
                    }
                    app.scroll_offset = scroll_offset;
                }
            }
        }   
    }
    
    tokio::time::sleep(Duration::from_millis(5)).await;
    Ok(ChatOutcome::Contiune)
}

fn next_selectable(selected_flags: &Vec<bool>, mut index: usize) -> usize {
    let len = selected_flags.len();
    for _ in 0..len {
        index = (index + 1) % len;
        if !selected_flags[index] {
            return index;
        }
    }
    index // fallback
}

fn previous_selectable(selected_flags: &Vec<bool>, mut index: usize) -> usize {
    let len = selected_flags.len();
    for _ in 0..len {
        if index == 0 {
            index = len - 1;
        } else {
            index -= 1;
        }
        if !selected_flags[index] {
            return index;
        }
    }
    index
}

fn process_incoming_message(app: &mut App, rx: &mut mpsc::Receiver<String>) {
    while let Ok(msg) = rx.try_recv() {
        if msg == "Thread work complete!" {
            match app.screen {
                Screen::SignIn => {
                    app.screen = Screen::History;
                }
                Screen::History => {
                    app.screen = Screen::Chat;
                }
                Screen::Chat => {
                    app.input_mode = InputMode::Normal;
                    app.llm_messages.push(app.receiving.clone());
                    app.receiving.clear();
                }
                _ => {}
            }
            continue;
        }

        match app.screen {
            Screen::SignIn => {
                app.llm_messages.push(msg);
            }
            Screen::History => {
                let start_index = match msg.find("[") {
                    Some(i) => i,
                    None => break,
                };
                let end_index = match msg.find("]:") {
                    Some(i) => i,
                    None => break,
                };

                let msg_id = &msg[start_index + 1 .. end_index];
                let id: u32 = msg_id.trim().parse().unwrap_or(0);

                let clean = msg[end_index + 2 ..].to_string();

                if id % 2 == 1 {
                    app.messages.push(clean);
                } else {
                    app.llm_messages.push(clean);
                }
            }
            Screen::Chat => {
                app.receiving.push_str(&msg);
            }
            Screen::ColourSelection => {
                // nothing to do
            }
        }
    }
}