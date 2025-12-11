
use anyhow::{Result};
use ratatui::Terminal;
use std::io;
use tokio::sync::mpsc;
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};

mod app;
mod chat_screen;
mod signin_screen;
mod history_screen;
mod colour_screen;
mod key_handler;

use app::{App, ChatOutcome, Screen};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Channel for streaming text
    let (tx, mut rx) = mpsc::channel::<String>(100);

    let mut app = App::new();

    loop {
        terminal.draw(|frame| {
            match app.screen {
                Screen::ColourSelection => colour_screen::render_colour(frame, &mut app),
                Screen::SignIn => signin_screen::render_signin(frame, &app),
                Screen::History => history_screen::render_history(frame, &app),
                Screen::Chat => chat_screen::render_chat(frame, &app),
            }
        })?;

        match key_handler::key_handler(&mut app, tx.clone(), &mut rx).await? {
            ChatOutcome::Contiune => {}
            ChatOutcome::Quit => break,
        }
    }
 
    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    // ratatui::restore();

    Ok(())
}
